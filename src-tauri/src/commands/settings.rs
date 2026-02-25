use crate::db::Database;
use crate::llm::ModelInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub openai_api_key: Option<String>,
    pub openai_base_url: Option<String>,
    pub claude_api_key: Option<String>,
    pub claude_base_url: Option<String>,
    pub ollama_host: Option<String>,
    pub default_model: Option<String>,
    pub theme: Option<String>,
}

const SETTING_KEYS: &[&str] = &[
    "openai_api_key",
    "openai_base_url",
    "claude_api_key",
    "claude_base_url",
    "ollama_host",
    "default_model",
    "theme",
];

#[tauri::command]
pub fn get_settings(db: State<'_, Database>) -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();
    for key in SETTING_KEYS {
        if let Some(value) = db.get_setting(key).map_err(|e| e.to_string())? {
            // Mask API keys for display
            if key.ends_with("_api_key") && value.len() > 8 {
                let masked = format!("{}...{}", &value[..4], &value[value.len() - 4..]);
                map.insert(key.to_string(), masked);
            } else {
                map.insert(key.to_string(), value);
            }
        }
    }
    Ok(map)
}

#[tauri::command]
pub fn set_setting(db: State<'_, Database>, key: String, value: String) -> Result<(), String> {
    if !SETTING_KEYS.contains(&key.as_str()) {
        return Err(format!("Unknown setting key: {}", key));
    }
    db.set_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_setting(db: State<'_, Database>, key: String) -> Result<(), String> {
    let conn = db.conn.lock().unwrap();
    conn.execute("DELETE FROM settings WHERE key = ?1", rusqlite::params![key])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_available_models(db: State<'_, Database>) -> Result<Vec<ModelInfo>, String> {
    let mut models = Vec::new();

    // OpenAI models
    if db
        .get_setting("openai_api_key")
        .ok()
        .flatten()
        .is_some()
    {
        models.extend([
            ModelInfo {
                id: "openai/gpt-4o".into(),
                name: "GPT-4o".into(),
                provider: "OpenAI".into(),
            },
            ModelInfo {
                id: "openai/gpt-4o-mini".into(),
                name: "GPT-4o Mini".into(),
                provider: "OpenAI".into(),
            },
            ModelInfo {
                id: "openai/gpt-4.1".into(),
                name: "GPT-4.1".into(),
                provider: "OpenAI".into(),
            },
        ]);
    }

    // Claude models
    if db
        .get_setting("claude_api_key")
        .ok()
        .flatten()
        .is_some()
    {
        models.extend([
            ModelInfo {
                id: "claude/claude-sonnet-4-20250514".into(),
                name: "Claude Sonnet 4".into(),
                provider: "Anthropic".into(),
            },
            ModelInfo {
                id: "claude/claude-haiku-3-5-20241022".into(),
                name: "Claude Haiku 3.5".into(),
                provider: "Anthropic".into(),
            },
        ]);
    }

    // Ollama models (always available — local)
    models.extend([
        ModelInfo {
            id: "ollama/llama3".into(),
            name: "Llama 3".into(),
            provider: "Ollama".into(),
        },
        ModelInfo {
            id: "ollama/qwen2.5".into(),
            name: "Qwen 2.5".into(),
            provider: "Ollama".into(),
        },
    ]);

    Ok(models)
}
