//! Optional, opt-in web search via DuckDuckGo's HTML endpoint. Best-effort HTML
//! parsing — it's a bonus tool, gated behind a Settings toggle for privacy.

use crate::models::WebResult;
use anyhow::{anyhow, Result};
use regex::Regex;

pub async fn search(query: &str, limit: usize) -> Result<Vec<WebResult>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Pebble")
        .build()
        .map_err(|e| anyhow!("http client error: {e}"))?;
    let resp = client
        .get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .send()
        .await
        .map_err(|e| anyhow!("couldn't reach the web: {e}"))?;
    let body = resp.text().await.map_err(|e| anyhow!("bad response: {e}"))?;

    let link_re = Regex::new(r#"(?s)class="result__a"[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#).unwrap();
    let snip_re = Regex::new(r#"(?s)class="result__snippet"[^>]*>(.*?)</a>"#).unwrap();

    let snippets: Vec<String> = snip_re.captures_iter(&body).map(|c| clean_html(&c[1])).collect();

    let mut out = Vec::new();
    for (i, cap) in link_re.captures_iter(&body).enumerate() {
        if out.len() >= limit {
            break;
        }
        let url = real_url(&cap[1]);
        let title = clean_html(&cap[2]);
        if title.is_empty() || url.is_empty() {
            continue;
        }
        let snippet = snippets.get(i).cloned().unwrap_or_default();
        out.push(WebResult { title, url, snippet });
    }
    Ok(out)
}

/// DuckDuckGo wraps links as //duckduckgo.com/l/?uddg=<encoded>. Pull the real URL.
fn real_url(href: &str) -> String {
    if let Some(idx) = href.find("uddg=") {
        let rest = &href[idx + 5..];
        let enc = rest.split('&').next().unwrap_or(rest);
        percent_decode(enc)
    } else if href.starts_with("//") {
        format!("https:{href}")
    } else {
        href.to_string()
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => match (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                (Some(h), Some(l)) => {
                    out.push(h * 16 + l);
                    i += 3;
                }
                _ => {
                    out.push(bytes[i]);
                    i += 1;
                }
            },
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Strip HTML tags and decode a few common entities.
fn clean_html(s: &str) -> String {
    let tag = Regex::new(r"<[^>]+>").unwrap();
    tag.replace_all(s, "")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}
