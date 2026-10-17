use crate::models::SessionTarget;
use crate::process::{now_ms, run};
use futures::future::join_all;
use serde_json::Value;
use std::time::Duration;

async fn cli(args: &[&str], timeout_secs: u64) -> Result<Value, String> {
    let arguments = args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    let output = run("orca", &arguments, Duration::from_secs(timeout_secs)).await?;
    let value: Value =
        serde_json::from_str(&output).map_err(|e| format!("Orca returned invalid JSON: {e}"))?;
    if value["ok"].as_bool() != Some(true) {
        return Err(value["error"]["message"]
            .as_str()
            .unwrap_or("Orca command failed")
            .to_string());
    }
    Ok(value)
}

pub async fn list_targets() -> Result<Vec<SessionTarget>, String> {
    let value = cli(&["terminal", "list", "--json"], 20).await?;
    let terminals = value["result"]["terminals"]
        .as_array()
        .ok_or("Orca did not return a terminal list")?;
    let candidates = terminals
        .iter()
        .filter_map(|terminal| {
            let agent = terminal["agentIdentity"].as_str().unwrap_or("");
            if agent.starts_with("opencode") && terminal["orphaned"].as_bool() != Some(true) {
                Some(terminal.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let errors = join_all(candidates.iter().map(|terminal| async move {
        let handle = terminal["handle"].as_str().unwrap_or("");
        read_error(handle).await
    }))
    .await;
    let mut targets = Vec::new();
    for (terminal, read_result) in candidates.iter().zip(errors) {
        let Some(handle) = terminal["handle"].as_str() else {
            continue;
        };
        let preview = terminal["preview"].as_str().unwrap_or("");
        let error = match read_result {
            Ok(error) => error,
            Err(_) => contains_rate_limit(preview).then(|| "Rate limit exceeded".into()),
        };
        let updated_at = terminal["lastOutputAt"].as_u64().unwrap_or_else(now_ms);
        targets.push(SessionTarget {
            id: format!("orca:{handle}"),
            source: "orca".into(),
            title: terminal["title"]
                .as_str()
                .unwrap_or("OpenCode terminal")
                .into(),
            project: terminal["worktreePath"].as_str().unwrap_or("").into(),
            state: if error.is_some() {
                "rate-limited"
            } else {
                "idle"
            }
            .into(),
            last_error: error.clone(),
            error_key: error.map(|_| format!("orca:{handle}:{updated_at}")),
            prompt_override: None,
            updated_at,
        });
    }
    Ok(targets)
}

fn contains_rate_limit(text: &str) -> bool {
    text.to_ascii_lowercase()
        .contains("error: rate limit exceeded")
}

async fn read_error(handle: &str) -> Result<Option<String>, String> {
    let value = cli(
        &[
            "terminal",
            "read",
            "--terminal",
            handle,
            "--limit",
            "1200",
            "--json",
        ],
        25,
    )
    .await?;
    let lines = value["result"]["terminal"]["tail"]
        .as_array()
        .ok_or("Orca did not return terminal output")?;
    Ok(lines
        .iter()
        .rev()
        .take(12)
        .filter_map(Value::as_str)
        .find(|line| contains_rate_limit(line))
        .map(str::trim)
        .map(str::to_string))
}

pub async fn is_idle(handle: &str) -> Result<bool, String> {
    let value = cli(
        &[
            "terminal",
            "wait",
            "--terminal",
            handle,
            "--for",
            "tui-idle",
            "--timeout-ms",
            "1000",
            "--json",
        ],
        12,
    )
    .await?;
    Ok(value["result"]["wait"]["satisfied"].as_bool() == Some(true))
}

pub async fn wait_until_idle(handle: &str, checks: usize) -> Result<bool, String> {
    for _ in 0..checks {
        if is_idle(handle).await? {
            return Ok(true);
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    Ok(false)
}

pub async fn send_prompt(handle: &str, prompt: &str) -> Result<String, String> {
    let value = cli(
        &[
            "terminal",
            "send",
            "--terminal",
            handle,
            "--text",
            prompt,
            "--enter",
            "--wait-submit",
            "10",
            "--json",
        ],
        25,
    )
    .await?;
    if !receipt_accepted(&value) {
        return Err("Orca did not confirm that the prompt was accepted".into());
    }
    let request_id = value
        .pointer("/result/requestId")
        .or_else(|| value.pointer("/result/receipt/requestId"))
        .and_then(Value::as_str)
        .unwrap_or("accepted");
    Ok(request_id.into())
}

fn receipt_accepted(value: &Value) -> bool {
    value.pointer("/result/accepted").and_then(Value::as_bool) == Some(true)
        || value
            .pointer("/result/receipt/accepted")
            .and_then(Value::as_bool)
            == Some(true)
        || value.to_string().contains("\"accepted\":true")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_rate_limit_errors_trigger() {
        assert!(contains_rate_limit(
            "Error: Rate limit exceeded. Please try again later."
        ));
        assert!(!contains_rate_limit("Discuss rate limit handling in docs"));
    }

    #[test]
    fn accepted_receipts_are_detected() {
        assert!(receipt_accepted(
            &serde_json::json!({"result":{"accepted":true}})
        ));
        assert!(!receipt_accepted(
            &serde_json::json!({"result":{"accepted":false}})
        ));
    }
}
