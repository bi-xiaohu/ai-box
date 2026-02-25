use crate::db::models::{Conversation, Message};
use crate::db::Database;
use crate::llm::{ChatMessage, ChatRequest, Provider, StreamChunk};
use serde::Serialize;
use tauri::{Emitter, State};

#[derive(Clone, Serialize)]
struct ChatStreamEvent {
    conversation_id: String,
    delta: String,
    done: bool,
}

/// Resolve an LLM provider from a model string like "openai/gpt-4o", "claude/...", "ollama/..."
fn resolve_provider(model: &str, db: &Database) -> Result<(Provider, String), String> {
    if let Some(model_id) = model.strip_prefix("ollama/") {
        let host = db
            .get_setting("ollama_host")
            .ok()
            .flatten()
            .unwrap_or_else(|| "http://localhost:11434".to_string());
        Ok((Provider::ollama(host), model_id.to_string()))
    } else if let Some(model_id) = model.strip_prefix("claude/") {
        let api_key = db
            .get_setting("claude_api_key")
            .ok()
            .flatten()
            .ok_or("Claude API key not configured")?;
        let base_url = db
            .get_setting("claude_base_url")
            .ok()
            .flatten()
            .unwrap_or_else(|| "https://api.anthropic.com".to_string());
        Ok((
            Provider::Claude(crate::llm::claude::ClaudeConfig { api_key, base_url }),
            model_id.to_string(),
        ))
    } else {
        let model_id = model.strip_prefix("openai/").unwrap_or(model);
        let api_key = db
            .get_setting("openai_api_key")
            .ok()
            .flatten()
            .ok_or("OpenAI API key not configured")?;
        let base_url = db
            .get_setting("openai_base_url")
            .ok()
            .flatten()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        Ok((
            Provider::OpenAi(crate::llm::openai::OpenAiConfig { api_key, base_url }),
            model_id.to_string(),
        ))
    }
}

#[tauri::command]
pub fn create_conversation(
    db: State<'_, Database>,
    title: String,
    model: Option<String>,
) -> Result<Conversation, String> {
    db.create_conversation(&title, model.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_conversations(db: State<'_, Database>) -> Result<Vec<Conversation>, String> {
    db.list_conversations().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_conversation(db: State<'_, Database>, id: String) -> Result<(), String> {
    db.delete_conversation(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_conversation(
    db: State<'_, Database>,
    id: String,
    title: String,
) -> Result<(), String> {
    db.update_conversation_title(&id, &title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_messages(
    db: State<'_, Database>,
    conversation_id: String,
) -> Result<Vec<Message>, String> {
    db.get_messages(&conversation_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_message(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    conversation_id: String,
    content: String,
    model: String,
) -> Result<Message, String> {
    // 1. Save user message
    db.add_message(&conversation_id, "user", &content)
        .map_err(|e| e.to_string())?;

    // 2. Load full conversation history for context
    let messages = db
        .get_messages(&conversation_id)
        .map_err(|e| e.to_string())?;
    let chat_messages: Vec<ChatMessage> = messages
        .iter()
        .map(|m| ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    // 3. Resolve provider
    let (provider, model_id) = resolve_provider(&model, &db)?;

    // 4. Stream response, emitting events to frontend
    let conv_id = conversation_id.clone();
    let request = ChatRequest {
        messages: chat_messages,
        model: model_id,
        stream: true,
    };

    let full_content = provider
        .chat_stream(&request, |chunk: StreamChunk| {
            let _ = app.emit(
                "chat-stream",
                ChatStreamEvent {
                    conversation_id: conv_id.clone(),
                    delta: chunk.delta,
                    done: chunk.done,
                },
            );
        })
        .await
        .map_err(|e| e.to_string())?;

    // 5. Save assistant message
    let assistant_msg = db
        .add_message(&conversation_id, "assistant", &full_content)
        .map_err(|e| e.to_string())?;

    Ok(assistant_msg)
}
