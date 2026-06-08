//! Minimal async Ollama HTTP client (localhost). Non-streaming chat (we need the
//! complete JSON object to parse actions reliably), plus model catalog and live
//! VRAM/usage stats.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

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

#[derive(Default, Deserialize)]
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

/// One line of a streaming `/api/chat` response (NDJSON).
#[derive(Deserialize)]
struct ChatStreamChunk {
    #[serde(default)]
    message: ChatMsgRaw,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    eval_count: Option<u64>,
    #[serde(default)]
    eval_duration: Option<u64>,
    #[serde(default)]
    total_duration: Option<u64>,
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
        // Fail fast when Ollama isn't listening, and cap any single request so a
        // stalled backend can't hang the app forever. The cap is generous on
        // purpose — generation on a slow machine is legitimately slow.
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            base: base.trim_end_matches('/').to_string(),
            http,
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

    /// Streaming chat completion. Calls `on_delta` with successive *prose* deltas.
    /// In JSON mode only the value of the top-level "message" field is surfaced as
    /// it streams; the full raw content is still returned so actions can be parsed
    /// once the object is complete. Stops early if `cancel` is set.
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: &[ChatMessage],
        json_format: bool,
        cancel: Arc<AtomicBool>,
        mut on_delta: impl FnMut(&str),
    ) -> Result<ChatOutcome> {
        let req = ChatRequest {
            model,
            messages,
            stream: true,
            format: if json_format { Some("json") } else { None },
            options: ChatOptions {
                temperature: 0.2,
                num_ctx: 8192,
            },
        };
        let url = format!("{}/api/chat", self.base);
        let mut resp = self
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

        let mut raw = String::new();
        let mut line_buf: Vec<u8> = Vec::new();
        let mut emitted = 0usize; // chars of prose already surfaced
        let mut eval_count = 0u64;
        let mut eval_duration = 0u64;
        let mut total_duration = 0u64;

        while let Some(chunk) = resp.chunk().await.map_err(|e| anyhow!("stream error: {e}"))? {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            line_buf.extend_from_slice(&chunk);
            while let Some(pos) = line_buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = line_buf.drain(..=pos).collect();
                let trimmed = &line[..line.len().saturating_sub(1)];
                if trimmed.is_empty() {
                    continue;
                }
                let parsed: ChatStreamChunk = match serde_json::from_slice(trimmed) {
                    Ok(p) => p,
                    Err(_) => continue, // skip a malformed line
                };
                raw.push_str(&parsed.message.content);
                if parsed.done {
                    eval_count = parsed.eval_count.unwrap_or(0);
                    eval_duration = parsed.eval_duration.unwrap_or(0);
                    total_duration = parsed.total_duration.unwrap_or(0);
                }
                let prose = if json_format {
                    extract_partial_message(&raw).unwrap_or_default()
                } else {
                    raw.clone()
                };
                let total = prose.chars().count();
                if total > emitted {
                    let delta: String = prose.chars().skip(emitted).collect();
                    emitted = total;
                    on_delta(&delta);
                }
            }
        }

        Ok(ChatOutcome {
            content: raw,
            eval_count,
            eval_duration_ns: eval_duration,
            total_ms: total_duration / 1_000_000,
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
        let resp = self.http.get(&url).timeout(Duration::from_secs(20)).send().await?;
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
            .timeout(Duration::from_secs(20))
            .send()
            .await
            .map_err(|e| anyhow!("cannot reach Ollama at {} ({e})", self.base))?;
        let parsed: TagsResponse = resp.json().await?;
        Ok(parsed.models)
    }

    async fn ps(&self) -> Result<Vec<PsModel>> {
        let url = format!("{}/api/ps", self.base);
        let resp = self.http.get(&url).timeout(Duration::from_secs(20)).send().await?;
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

/// Best-effort: pull the (possibly still-streaming) value of the top-level
/// "message" string out of a partial JSON object, decoding escapes. Returns the
/// text accumulated so far, or `None` if the message field hasn't started yet.
pub fn extract_partial_message(buf: &str) -> Option<String> {
    let key = buf.find("\"message\"")?;
    let rest = buf[key + "\"message\"".len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let body = rest.strip_prefix('"')?;

    let mut out = String::new();
    let mut chars = body.chars();
    let mut esc = false;
    while let Some(c) = chars.next() {
        if esc {
            match c {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'b' => out.push('\u{0008}'),
                'f' => out.push('\u{000C}'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'u' => {
                    let mut code = 0u32;
                    let mut ok = true;
                    for _ in 0..4 {
                        match chars.next().and_then(|h| h.to_digit(16)) {
                            Some(d) => code = code * 16 + d,
                            None => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if ok {
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                        }
                    } else {
                        break; // partial \u escape at the end of the buffer
                    }
                }
                other => out.push(other),
            }
            esc = false;
        } else {
            match c {
                '\\' => esc = true,
                '"' => break, // end of the message value
                _ => out.push(c),
            }
        }
    }
    Some(out)
}
