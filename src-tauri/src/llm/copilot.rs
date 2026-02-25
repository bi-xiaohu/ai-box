use super::{ChatRequest, ChatResponse, LlmError, StreamChunk};
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct CopilotConfig {
    /// GitHub PAT (or OAuth token) with Copilot access.
    pub github_token: String,
}

/// Cached Copilot session token with expiry.
struct CachedToken {
    token: String,
    expires_at: u64,
}

static TOKEN_CACHE: Mutex<Option<CachedToken>> = Mutex::new(None);

/// Exchange a GitHub token for a short-lived Copilot API token.
async fn get_copilot_token(github_token: &str) -> Result<String, LlmError> {
    // Check cache first
    {
        let cache = TOKEN_CACHE.lock().unwrap();
        if let Some(cached) = cache.as_ref() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            // Refresh 60s before expiry
            if now + 60 < cached.expires_at {
                return Ok(cached.token.clone());
            }
        }
    }

    let client = Client::new();
    let resp = client
        .get("https://api.github.com/copilot_internal/v2/token")
        .header("Authorization", format!("token {}", github_token))
        .header("User-Agent", "ai-box/0.1.0")
        .header("Accept", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Api {
            status,
            message: format!("Failed to get Copilot token: {}", text),
        });
    }

    let data: CopilotTokenResponse = resp.json().await.map_err(|e| LlmError::Parse(e.to_string()))?;

    // Cache the token
    {
        let mut cache = TOKEN_CACHE.lock().unwrap();
        *cache = Some(CachedToken {
            token: data.token.clone(),
            expires_at: data.expires_at,
        });
    }

    Ok(data.token)
}

#[derive(Deserialize)]
struct CopilotTokenResponse {
    token: String,
    expires_at: u64,
}

#[derive(Serialize)]
struct CopilotRequest {
    model: String,
    messages: Vec<CopilotMessage>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct CopilotMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct CopilotResponse {
    choices: Vec<CopilotChoice>,
}

#[derive(Deserialize)]
struct CopilotChoice {
    message: CopilotMessage,
}

#[derive(Deserialize)]
struct CopilotStreamResponse {
    choices: Vec<CopilotStreamChoice>,
}

#[derive(Deserialize)]
struct CopilotStreamChoice {
    delta: CopilotDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct CopilotDelta {
    content: Option<String>,
}

const COPILOT_CHAT_URL: &str = "https://api.githubcopilot.com/chat/completions";

pub async fn chat(config: &CopilotConfig, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
    let token = get_copilot_token(&config.github_token).await?;

    let client = Client::new();
    let messages: Vec<CopilotMessage> = request
        .messages
        .iter()
        .map(|m| CopilotMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    let body = CopilotRequest {
        model: request.model.clone(),
        messages,
        stream: false,
    };

    let resp = client
        .post(COPILOT_CHAT_URL)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .header("Copilot-Integration-Id", "vscode-chat")
        .header("Editor-Version", "ai-box/0.1.0")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Api {
            status,
            message: text,
        });
    }

    let data: CopilotResponse = resp.json().await?;
    let content = data
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    Ok(ChatResponse {
        content,
        model: request.model.clone(),
    })
}

pub async fn chat_stream(
    config: &CopilotConfig,
    request: &ChatRequest,
    on_chunk: impl Fn(StreamChunk) + Send,
) -> Result<String, LlmError> {
    let token = get_copilot_token(&config.github_token).await?;

    let client = Client::new();
    let messages: Vec<CopilotMessage> = request
        .messages
        .iter()
        .map(|m| CopilotMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    let body = CopilotRequest {
        model: request.model.clone(),
        messages,
        stream: true,
    };

    let resp = client
        .post(COPILOT_CHAT_URL)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .header("Copilot-Integration-Id", "vscode-chat")
        .header("Editor-Version", "ai-box/0.1.0")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Api {
            status,
            message: text,
        });
    }

    let mut full_content = String::new();
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    on_chunk(StreamChunk {
                        delta: String::new(),
                        done: true,
                    });
                    return Ok(full_content);
                }

                if let Ok(parsed) = serde_json::from_str::<CopilotStreamResponse>(data) {
                    if let Some(choice) = parsed.choices.first() {
                        if let Some(content) = &choice.delta.content {
                            full_content.push_str(content);
                            on_chunk(StreamChunk {
                                delta: content.clone(),
                                done: false,
                            });
                        }
                        if choice.finish_reason.is_some() {
                            on_chunk(StreamChunk {
                                delta: String::new(),
                                done: true,
                            });
                            return Ok(full_content);
                        }
                    }
                }
            }
        }
    }

    on_chunk(StreamChunk {
        delta: String::new(),
        done: true,
    });
    Ok(full_content)
}

/// Fetch available models from the Copilot API.
pub async fn fetch_models(github_token: &str) -> Result<Vec<super::ModelInfo>, LlmError> {
    let token = get_copilot_token(github_token).await?;

    let client = Client::new();
    let resp = client
        .get("https://api.githubcopilot.com/models")
        .header("Authorization", format!("Bearer {}", token))
        .header("Copilot-Integration-Id", "vscode-chat")
        .header("Editor-Version", "ai-box/0.1.0")
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Api {
            status,
            message: format!("Failed to fetch Copilot models: {}", text),
        });
    }

    let catalog: CopilotModelCatalog = resp
        .json()
        .await
        .map_err(|e| LlmError::Parse(e.to_string()))?;

    let models = catalog
        .models
        .into_iter()
        .filter(|m| m.capabilities.chat)
        .map(|m| super::ModelInfo {
            id: format!("copilot/{}", m.id),
            name: m.name,
            provider: "Copilot".into(),
        })
        .collect();

    Ok(models)
}

#[derive(Deserialize)]
struct CopilotModelCatalog {
    models: Vec<CopilotModelEntry>,
}

#[derive(Deserialize)]
struct CopilotModelEntry {
    id: String,
    name: String,
    #[serde(default)]
    capabilities: CopilotModelCapabilities,
}

#[derive(Deserialize, Default)]
struct CopilotModelCapabilities {
    #[serde(default)]
    chat: bool,
}
