//! Intent parser. Turns the model's raw text into a validated [`AiPlan`].
//!
//! Defensive by design: a single malformed action is dropped rather than failing
//! the whole turn, and non-JSON output degrades gracefully to a plain chat reply.

use crate::models::{Action, AiPlan};
use serde::Deserialize;

#[derive(Deserialize)]
struct RawPlan {
    #[serde(default)]
    message: String,
    #[serde(default)]
    actions: Vec<serde_json::Value>,
}

/// Parse model output into an [`AiPlan`]. Never panics; worst case it returns the
/// text as a chat message with no actions.
pub fn parse(content: &str) -> AiPlan {
    let cleaned = strip_code_fences(content);

    // Primary: parse the (expected) JSON object leniently.
    if let Some(plan) = try_parse(&cleaned) {
        return plan;
    }
    // Secondary: extract the first {...} block and try again.
    if let Some(obj) = extract_json_object(&cleaned) {
        if let Some(plan) = try_parse(&obj) {
            return plan;
        }
    }
    // Fallback: treat the whole thing as a chat message.
    AiPlan {
        message: content.trim().to_string(),
        actions: Vec::new(),
    }
}

fn try_parse(s: &str) -> Option<AiPlan> {
    let raw: RawPlan = serde_json::from_str(s).ok()?;
    let actions = raw
        .actions
        .into_iter()
        .filter_map(|v| serde_json::from_value::<Action>(v).ok()) // drop unknown/invalid
        .collect();
    Some(AiPlan {
        message: raw.message,
        actions,
    })
}

fn strip_code_fences(s: &str) -> String {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix("```") {
        // remove an optional language tag on the first line, and the trailing fence
        let rest = rest.splitn(2, '\n').nth(1).unwrap_or(rest);
        return rest.trim_end_matches("```").trim().to_string();
    }
    t.to_string()
}

fn extract_json_object(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    for (i, c) in s[start..].char_indices() {
        match c {
            '"' if !escaped => in_str = !in_str,
            '\\' if in_str => {
                escaped = !escaped;
                continue;
            }
            '{' if !in_str => depth += 1,
            '}' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
        escaped = false;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_object() {
        let p = parse(r#"{"message":"hi","actions":[{"action":"storage_stats"}]}"#);
        assert_eq!(p.message, "hi");
        assert_eq!(p.actions.len(), 1);
    }

    #[test]
    fn drops_unknown_action_keeps_valid() {
        let p = parse(
            r#"{"message":"x","actions":[{"action":"nonexistent"},{"action":"find_duplicates","root":"C:/x"}]}"#,
        );
        assert_eq!(p.actions.len(), 1);
    }

    #[test]
    fn falls_back_to_chat() {
        let p = parse("just some text");
        assert_eq!(p.message, "just some text");
        assert!(p.actions.is_empty());
    }
}
