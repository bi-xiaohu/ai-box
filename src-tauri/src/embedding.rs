use crate::llm::openai::OpenAiConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// Generate embeddings for a list of texts using the OpenAI-compatible API
pub async fn generate_embeddings(
    config: &OpenAiConfig,
    texts: &[String],
    model: &str,
) -> Result<Vec<Vec<f32>>, String> {
    let client = Client::new();

    let body = EmbeddingRequest {
        model: model.to_string(),
        input: texts.to_vec(),
    };

    let mut req = client
        .post(format!("{}/embeddings", config.base_url))
        .header("Content-Type", "application/json")
        .json(&body);

    if !config.api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", config.api_key));
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Embedding API error: {}", text));
    }

    let data: EmbeddingResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(data.data.into_iter().map(|d| d.embedding).collect())
}

/// Cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Search for the most relevant chunks given a query embedding
pub fn search_similar(
    query_embedding: &[f32],
    chunk_embeddings: &[(String, Vec<f32>)], // (chunk_id, embedding)
    top_k: usize,
) -> Vec<(String, f32)> {
    let mut scored: Vec<(String, f32)> = chunk_embeddings
        .iter()
        .map(|(id, emb)| (id.clone(), cosine_similarity(query_embedding, emb)))
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);
    scored
}

/// Serialize embedding to bytes for SQLite BLOB storage
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

/// Deserialize embedding from SQLite BLOB bytes
pub fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&a, &a) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn test_embedding_roundtrip() {
        let emb = vec![0.1, 0.2, -0.3, 0.4];
        let bytes = embedding_to_bytes(&emb);
        let back = bytes_to_embedding(&bytes);
        assert_eq!(emb, back);
    }
}
