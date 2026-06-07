//! Minimal async Ollama HTTP client (localhost). Non-streaming chat (we need the
//! complete JSON object to parse actions reliably), plus model catalog and live
//! VRAM/usage stats.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct OllamaClient {
    base: String,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
struct ChatOptions {
    temperature: f32,
    num_ctx: u32,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'a str>,
    options: ChatOptions,
}

#[derive(Deserialize)]
struct ChatMsgRaw {
    #[serde(default)]
    content: String,
}

#[derive(Deserialize)]
struct ChatResponseRaw {
    message: ChatMsgRaw,
    #[serde(default)]
    eval_count: Option<u64>,
    #[serde(default)]
    eval_duration: Option<u64>, // nanoseconds
    #[serde(default)]
    total_duration: Option<u64>, // nanoseconds
}

/// The useful parts of a chat completion, including performance stats.
#[derive(Debug, Clone)]
pub struct ChatOutcome {
    pub content: String,
    pub eval_count: u64,
    pub eval_duration_ns: u64,
    pub total_ms: u64,
}

impl ChatOutcome {
    pub fn tokens_per_sec(&self) -> f64 {
        if self.eval_duration_ns == 0 {
            0.0
        } else {
            self.eval_count as f64 / (self.eval_duration_ns as f64 / 1e9)
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ModelDetails {
    #[serde(default)]
    pub parameter_size: String,
    #[serde(default)]
    pub quantization_level: String,
    #[serde(default)]
    pub family: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TagModel {
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    details: ModelDetails,
}

#[derive(Debug, Clone, Deserialize)]
struct TagsResponse {
    #[serde(default)]
    models: Vec<TagModel>,
}

#[derive(Debug, Clone, Deserialize)]
struct PsModel {
    name: String,
    #[serde(default)]
    size_vram: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct PsResponse {
    #[serde(default)]
    models: Vec<PsModel>,
}

/// Merged view of an installed model for the Settings UI.
#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    pub name: String,
    /// On-disk size in bytes.
    pub size: u64,
    pub parameter_size: String,
    pub quantization: String,
    pub family: String,
    /// Currently loaded into memory by Ollama?
    pub loaded: bool,
    /// Live VRAM (if loaded) otherwise an estimate based on the on-disk size.
    pub vram_bytes: u64,
}

impl OllamaClient {
    pub fn new(base: &str) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    /// Non-streaming chat completion. `json_format` forces structured JSON output.
    pub async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        json_format: bool,
    ) -> Result<ChatOutcome> {
        let req = ChatRequest {
            model,
            messages,
            stream: false,
            format: if json_format { Some("json") } else { None },
            options: ChatOptions {
                temperature: 0.2,
                num_ctx: 8192,
            },
        };
        let url = format!("{}/api/chat", self.base);
        let resp = self
            .http
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| anyhow!("cannot reach Ollama at {} ({e})", self.base))?;

        if !resp.status().is_success() {
            let code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Ollama returned {code}: {body}"));
        }
        let raw: ChatResponseRaw = resp.json().await?;
        Ok(ChatOutcome {
            content: raw.message.content,
            eval_count: raw.eval_count.unwrap_or(0),
            eval_duration_ns: raw.eval_duration.unwrap_or(0),
            total_ms: raw.total_duration.unwrap_or(0) / 1_000_000,
        })
    }

    /// Summarize free text with the model (used for document summaries).
    pub async fn summarize(&self, model: &str, text: &str) -> Result<String> {
        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: "You summarize documents concisely and accurately. Reply with a short \
                          summary in plain prose — no preamble."
                    .into(),
            },
            ChatMessage {
                role: "user".into(),
                content: format!("Summarize the following document:\n\n{text}"),
            },
        ];
        let out = self.chat(model, &messages, false).await?;
        Ok(out.content.trim().to_string())
    }

    pub async fn version(&self) -> Result<String> {
        let url = format!("{}/api/version", self.base);
        let resp = self.http.get(&url).send().await?;
        let v: serde_json::Value = resp.json().await?;
        Ok(v.get("version")
            .and_then(|x| x.as_str())
            .unwrap_or("unknown")
            .to_string())
    }

    async fn tags(&self) -> Result<Vec<TagModel>> {
        let url = format!("{}/api/tags", self.base);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("cannot reach Ollama at {} ({e})", self.base))?;
        let parsed: TagsResponse = resp.json().await?;
        Ok(parsed.models)
    }

    async fn ps(&self) -> Result<Vec<PsModel>> {
        let url = format!("{}/api/ps", self.base);
        let resp = self.http.get(&url).send().await?;
        let parsed: PsResponse = resp.json().await?;
        Ok(parsed.models)
    }

    /// Installed models merged with live load/VRAM status.
    pub async fn catalog(&self) -> Result<Vec<ModelStatus>> {
        let tags = self.tags().await?;
        let running = self.ps().await.unwrap_or_default();
        let out = tags
            .into_iter()
            .map(|m| {
                let live = running.iter().find(|r| r.name == m.name);
                ModelStatus {
                    loaded: live.is_some(),
                    vram_bytes: live.map(|r| r.size_vram).unwrap_or(m.size),
                    name: m.name,
                    size: m.size,
                    parameter_size: m.details.parameter_size,
                    quantization: m.details.quantization_level,
                    family: m.details.family,
                }
            })
            .collect();
        Ok(out)
    }
}
