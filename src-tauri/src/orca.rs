use crate::models::SessionTarget;
use crate::process::{now_ms, run};
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
    Ok(terminals
        .iter()
        .filter(|terminal| {
            terminal["agentIdentity"]
                .as_str()
                .unwrap_or("")
                .starts_with("opencode")
                && terminal["orphaned"].as_bool() != Some(true)
        })
        .filter_map(|terminal| {
            let handle = terminal["handle"].as_str()?;
            Some(SessionTarget {
                id: format!("orca:{handle}"),
                source: "orca".into(),
                title: terminal["title"]
                    .as_str()
                    .unwrap_or("OpenCode terminal")
                    .into(),
                project: terminal["worktreePath"].as_str().unwrap_or("").into(),
                state: "unknown".into(),
                updated_at: terminal["lastOutputAt"].as_u64().unwrap_or_else(now_ms),
                terminal_id: pane_key(terminal),
                terminal_title: Some("Orca terminal".into()),
                orca_handle: Some(handle.into()),
                ..Default::default()
            })
        })
        .collect())
}

fn pane_key(terminal: &Value) -> Option<String> {
    // ORCA_PANE_KEY identifies the UI tab and leaf, not the underlying PTY ID.
    match (terminal["tabId"].as_str(), terminal["leafId"].as_str()) {
        (Some(tab), Some(leaf)) => Some(format!("{tab}:{leaf}")),
        _ => terminal["ptyId"].as_str().map(str::to_string),
    }
}

pub async fn select_terminal(handle: &str) -> Result<(), String> {
    cli(&["terminal", "switch", "--terminal", handle, "--json"], 12)
        .await
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_terminal_identity_matches_the_live_plugin_pane_key() {
        let terminal = serde_json::json!({
            "tabId": "6d01237e-d23c-4671-b2f4-8576320ab5c1",
            "leafId": "10095741-f45c-4fec-ae78-cbc31b36dcfe",
            "ptyId": "worktree::directory@@pty"
        });
        assert_eq!(
            pane_key(&terminal).as_deref(),
            Some("6d01237e-d23c-4671-b2f4-8576320ab5c1:10095741-f45c-4fec-ae78-cbc31b36dcfe")
        );
        assert_eq!(
            pane_key(&serde_json::json!({"ptyId":"legacy"})).as_deref(),
            Some("legacy")
        );
        assert!(pane_key(&serde_json::json!({})).is_none());
    }
}
