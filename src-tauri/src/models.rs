use serde::{Deserialize, Serialize};

pub const DEFAULT_PROMPT: &str = "Continue from where you stopped after the rate limit";
pub const DEFAULT_CONTINUE_PROMPT: &str = "Continue where you left off.";
/// Watch for provider rate-limit failures and recover by changing the public IP.
pub const WATCH_RATE_LIMIT: &str = "rate-limit";
/// Watch for a finished turn and continue it with a prompt, without changing the IP.
pub const WATCH_CONTINUE: &str = "continue";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VpnServer {
    pub id: String,
    pub host: String,
    pub ip: String,
    pub port: u16,
    pub country: String,
    pub ping_ms: u32,
    pub speed_mbps: f64,
    pub sessions: u32,
    pub score: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTarget {
    pub id: String,
    pub source: String,
    pub title: String,
    pub project: String,
    pub state: String,
    pub last_error: Option<String>,
    pub error_key: Option<String>,
    pub prompt_override: Option<String>,
    pub updated_at: u64,
    #[serde(default)]
    pub terminal_id: Option<String>,
    #[serde(default)]
    pub terminal_title: Option<String>,
    #[serde(default)]
    pub selected_tab: bool,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub orca_handle: Option<String>,
    #[serde(default)]
    pub via_plugin: bool,
    /// Identifies the operator input a stopped session is waiting on.
    #[serde(default)]
    pub stop_key: Option<String>,
    /// Child sessions of this session that are still executing.
    #[serde(default)]
    pub working_subagents: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub at: u64,
    pub level: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryProgress {
    pub stage: String,
    pub detail: String,
    pub updated_at: u64,
    pub retry_at: Option<u64>,
    pub ip_changes: u8,
    pub max_ip_changes: u8,
    pub last_sent_at: Option<u64>,
    /// Whether this recovery changes the machine's public IP.
    #[serde(default = "default_true")]
    pub changes_ip: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub monitoring: bool,
    #[serde(default)]
    pub session_monitoring: std::collections::HashMap<String, bool>,
    #[serde(default)]
    pub terminal_monitoring: std::collections::HashMap<String, bool>,
    #[serde(default)]
    pub session_terminals: std::collections::HashMap<String, String>,
    pub global_prompt: String,
    pub prompt_overrides: std::collections::HashMap<String, String>,
    /// The watcher mode every session follows unless it overrides it.
    #[serde(default = "default_watcher")]
    pub watcher: String,
    #[serde(default)]
    pub session_watcher: std::collections::HashMap<String, String>,
    /// The prompt sent to a session that simply finished its turn.
    #[serde(default = "default_continue_prompt")]
    pub continue_prompt: String,
    pub managed_profile: Option<String>,
    pub preferred_server: Option<String>,
}

fn default_watcher() -> String {
    WATCH_RATE_LIMIT.into()
}

fn default_continue_prompt() -> String {
    DEFAULT_CONTINUE_PROMPT.into()
}

impl Settings {
    pub fn monitors(&self, target: &SessionTarget) -> bool {
        self.session_monitoring
            .get(&target.id)
            .copied()
            .or_else(|| {
                terminal_key(target)
                    .or_else(|| self.session_terminals.get(&target.id).cloned())
                    .and_then(|key| self.terminal_monitoring.get(&key).copied())
            })
            .unwrap_or(true)
    }

    pub fn monitor_terminal(
        &mut self,
        target: &SessionTarget,
        targets: &[SessionTarget],
        enabled: bool,
    ) -> Result<(), String> {
        let key =
            terminal_key(target).ok_or("Terminal identity is unavailable for this session")?;
        self.terminal_monitoring.insert(key.clone(), enabled);
        for (id, owner) in &self.session_terminals {
            if owner == &key {
                self.session_monitoring.remove(id);
            }
        }
        for sibling in targets
            .iter()
            .filter(|item| terminal_key(item).as_ref() == Some(&key))
        {
            self.session_monitoring.remove(&sibling.id);
        }
        Ok(())
    }

    pub fn migrate_default_prompt(&mut self) {
        if self.global_prompt == "Continue from where you stopped after the rate limit. Check the current state and avoid repeating completed actions." {
            self.global_prompt = DEFAULT_PROMPT.into();
        }
    }

    /// The watcher mode that applies to a session: its own choice, then the
    /// app-wide default.
    pub fn watcher_for(&self, target: &SessionTarget) -> &str {
        self.session_watcher
            .get(&target.id)
            .map(String::as_str)
            .unwrap_or(self.watcher.as_str())
    }

    pub fn watches_continue(&self, target: &SessionTarget) -> bool {
        self.watcher_for(target) == WATCH_CONTINUE
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            monitoring: false,
            session_monitoring: Default::default(),
            terminal_monitoring: Default::default(),
            session_terminals: Default::default(),
            global_prompt: DEFAULT_PROMPT.to_string(),
            prompt_overrides: Default::default(),
            watcher: WATCH_RATE_LIMIT.to_string(),
            session_watcher: Default::default(),
            continue_prompt: DEFAULT_CONTINUE_PROMPT.to_string(),
            managed_profile: None,
            preferred_server: None,
        }
    }
}

pub fn terminal_key(target: &SessionTarget) -> Option<String> {
    target
        .terminal_id
        .as_ref()
        .map(|id| format!("{}:{id}", target.source))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_selection_inherits_for_new_tabs_and_session_overrides_are_saved() {
        let first = SessionTarget {
            id: "api:first".into(),
            source: "orca".into(),
            terminal_id: Some("pane".into()),
            ..Default::default()
        };
        let second = SessionTarget {
            id: "api:second".into(),
            ..first.clone()
        };
        let future = SessionTarget {
            id: "api:future".into(),
            ..first.clone()
        };
        let outside = SessionTarget {
            id: "api:outside".into(),
            source: "other".into(),
            ..first.clone()
        };
        let mut settings = Settings::default();
        settings.session_monitoring.insert(first.id.clone(), true);
        settings.session_monitoring.insert(future.id.clone(), true);
        settings
            .session_terminals
            .insert(future.id.clone(), "orca:pane".into());
        settings
            .monitor_terminal(
                &first,
                &[first.clone(), second.clone(), outside.clone()],
                false,
            )
            .unwrap();
        assert!(!settings.monitors(&first));
        assert!(!settings.monitors(&second));
        assert!(!settings.monitors(&future));
        assert!(settings.monitors(&outside));
        settings.session_monitoring.insert(second.id.clone(), true);
        let saved: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert!(!saved.monitors(&first));
        assert!(saved.monitors(&second));
        assert!(!saved.monitors(&future));
        settings
            .monitor_terminal(&first, &[first.clone(), second.clone()], true)
            .unwrap();
        assert!(
            settings.monitors(&first) && settings.monitors(&second) && settings.monitors(&future)
        );
        assert!(settings.session_monitoring.is_empty());
    }

    #[test]
    fn terminal_exclusions_survive_api_fallback_without_terminal_metadata() {
        let mut settings = Settings::default();
        settings
            .terminal_monitoring
            .insert("other:client:123".into(), false);
        settings
            .session_terminals
            .insert("api:one".into(), "other:client:123".into());
        let fallback = SessionTarget {
            id: "api:one".into(),
            source: "other".into(),
            ..Default::default()
        };
        assert!(!settings.monitors(&fallback));
        let saved: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert!(!saved.monitors(&fallback));
        settings
            .session_monitoring
            .insert(fallback.id.clone(), true);
        assert!(settings.monitors(&fallback));
    }

    #[test]
    fn old_settings_keep_existing_monitoring_behavior_and_api_targets_stay_individual() {
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        value.as_object_mut().unwrap().remove("sessionMonitoring");
        value.as_object_mut().unwrap().remove("terminalMonitoring");
        value.as_object_mut().unwrap().remove("sessionTerminals");
        let mut settings: Settings = serde_json::from_value(value).unwrap();
        let target = SessionTarget {
            id: "api:one".into(),
            source: "other".into(),
            ..Default::default()
        };
        assert!(settings.monitors(&target));
        assert!(settings
            .monitor_terminal(&target, &[target.clone()], false)
            .is_err());
        settings.session_monitoring.insert(target.id.clone(), false);
        assert!(!settings.monitors(&target));
    }

    #[test]
    fn watcher_precedence_prefers_the_session_then_the_global_default() {
        let mut settings = Settings::default();
        assert_eq!(settings.watcher, WATCH_RATE_LIMIT);
        assert_eq!(settings.continue_prompt, DEFAULT_CONTINUE_PROMPT);

        let session = SessionTarget {
            id: "api:one".into(),
            source: "other".into(),
            terminal_id: Some("pane".into()),
            ..Default::default()
        };
        // With no override the global default applies.
        assert_eq!(settings.watcher_for(&session), WATCH_RATE_LIMIT);
        assert!(!settings.watches_continue(&session));

        settings.watcher = WATCH_CONTINUE.into();
        // A session with no override follows the global default.
        assert_eq!(settings.watcher_for(&session), WATCH_CONTINUE);
        assert!(settings.watches_continue(&session));

        settings
            .session_watcher
            .insert(session.id.clone(), WATCH_RATE_LIMIT.into());
        // The session's own choice wins over the global default.
        assert_eq!(settings.watcher_for(&session), WATCH_RATE_LIMIT);
        assert!(!settings.watches_continue(&session));

        // A sibling with no override still follows the global default.
        let sibling = SessionTarget {
            id: "api:two".into(),
            ..session.clone()
        };
        assert_eq!(settings.watcher_for(&sibling), WATCH_CONTINUE);
        assert!(settings.watches_continue(&sibling));
    }

    #[test]
    fn old_settings_keep_the_rate_limit_watcher_and_the_default_continue_prompt() {
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        value.as_object_mut().unwrap().remove("watcher");
        value.as_object_mut().unwrap().remove("sessionWatcher");
        value.as_object_mut().unwrap().remove("continuePrompt");
        let settings: Settings = serde_json::from_value(value).unwrap();
        assert_eq!(settings.watcher, WATCH_RATE_LIMIT);
        assert_eq!(settings.continue_prompt, DEFAULT_CONTINUE_PROMPT);
        assert!(settings.session_watcher.is_empty());
        let target = SessionTarget {
            id: "api:one".into(),
            source: "other".into(),
            ..Default::default()
        };
        assert_eq!(settings.watcher_for(&target), WATCH_RATE_LIMIT);
        assert!(!settings.watches_continue(&target));

        // An old session record keeps working without the new fields.
        let old_target = serde_json::json!({
            "id": "api:one",
            "source": "other",
            "title": "One",
            "project": "C:/project",
            "state": "idle",
            "lastError": null,
            "errorKey": null,
            "promptOverride": null,
            "updatedAt": 0,
        });
        let target: SessionTarget = serde_json::from_value(old_target).unwrap();
        assert!(target.stop_key.is_none());
        assert_eq!(target.working_subagents, 0);
    }

    #[test]
    fn migrates_the_previous_default_without_replacing_custom_prompts() {
        let mut settings = Settings {
            global_prompt: "Continue from where you stopped after the rate limit. Check the current state and avoid repeating completed actions.".into(),
            ..Settings::default()
        };
        settings.prompt_overrides.insert(
            "api:session".into(),
            "Check the current branch first".into(),
        );
        settings.migrate_default_prompt();
        assert_eq!(
            settings.global_prompt,
            "Continue from where you stopped after the rate limit"
        );
        assert_eq!(
            settings.prompt_overrides["api:session"],
            "Check the current branch first"
        );
        settings.global_prompt = "Resume the documentation review".into();
        settings.migrate_default_prompt();
        assert_eq!(settings.global_prompt, "Resume the documentation review");
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub recovery: std::collections::HashMap<String, RecoveryProgress>,
    pub monitoring: bool,
    pub session_monitoring: std::collections::HashMap<String, bool>,
    pub terminal_monitoring: std::collections::HashMap<String, bool>,
    pub session_terminals: std::collections::HashMap<String, String>,
    pub phase: String,
    pub vpn_status: String,
    pub public_ip: Option<String>,
    pub connected_server: Option<VpnServer>,
    pub servers: Vec<VpnServer>,
    pub refreshed_at: Option<u64>,
    pub orca_sessions: Vec<SessionTarget>,
    pub other_sessions: Vec<SessionTarget>,
    pub other_sessions_notice: Option<String>,
    pub history: Vec<HistoryEntry>,
    pub global_prompt: String,
    pub watcher: String,
    pub session_watcher: std::collections::HashMap<String, String>,
    pub continue_prompt: String,
    /// Whether anything on screen still depends on the VPN route.
    pub vpn_needed: bool,
    pub preferred_server: Option<String>,
    pub last_error: Option<String>,
}
