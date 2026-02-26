use super::{ChatRequest, ChatResponse, LlmError, StreamChunk};
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Well-known client_id used by Copilot IDE integrations (copilot.vim etc.)
const GITHUB_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
const TOKEN_AUTH_URL: &str = "https://api.github.com/copilot_internal/v2/token";
const COPILOT_CHAT_URL: &str = "https://api.githubcopilot.com/chat/completions";
const COPILOT_MODELS_URL: &str = "https://api.githubcopilot.com/models";

#[derive(Debug, Clone)]
pub struct CopilotConfig {
    /// GitHub OAuth token obtained via device flow.
    pub oauth_token: String,
}

// ── Device OAuth Flow ──

#[derive(Deserialize, Serialize, Clone)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
}

/// Step 1: Request a device code from GitHub.
pub async fn start_device_flow() -> Result<DeviceCodeResponse, LlmError> {
    let client = Client::new();
    let resp = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[("client_id", GITHUB_CLIENT_ID), ("scope", "copilot")])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Api { status, message: text });
    }

    resp.json().await.map_err(|e| LlmError::Parse(e.to_string()))
}

#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

/// Step 2: Poll GitHub for the OAuth token after user authorizes.
pub async fn poll_device_flow(device_code: &str) -> Result<Option<String>, LlmError> {
    let client = Client::new();
    let resp = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", GITHUB_CLIENT_ID),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Api { status, message: text });
    }

    let raw = resp.text().await.unwrap_or_default();
    let data: OAuthTokenResponse = serde_json::from_str(&raw).map_err(|e| LlmError::Parse(e.to_string()))?;

    if let Some(token) = data.access_token {
        return Ok(Some(token));
    }

    match data.error.as_deref() {
        Some("authorization_pending") | Some("slow_down") => Ok(None),
        Some(err) => Err(LlmError::Api { status: 400, message: err.to_string() }),
        None => Ok(None),
    }
}

// ── Copilot API Token (exchanged from OAuth token) ──

struct CachedToken {
    token: String,
    expires_at: u64,
}

static TOKEN_CACHE: Mutex<Option<CachedToken>> = Mutex::new(None);

/// Exchange OAuth token for a short-lived Copilot API token.
async fn get_copilot_token(oauth_token: &str) -> Result<String, LlmError> {
    {
        let cache = TOKEN_CACHE.lock().unwrap();
        if let Some(cached) = cache.as_ref() {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            if now + 120 < cached.expires_at {
                return Ok(cached.token.clone());
            }
        }
    }

    let client = Client::new();
    let resp = client
        .get(TOKEN_AUTH_URL)
        .header("Authorization", format!("token {}", oauth_token))
        .header("Accept", "application/json")
        .header("Editor-Plugin-Version", "copilot/1.0.0")
        .header("User-Agent", "ai-box/0.1.0")
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Api {
            status,
            message: format!("Copilot token exchange failed: {}", text),
        });
    }

    let data: CopilotTokenResp = resp.json().await.map_err(|e| LlmError::Parse(e.to_string()))?;

    {
        let mut cache = TOKEN_CACHE.lock().unwrap();
        *cache = Some(CachedToken { token: data.token.clone(), expires_at: data.expires_at });
    }

    Ok(data.token)
}

#[derive(Deserialize)]
struct CopilotTokenResp {
    token: String,
    expires_at: u64,
}

// ── Chat ──

#[derive(Serialize)]
struct ChatBody {
    model: String,
    messages: Vec<Msg>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct Msg {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResp {
    choices: Vec<ChatChoice>,
}
#[derive(Deserialize)]
struct ChatChoice {
    message: Msg,
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

fn copilot_headers(token: &str) -> Vec<(&'static str, String)> {
    vec![
        ("Authorization", format!("Bearer {}", token)),
        ("Copilot-Integration-Id", "vscode-chat".into()),
        ("Editor-Version", "ai-box/0.1.0".into()),
    ]
}

pub async fn chat(config: &CopilotConfig, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
    let token = get_copilot_token(&config.oauth_token).await?;
    let client = Client::new();
    let messages: Vec<Msg> = request.messages.iter()
        .map(|m| Msg { role: m.role.clone(), content: m.content.clone() })
        .collect();

    let body = ChatBody { model: request.model.clone(), messages, stream: false };

    let mut req = client.post(COPILOT_CHAT_URL);
    for (k, v) in copilot_headers(&token) { req = req.header(k, v); }
    let resp = req.json(&body).send().await?;

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
    let token = get_copilot_token(&config.oauth_token).await?;
    let client = Client::new();
    let messages: Vec<Msg> = request.messages.iter()
        .map(|m| Msg { role: m.role.clone(), content: m.content.clone() })
        .collect();

    let body = ChatBody { model: request.model.clone(), messages, stream: true };

    let mut req = client.post(COPILOT_CHAT_URL);
    for (k, v) in copilot_headers(&token) { req = req.header(k, v); }
    let resp = req.json(&body).send().await?;

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

// ── Models ──

pub async fn fetch_models(oauth_token: &str) -> Result<Vec<super::ModelInfo>, LlmError> {
    let token = get_copilot_token(oauth_token).await?;
    let client = Client::new();

    let mut req = client.get(COPILOT_MODELS_URL);
    for (k, v) in copilot_headers(&token) { req = req.header(k, v); }
    let resp = req.send().await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Api { status, message: format!("Failed to fetch models: {}", text) });
    }

    let catalog: ModelCatalog = resp.json().await.map_err(|e| LlmError::Parse(e.to_string()))?;

    let models = catalog.data.into_iter()
        .map(|m| super::ModelInfo {
            id: format!("copilot/{}", m.id),
            name: m.id.clone(),
            provider: m.vendor.unwrap_or_else(|| "Copilot".into()),
        })
        .collect();

    Ok(models)
}

#[derive(Deserialize)]
struct ModelCatalog {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
    vendor: Option<String>,
}
