use serde::{Deserialize, Serialize};

pub const DEFAULT_PROMPT: &str = "Continue from where you stopped after the rate limit. Check the current state and avoid repeating completed actions.";

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
    pub global_prompt: String,
    pub prompt_overrides: std::collections::HashMap<String, String>,
    pub managed_profile: Option<String>,
    pub preferred_server: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            monitoring: false,
            global_prompt: DEFAULT_PROMPT.to_string(),
            prompt_overrides: Default::default(),
            managed_profile: None,
            preferred_server: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub monitoring: bool,
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
