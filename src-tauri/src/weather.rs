//! Optional weather lookup via wttr.in (no API key needed). Opt-in: it uses the
//! internet, so it's gated behind a Settings toggle like web search.

use crate::models::WeatherInfo;
use anyhow::{anyhow, Result};

/// Fetch current weather. `location` empty/None → wttr.in geolocates by IP.
pub async fn fetch(location: Option<&str>) -> Result<WeatherInfo> {
    let loc = location.map(|s| s.trim()).unwrap_or("");
    let url = format!("https://wttr.in/{}?format=j1", urlencode(loc));
    let client = reqwest::Client::builder()
        .user_agent("Pebble (local assistant)")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| anyhow!("http client error: {e}"))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("couldn't reach the weather service: {e}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("weather service returned {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| anyhow!("bad weather response: {e}"))?;

    let cur = v
        .get("current_condition")
        .and_then(|c| c.get(0))
        .ok_or_else(|| anyhow!("no current conditions in response"))?;
    let get = |k: &str| cur.get(k).and_then(|s| s.as_str()).unwrap_or("").to_string();
    let desc = cur
        .get("weatherDesc")
        .and_then(|d| d.get(0))
        .and_then(|d| d.get("value"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let location_name = v
        .get("nearest_area")
        .and_then(|a| a.get(0))
        .and_then(|a| a.get("areaName"))
        .and_then(|n| n.get(0))
        .and_then(|n| n.get("value"))
        .and_then(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| if loc.is_empty() { "your area".to_string() } else { loc.to_string() });

    Ok(WeatherInfo {
        location: location_name,
        temp_c: get("temp_C"),
        temp_f: get("temp_F"),
        feels_like_c: get("FeelsLikeC"),
        description: desc,
        humidity: get("humidity"),
        wind_kmph: get("windspeedKmph"),
    })
}

/// Minimal percent-encoding of the location for the URL path (UTF-8 safe).
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b' ' => out.push('+'),
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b',' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
