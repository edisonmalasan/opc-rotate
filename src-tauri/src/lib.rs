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
}

enum RecoveryPlan {
    None,
    Resume(Vec<String>),
    Rotate(Vec<String>),
}

impl Inner {
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
        let persisted: Persisted = std::fs::read_to_string(&settings_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default();
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
    let orca_result = orca::list_targets().await;
    let orca_targets = orca_result.as_ref().cloned().unwrap_or_default();
    let previously_active = controller
        .inner
        .lock()
        .await
        .previously_active_other
        .clone();
    let plugin_error = presence::install().err();
    let (api_result, setup_notice) = match opencode::OpenCodeClient::discover(
        controller.http.clone(),
    ) {
        Ok(client) => match presence::discover(client.service_pid) {
            Ok(presence) => {
                let notice = plugin_error.or_else(|| (presence.clients == 0).then(||
                    "Idle terminals appear after OpenCode loads the local presence plugin. If an open terminal is missing, reopen that OpenCode client once. opencode --auto is supported.".into()));
                (
                    client.list_targets(&presence, &previously_active).await,
                    notice,
                )
            }
            Err(error) => (Err(error), None),
        },
        Err(e) => (Err(e), plugin_error),
    };
    let other_targets = api_result.as_ref().cloned().unwrap_or_default();
    let new_incidents = {
        let mut inner = controller.inner.lock().await;
        inner.other_sessions_notice = api_result.as_ref().err().cloned().or(setup_notice);
        if orca_result.is_ok() {
            inner.orca_sessions = orca_targets;
        }
        if api_result.is_ok() {
            inner.other_sessions = other_targets;
            inner.previously_active_other = inner
                .other_sessions
                .iter()
                .filter(|target| target.state == "busy" || target.state == "rate-limited")
                .filter_map(|target| target.id.strip_prefix("api:").map(str::to_string))
                .collect();
        } else {
            // A dead or restarted service must not leave old busy/idle rows looking live.
            inner
                .other_sessions
                .retain(|target| target.state == "rate-limited");
        }
        let overrides = inner.settings.prompt_overrides.clone();
        for target in inner.orca_sessions.iter_mut() {
            target.prompt_override = overrides.get(&target.id).cloned();
        }
        for target in inner.other_sessions.iter_mut() {
            target.prompt_override = overrides.get(&target.id).cloned();
        }
        queue_incidents(&mut inner)
    };
    if !new_incidents.is_empty() {
        controller
            .record(
                app,
                "warning",
                format!("Rate limit detected: {}", new_incidents.join(", ")),
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
        if target.state != "rate-limited" {
            if target.state == "completed" {
                inner.ip_attempts.remove(&target.id);
                inner.last_recovery_at.remove(&target.id);
            }
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
                "Rate limit persisted after two new IPs for {}",
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

async fn busy_session_count(controller: &Arc<Controller>) -> Result<usize, String> {
    let mut busy = match opencode::OpenCodeClient::discover(controller.http.clone()) {
        Ok(client) => client.active_ids().await?.len(),
        Err(error) if error == "OpenCode shared service is not running" => 0,
        Err(error) => return Err(error),
    };
    let orca_targets = controller.inner.lock().await.orca_sessions.clone();
    for target in orca_targets {
        let handle = target.id.trim_start_matches("orca:");
        if !orca::is_idle(handle).await? {
            busy += 1;
        }
    }
    Ok(busy)
}

enum ResumeChannel {
    Orca(String),
    Api(opencode::OpenCodeClient, String),
}

async fn resume_pending(app: &AppHandle, controller: &Arc<Controller>, pending: &[String]) {
    for id in pending {
        let (target, prompt) = {
            let inner = controller.inner.lock().await;
            let target = inner.target(id);
            let prompt = inner
                .settings
                .prompt_overrides
                .get(id)
                .cloned()
                .unwrap_or_else(|| inner.settings.global_prompt.clone());
            (target, prompt)
        };
        let Some(target) = target else {
            let mut inner = controller.inner.lock().await;
            inner.pending.remove(id);
            inner.ready_to_resume.remove(id);
            continue;
        };
        let channel: Result<Option<ResumeChannel>, String> = if target.source == "orca" {
            let handle = id.trim_start_matches("orca:");
            orca::is_idle(handle)
                .await
                .map(|idle| idle.then(|| ResumeChannel::Orca(handle.to_string())))
        } else {
            let session_id = id.trim_start_matches("api:");
            match opencode::OpenCodeClient::discover(controller.http.clone()) {
                Ok(client) => client.active_ids().await.map(|active| {
                    (!active.contains(session_id))
                        .then(|| ResumeChannel::Api(client, session_id.to_string()))
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
            if let Some(key) = target.error_key.clone() {
                // A failed transport may still have delivered the prompt. Never send this incident twice.
                inner.handled.insert(key);
            }
            inner.pending.remove(id);
            inner.ready_to_resume.remove(id);
        }
        let result = match channel {
            ResumeChannel::Orca(handle) => orca::send_prompt(&handle, &prompt).await,
            ResumeChannel::Api(client, session_id) => client.prompt(&session_id, &prompt).await,
        };
        match result {
            Ok(receipt) => {
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
    state.inner.lock().await.settings.monitoring = enabled;
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
async fn recover_now(
    app: AppHandle,
    state: State<'_, Arc<Controller>>,
    target_id: String,
) -> Result<Snapshot, String> {
    {
        let mut inner = state.inner.lock().await;
        if inner.target(&target_id).is_none() {
            return Err("Session was not found".into());
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
            set_global_prompt,
            set_prompt_override,
            set_preferred_server,
            recover_now,
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
        }
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
