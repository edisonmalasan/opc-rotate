use crate::process::now_ms;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const PLUGIN: &str = include_str!("../../integrations/opencode/tui.js");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Heartbeat {
    version: u8,
    pid: u32,
    service_pid: u32,
    #[serde(rename = "sessionID")]
    session_id: Option<String>,
    orca: bool,
    updated_at: u64,
}

#[derive(Default)]
pub struct Presence {
    pub open: HashSet<String>,
    pub orca: HashSet<String>,
    pub clients: usize,
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

fn read_at(directory: &Path, service_pid: u32, now: u64) -> Presence {
    let mut presence = Presence::default();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return presence;
    };
    for entry in entries.flatten().take(256) {
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json")
            || entry.metadata().map(|m| m.len() > 8192).unwrap_or(true)
        {
            continue;
        }
        let Some(record) = std::fs::read(entry.path())
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Heartbeat>(&bytes).ok())
        else {
            continue;
        };
        // A short lease bounds stale visibility after a crash; never use saved TUI tabs.
        if record.version != 1
            || record.pid == 0
            || record.service_pid != service_pid
            || record.updated_at > now + 1000
            || now.saturating_sub(record.updated_at) > 8000
        {
            continue;
        }
        if record.session_id.as_ref().is_some_and(|id| !valid_id(id)) {
            continue;
        }
        if !record.orca {
            presence.clients += 1;
        }
        if let Some(id) = record.session_id {
            if record.orca {
                presence.orca.insert(id);
            } else {
                presence.open.insert(id);
            }
        }
    }
    presence
}

pub fn discover(service_pid: u32) -> Result<Presence, String> {
    let home = dirs::home_dir().ok_or("Home directory not found")?;
    Ok(read_at(
        &home.join(".local/state/opc-rotate/presence"),
        service_pid,
        now_ms(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_live_matching_service_presence_is_visible() {
        let directory = tempfile::tempdir().unwrap();
        for (pid, service, time, session, orca) in [
            (1, 10, 20000, "outside", false),
            (2, 10, 20000, "orca", true),
            (3, 11, 20000, "standalone", false),
            (4, 10, 10000, "closed", false),
            (5, 10, 25000, "future", false),
            (6, 10, 20000, "../bad", false),
        ] {
            std::fs::write(directory.path().join(format!("{pid}.json")), serde_json::json!({
                "version":1,"pid":pid,"servicePid":service,"updatedAt":time,"sessionID":session,"orca":orca
            }).to_string()).unwrap();
        }
        std::fs::write(directory.path().join("partial.json"), "{").unwrap();
        let presence = read_at(directory.path(), 10, 20000);
        assert_eq!(presence.open, HashSet::from(["outside".into()]));
        assert_eq!(presence.orca, HashSet::from(["orca".into()]));
        assert!(read_at(directory.path(), 10, 40000).open.is_empty());
    }

    #[test]
    fn installs_only_its_own_plugin_and_updates_it_idempotently() {
        let directory = tempfile::tempdir().unwrap();
        let own = directory.path().join("opc-rotate-presence");
        let unrelated = directory.path().join("other.js");
        std::fs::write(&unrelated, "user plugin").unwrap();
        install_at(&own).unwrap();
        install_at(&own).unwrap();
        assert_eq!(std::fs::read_to_string(own.join("tui.js")).unwrap(), PLUGIN);
        assert_eq!(std::fs::read_to_string(unrelated).unwrap(), "user plugin");
    }
}
