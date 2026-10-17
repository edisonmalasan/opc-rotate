use crate::models::SessionTarget;
use crate::process::now_ms;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

const PLUGIN: &str = include_str!("../../integrations/opencode/tui.js");

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Tab {
    #[serde(rename = "sessionID")]
    session_id: String,
    title: String,
    project: String,
    state: String,
    #[serde(default)]
    selected: bool,
    #[serde(default)]
    last_error: Option<String>,
    #[serde(default)]
    error_key: Option<String>,
    #[serde(default)]
    updated_at: u64,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Heartbeat {
    version: u8,
    pid: u32,
    service_pid: u32,
    orca: bool,
    updated_at: u64,
    #[serde(default)]
    client_id: Option<String>,
    #[serde(default)]
    pane_key: Option<String>,
    #[serde(default)]
    sessions: Vec<Tab>,
    #[serde(default, rename = "busySessionIDs")]
    busy_session_ids: Vec<String>,
    #[serde(default)]
    status_known: bool,
    #[serde(default, rename = "sessionID")]
    session_id: Option<String>,
}

#[derive(Default)]
pub struct Presence {
    pub open: HashSet<String>,
    pub orca: HashSet<String>,
    pub clients: usize,
    pub targets: Vec<SessionTarget>,
    pub covered_terminals: HashSet<String>,
    pub busy: HashSet<String>,
    pub unknown_clients: bool,
    routes: HashMap<String, Heartbeat>,
}

fn root() -> Result<PathBuf, String> {
    Ok(dirs::home_dir()
        .ok_or("Home directory not found")?
        .join(".local/state/opc-rotate"))
}

pub fn install() -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Home directory not found")?;
    let config = std::env::var_os("OPENCODE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config/opencode"));
    install_at(&config.join("plugins/opc-rotate-presence"))
}

fn install_at(directory: &Path) -> Result<(), String> {
    let path = directory.join("tui.js");
    if std::fs::read_to_string(&path).ok().as_deref() == Some(PLUGIN) {
        return Ok(());
    }
    std::fs::create_dir_all(directory)
        .and_then(|_| std::fs::write(directory.join("tui.js.tmp"), PLUGIN))
        .and_then(|_| std::fs::rename(directory.join("tui.js.tmp"), path))
        .map_err(|e| format!("Could not install OpenCode presence plugin: {e}"))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn read_at(directory: &Path, shared_pid: u32, now: u64) -> Presence {
    let mut presence = Presence::default();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return presence;
    };
    for entry in entries.flatten() {
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json")
            || entry.metadata().map(|m| m.len() > 262144).unwrap_or(true)
        {
            continue;
        }
        let Some(record) = std::fs::read(entry.path())
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Heartbeat>(&bytes).ok())
        else {
            continue;
        };
        if record.pid == 0
            || record.service_pid == 0
            || record.updated_at > now + 1000
            || now.saturating_sub(record.updated_at) > 8000
        {
            continue;
        }
        if record.version == 1 {
            if record.service_pid == shared_pid {
                if let Some(id) = record.session_id.filter(|id| valid_id(id)) {
                    if record.orca {
                        presence.orca.insert(id);
                    } else {
                        presence.open.insert(id);
                    }
                }
            }
            continue;
        }
        let Some(client_id) = record
            .client_id
            .clone()
            .filter(|id| uuid::Uuid::parse_str(id).is_ok())
        else {
            continue;
        };
        if record.version != 2 || record.sessions.len() > 128 {
            continue;
        }
        if !record.orca {
            presence.clients += 1;
        }
        presence.unknown_clients |= !record.status_known;
        presence.busy.extend(
            record
                .busy_session_ids
                .iter()
                .filter(|id| valid_id(id))
                .map(|id| format!("{}:{id}", record.service_pid)),
        );
        if let Some(pane) = &record.pane_key {
            if record.orca {
                presence.covered_terminals.insert(pane.clone());
            }
        }
        for tab in &record.sessions {
            if !valid_id(&tab.session_id) {
                continue;
            }
            if record.service_pid == shared_pid {
                if record.orca {
                    presence.orca.insert(tab.session_id.clone());
                } else {
                    presence.open.insert(tab.session_id.clone());
                }
            }
            presence.targets.push(SessionTarget {
                id: format!("api:{}", tab.session_id),
                source: if record.orca { "orca" } else { "other" }.into(),
                title: tab.title.clone(),
                project: tab.project.clone(),
                state: match tab.state.as_str() {
                    "idle" | "busy" | "rate-limited" => &tab.state,
                    _ => "unknown",
                }
                .into(),
                last_error: tab.last_error.clone(),
                error_key: tab.error_key.clone(),
                updated_at: tab.updated_at,
                terminal_id: Some(
                    record
                        .pane_key
                        .clone()
                        .unwrap_or_else(|| format!("client:{}", record.pid)),
                ),
                terminal_title: Some(
                    if record.orca {
                        "Orca terminal"
                    } else {
                        "OpenCode terminal"
                    }
                    .into(),
                ),
                selected_tab: tab.selected,
                client_id: Some(client_id.clone()),
                session_id: Some(tab.session_id.clone()),
                via_plugin: true,
                ..Default::default()
            });
        }
        presence.routes.insert(client_id, record);
    }
    // A shared session opened in multiple clients remains one recovery target.
    presence.targets.sort_by(|a, b| {
        a.id.cmp(&b.id)
            .then_with(|| (b.source == "orca").cmp(&(a.source == "orca")))
            .then_with(|| b.selected_tab.cmp(&a.selected_tab))
            .then_with(|| a.terminal_id.cmp(&b.terminal_id))
    });
    presence.targets.dedup_by(|a, b| a.id == b.id);
    presence
}

pub fn discover(shared_pid: u32) -> Result<Presence, String> {
    Ok(read_at(&root()?.join("presence"), shared_pid, now_ms()))
}

pub fn merge_orca(presence: &mut Presence, legacy: &[SessionTarget]) -> Vec<SessionTarget> {
    for target in &mut presence.targets {
        if target.source != "orca" {
            continue;
        }
        if let Some(terminal) = legacy
            .iter()
            .find(|legacy| legacy.terminal_id == target.terminal_id)
        {
            target.orca_handle = terminal.orca_handle.clone();
        }
    }
    let mut targets = presence
        .targets
        .iter()
        .filter(|t| t.source == "orca")
        .cloned()
        .collect::<Vec<_>>();
    targets.extend(
        legacy
            .iter()
            .filter(|t| {
                !t.terminal_id
                    .as_ref()
                    .is_some_and(|id| presence.covered_terminals.contains(id))
            })
            .cloned()
            .map(|mut target| {
                target.state = "unknown".into();
                target.last_error = None;
                target.error_key = None;
                target
            }),
    );
    targets
}

pub enum Delivery {
    Accepted(String),
    Busy,
}

pub async fn command(
    target: &SessionTarget,
    action: &str,
    prompt: Option<&str>,
) -> Result<Delivery, String> {
    command_at(&root()?, target, action, prompt, Duration::from_secs(12)).await
}

async fn command_at(
    root: &Path,
    target: &SessionTarget,
    action: &str,
    prompt: Option<&str>,
    timeout: Duration,
) -> Result<Delivery, String> {
    let presence = read_at(&root.join("presence"), 0, now_ms());
    let live = presence
        .targets
        .iter()
        .find(|t| t.id == target.id)
        .ok_or("The OpenCode session is no longer open in a live client")?;
    let client_id = live
        .client_id
        .as_ref()
        .ok_or("OpenCode client identity is missing")?;
    let session_id = live
        .session_id
        .as_ref()
        .ok_or("OpenCode session identity is missing")?;
    let route = presence
        .routes
        .get(client_id)
        .ok_or("OpenCode client is unavailable")?;
    if action == "prompt" && (!route.status_known || live.state == "unknown") {
        return Err("OpenCode execution status is unavailable".into());
    }
    if action == "prompt" && live.state == "busy" {
        return Ok(Delivery::Busy);
    }
    if prompt.is_some_and(|text| text.len() > 32000) {
        return Err("Continuation prompt is too long".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let request = root
        .join("requests")
        .join(client_id)
        .join(format!("{id}.json"));
    let receipt = root
        .join("receipts")
        .join(client_id)
        .join(format!("{id}.json"));
    let temporary = request.with_extension("tmp");
    let value = serde_json::json!({"id":id,"clientId":client_id,"servicePid":route.service_pid,
        "sessionID":session_id,"action":action,"prompt":prompt,"expiresAt":now_ms() + timeout.as_millis() as u64});
    std::fs::create_dir_all(request.parent().unwrap())
        .and_then(|_| std::fs::write(&temporary, value.to_string()))
        .and_then(|_| std::fs::rename(&temporary, &request))
        .map_err(|e| format!("Could not queue OpenCode request: {e}"))?;
    let deadline = tokio::time::Instant::now() + timeout;
    while tokio::time::Instant::now() < deadline {
        if let Some(value) = std::fs::read(&receipt)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        {
            match value["status"].as_str() {
                Some("accepted") => {
                    return value["receipt"]
                        .as_str()
                        .filter(|id| !id.is_empty())
                        .map(|id| Delivery::Accepted(id.into()))
                        .ok_or("OpenCode receipt is invalid".into())
                }
                Some("busy") => return Ok(Delivery::Busy),
                Some("rejected") => {
                    return Err(value["error"]
                        .as_str()
                        .unwrap_or("OpenCode request was rejected")
                        .into())
                }
                _ => {}
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let _ = std::fs::remove_file(request);
    Err("OpenCode delivery is uncertain; the request will not be resent automatically".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heartbeat(directory: &Path, client: &str, pid: u32, service: u32, orca: bool, time: u64) {
        std::fs::create_dir_all(directory).unwrap();
        std::fs::write(directory.join(format!("{pid}.json")), serde_json::json!({
            "version":2,"clientId":client,"pid":pid,"servicePid":service,"orca":orca,"updatedAt":time,
            "paneKey":if orca {Some("pane-one")} else {None}, "statusKnown":true,
            "busySessionIDs":["ses_background"], "sessions":[
                {"sessionID":"ses_idle","title":"Idle tab","project":"C:/project","state":"idle","selected":true},
                {"sessionID":"ses_background","title":"Working tab","project":"C:/project","state":"busy","selected":false}
            ]
        }).to_string()).unwrap();
    }

    #[test]
    fn live_tabs_include_private_clients_and_expire_without_history() {
        let root = tempfile::tempdir().unwrap();
        heartbeat(
            root.path(),
            "00000000-0000-4000-8000-000000000001",
            1,
            99,
            false,
            20000,
        );
        let live = read_at(root.path(), 10, 20000);
        assert_eq!(live.targets.len(), 2);
        assert!(live
            .targets
            .iter()
            .all(|t| t.source == "other" && t.via_plugin));
        assert!(live
            .targets
            .iter()
            .any(|t| t.state == "busy" && !t.selected_tab));
        assert!(live
            .targets
            .iter()
            .any(|t| t.state == "idle" && t.selected_tab));
        assert_eq!(live.busy.len(), 1);
        assert!(read_at(root.path(), 10, 30000).targets.is_empty());
        assert!(read_at(root.path(), 10, 10000).targets.is_empty());
    }

    #[test]
    fn same_session_in_two_clients_is_one_target_and_exact_orca_terminal_is_replaced() {
        let root = tempfile::tempdir().unwrap();
        heartbeat(
            root.path(),
            "00000000-0000-4000-8000-000000000001",
            1,
            10,
            false,
            20000,
        );
        heartbeat(
            root.path(),
            "00000000-0000-4000-8000-000000000002",
            2,
            10,
            true,
            20000,
        );
        let mut live = read_at(root.path(), 10, 20000);
        assert_eq!(live.targets.len(), 2);
        assert!(live.targets.iter().all(|t| t.source == "orca"));
        let legacy = vec![SessionTarget {
            id: "orca:one".into(),
            terminal_id: Some("pane-one".into()),
            orca_handle: Some("one".into()),
            ..Default::default()
        }];
        let merged = merge_orca(&mut live, &legacy);
        assert_eq!(merged.len(), 2);
        assert!(merged
            .iter()
            .all(|t| t.orca_handle.as_deref() == Some("one")));
        assert_eq!(live.busy.len(), 1);
    }

    #[test]
    fn malformed_and_unknown_status_reports_do_not_claim_idle() {
        let root = tempfile::tempdir().unwrap();
        heartbeat(
            root.path(),
            "00000000-0000-4000-8000-000000000001",
            1,
            10,
            false,
            20000,
        );
        let file = root.path().join("1.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        value["statusKnown"] = false.into();
        value["sessions"][0]["state"] = "unknown".into();
        std::fs::write(&file, value.to_string()).unwrap();
        std::fs::write(root.path().join("partial.json"), "{").unwrap();
        let live = read_at(root.path(), 10, 20000);
        assert!(live.unknown_clients);
        assert!(live.targets.iter().any(|t| t.state == "unknown"));
        value["clientId"] = "../../invalid".into();
        std::fs::write(&file, value.to_string()).unwrap();
        assert!(read_at(root.path(), 10, 20000).targets.is_empty());
    }

    #[tokio::test]
    async fn bridge_submits_exact_session_once_and_reads_its_receipt() {
        let root = tempfile::tempdir().unwrap();
        let client = "00000000-0000-4000-8000-000000000001";
        heartbeat(
            &root.path().join("presence"),
            client,
            1,
            99,
            false,
            now_ms(),
        );
        let target = read_at(&root.path().join("presence"), 0, now_ms())
            .targets
            .into_iter()
            .find(|t| t.state == "idle")
            .unwrap();
        let worker_root = root.path().to_path_buf();
        let worker = std::thread::spawn(move || {
            let requests = worker_root.join("requests").join(client);
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            loop {
                if let Some(file) = std::fs::read_dir(&requests).ok().and_then(|entries| {
                    entries.flatten().find(|entry| {
                        entry.path().extension().and_then(|s| s.to_str()) == Some("json")
                    })
                }) {
                    let request: serde_json::Value =
                        serde_json::from_slice(&std::fs::read(file.path()).unwrap()).unwrap();
                    assert_eq!(request["sessionID"], "ses_idle");
                    assert_eq!(request["servicePid"], 99);
                    assert_eq!(request["prompt"], "Continue exact session");
                    let receipts = worker_root.join("receipts").join(client);
                    std::fs::create_dir_all(&receipts).unwrap();
                    std::fs::write(
                        receipts.join(file.file_name()),
                        r#"{"status":"accepted","receipt":"inbox-one"}"#,
                    )
                    .unwrap();
                    return;
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        let result = command_at(
            root.path(),
            &target,
            "prompt",
            Some("Continue exact session"),
            Duration::from_secs(3),
        )
        .await
        .unwrap();
        assert!(matches!(result, Delivery::Accepted(id) if id == "inbox-one"));
        worker.join().unwrap();
    }

    #[tokio::test]
    async fn busy_and_closed_targets_are_not_enqueued_and_timeouts_are_uncertain() {
        let root = tempfile::tempdir().unwrap();
        let client = "00000000-0000-4000-8000-000000000001";
        heartbeat(
            &root.path().join("presence"),
            client,
            1,
            99,
            false,
            now_ms(),
        );
        let live = read_at(&root.path().join("presence"), 0, now_ms());
        let busy = live.targets.iter().find(|t| t.state == "busy").unwrap();
        assert!(matches!(
            command_at(
                root.path(),
                busy,
                "prompt",
                Some("Continue"),
                Duration::from_millis(150)
            )
            .await
            .unwrap(),
            Delivery::Busy
        ));
        assert!(!root.path().join("requests").exists());
        let idle = live.targets.iter().find(|t| t.state == "idle").unwrap();
        let error = command_at(
            root.path(),
            idle,
            "prompt",
            Some("Continue"),
            Duration::from_millis(150),
        )
        .await
        .err()
        .unwrap();
        assert!(error.contains("uncertain"));
        assert_eq!(
            std::fs::read_dir(root.path().join("requests").join(client))
                .unwrap()
                .count(),
            0
        );
        std::fs::remove_file(root.path().join("presence/1.json")).unwrap();
        assert!(
            command_at(root.path(), idle, "focus", None, Duration::from_millis(150))
                .await
                .is_err()
        );
    }
    #[test]
    fn installs_only_its_own_plugin() {
        let directory = tempfile::tempdir().unwrap();
        let own = directory.path().join("opc-rotate-presence");
        std::fs::write(directory.path().join("other.js"), "user plugin").unwrap();
        install_at(&own).unwrap();
        install_at(&own).unwrap();
        assert_eq!(std::fs::read_to_string(own.join("tui.js")).unwrap(), PLUGIN);
        assert_eq!(
            std::fs::read_to_string(directory.path().join("other.js")).unwrap(),
            "user plugin"
        );
    }
}
