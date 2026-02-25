use crate::db::models::Document;
use crate::db::Database;
use crate::doc_processor;
use crate::embedding::{
    self, bytes_to_embedding, embedding_to_bytes, generate_embeddings, search_similar,
};
use crate::llm::openai::OpenAiConfig;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub id: String,
    pub content: String,
    pub chunk_index: i32,
    pub score: Option<f32>,
}

#[tauri::command]
pub fn list_documents(db: State<'_, Database>) -> Result<Vec<Document>, String> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, filename, file_type, file_path, file_size, created_at FROM documents ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Document {
                id: row.get(0)?,
                filename: row.get(1)?,
                file_type: row.get(2)?,
                file_path: row.get(3)?,
                file_size: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upload_document(
    db: State<'_, Database>,
    file_path: String,
) -> Result<Document, String> {
    let path = Path::new(&file_path);
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let file_size = std::fs::metadata(path)
        .map(|m| m.len() as i64)
        .ok();

    // Parse file content
    let parsed = doc_processor::parse_file(path)?;

    // Chunk the text
    let chunks = doc_processor::chunk_text(&parsed.content, 512, 64);
    if chunks.is_empty() {
        return Err("Document is empty or could not be parsed".into());
    }

    // Save document and chunks to DB (sync block — no await inside)
    let doc_id = uuid::Uuid::new_v4().to_string();
    let (api_key, base_url, chunk_rows) = {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO documents (id, filename, file_type, file_path, file_size) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![doc_id, filename, parsed.file_type, file_path, file_size],
        )
        .map_err(|e| e.to_string())?;

        let mut saved_chunks = Vec::new();
        for (i, chunk_text) in chunks.iter().enumerate() {
            let chunk_id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO chunks (id, document_id, content, chunk_index) VALUES (?1, ?2, ?3, ?4)",
                params![chunk_id, doc_id, chunk_text, i as i32],
            )
            .map_err(|e| e.to_string())?;
            saved_chunks.push((chunk_id, chunk_text.clone()));
        }

        // Read settings while we have the lock
        let api_key = db.get_setting("openai_api_key").ok().flatten();
        let base_url = db
            .get_setting("openai_base_url")
            .ok()
            .flatten()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        (api_key, base_url, saved_chunks)
    }; // lock released here

    // Generate embeddings asynchronously (if API key is available)
    if let Some(api_key) = api_key {
        let config = OpenAiConfig {
            api_key,
            base_url,
        };
        let batch_size = 20;
        for batch in chunk_rows.chunks(batch_size) {
            let texts: Vec<String> = batch.iter().map(|(_, c)| c.clone()).collect();
            match generate_embeddings(&config, &texts, "text-embedding-3-small").await {
                Ok(embeddings) => {
                    let conn = db.conn.lock().unwrap();
                    for ((chunk_id, _), emb) in batch.iter().zip(embeddings.iter()) {
                        let bytes = embedding_to_bytes(emb);
                        conn.execute(
                            "UPDATE chunks SET embedding = ?1 WHERE id = ?2",
                            params![bytes, chunk_id],
                        )
                        .ok();
                    }
                }
                Err(e) => {
                    eprintln!("Embedding generation failed (non-fatal): {}", e);
                    break;
                }
            }
        }
    }

    // Return the created document
    let doc = {
        let conn = db.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, filename, file_type, file_path, file_size, created_at FROM documents WHERE id = ?1",
            params![doc_id],
            |row| {
                Ok(Document {
                    id: row.get(0)?,
                    filename: row.get(1)?,
                    file_type: row.get(2)?,
                    file_path: row.get(3)?,
                    file_size: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
        .map_err(|e| e.to_string())?
    };
    Ok(doc)
}

#[tauri::command]
pub fn delete_document(db: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = db.conn.lock().unwrap();
    conn.execute("DELETE FROM documents WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Search knowledge base for chunks relevant to a query
#[tauri::command]
pub async fn search_knowledge_base(
    db: State<'_, Database>,
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<ChunkInfo>, String> {
    let top_k = top_k.unwrap_or(5);

    // Read settings and chunk data synchronously (before any await)
    let (config, chunk_data) = {
        let api_key = db
            .get_setting("openai_api_key")
            .ok()
            .flatten()
            .ok_or("OpenAI API key required for knowledge base search")?;
        let base_url = db
            .get_setting("openai_base_url")
            .ok()
            .flatten()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let config = OpenAiConfig { api_key, base_url };

        let conn = db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, content, chunk_index, embedding FROM chunks WHERE embedding IS NOT NULL")
            .map_err(|e| e.to_string())?;
        let data: Vec<(String, String, i32, Vec<f32>)> = stmt
            .query_map([], |row| {
                let bytes: Vec<u8> = row.get(3)?;
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    bytes_to_embedding(&bytes),
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        (config, data)
    }; // lock released

    // Generate query embedding (async)
    let query_embeddings =
        generate_embeddings(&config, &[query], "text-embedding-3-small").await?;
    let query_emb = query_embeddings
        .first()
        .ok_or("Failed to generate query embedding")?;

    // Build (id, embedding) pairs for search
    let emb_pairs: Vec<(String, Vec<f32>)> = chunk_data
        .iter()
        .map(|(id, _, _, emb)| (id.clone(), emb.clone()))
        .collect();

    let results = search_similar(query_emb, &emb_pairs, top_k);

    // Map back to ChunkInfo
    let chunks: Vec<ChunkInfo> = results
        .iter()
        .filter_map(|(id, score)| {
            chunk_data.iter().find(|(cid, _, _, _)| cid == id).map(
                |(_, content, idx, _)| ChunkInfo {
                    id: id.clone(),
                    content: content.clone(),
                    chunk_index: *idx,
                    score: Some(*score),
                },
            )
        })
        .collect();

    Ok(chunks)
}
