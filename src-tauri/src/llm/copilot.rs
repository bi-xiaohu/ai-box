use super::{ChatRequest, ChatResponse, LlmError, StreamChunk};
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://models.inference.ai.azure.com";

#[derive(Debug, Clone)]
pub struct CopilotConfig {
    /// GitHub PAT with models access.
    pub github_token: String,
}

#[derive(Serialize)]
struct ChatBody {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResp {
    choices: Vec<ChatRespChoice>,
}

#[derive(Deserialize)]
struct ChatRespChoice {
    message: Message,
}

#[derive(Deserialize)]
struct StreamResp {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
}

fn client_with_auth(token: &str) -> (Client, String) {
    (Client::new(), format!("Bearer {}", token))
}

pub async fn chat(config: &CopilotConfig, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
    let (client, auth) = client_with_auth(&config.github_token);
    let messages: Vec<Message> = request
        .messages
        .iter()
        .map(|m| Message { role: m.role.clone(), content: m.content.clone() })
        .collect();

    let body = ChatBody { model: request.model.clone(), messages, stream: false };

    let resp = client
        .post(format!("{}/chat/completions", BASE_URL))
        .header("Authorization", &auth)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Api { status, message: text });
    }

    let data: ChatResp = resp.json().await?;
    let content = data.choices.first().map(|c| c.message.content.clone()).unwrap_or_default();

    Ok(ChatResponse { content, model: request.model.clone() })
}

pub async fn chat_stream(
    config: &CopilotConfig,
    request: &ChatRequest,
    on_chunk: impl Fn(StreamChunk) + Send,
) -> Result<String, LlmError> {
    let (client, auth) = client_with_auth(&config.github_token);
    let messages: Vec<Message> = request
        .messages
        .iter()
        .map(|m| Message { role: m.role.clone(), content: m.content.clone() })
        .collect();

    let body = ChatBody { model: request.model.clone(), messages, stream: true };

    let resp = client
        .post(format!("{}/chat/completions", BASE_URL))
        .header("Authorization", &auth)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Api { status, message: text });
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
                    on_chunk(StreamChunk { delta: String::new(), done: true });
                    return Ok(full_content);
                }

                if let Ok(parsed) = serde_json::from_str::<StreamResp>(data) {
                    if let Some(choice) = parsed.choices.first() {
                        if let Some(content) = &choice.delta.content {
                            full_content.push_str(content);
                            on_chunk(StreamChunk { delta: content.clone(), done: false });
                        }
                        if choice.finish_reason.is_some() {
                            on_chunk(StreamChunk { delta: String::new(), done: true });
                            return Ok(full_content);
                        }
                    }
                }
            }
        }
    }

    on_chunk(StreamChunk { delta: String::new(), done: true });
    Ok(full_content)
}

/// Fetch available models from the GitHub Models API.
pub async fn fetch_models(github_token: &str) -> Result<Vec<super::ModelInfo>, LlmError> {
    let (client, auth) = client_with_auth(github_token);
    let resp = client
        .get(format!("{}/models", BASE_URL))
        .header("Authorization", &auth)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Api {
            status,
            message: format!("Failed to fetch models: {}", text),
        });
    }

    let catalog: Vec<ModelEntry> = resp
        .json()
        .await
        .map_err(|e| LlmError::Parse(e.to_string()))?;

    let models = catalog
        .into_iter()
        .filter(|m| m.task == "chat-completion")
        .map(|m| super::ModelInfo {
            id: format!("copilot/{}", m.name),
            name: m.friendly_name,
            provider: "GitHub Models".into(),
        })
        .collect();

    Ok(models)
}

#[derive(Deserialize)]
struct ModelEntry {
    name: String,
    friendly_name: String,
    #[serde(default)]
    task: String,
}
