pub mod claude;
pub mod openai;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub delta: String,
    pub done: bool,
}

/// Unified LLM provider enum — dispatches to OpenAI-compatible or Claude backends.
#[derive(Debug, Clone)]
pub enum Provider {
    OpenAi(openai::OpenAiConfig),
    Claude(claude::ClaudeConfig),
    Ollama(openai::OpenAiConfig),
}

impl Provider {
    pub fn openai(api_key: String) -> Self {
        Provider::OpenAi(openai::OpenAiConfig {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
        })
    }

    pub fn claude(api_key: String) -> Self {
        Provider::Claude(claude::ClaudeConfig {
            api_key,
            base_url: "https://api.anthropic.com".to_string(),
        })
    }

    pub fn ollama(host: String) -> Self {
        Provider::Ollama(openai::OpenAiConfig {
            api_key: String::new(),
            base_url: format!("{}/v1", host),
        })
    }

    pub async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
        match self {
            Provider::OpenAi(config) | Provider::Ollama(config) => {
                openai::chat(config, request).await
            }
            Provider::Claude(config) => claude::chat(config, request).await,
        }
    }

    pub async fn chat_stream(
        &self,
        request: &ChatRequest,
        on_chunk: impl Fn(StreamChunk) + Send,
    ) -> Result<String, LlmError> {
        match self {
            Provider::OpenAi(config) | Provider::Ollama(config) => {
                openai::chat_stream(config, request, on_chunk).await
            }
            Provider::Claude(config) => claude::chat_stream(config, request, on_chunk).await,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {status} - {message}")]
    Api { status: u16, message: String },
    #[error("Parse error: {0}")]
    Parse(String),
}

impl Serialize for LlmError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
