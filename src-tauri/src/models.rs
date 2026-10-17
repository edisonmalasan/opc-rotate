use serde::{Deserialize, Serialize};

pub const DEFAULT_PROMPT: &str = "Continue from where you stopped after the rate limit";

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
    pub managed_profile: Option<String>,
    pub preferred_server: Option<String>,
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
    pub preferred_server: Option<String>,
    pub last_error: Option<String>,
}
