mod models;
mod opencode;
mod orca;
mod presence;
mod process;
mod softether;
mod vpngate;

use models::{HistoryEntry, RecoveryProgress, SessionTarget, Settings, Snapshot, VpnServer};
use serde::{Deserialize, Serialize};
use softether::{RealVpnRunner, SoftEther};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

const BUSY_DEFERRED: &str = "Other OpenCode sessions are still active; recovery remains queued";
const MAX_IP_CHANGES: u8 = 2;
const ORCA_RETRY_GAP_MS: u64 = 30_000;

#[derive(Serialize, Deserialize, Default, Clone)]
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
    recovery: HashMap<String, RecoveryProgress>,
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
    continue_pending: HashSet<String>,
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
    Continue(Vec<String>),
}

impl Inner {
    fn progress(
        &mut self,
        id: &str,
        stage: &str,
        detail: impl Into<String>,
        retry_at: Option<u64>,
    ) {
        self.progress_ip(id, stage, detail, retry_at, true);
    }

    fn progress_ip(
        &mut self,
        id: &str,
        stage: &str,
        detail: impl Into<String>,
        retry_at: Option<u64>,
        changes_ip: bool,
    ) {
        let detail = detail.into();
        let old = self.recovery.get(id);
        let updated_at = old
            .filter(|old| old.stage == stage && old.detail == detail && old.retry_at == retry_at)
            .map(|old| old.updated_at)
            .unwrap_or_else(process::now_ms);
        self.recovery.insert(
            id.into(),
            RecoveryProgress {
                stage: stage.into(),
                detail,
                updated_at,
                retry_at,
                ip_changes: self.ip_attempts.get(id).copied().unwrap_or(0),
                max_ip_changes: MAX_IP_CHANGES,
                last_sent_at: self.last_recovery_at.get(id).copied(),
                changes_ip,
            },
        );
    }

    fn pending_progress(&mut self, stage: &str, detail: &str) {
        for id in self.pending.clone() {
            if !self.ready_to_resume.contains(&id) {
                self.progress(&id, stage, detail, None);
            }
        }
    }

    fn prune_unmonitored(&mut self) {
        let allowed = self
            .orca_sessions
            .iter()
            .chain(&self.other_sessions)
            .filter(|target| self.settings.monitors(target))
            .map(|target| target.id.clone())
            .collect::<HashSet<_>>();
        let queued = self
            .pending
            .iter()
            .cloned()
            .chain(
                self.recovery
                    .iter()
                    .filter(|(_, p)| p.stage == "cooldown")
                    .map(|(id, _)| id.clone()),
            )
            .collect::<HashSet<_>>();
        for id in queued.into_iter().filter(|id| !allowed.contains(id)) {
            self.progress(
                &id,
                "paused",
                "Not monitored; queued recovery canceled",
                None,
            );
        }
        self.pending.retain(|id| allowed.contains(id));
        self.ready_to_resume.retain(|id| allowed.contains(id));
        self.continue_pending.retain(|id| allowed.contains(id));
    }

    fn recovery_plan(&self) -> RecoveryPlan {
        if !self.continue_pending.is_empty() {
            // Continuing a stopped session never touches the VPN, so it runs first.
            return RecoveryPlan::Continue(self.continue_pending.iter().cloned().collect());
        }
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
            let attempts = self.ip_attempts.entry(id.clone()).or_default();
            *attempts = attempts.saturating_add(1);
            self.ready_to_resume.insert(id.clone());
            self.progress(
                &id,
                "waiting-target",
                "New IPv4 verified; waiting for this session to be idle",
                None,
            );
        }
    }

    fn snapshot(&self) -> Snapshot {
        let mut recovery = self.recovery.clone();
        for target in self.orca_sessions.iter().chain(&self.other_sessions) {
            if let Some(&ip_changes) = self.ip_attempts.get(&target.id) {
                recovery.entry(target.id.clone()).or_insert_with(|| RecoveryProgress {
                    stage: if ip_changes >= MAX_IP_CHANGES && target.state == "rate-limited" { "stopped" } else { "history" }.into(),
                    detail: if ip_changes >= MAX_IP_CHANGES && target.state == "rate-limited" {
                        "Automatic IP-change limit used. Recover session starts an explicit manual attempt"
                    } else { "Saved IP-change count for this session" }.into(),
                    updated_at: 0, retry_at: None, ip_changes,
                    max_ip_changes: MAX_IP_CHANGES, last_sent_at: self.last_recovery_at.get(&target.id).copied(),
                    changes_ip: true,
                });
            }
        }
        Snapshot {
            recovery,
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
            watcher: self.settings.watcher.clone(),
            session_watcher: self.settings.session_watcher.clone(),

            // The route only matters while a relay is connected, a recovery
            // still has to change the public IP, or some session still watches
            // rate limits.
            vpn_needed: self.connected_server.is_some()
                || self.recovery.values().any(|progress| progress.changes_ip)
                || self.settings.rate_limit_watchers(
                    &self
                        .orca_sessions
                        .iter()
                        .chain(&self.other_sessions)
                        .cloned()
                        .collect::<Vec<_>>(),
                ),
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

fn write_settings_file(path: &std::path::Path, data: &Persisted) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_vec_pretty(data) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
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
        let persisted_before = persisted.clone();
        persisted.settings.migrate_default_prompt();
        persisted.settings.migrate_watcher();
        // The prompt migration replaces a saved prompt, so it has to be
        // written out now. Otherwise it would run again on every start and
        // discard whatever the operator wrote afterwards.
        if serde_json::to_string(&persisted.settings).ok()
            != serde_json::to_string(&persisted_before.settings).ok()
        {
            write_settings_file(&settings_path, &persisted);
        }
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
                recovery: HashMap::new(),
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
                continue_pending: HashSet::new(),
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
        write_settings_file(&self.settings_path, &data);
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

/// A stop is continued once. A session that has already been continued for
/// the stop it is on is waiting for new operator input, and the interface
/// says so rather than leaving it looking unhandled.
fn mark_continued(targets: &mut [SessionTarget], handled: &HashSet<String>) {
    for target in targets.iter_mut() {
        target.continued = target
            .stop_key
            .as_ref()
            .is_some_and(|key| handled.contains(key));
    }
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
        for id in inner
            .pending
            .clone()
            .into_iter()
            .filter(|id| !available.contains(id))
        {
            inner.progress(
                &id,
                "unavailable",
                "The tab is closed or its live client is unavailable",
                None,
            );
        }
        inner.pending.retain(|id| available.contains(id));
        inner.continue_pending.retain(|id| available.contains(id));
        inner.ready_to_resume.retain(|id| available.contains(id));
        let visible = inner
            .orca_sessions
            .iter()
            .chain(&inner.other_sessions)
            .map(|target| target.id.clone())
            .collect::<HashSet<_>>();
        inner.recovery.retain(|id, _| visible.contains(id));
        let overrides = inner.settings.prompt_overrides.clone();
        let handled = inner.handled.clone();
        for target in inner.orca_sessions.iter_mut() {
            target.prompt_override = overrides.get(&target.id).cloned();
        }
        for target in inner.other_sessions.iter_mut() {
            target.prompt_override = overrides.get(&target.id).cloned();
        }
        mark_continued(&mut inner.orca_sessions, &handled);
        mark_continued(&mut inner.other_sessions, &handled);
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
        if inner.settings.watches_worker_idle(&target) {
            // This session is watched for a finished turn, not a provider
            // failure. It is continued with a prompt and never a new IP.
            if target.state != "stopped" {
                if inner
                    .recovery
                    .get(&target.id)
                    .is_some_and(|p| p.stage == "cooldown")
                {
                    inner.recovery.remove(&target.id);
                }
                inner.continue_pending.remove(&target.id);
                continue;
            }
            let Some(key) = target.stop_key.clone() else {
                continue;
            };
            if (target.via_plugin && target.client_id.is_none())
                || (target.source == "orca" && !target.via_plugin)
                || inner.handled.contains(&key)
                || inner.continue_pending.contains(&target.id)
                || !inner.settings.monitoring
            {
                continue;
            }
            inner.continue_pending.insert(target.id.clone());
            inner.progress_ip(
                &target.id,
                "queued",
                "Queued to continue this session with a prompt; the public IP is not changed",
                None,
                false,
            );
            new_incidents.push(target.title.clone());
            continue;
        }
        if target.state != "rate-limited" {
            if inner
                .recovery
                .get(&target.id)
                .is_some_and(|p| p.stage == "cooldown")
            {
                inner.recovery.remove(&target.id);
            }
            if target.state == "completed" || target.state == "stopped" {
                inner.ip_attempts.remove(&target.id);
                inner.last_recovery_at.remove(&target.id);
                inner.recovery.remove(&target.id);
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
        let retry_at = inner
            .last_recovery_at
            .get(&target.id)
            .map(|at| at.saturating_add(ORCA_RETRY_GAP_MS));
        if target.source == "orca" && retry_at.is_some_and(|at| process::now_ms() < at) {
            inner.progress(&target.id, "cooldown", "Minimum 30 seconds after the last confirmed continuation; work and VPN checks can add time", retry_at);
            continue;
        }
        if inner.ip_attempts.get(&target.id).copied().unwrap_or(0) >= MAX_IP_CHANGES {
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
            inner.progress(&target.id, "stopped", "Automatic recovery stopped after two verified IP changes. Recover session starts a manual attempt", None);
        } else {
            inner.pending.insert(target.id.clone());
            inner.progress(
                &target.id,
                "queued",
                "Waiting for the shared VPN recovery controller",
                None,
            );
            new_incidents.push(target.title.clone());
        }
    }
    new_incidents
}

async fn start_recovery_if_needed(app: AppHandle, controller: Arc<Controller>) {
    {
        let mut inner = controller.inner.lock().await;
        if inner.recovery_running || (inner.pending.is_empty() && inner.continue_pending.is_empty())
        {
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
                inner.progress(&id, "failed", &error, None);
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
        RecoveryPlan::Continue(ready) => {
            continue_stopped(app, controller, &ready).await;
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
            inner.pending_progress(
                "waiting",
                &format!("Waiting for {busy} active session(s) before changing the machine's VPN"),
            );
        }
        controller.emit(app).await;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    let busy = busy_session_count(controller).await?;
    if busy > 0 {
        return Err(BUSY_DEFERRED.into());
    }

    controller.inner.lock().await.pending_progress(
        "checking",
        "Checking current public IPv4 and preparing the VPN relays",
    );
    controller.emit(app).await;
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
        controller.inner.lock().await.pending_progress(
            "disconnecting",
            "Disconnecting the previous app-owned VPN profile",
        );
        controller.emit(app).await;
        vpn.cleanup(&profile).await?;
        let mut inner = controller.inner.lock().await;
        inner.settings.managed_profile = None;
        inner.connected_server = None;
        inner.vpn_status = "disconnected".into();
        drop(inner);
        controller.save().await;
    }
    let relay_total = servers.len();
    for (index, server) in servers.into_iter().enumerate() {
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
            inner.pending_progress(
                "connecting",
                &format!(
                    "Relay {} of {relay_total}: {} · {}. Connection timeout: 20 seconds",
                    index + 1,
                    server.country,
                    server.host
                ),
            );
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
        controller.inner.lock().await.pending_progress(
            "verifying",
            "VPN connected; verifying that the public IPv4 changed",
        );
        controller.emit(app).await;
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
            inner.progress(
                id,
                "failed",
                "All VPN relay candidates failed; inspect Activity before retrying",
                None,
            );
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
        Ok(client) => match client.active_ids().await {
            Ok(ids) => busy += ids.len(),
            // A shared service that is gone has no live sessions of its own,
            // so it cannot be protecting any work.
            Err(error) if opencode::service_is_gone(&error) => {}
            Err(error) => return Err(error),
        },
        Err(error) if opencode::service_is_gone(&error) => {}
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

/// Continue a session that finished its turn. The public IP is never changed:
/// the session is simply asked to keep going, and the receipt is read back.
async fn continue_stopped(app: &AppHandle, controller: &Arc<Controller>, pending: &[String]) {
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
            controller.inner.lock().await.continue_pending.remove(id);
            continue;
        };
        // Only a live client can confirm that a continuation landed.
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
        } else {
            Err("A stopped session can only be continued through its live client".into())
        };
        let channel = match channel {
            Ok(Some(channel)) => channel,
            Ok(None) => {
                // The session started working again, so there is nothing to continue.
                let mut inner = controller.inner.lock().await;
                if inner.continue_pending.contains(id) && inner.pause_epoch == pause_epoch {
                    if let Some(key) = &target.stop_key {
                        inner.handled.insert(key.clone());
                    }
                    inner.continue_pending.remove(id);
                    inner.progress_ip(
                        id,
                        "waiting-target",
                        "The session started working again; no continuation was needed",
                        None,
                        false,
                    );
                }
                drop(inner);
                controller.emit(app).await;
                continue;
            }
            Err(error) => {
                let mut inner = controller.inner.lock().await;
                if inner.continue_pending.contains(id) && inner.pause_epoch == pause_epoch {
                    if let Some(key) = &target.stop_key {
                        inner.handled.insert(key.clone());
                    }
                    inner.continue_pending.remove(id);
                    inner.progress_ip(
                        id,
                        "blocked",
                        format!("Waiting to continue: {error}"),
                        None,
                        false,
                    );
                }
                drop(inner);
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
            if !inner.continue_pending.contains(id)
                || !inner.settings.monitors(&target)
                || inner.pause_epoch != pause_epoch
            {
                inner.continue_pending.remove(id);
                inner.progress_ip(
                    id,
                    "paused",
                    "Queued recovery canceled before submission",
                    None,
                    false,
                );
                continue;
            }
            // One continuation per stop: the operator's own message identifies it.
            if let Some(key) = &target.stop_key {
                inner.handled.insert(key.clone());
            }
            inner.continue_pending.remove(id);
            inner.progress_ip(
                id,
                "sending",
                "Submitting the continuation to this exact session; waiting for its receipt",
                None,
                false,
            );
        }
        controller.save().await;
        controller.emit(app).await;
        let result = match channel {
            ResumeChannel::Bridge => presence::command(&target, "prompt", Some(&prompt)).await,
            ResumeChannel::Api(client, session_id) => client
                .prompt(&session_id, &prompt)
                .await
                .map(presence::Delivery::Accepted),
        };
        match result {
            Ok(presence::Delivery::Busy) => {
                controller.inner.lock().await.progress_ip(
                    id,
                    "waiting-target",
                    "The session became busy; no continuation was needed",
                    None,
                    false,
                );
            }
            Ok(presence::Delivery::Accepted(receipt)) => {
                let mut inner = controller.inner.lock().await;
                inner.last_recovery_at.insert(id.clone(), process::now_ms());
                inner.progress_ip(
                    id,
                    "sent",
                    "Continuation receipt confirmed. Waiting for OpenCode's next result",
                    None,
                    false,
                );
                drop(inner);
                controller
                    .record(
                        app,
                        "success",
                        format!("Sent continuation to {} (receipt {receipt})", target.title),
                    )
                    .await;
            }
            Err(error) => {
                controller.inner.lock().await.progress_ip(id, "uncertain", format!("Delivery could not be confirmed; inspect the session before retrying. {error}"), None, false);
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
            Ok(None) => {
                let mut inner = controller.inner.lock().await;
                if inner.pending.contains(id) && inner.pause_epoch == pause_epoch {
                    inner.progress(
                        id,
                        "waiting-target",
                        "New IPv4 verified; this session is still working",
                        None,
                    );
                }
                drop(inner);
                controller.emit(app).await;
                continue;
            }
            Err(error) => {
                let mut inner = controller.inner.lock().await;
                if inner.pending.contains(id) && inner.pause_epoch == pause_epoch {
                    inner.progress(
                        id,
                        "blocked",
                        format!("New IPv4 verified; waiting to continue: {error}"),
                        None,
                    );
                }
                drop(inner);
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
                inner.progress(
                    id,
                    "paused",
                    "Queued recovery canceled before submission",
                    None,
                );
                continue;
            }
            if let Some(key) = target.error_key.clone() {
                // A failed transport may still have delivered the prompt. Never send this incident twice.
                inner.handled.insert(key);
            }
            inner.pending.remove(id);
            inner.ready_to_resume.remove(id);
            inner.progress(
                id,
                "sending",
                "Submitting the continuation to this exact session; waiting for its receipt",
                None,
            );
        }
        controller.save().await;
        controller.emit(app).await;
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
                    inner.progress(
                        id,
                        "waiting-target",
                        "The session became busy; keeping the verified IP until it is idle",
                        None,
                    );
                } else {
                    inner.progress(id, "paused", "Recovery was paused before submission", None);
                }
                drop(inner);
                controller.save().await;
                controller.emit(app).await;
                continue;
            }
            Ok(presence::Delivery::Accepted(receipt)) => {
                let mut inner = controller.inner.lock().await;
                inner.last_recovery_at.insert(id.clone(), process::now_ms());
                inner.progress(
                    id,
                    "sent",
                    "Continuation receipt confirmed. Waiting for OpenCode's next result",
                    None,
                );
                drop(inner);
                controller
                    .record(
                        app,
                        "success",
                        format!("Sent continuation to {} (receipt {receipt})", target.title),
                    )
                    .await;
            }
            Err(error) => {
                controller.inner.lock().await.progress(id, "uncertain", format!("Delivery could not be confirmed; inspect the session before retrying. {error}"), None);
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
            let queued = inner
                .pending
                .iter()
                .cloned()
                .chain(inner.continue_pending.iter().cloned())
                .chain(
                    inner
                        .recovery
                        .iter()
                        .filter(|(_, p)| p.stage == "cooldown")
                        .map(|(id, _)| id.clone()),
                )
                .collect::<HashSet<_>>();
            for id in queued {
                inner.progress(
                    &id,
                    "paused",
                    "Monitoring paused; queued recovery canceled",
                    None,
                );
            }
            inner.pending.clear();
            inner.continue_pending.clear();
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

/// The watcher mode every session follows unless it overrides it.
#[tauri::command]
async fn set_watcher(
    app: AppHandle,
    state: State<'_, Arc<Controller>>,
    watcher: String,
) -> Result<Snapshot, String> {
    let watcher = normalize_watcher(&watcher)?;
    {
        let mut inner = state.inner.lock().await;
        inner.settings.watcher = watcher.to_string();
        inner.continue_pending.clear();
        inner.pending.clear();
        inner.ready_to_resume.clear();
    }
    state.save().await;
    state.emit(&app).await;
    scan_sessions(&app, state.inner()).await;
    Ok(state.snapshot().await)
}

/// One session's own watcher mode, overriding its terminal and the app default.
#[tauri::command]
async fn set_session_watcher(
    app: AppHandle,
    state: State<'_, Arc<Controller>>,
    target_id: String,
    watcher: String,
) -> Result<Snapshot, String> {
    let watcher = normalize_watcher(&watcher)?;
    {
        let mut inner = state.inner.lock().await;
        let target = match inner.target(&target_id) {
            Some(target) => target,
            None => return Err("Session was not found".into()),
        };
        if watcher == models::WATCH_DEFAULT {
            inner.settings.session_watcher.remove(&target_id);
        } else {
            inner
                .settings
                .session_watcher
                .insert(target_id.clone(), watcher.to_string());
        }
        // A session that no longer watches rate limits must not keep a queued
        // route change, and one that no longer watches finished turns must not
        // keep a queued continuation.
        let now_worker_idle = inner.settings.watches_worker_idle(&target);
        if now_worker_idle && inner.pending.remove(&target_id) {
            inner.ready_to_resume.remove(&target_id);
            inner.progress_ip(
                &target_id,
                "paused",
                "This session now watches finished turns, so the queued route change was canceled",
                None,
                false,
            );
        }
        if !now_worker_idle && inner.continue_pending.remove(&target_id) {
            inner.progress_ip(
                &target_id,
                "paused",
                "This session now watches rate limits, so the queued continuation was canceled",
                None,
                true,
            );
        }
    }
    state.save().await;
    state.emit(&app).await;
    scan_sessions(&app, state.inner()).await;
    Ok(state.snapshot().await)
}

fn normalize_watcher(watcher: &str) -> Result<&'static str, String> {
    match watcher {
        models::WATCH_RATE_LIMIT => Ok(models::WATCH_RATE_LIMIT),
        models::WATCH_WORKER_IDLE => Ok(models::WATCH_WORKER_IDLE),
        models::WATCH_DEFAULT => Ok(models::WATCH_DEFAULT),
        _ => Err("Unknown watcher mode".into()),
    }
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
        if inner.settings.watches_worker_idle(&target) {
            // A worker that has gone idle is continued with a prompt; the
            // public IP is not touched.
            inner.continue_pending.insert(target_id.clone());
            inner.pending.remove(&target_id);
            inner.ready_to_resume.remove(&target_id);
            inner.progress_ip(
                &target_id,
                "queued",
                "Manual continuation queued; the public IP is not changed",
                None,
                false,
            );
        } else {
            inner.pending.insert(target_id.clone());
            // A manual recovery is the full route change, so it supersedes an
            // automatic continuation that may already be queued for this session.
            inner.continue_pending.remove(&target_id);
            inner.progress(
                &target_id,
                "queued",
                "Manual recovery queued; waits for busy work before changing the VPN",
                None,
            );
        }
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
            set_watcher,
            set_session_watcher,
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

    /// A session whose turn simply ended, waiting on the operator's message.
    fn stopped_target(id: &str, source: &str, stop_key: &str) -> SessionTarget {
        let mut value = target(id, source, "stopped", None);
        value.stop_key = Some(stop_key.into());
        value.working_subagents = 0;
        value
    }

    fn inner() -> Inner {
        Inner {
            recovery: HashMap::new(),
            settings: Settings {
                monitoring: true,
                // Most controller tests exercise the rate-limit path.
                watcher: models::WATCH_RATE_LIMIT.into(),
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
            continue_pending: HashSet::new(),
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
    fn orca_gap_is_measured_from_confirmed_send_and_does_not_delay_outside_sessions() {
        let mut state = inner();
        state.orca_sessions = vec![target(
            "orca:one",
            "orca",
            "rate-limited",
            Some("new-orca-error"),
        )];
        state.other_sessions = vec![target(
            "api:two",
            "other",
            "rate-limited",
            Some("new-api-error"),
        )];
        let sent = process::now_ms();
        state.last_recovery_at.insert("orca:one".into(), sent);
        state.last_recovery_at.insert("api:two".into(), sent);
        assert_eq!(queue_incidents(&mut state).len(), 1);
        assert!(!state.pending.contains("orca:one"));
        assert!(state.pending.contains("api:two"));
        assert_eq!(state.recovery["orca:one"].stage, "cooldown");
        assert_eq!(
            state.recovery["orca:one"].retry_at,
            Some(sent + ORCA_RETRY_GAP_MS)
        );
        state
            .last_recovery_at
            .insert("orca:one".into(), sent - ORCA_RETRY_GAP_MS);
        assert_eq!(queue_incidents(&mut state).len(), 1);
        assert_eq!(state.recovery["orca:one"].stage, "queued");
        assert!(state.recovery["orca:one"].retry_at.is_none());
    }

    #[test]
    fn progress_is_per_session_counts_only_verified_ips_and_stops_when_excluded() {
        let mut state = inner();
        state.orca_sessions = vec![target("orca:one", "orca", "rate-limited", Some("failure"))];
        state.other_sessions = vec![target("api:busy", "other", "busy", None)];
        queue_incidents(&mut state);
        state.pending_progress("connecting", "Relay 2 of 5");
        assert_eq!(state.recovery["orca:one"].ip_changes, 0);
        assert!(!state.recovery.contains_key("api:busy"));
        state.mark_verified_ip();
        assert_eq!(state.recovery["orca:one"].stage, "waiting-target");
        assert_eq!(state.recovery["orca:one"].ip_changes, 1);
        state
            .settings
            .session_monitoring
            .insert("orca:one".into(), false);
        state.prune_unmonitored();
        assert_eq!(state.recovery["orca:one"].stage, "paused");
        assert!(state.pending.is_empty() && state.ready_to_resume.is_empty());
        // A cooldown is not in the pending queue, but exclusion must cancel its countdown too.
        state.progress(
            "orca:one",
            "cooldown",
            "Waiting",
            Some(process::now_ms() + 10_000),
        );
        state.prune_unmonitored();
        assert_eq!(state.recovery["orca:one"].stage, "paused");
        assert!(state.recovery["orca:one"].retry_at.is_none());
    }

    #[test]
    fn saved_attempts_show_the_limit_after_restart_and_repeated_updates_keep_the_stage_time() {
        let mut state = inner();
        state.orca_sessions = vec![target("orca:one", "orca", "rate-limited", Some("failure"))];
        state.ip_attempts.insert("orca:one".into(), MAX_IP_CHANGES);
        let snapshot = state.snapshot();
        assert_eq!(snapshot.recovery["orca:one"].stage, "stopped");
        assert_eq!(snapshot.recovery["orca:one"].ip_changes, 2);
        state.progress("orca:one", "waiting", "Waiting for work", None);
        state.recovery.get_mut("orca:one").unwrap().updated_at = 42;
        state.progress("orca:one", "waiting", "Waiting for work", None);
        assert_eq!(state.recovery["orca:one"].updated_at, 42);
        state.orca_sessions[0].state = "completed".into();
        queue_incidents(&mut state);
        assert!(!state.snapshot().recovery.contains_key("orca:one"));
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

    #[test]
    fn a_stopped_session_in_continue_mode_is_queued_without_changing_the_public_ip() {
        let mut state = inner();
        state.settings.watcher = models::WATCH_WORKER_IDLE.into();
        state.orca_sessions = vec![stopped_target("api:one", "orca", "stop:api:one:operator-1")];
        assert_eq!(queue_incidents(&mut state).len(), 1);
        assert!(state.continue_pending.contains("api:one"));
        assert!(state.pending.is_empty());
        assert_eq!(state.recovery["api:one"].stage, "queued");
        assert!(!state.recovery["api:one"].changes_ip);
    }

    #[test]
    fn a_session_that_was_continued_for_its_stop_is_marked_as_continued() {
        let mut targets = vec![
            SessionTarget {
                id: "api:one".into(),
                stop_key: Some("stop:api:one:operator-1".into()),
                ..Default::default()
            },
            SessionTarget {
                id: "api:two".into(),
                stop_key: Some("stop:api:two:operator-1".into()),
                ..Default::default()
            },
            SessionTarget {
                id: "api:three".into(),
                ..Default::default()
            },
        ];
        let handled = ["stop:api:one:operator-1".to_string()]
            .into_iter()
            .collect::<HashSet<String>>();
        mark_continued(&mut targets, &handled);

        // The stop that was continued is marked, the one that was not is not,
        // and a session with no stop at all is left alone.
        assert!(targets[0].continued);
        assert!(!targets[1].continued);
        assert!(!targets[2].continued);
    }

    #[test]
    fn a_session_set_to_follow_the_default_uses_the_app_wide_mode() {
        let mut state = inner();
        state.settings.watcher = models::WATCH_WORKER_IDLE.into();
        state
            .settings
            .session_watcher
            .insert("api:one".into(), models::WATCH_DEFAULT.into());
        state.orca_sessions = vec![stopped_target("api:one", "orca", "stop:api:one:operator-1")];
        // The session has no choice of its own, so the app-wide mode decides.
        assert_eq!(queue_incidents(&mut state).len(), 1);
        assert!(state.continue_pending.contains("api:one"));
        assert!(state.pending.is_empty());
    }

    #[test]
    fn an_explicit_session_mode_survives_the_app_wide_mode_changing() {
        let mut state = inner();
        state.settings.watcher = models::WATCH_WORKER_IDLE.into();
        state
            .settings
            .session_watcher
            .insert("api:one".into(), models::WATCH_RATE_LIMIT.into());
        state.orca_sessions = vec![target("api:one", "orca", "rate-limited", Some("failure"))];
        assert_eq!(queue_incidents(&mut state).len(), 1);
        // The session still watches rate limits, so it needs the route.
        assert!(state.pending.contains("api:one"));
        assert!(state.continue_pending.is_empty());
    }

    #[test]
    fn the_route_is_not_needed_while_nothing_watches_rate_limits() {
        let mut state = inner();
        state.settings.watcher = models::WATCH_WORKER_IDLE.into();
        state.orca_sessions = vec![stopped_target("api:one", "orca", "stop:api:one:operator-1")];
        queue_incidents(&mut state);
        assert!(!state.snapshot().vpn_needed);

        // One session that still watches rate limits brings the route back.
        state
            .settings
            .session_watcher
            .insert("api:one".into(), models::WATCH_RATE_LIMIT.into());
        assert!(state.snapshot().vpn_needed);
    }

    #[test]
    fn a_session_that_stops_watching_rate_limits_drops_its_queued_route_change() {
        let mut state = inner();
        state.orca_sessions = vec![target("api:one", "orca", "rate-limited", Some("failure"))];
        assert_eq!(queue_incidents(&mut state).len(), 1);
        assert!(state.pending.contains("api:one"));

        // Switching the session to finished turns cancels the queued rotation.
        state
            .settings
            .session_watcher
            .insert("api:one".into(), models::WATCH_WORKER_IDLE.into());
        let target = state.target("api:one").unwrap();
        let now_worker_idle = state.settings.watches_worker_idle(&target);
        assert!(now_worker_idle);
        state.pending.remove("api:one");
        state.ready_to_resume.remove("api:one");
        state.progress_ip(
            "api:one",
            "paused",
            "This session now watches finished turns, so the queued route change was canceled",
            None,
            false,
        );
        assert!(state.pending.is_empty());
        assert_eq!(state.recovery["api:one"].stage, "paused");
    }

    #[test]
    fn a_rate_limited_session_still_needs_a_new_ip_in_rate_limit_mode() {
        let mut state = inner();
        state.orca_sessions = vec![target("api:one", "orca", "rate-limited", Some("failure"))];
        assert_eq!(queue_incidents(&mut state).len(), 1);
        assert!(state.pending.contains("api:one"));
        assert!(state.continue_pending.is_empty());
        assert_eq!(state.recovery["api:one"].stage, "queued");
        assert!(state.recovery["api:one"].changes_ip);
    }

    #[test]
    fn continue_mode_ignores_a_rate_limited_session() {
        let mut state = inner();
        state.settings.watcher = models::WATCH_WORKER_IDLE.into();
        state.orca_sessions = vec![target("api:one", "orca", "rate-limited", Some("failure"))];
        assert!(queue_incidents(&mut state).is_empty());
        assert!(state.pending.is_empty());
        assert!(state.continue_pending.is_empty());
        assert!(!state.recovery.contains_key("api:one"));
    }

    #[test]
    fn a_stop_key_is_continued_only_once() {
        let mut state = inner();
        state.settings.watcher = models::WATCH_WORKER_IDLE.into();
        state.orca_sessions = vec![stopped_target("api:one", "orca", "stop:api:one:operator-1")];
        assert_eq!(queue_incidents(&mut state).len(), 1);
        assert!(state.continue_pending.contains("api:one"));
        // The same stop is never queued twice while it is already queued.
        assert!(queue_incidents(&mut state).is_empty());
        assert_eq!(state.continue_pending.len(), 1);
        // Once the continuation is delivered the stop key stays handled.
        state.handled.insert("stop:api:one:operator-1".into());
        state.continue_pending.clear();
        assert!(queue_incidents(&mut state).is_empty());
        assert!(state.continue_pending.is_empty());
        assert!(state.pending.is_empty());
    }

    #[test]
    fn the_vpn_route_is_needed_only_when_something_changes_the_public_ip() {
        let mut state = inner();
        state.settings.watcher = models::WATCH_WORKER_IDLE.into();
        state.orca_sessions = vec![stopped_target("api:one", "orca", "stop:api:one:operator-1")];
        assert!(!state.snapshot().vpn_needed);
        queue_incidents(&mut state);
        // Continuing a stopped session never needs the VPN route.
        assert!(!state.snapshot().vpn_needed);

        let mut rotating = inner();
        rotating.orca_sessions = vec![target("api:two", "orca", "rate-limited", Some("failure"))];
        queue_incidents(&mut rotating);
        assert!(rotating.snapshot().vpn_needed);

        // A connected relay keeps the route visible even without a pending incident.
        let mut connected = inner();
        connected.connected_server = Some(VpnServer {
            id: "relay".into(),
            host: "relay.example".into(),
            ip: "198.51.100.7".into(),
            port: 443,
            country: "Japan".into(),
            ping_ms: 42,
            speed_mbps: 120.0,
            sessions: 3,
            score: 1.0,
        });
        assert!(connected.snapshot().vpn_needed);
    }

    #[test]
    fn a_continuation_is_planned_before_any_ip_rotation() {
        let mut state = inner();
        state.pending.insert("api:one".into());
        state.continue_pending.insert("api:two".into());
        match state.recovery_plan() {
            RecoveryPlan::Continue(ids) => assert_eq!(ids, vec!["api:two".to_string()]),
            _ => panic!("a stopped session is continued before any rotation"),
        }
        state.continue_pending.clear();
        assert!(matches!(state.recovery_plan(), RecoveryPlan::Rotate(_)));
    }
}
