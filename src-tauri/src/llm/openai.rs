use super::{ChatRequest, ChatResponse, LlmError, StreamChunk};
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub api_key: String,
    pub base_url: String,
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiStreamResponse {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
}

pub async fn chat(config: &OpenAiConfig, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
    let client = Client::new();
    let messages: Vec<OpenAiMessage> = request
        .messages
        .iter()
        .map(|m| OpenAiMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    let body = OpenAiRequest {
        model: request.model.clone(),
        messages,
        stream: false,
    };

    let mut req = client
        .post(format!("{}/chat/completions", config.base_url))
        .header("Content-Type", "application/json")
        .json(&body);

    if !config.api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", config.api_key));
    }

    let resp = req.send().await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Api {
            status,
            message: text,
        });
    }

    let data: OpenAiResponse = resp.json().await?;
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
    config: &OpenAiConfig,
    request: &ChatRequest,
    on_chunk: impl Fn(StreamChunk) + Send,
) -> Result<String, LlmError> {
    let client = Client::new();
    let messages: Vec<OpenAiMessage> = request
        .messages
        .iter()
        .map(|m| OpenAiMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    let body = OpenAiRequest {
        model: request.model.clone(),
        messages,
        stream: true,
    };

    let mut req = client
        .post(format!("{}/chat/completions", config.base_url))
        .header("Content-Type", "application/json")
        .json(&body);

    if !config.api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", config.api_key));
    }

    let resp = req.send().await?;

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

                if let Ok(parsed) = serde_json::from_str::<OpenAiStreamResponse>(data) {
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
