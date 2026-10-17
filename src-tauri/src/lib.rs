mod models;
mod opencode;
mod orca;
mod presence;
mod process;
mod softether;
mod vpngate;

use models::{HistoryEntry, SessionTarget, Settings, Snapshot, VpnServer};
use serde::{Deserialize, Serialize};
use softether::{RealVpnRunner, SoftEther};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

const BUSY_DEFERRED: &str = "Other OpenCode sessions are still active; recovery remains queued";

#[derive(Serialize, Deserialize, Default)]
struct Persisted {
    settings: Settings,
    history: Vec<HistoryEntry>,
    connected_server: Option<VpnServer>,
    #[serde(default)]
    handled: HashSet<String>,
    #[serde(default)]
    ip_attempts: HashMap<String, u8>,
}

struct Inner {
    settings: Settings,
    servers: Vec<VpnServer>,
    refreshed_at: Option<u64>,
    orca_sessions: Vec<SessionTarget>,
    other_sessions: Vec<SessionTarget>,
    other_sessions_notice: Option<String>,
    previously_active_other: HashSet<String>,
    history: Vec<HistoryEntry>,
    phase: String,
    vpn_status: String,
    public_ip: Option<String>,
    connected_server: Option<VpnServer>,
    last_error: Option<String>,
    pending: HashSet<String>,
    ready_to_resume: HashSet<String>,
    handled: HashSet<String>,
    ip_attempts: HashMap<String, u8>,
    last_recovery_at: HashMap<String, u64>,
    recovery_running: bool,
    pause_epoch: u64,
}

enum RecoveryPlan {
    None,
    Resume(Vec<String>),
    Rotate(Vec<String>),
}

impl Inner {
    fn prune_unmonitored(&mut self) {
        let allowed = self
            .orca_sessions
            .iter()
            .chain(&self.other_sessions)
            .filter(|target| self.settings.monitors(target))
            .map(|target| target.id.clone())
            .collect::<HashSet<_>>();
        self.pending.retain(|id| allowed.contains(id));
        self.ready_to_resume.retain(|id| allowed.contains(id));
    }

    fn recovery_plan(&self) -> RecoveryPlan {
        if self.pending.is_empty() {
            return RecoveryPlan::None;
        }
        let ready = self
            .pending
            .intersection(&self.ready_to_resume)
            .cloned()
            .collect::<Vec<_>>();
        if !ready.is_empty() {
            RecoveryPlan::Resume(ready)
        } else {
            RecoveryPlan::Rotate(self.pending.iter().cloned().collect())
        }
    }

    fn mark_verified_ip(&mut self) {
        for id in self.pending.clone() {
            *self.ip_attempts.entry(id.clone()).or_default() += 1;
            self.ready_to_resume.insert(id);
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            monitoring: self.settings.monitoring,
            session_monitoring: self.settings.session_monitoring.clone(),
            terminal_monitoring: self.settings.terminal_monitoring.clone(),
            session_terminals: self.settings.session_terminals.clone(),
            phase: self.phase.clone(),
            vpn_status: self.vpn_status.clone(),
            public_ip: self.public_ip.clone(),
            connected_server: self.connected_server.clone(),
            servers: self.servers.clone(),
            refreshed_at: self.refreshed_at,
            orca_sessions: self.orca_sessions.clone(),
            other_sessions: self.other_sessions.clone(),
            other_sessions_notice: self.other_sessions_notice.clone(),
            history: self.history.clone(),
            global_prompt: self.settings.global_prompt.clone(),
            preferred_server: self.settings.preferred_server.clone(),
            last_error: self.last_error.clone(),
        }
    }

    fn persisted(&self) -> Persisted {
        Persisted {
            settings: self.settings.clone(),
            history: self.history.clone(),
            connected_server: self.connected_server.clone(),
            handled: self.handled.clone(),
            ip_attempts: self.ip_attempts.clone(),
        }
    }

    fn add_history(&mut self, level: &str, message: impl Into<String>) {
        self.history.insert(
            0,
            HistoryEntry {
                at: process::now_ms(),
                level: level.into(),
                message: message.into(),
            },
        );
        self.history.truncate(100);
    }

    fn target(&self, id: &str) -> Option<SessionTarget> {
        self.orca_sessions
            .iter()
            .chain(&self.other_sessions)
            .find(|target| target.id == id)
            .cloned()
    }
}

struct Controller {
    inner: Mutex<Inner>,
    http: reqwest::Client,
    settings_path: PathBuf,
}

impl Controller {
    fn new() -> Self {
        let settings_path = dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("opc-rotate")
            .join("settings.json");
        let mut persisted: Persisted = std::fs::read_to_string(&settings_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default();
        persisted.settings.migrate_default_prompt();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(12))
            .build()
            .expect("HTTP client should initialize");
        let vpn_status = if persisted.settings.managed_profile.is_some() {
            "checking"
        } else {
            "disconnected"
        };
        Self {
            inner: Mutex::new(Inner {
                settings: persisted.settings,
                servers: Vec::new(),
                refreshed_at: None,
                orca_sessions: Vec::new(),
                other_sessions: Vec::new(),
                other_sessions_notice: None,
                previously_active_other: HashSet::new(),
                history: persisted.history,
                phase: "ready".into(),
                vpn_status: vpn_status.into(),
                public_ip: None,
                connected_server: persisted.connected_server,
                last_error: None,
                pending: HashSet::new(),
                ready_to_resume: HashSet::new(),
                handled: persisted.handled,
                ip_attempts: persisted.ip_attempts,
                last_recovery_at: HashMap::new(),
                recovery_running: false,
                pause_epoch: 0,
            }),
            http,
            settings_path,
        }
    }

    async fn snapshot(&self) -> Snapshot {
        self.inner.lock().await.snapshot()
    }

    async fn emit(&self, app: &AppHandle) {
        let _ = app.emit("snapshot", self.snapshot().await);
    }

    async fn save(&self) {
        let data = self.inner.lock().await.persisted();
        if let Some(parent) = self.settings_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_vec_pretty(&data) {
            let tmp = self.settings_path.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, &self.settings_path);
            }
        }
    }

    async fn record(&self, app: &AppHandle, level: &str, message: impl Into<String>) {
        self.inner.lock().await.add_history(level, message);
        self.save().await;
        self.emit(app).await;
    }

    async fn set_error(&self, app: &AppHandle, message: impl Into<String>) {
        let message = message.into();
        {
            let mut inner = self.inner.lock().await;
            inner.last_error = Some(message.clone());
            inner.phase = "attention".into();
        }
        self.record(app, "error", message).await;
    }
}

async fn public_ip(_http: &reqwest::Client) -> Result<String, String> {
    // Do not reuse a socket opened before the VPN route changed.
    let client = reqwest::Client::builder()
        .no_proxy()
        .pool_max_idle_per_host(0)
        .timeout(Duration::from_secs(7))
        .build()
        .map_err(|e| e.to_string())?;
    let value: serde_json::Value = client
        .get(format!(
            "https://api4.ipify.org?format=json&t={}",
            process::now_ms()
        ))
        .header(reqwest::header::CONNECTION, "close")
        .header(reqwest::header::CACHE_CONTROL, "no-cache")
        .send()
        .await
        .map_err(|e| format!("Could not check public IP: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Public IP check failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Public IP response was invalid: {e}"))?;
    value["ip"]
        .as_str()
        .map(str::to_string)
        .ok_or("Public IP response contained no address".into())
}

async fn refresh_catalog(app: &AppHandle, controller: &Arc<Controller>) -> Result<(), String> {
    let servers = vpngate::fetch_servers(&controller.http).await?;
    let count = servers.len();
    {
        let mut inner = controller.inner.lock().await;
        inner.servers = servers;
        inner.refreshed_at = Some(process::now_ms());
        inner.last_error = None;
    }
    controller
        .record(
            app,
            "info",
            format!("Refreshed {count} eligible VPN Gate relays"),
        )
        .await;
    Ok(())
}

async fn scan_sessions(app: &AppHandle, controller: &Arc<Controller>) {
    let plugin_error = presence::install().err();
    let client = opencode::OpenCodeClient::discover(controller.http.clone());
    let mut live =
        presence::discover(client.as_ref().map(|c| c.service_pid).unwrap_or(0)).unwrap_or_default();
    let orca_result = orca::list_targets().await;
    let orca_targets = presence::merge_orca(&mut live, orca_result.as_deref().unwrap_or(&[]));
    let previously_active = controller
        .inner
        .lock()
        .await
        .previously_active_other
        .clone();
    let api_result = match client {
        Ok(client) => client.list_targets(&live, &previously_active).await,
        Err(error) => Err(error),
    };
    let mut other_targets = live
        .targets
        .iter()
        .filter(|t| t.source == "other")
        .cloned()
        .collect::<Vec<_>>();
    other_targets.extend(api_result.as_ref().cloned().unwrap_or_default());
    let notice = plugin_error.or_else(|| {
        if live.targets.is_empty() { api_result.as_ref().err().cloned().or_else(|| Some(
            "OpenCode tabs appear after the local presence plugin loads. If an open client is missing, reopen it once.".into()))
        } else { None }
    });
    let (new_incidents, bindings_changed) = {
        let mut inner = controller.inner.lock().await;
        inner.other_sessions_notice = notice;
        inner.orca_sessions = retain_incidents(&inner.orca_sessions, orca_targets);
        inner.other_sessions = retain_incidents(&inner.other_sessions, other_targets);
        inner.previously_active_other = inner
            .other_sessions
            .iter()
            .filter(|t| !t.via_plugin && (t.state == "busy" || t.state == "rate-limited"))
            .filter_map(|t| t.id.strip_prefix("api:").map(str::to_string))
            .collect();
        let available = inner
            .orca_sessions
            .iter()
            .chain(inner.other_sessions.iter())
            .filter(|t| {
                (!t.via_plugin || t.client_id.is_some())
                    && (t.source != "orca" || t.client_id.is_some())
            })
            .map(|t| t.id.clone())
            .collect::<HashSet<_>>();
        inner.pending.retain(|id| available.contains(id));
        inner.ready_to_resume.retain(|id| available.contains(id));
        let overrides = inner.settings.prompt_overrides.clone();
        for target in inner.orca_sessions.iter_mut() {
            target.prompt_override = overrides.get(&target.id).cloned();
        }
        for target in inner.other_sessions.iter_mut() {
            target.prompt_override = overrides.get(&target.id).cloned();
        }
        let bindings = inner
            .orca_sessions
            .iter()
            .chain(&inner.other_sessions)
            .filter_map(|target| models::terminal_key(target).map(|key| (target.id.clone(), key)))
            .collect::<Vec<_>>();
        let mut bindings_changed = false;
        for (id, key) in bindings {
            if inner.settings.session_terminals.get(&id) != Some(&key) {
                inner.settings.session_terminals.insert(id, key);
                bindings_changed = true;
            }
        }
        inner.prune_unmonitored();
        (queue_incidents(&mut inner), bindings_changed)
    };
    if bindings_changed {
        controller.save().await;
    }
    if !new_incidents.is_empty() {
        controller
            .record(
                app,
                "warning",
                format!("Recovery trigger detected: {}", new_incidents.join(", ")),
            )
            .await;
    } else {
        controller.emit(app).await;
    }
    start_recovery_if_needed(app.clone(), controller.clone()).await;
}

fn queue_incidents(inner: &mut Inner) -> Vec<String> {
    let mut new_incidents = Vec::new();
    let targets = inner
        .orca_sessions
        .iter()
        .chain(inner.other_sessions.iter())
        .cloned()
        .collect::<Vec<_>>();
    for target in targets {
        if !inner.settings.monitors(&target) {
            continue;
        }
        if target.state != "rate-limited" {
            if target.state == "completed" {
                inner.ip_attempts.remove(&target.id);
                inner.last_recovery_at.remove(&target.id);
            }
            continue;
        }
        if (target.via_plugin && target.client_id.is_none())
            || (target.source == "orca" && !target.via_plugin)
        {
            continue;
        }
        let Some(key) = &target.error_key else {
            continue;
        };
        if inner.handled.contains(key)
            || inner.pending.contains(&target.id)
            || !inner.settings.monitoring
        {
            continue;
        }
        if target.source == "orca"
            && inner
                .last_recovery_at
                .get(&target.id)
                .is_some_and(|at| process::now_ms().saturating_sub(*at) < 30_000)
        {
            continue;
        }
        if inner.ip_attempts.get(&target.id).copied().unwrap_or(0) >= 2 {
            inner.handled.insert(key.clone());
            inner.last_error = Some(format!(
                "Recovery error persisted after two new IPs for {}",
                target.title
            ));
            inner.add_history(
                "error",
                format!(
                    "Stopped rotating for {} after two verified IPs",
                    target.title
                ),
            );
        } else {
            inner.pending.insert(target.id.clone());
            new_incidents.push(target.title.clone());
        }
    }
    new_incidents
}

async fn start_recovery_if_needed(app: AppHandle, controller: Arc<Controller>) {
    {
        let mut inner = controller.inner.lock().await;
        if inner.recovery_running || inner.pending.is_empty() {
            return;
        }
        inner.recovery_running = true;
        inner.phase = "waiting".into();
    }
    controller.emit(&app).await;
    tauri::async_runtime::spawn(async move {
        if let Err(error) = recover(&app, &controller).await {
            if error == BUSY_DEFERRED {
                let mut inner = controller.inner.lock().await;
                inner.recovery_running = false;
                inner.phase = "waiting for active sessions".into();
                drop(inner);
                controller.emit(&app).await;
                return;
            }
            let mut inner = controller.inner.lock().await;
            let failed = inner.pending.drain().collect::<Vec<_>>();
            for id in failed {
                if let Some(key) = inner.target(&id).and_then(|target| target.error_key) {
                    inner.handled.insert(key);
                }
            }
            drop(inner);
            controller.set_error(&app, error).await;
        }
        {
            let mut inner = controller.inner.lock().await;
            inner.recovery_running = false;
            if inner.phase != "attention" {
                inner.phase = if inner.ready_to_resume.is_empty() {
                    "ready"
                } else {
                    "waiting for affected sessions"
                }
                .into();
            }
        }
        controller.emit(&app).await;
    });
}

async fn recover(app: &AppHandle, controller: &Arc<Controller>) -> Result<(), String> {
    let plan = controller.inner.lock().await.recovery_plan();
    let pending = match plan {
        RecoveryPlan::None => return Ok(()),
        RecoveryPlan::Resume(ready) => {
            resume_pending(app, controller, &ready).await;
            return Ok(());
        }
        RecoveryPlan::Rotate(pending) => pending,
    };

    // A VPN route change can break another provider call, including one in an Orca terminal.
    for _ in 0..24 {
        if controller.inner.lock().await.pending.is_empty() {
            return Ok(());
        }
        let busy = busy_session_count(controller).await?;
        if busy == 0 {
            break;
        }
        {
            let mut inner = controller.inner.lock().await;
            inner.phase = format!("waiting for {busy} active session(s)");
        }
        controller.emit(app).await;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    let busy = busy_session_count(controller).await?;
    if busy > 0 {
        return Err(BUSY_DEFERRED.into());
    }

    let baseline_ip = public_ip(&controller.http).await?;
    let (mut servers, previous_profile, previous_server, preferred) = {
        let inner = controller.inner.lock().await;
        (
            inner.servers.clone(),
            inner.settings.managed_profile.clone(),
            inner.connected_server.clone(),
            inner.settings.preferred_server.clone(),
        )
    };
    if servers.is_empty()
        || controller
            .inner
            .lock()
            .await
            .refreshed_at
            .is_none_or(|at| process::now_ms().saturating_sub(at) > 5 * 60 * 1000)
    {
        refresh_catalog(app, controller).await?;
        servers = controller.inner.lock().await.servers.clone();
    }
    servers.retain(|s| {
        previous_server
            .as_ref()
            .is_none_or(|previous| previous.id != s.id)
    });
    if let Some(preferred) = preferred {
        if let Some(index) = servers.iter().position(|server| server.id == preferred) {
            let server = servers.remove(index);
            servers.insert(0, server);
        }
    }
    servers.truncate(5);
    if servers.is_empty() {
        return Err("No different VPN Gate relay is available".into());
    }
    let runner = RealVpnRunner::discover()?;
    if controller.inner.lock().await.pending.is_empty() {
        return Ok(());
    }
    let vpn = SoftEther::new(runner, "VPN");
    for profile in vpn.list_profiles().await? {
        if profile.starts_with("OPCRotate_")
            && Some(profile.as_str()) != previous_profile.as_deref()
        {
            vpn.cleanup(&profile).await?;
            controller
                .record(
                    app,
                    "warning",
                    format!("Removed incomplete app profile {profile}"),
                )
                .await;
        }
    }
    let connected_profiles = vpn.connected_profiles().await?;
    let unmanaged = connected_profiles
        .iter()
        .filter(|profile| Some((*profile).as_str()) != previous_profile.as_deref())
        .collect::<Vec<_>>();
    if !unmanaged.is_empty() {
        return Err("Another SoftEther connection is active. Disconnect it manually before automatic recovery.".into());
    }
    if let Some(profile) = previous_profile {
        vpn.cleanup(&profile).await?;
        let mut inner = controller.inner.lock().await;
        inner.settings.managed_profile = None;
        inner.connected_server = None;
        inner.vpn_status = "disconnected".into();
        drop(inner);
        controller.save().await;
    }
    for server in servers {
        if controller.inner.lock().await.pending.is_empty() {
            return Ok(());
        }
        let profile = format!(
            "OPCRotate_{}",
            &uuid::Uuid::new_v4().simple().to_string()[..12]
        );
        {
            let mut inner = controller.inner.lock().await;
            inner.phase = format!("connecting to {}", server.host);
            inner.vpn_status = "connecting".into();
        }
        controller.emit(app).await;
        match vpn
            .connect(&server, &profile, Duration::from_secs(20))
            .await
        {
            Ok(()) => {}
            Err(error) => {
                if error.contains("cleanup failed") {
                    return Err(error);
                }
                controller
                    .record(app, "warning", format!("{} failed: {error}", server.host))
                    .await;
                continue;
            }
        }
        let mut new_ip = None;
        for _ in 0..4 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if let Ok(ip) = public_ip(&controller.http).await {
                if ip != baseline_ip {
                    new_ip = Some(ip);
                    break;
                }
            }
        }
        let Some(ip) = new_ip else {
            vpn.cleanup(&profile).await?;
            controller
                .record(
                    app,
                    "warning",
                    format!("{} connected but the public IP did not change", server.host),
                )
                .await;
            continue;
        };
        {
            let mut inner = controller.inner.lock().await;
            inner.settings.managed_profile = Some(profile);
            inner.connected_server = Some(server.clone());
            inner.public_ip = Some(ip.clone());
            inner.vpn_status = "connected".into();
            inner.phase = "resuming sessions".into();
            inner.last_error = None;
            // Incidents discovered during the connection attempt share this verified IP.
            inner.mark_verified_ip();
        }
        controller
            .record(
                app,
                "success",
                format!(
                    "Connected to {} ({}) with public IP {ip}",
                    server.host, server.country
                ),
            )
            .await;
        let all_pending = controller
            .inner
            .lock()
            .await
            .pending
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        resume_pending(app, controller, &all_pending).await;
        return Ok(());
    }
    {
        let mut inner = controller.inner.lock().await;
        inner.vpn_status = "disconnected".into();
        for id in &pending {
            inner.pending.remove(id);
        }
    }
    Err("All five VPN Gate connection candidates failed".into())
}

fn retain_incidents(
    previous: &[SessionTarget],
    mut fresh: Vec<SessionTarget>,
) -> Vec<SessionTarget> {
    for target in previous.iter().filter(|t| t.state == "rate-limited") {
        if !fresh.iter().any(|t| t.id == target.id) {
            let mut closed = target.clone();
            if closed.via_plugin {
                closed.client_id = None;
                closed.selected_tab = false;
            }
            fresh.push(closed);
        }
    }
    fresh
}

async fn busy_session_count(controller: &Arc<Controller>) -> Result<usize, String> {
    let live = presence::discover(0)?;
    if live.unknown_clients {
        return Err("Waiting for OpenCode execution status before changing VPN".into());
    }
    let mut busy = live.busy.len();
    match opencode::OpenCodeClient::discover(controller.http.clone()) {
        Ok(client) => busy += client.active_ids().await?.len(),
        Err(error) if error == "OpenCode shared service is not running" => {}
        Err(error) => return Err(error),
    }
    let orca_targets = controller.inner.lock().await.orca_sessions.clone();
    if orca_targets
        .iter()
        .any(|target| !target.via_plugin && target.client_id.is_none())
    {
        return Err(
            "Waiting for the OpenCode tab plugin in every Orca terminal before changing VPN".into(),
        );
    }
    Ok(busy)
}

enum ResumeChannel {
    Bridge,
    Api(opencode::OpenCodeClient, String),
}

async fn resume_pending(app: &AppHandle, controller: &Arc<Controller>, pending: &[String]) {
    for id in pending {
        let (target, prompt, pause_epoch) = {
            let inner = controller.inner.lock().await;
            let target = inner.target(id);
            let prompt = inner
                .settings
                .prompt_overrides
                .get(id)
                .cloned()
                .unwrap_or_else(|| inner.settings.global_prompt.clone());
            (target, prompt, inner.pause_epoch)
        };
        let Some(target) = target else {
            let mut inner = controller.inner.lock().await;
            inner.pending.remove(id);
            inner.ready_to_resume.remove(id);
            continue;
        };
        let channel: Result<Option<ResumeChannel>, String> = if target.via_plugin {
            presence::discover(0).and_then(|live| {
                let fresh = live
                    .targets
                    .iter()
                    .find(|t| t.id == target.id)
                    .ok_or("The OpenCode tab is no longer open")?;
                if fresh.state == "unknown" {
                    return Err("OpenCode status is unavailable".into());
                }
                Ok((fresh.state != "busy").then_some(ResumeChannel::Bridge))
            })
        } else if target.source == "orca" {
            Err("The OpenCode tab plugin is required to address this session".into())
        } else {
            let session_id = id.trim_start_matches("api:");
            match opencode::OpenCodeClient::discover(controller.http.clone()) {
                Ok(client) => client.active_ids().await.map(|active| {
                    (!active.contains(session_id))
                        .then(|| ResumeChannel::Api(client, session_id.into()))
                }),
                Err(error) => Err(error),
            }
        };
        let channel = match channel {
            Ok(Some(channel)) => channel,
            Ok(None) => continue,
            Err(error) => {
                controller
                    .record(
                        app,
                        "warning",
                        format!("Waiting to continue {}: {error}", target.title),
                    )
                    .await;
                continue;
            }
        };
        {
            let mut inner = controller.inner.lock().await;
            if !inner.pending.contains(id)
                || !inner.settings.monitors(&target)
                || inner.pause_epoch != pause_epoch
            {
                inner.pending.remove(id);
                inner.ready_to_resume.remove(id);
                continue;
            }
            if let Some(key) = target.error_key.clone() {
                // A failed transport may still have delivered the prompt. Never send this incident twice.
                inner.handled.insert(key);
            }
            inner.pending.remove(id);
            inner.ready_to_resume.remove(id);
        }
        controller.save().await;
        let result = match channel {
            ResumeChannel::Bridge => presence::command(&target, "prompt", Some(&prompt)).await,
            ResumeChannel::Api(client, session_id) => client
                .prompt(&session_id, &prompt)
                .await
                .map(presence::Delivery::Accepted),
        };
        match result {
            Ok(presence::Delivery::Busy) => {
                let mut inner = controller.inner.lock().await;
                if let Some(key) = &target.error_key {
                    inner.handled.remove(key);
                }
                if inner.settings.monitors(&target) && inner.pause_epoch == pause_epoch {
                    inner.pending.insert(id.clone());
                    inner.ready_to_resume.insert(id.clone());
                }
                drop(inner);
                controller.save().await;
                continue;
            }
            Ok(presence::Delivery::Accepted(receipt)) => {
                controller
                    .inner
                    .lock()
                    .await
                    .last_recovery_at
                    .insert(id.clone(), process::now_ms());
                controller
                    .record(
                        app,
                        "success",
                        format!("Sent continuation to {} (receipt {receipt})", target.title),
                    )
                    .await;
            }
            Err(error) => {
                controller
                    .record(
                        app,
                        "error",
                        format!(
                            "Could not confirm continuation for {}: {error}",
                            target.title
                        ),
                    )
                    .await
            }
        }
    }
    controller.save().await;
}

async fn background_loop(app: AppHandle, controller: Arc<Controller>) {
    if let Ok(ip) = public_ip(&controller.http).await {
        controller.inner.lock().await.public_ip = Some(ip);
    }
    let profile = controller
        .inner
        .lock()
        .await
        .settings
        .managed_profile
        .clone();
    if let Some(profile) = profile {
        if let Ok(runner) = RealVpnRunner::discover() {
            let vpn = SoftEther::new(runner, "VPN");
            let connected = vpn.status(&profile).await;
            controller.inner.lock().await.vpn_status = if connected {
                "connected"
            } else {
                "disconnected"
            }
            .into();
        }
    }
    if let Err(error) = refresh_catalog(&app, &controller).await {
        controller.set_error(&app, error).await;
    }
    loop {
        scan_sessions(&app, &controller).await;
        let due = controller
            .inner
            .lock()
            .await
            .refreshed_at
            .is_none_or(|at| process::now_ms().saturating_sub(at) >= 5 * 60 * 1000);
        if due {
            let _ = refresh_catalog(&app, &controller).await;
        }
        tokio::time::sleep(Duration::from_secs(6)).await;
    }
}

#[tauri::command]
async fn get_snapshot(state: State<'_, Arc<Controller>>) -> Result<Snapshot, String> {
    Ok(state.snapshot().await)
}

#[tauri::command]
async fn refresh_servers(
    app: AppHandle,
    state: State<'_, Arc<Controller>>,
) -> Result<Snapshot, String> {
    refresh_catalog(&app, state.inner()).await?;
    Ok(state.snapshot().await)
}

#[tauri::command]
async fn set_monitoring(
    app: AppHandle,
    state: State<'_, Arc<Controller>>,
    enabled: bool,
) -> Result<Snapshot, String> {
    {
        let mut inner = state.inner.lock().await;
        inner.settings.monitoring = enabled;
        if !enabled {
            inner.pause_epoch = inner.pause_epoch.wrapping_add(1);
            inner.pending.clear();
            inner.ready_to_resume.clear();
        }
    }
    state
        .record(
            &app,
            "info",
            if enabled {
                "Monitoring enabled"
            } else {
                "Monitoring paused"
            },
        )
        .await;
    if enabled {
        scan_sessions(&app, state.inner()).await;
    }
    Ok(state.snapshot().await)
}

#[tauri::command]
async fn set_session_monitoring(
    app: AppHandle,
    state: State<'_, Arc<Controller>>,
    target_id: String,
    enabled: bool,
    whole_terminal: bool,
) -> Result<Snapshot, String> {
    {
        let mut inner = state.inner.lock().await;
        let target = inner.target(&target_id).ok_or("Session was not found")?;
        if whole_terminal {
            let targets = inner
                .orca_sessions
                .iter()
                .chain(&inner.other_sessions)
                .cloned()
                .collect::<Vec<_>>();
            inner
                .settings
                .monitor_terminal(&target, &targets, enabled)?;
        } else {
            inner.settings.session_monitoring.insert(target_id, enabled);
        }
        inner.prune_unmonitored();
    }
    state.save().await;
    state.emit(&app).await;
    // Selection takes effect immediately while the master monitoring switch is on.
    scan_sessions(&app, state.inner()).await;
    Ok(state.snapshot().await)
}

#[tauri::command]
async fn set_global_prompt(
    app: AppHandle,
    state: State<'_, Arc<Controller>>,
    prompt: String,
) -> Result<Snapshot, String> {
    if prompt.trim().is_empty() {
        return Err("Continuation prompt cannot be empty".into());
    }
    state.inner.lock().await.settings.global_prompt = prompt.trim().to_string();
    state.save().await;
    state.emit(&app).await;
    Ok(state.snapshot().await)
}

#[tauri::command]
async fn set_prompt_override(
    app: AppHandle,
    state: State<'_, Arc<Controller>>,
    target_id: String,
    prompt: Option<String>,
) -> Result<Snapshot, String> {
    let mut inner = state.inner.lock().await;
    if inner.target(&target_id).is_none() {
        return Err("Session was not found".into());
    }
    if let Some(prompt) = prompt.filter(|p| !p.trim().is_empty()) {
        inner
            .settings
            .prompt_overrides
            .insert(target_id, prompt.trim().to_string());
    } else {
        inner.settings.prompt_overrides.remove(&target_id);
    }
    drop(inner);
    state.save().await;
    state.emit(&app).await;
    Ok(state.snapshot().await)
}

#[tauri::command]
async fn set_preferred_server(
    app: AppHandle,
    state: State<'_, Arc<Controller>>,
    server_id: Option<String>,
) -> Result<Snapshot, String> {
    let mut inner = state.inner.lock().await;
    if server_id
        .as_ref()
        .is_some_and(|id| !inner.servers.iter().any(|server| &server.id == id))
    {
        return Err("VPN server was not found in the current list".into());
    }
    inner.settings.preferred_server = server_id;
    drop(inner);
    state.save().await;
    state.emit(&app).await;
    Ok(state.snapshot().await)
}

#[tauri::command]
async fn select_session_tab(
    app: AppHandle,
    state: State<'_, Arc<Controller>>,
    target_id: String,
) -> Result<Snapshot, String> {
    let target = state
        .inner
        .lock()
        .await
        .target(&target_id)
        .ok_or("Session was not found")?;
    match presence::command(&target, "focus", None).await? {
        presence::Delivery::Accepted(_) => {
            if let Some(handle) = target.orca_handle.as_deref() {
                orca::select_terminal(handle).await?;
            }
            let mut inner = state.inner.lock().await;
            for item in inner.orca_sessions.iter_mut() {
                if item.terminal_id == target.terminal_id {
                    item.selected_tab = item.id == target.id;
                }
            }
            for item in inner.other_sessions.iter_mut() {
                if item.terminal_id == target.terminal_id {
                    item.selected_tab = item.id == target.id;
                }
            }
        }
        presence::Delivery::Busy => return Err("OpenCode could not select this tab".into()),
    }
    state.emit(&app).await;
    Ok(state.snapshot().await)
}

#[tauri::command]
async fn recover_now(
    app: AppHandle,
    state: State<'_, Arc<Controller>>,
    target_id: String,
) -> Result<Snapshot, String> {
    {
        let mut inner = state.inner.lock().await;
        let target = inner.target(&target_id).ok_or("Session was not found")?;
        if !inner.settings.monitors(&target) {
            return Err("Enable monitoring for this session before recovering".into());
        }
        if (target.via_plugin && target.client_id.is_none())
            || (target.source == "orca" && !target.via_plugin)
            || target.state == "unknown"
        {
            return Err(
                "Open this session in a client with the Recovery tab plugin before recovering"
                    .into(),
            );
        }
        inner.pending.insert(target_id);
    }
    start_recovery_if_needed(app, state.inner().clone()).await;
    Ok(state.snapshot().await)
}

#[tauri::command]
async fn disconnect_vpn(
    app: AppHandle,
    state: State<'_, Arc<Controller>>,
) -> Result<Snapshot, String> {
    if state.inner.lock().await.recovery_running {
        return Err("Wait for the current recovery attempt to finish".into());
    }
    let profile = state.inner.lock().await.settings.managed_profile.clone();
    if let Some(profile) = profile {
        let vpn = SoftEther::new(RealVpnRunner::discover()?, "VPN");
        vpn.cleanup(&profile).await?;
        let mut inner = state.inner.lock().await;
        inner.settings.managed_profile = None;
        inner.connected_server = None;
        inner.vpn_status = "disconnected".into();
        drop(inner);
        if let Ok(ip) = public_ip(&state.http).await {
            state.inner.lock().await.public_ip = Some(ip);
        }
        state
            .record(&app, "info", "Disconnected app-managed VPN")
            .await;
    }
    Ok(state.snapshot().await)
}

pub fn run() {
    let controller = Arc::new(Controller::new());
    tauri::Builder::default()
        .manage(controller.clone())
        .setup(move |app| {
            let handle = app.handle().clone();
            let state = controller.clone();
            tauri::async_runtime::spawn(async move { background_loop(handle, state).await });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            refresh_servers,
            set_monitoring,
            set_session_monitoring,
            set_global_prompt,
            set_prompt_override,
            set_preferred_server,
            recover_now,
            select_session_tab,
            disconnect_vpn
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenCode Recovery");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(id: &str, source: &str, state: &str, key: Option<&str>) -> SessionTarget {
        SessionTarget {
            id: id.into(),
            source: source.into(),
            title: id.into(),
            project: id.into(),
            state: state.into(),
            last_error: key.map(|_| "Rate limit exceeded".into()),
            error_key: key.map(str::to_string),
            prompt_override: None,
            updated_at: 0,
            via_plugin: source == "orca",
            client_id: (source == "orca").then(|| "test-client".into()),
            ..Default::default()
        }
    }

    fn inner() -> Inner {
        Inner {
            settings: Settings {
                monitoring: true,
                ..Settings::default()
            },
            servers: Vec::new(),
            refreshed_at: None,
            orca_sessions: Vec::new(),
            other_sessions: Vec::new(),
            other_sessions_notice: None,
            previously_active_other: HashSet::new(),
            history: Vec::new(),
            phase: "ready".into(),
            vpn_status: "disconnected".into(),
            public_ip: None,
            connected_server: None,
            last_error: None,
            pending: HashSet::new(),
            ready_to_resume: HashSet::new(),
            handled: HashSet::new(),
            ip_attempts: HashMap::new(),
            last_recovery_at: HashMap::new(),
            recovery_running: false,
            pause_epoch: 0,
        }
    }

    #[test]
    fn unmonitored_sessions_do_not_queue_or_resume_and_can_be_reenabled() {
        let mut state = inner();
        state.orca_sessions = vec![target(
            "api:orca",
            "orca",
            "rate-limited",
            Some("orca:failure"),
        )];
        state.other_sessions = vec![target(
            "api:outside",
            "other",
            "rate-limited",
            Some("outside:failure"),
        )];
        state
            .settings
            .session_monitoring
            .insert("api:orca".into(), false);
        assert_eq!(queue_incidents(&mut state).len(), 1);
        assert!(!state.pending.contains("api:orca"));
        assert!(state.pending.contains("api:outside"));
        state.mark_verified_ip();
        state
            .settings
            .session_monitoring
            .insert("api:outside".into(), false);
        state.prune_unmonitored();
        assert!(state.pending.is_empty() && state.ready_to_resume.is_empty());
        assert!(matches!(state.recovery_plan(), RecoveryPlan::None));
        assert!(state.handled.is_empty());
        state
            .settings
            .session_monitoring
            .insert("api:orca".into(), true);
        assert_eq!(queue_incidents(&mut state).len(), 1);
        assert!(state.pending.contains("api:orca"));
        assert!(!state.pending.contains("api:outside"));
    }

    #[test]
    fn closed_tab_incidents_stay_visible_without_a_delivery_route() {
        let mut previous = target("api:one", "orca", "rate-limited", Some("failure"));
        previous.selected_tab = true;
        let closed = retain_incidents(&[previous], vec![]);
        assert_eq!(closed.len(), 1);
        assert!(closed[0].client_id.is_none());
        assert!(!closed[0].selected_tab);
        let mut state = inner();
        state.orca_sessions = closed;
        assert!(queue_incidents(&mut state).is_empty());
    }

    #[test]
    fn queues_simultaneous_orca_projects_and_outside_session_once() {
        let mut state = inner();
        state.orca_sessions = vec![
            target("orca:one", "orca", "rate-limited", Some("one:failure")),
            target("orca:two", "orca", "rate-limited", Some("two:failure")),
        ];
        state.other_sessions = vec![
            target("api:three", "other", "rate-limited", Some("three:failure")),
            target("api:busy", "other", "busy", None),
        ];
        assert_eq!(queue_incidents(&mut state).len(), 3);
        assert_eq!(state.pending.len(), 3);
        assert!(queue_incidents(&mut state).is_empty());
        state.pending.clear();
        state.handled.insert("one:failure".into());
        state.handled.insert("two:failure".into());
        state.handled.insert("three:failure".into());
        assert!(queue_incidents(&mut state).is_empty());
    }

    #[test]
    fn verified_ip_retries_busy_continuations_before_rotating_again() {
        let mut state = inner();
        state.pending.extend(["orca:one".into(), "api:two".into()]);
        assert!(matches!(state.recovery_plan(), RecoveryPlan::Rotate(_)));
        state.mark_verified_ip();
        assert_eq!(state.ip_attempts["orca:one"], 1);
        assert_eq!(state.ip_attempts["api:two"], 1);
        match state.recovery_plan() {
            RecoveryPlan::Resume(ids) => assert_eq!(ids.len(), 2),
            _ => panic!("verified IP should resume pending sessions"),
        }
        state.pending.remove("orca:one");
        state.ready_to_resume.remove("orca:one");
        assert!(matches!(state.recovery_plan(), RecoveryPlan::Resume(_)));
        state.pending.remove("api:two");
        state.ready_to_resume.remove("api:two");
        assert!(matches!(state.recovery_plan(), RecoveryPlan::None));
    }

    #[test]
    fn stops_after_two_verified_ips_for_same_session() {
        let mut state = inner();
        state.other_sessions = vec![target(
            "api:one",
            "other",
            "rate-limited",
            Some("second-failure"),
        )];
        state.ip_attempts.insert("api:one".into(), 2);
        assert!(queue_incidents(&mut state).is_empty());
        assert!(state.pending.is_empty());
        assert!(state.handled.contains("second-failure"));
        assert!(state
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("two new IPs")));
    }
}
