use super::{ChatRequest, ChatResponse, LlmError, StreamChunk};
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ClaudeConfig {
    pub api_key: String,
    pub base_url: String,
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    text: String,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ClaudeStreamEvent {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: ClaudeDelta },
    #[serde(rename = "message_stop")]
    MessageStop {},
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct ClaudeDelta {
    text: Option<String>,
}

fn build_request(request: &ChatRequest) -> ClaudeRequest {
    let system_msg = request
        .messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone());

    let messages: Vec<ClaudeMessage> = request
        .messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| ClaudeMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    ClaudeRequest {
        model: request.model.clone(),
        max_tokens: 4096,
        messages,
        stream: request.stream,
        system: system_msg,
    }
}

pub async fn chat(config: &ClaudeConfig, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
    let client = Client::new();
    let body = build_request(request);

    let resp = client
        .post(format!("{}/v1/messages", config.base_url))
        .header("Content-Type", "application/json")
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
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

    let data: ClaudeResponse = resp.json().await?;
    let content = data
        .content
        .first()
        .map(|c| c.text.clone())
        .unwrap_or_default();

    Ok(ChatResponse {
        content,
        model: request.model.clone(),
    })
}

pub async fn chat_stream(
    config: &ClaudeConfig,
    request: &ChatRequest,
    on_chunk: impl Fn(StreamChunk) + Send,
) -> Result<String, LlmError> {
    let client = Client::new();
    let mut body = build_request(request);
    body.stream = true;

    let resp = client
        .post(format!("{}/v1/messages", config.base_url))
        .header("Content-Type", "application/json")
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
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
                if let Ok(event) = serde_json::from_str::<ClaudeStreamEvent>(data) {
                    match event {
                        ClaudeStreamEvent::ContentBlockDelta { delta } => {
                            if let Some(text) = delta.text {
                                full_content.push_str(&text);
                                on_chunk(StreamChunk {
                                    delta: text,
                                    done: false,
                                });
                            }
                        }
                        ClaudeStreamEvent::MessageStop {} => {
                            on_chunk(StreamChunk {
                                delta: String::new(),
                                done: true,
                            });
                            return Ok(full_content);
                        }
                        ClaudeStreamEvent::Other => {}
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
