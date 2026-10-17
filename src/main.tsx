import React, { useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity, ArrowUpRight, ChevronDown, CircleAlert, Clock3, Command,
  FolderClosed, Globe2, HardDrive, ListRestart, LoaderCircle, LockKeyhole,
  Monitor, Pause, Play, Radio, RefreshCw, RotateCcw, Save, Server,
  Settings2, ShieldCheck, Terminal, Wifi, WifiOff, X
} from "lucide-react";
import "./styles.css";

type VpnServer = {
  id: string; host: string; ip: string; port: number; country: string;
  pingMs: number; speedMbps: number; sessions: number; score: number;
};

type SessionTarget = {
  id: string; source: "orca" | "other"; title: string; project: string;
  state: string; lastError: string | null; errorKey: string | null;
  promptOverride: string | null; updatedAt: number;
};

type HistoryEntry = { at: number; level: string; message: string };

type Snapshot = {
  monitoring: boolean; phase: string; vpnStatus: string; publicIp: string | null;
  connectedServer: VpnServer | null; servers: VpnServer[]; refreshedAt: number | null;
  orcaSessions: SessionTarget[]; otherSessions: SessionTarget[]; history: HistoryEntry[];
  globalPrompt: string; preferredServer: string | null; lastError: string | null;
};

const empty: Snapshot = {
  monitoring: false, phase: "loading", vpnStatus: "checking", publicIp: null,
  connectedServer: null, servers: [], refreshedAt: null, orcaSessions: [],
  otherSessions: [], history: [], globalPrompt: "", preferredServer: null, lastError: null
};

function timeAgo(timestamp: number | null) {
  if (!timestamp) return "Never";
  const seconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return new Date(timestamp).toLocaleDateString();
}

function shortPath(path: string) {
  if (!path) return "Unknown project";
  const parts = path.replaceAll("\\", "/").split("/").filter(Boolean);
  return parts.length > 2 ? `${parts.slice(-2).join(" / ")}` : parts.join(" / ");
}

function projectName(path: string) {
  const parts = path.replaceAll("\\", "/").split("/").filter(Boolean);
  return parts.at(-1) || "Unknown project";
}

function statusLabel(state: string) {
  if (state === "rate-limited") return "Rate limited";
  if (state === "busy") return "Working";
  return "Ready";
}

function SessionRow({ target, onRecover, onPrompt, busy, showProject = true }: {
  target: SessionTarget;
  onRecover: (id: string) => void;
  onPrompt: (target: SessionTarget) => void;
  busy: boolean;
  showProject?: boolean;
}) {
  const isLimited = target.state === "rate-limited";
  return (
    <div className={`session-row ${isLimited ? "session-alert" : ""}`}>
      <div className="session-mark"><Terminal size={18} strokeWidth={1.8} /></div>
      <div className="session-copy">
        <div className="session-title-line">
          <strong title={target.title}>{target.title}</strong>
          <span className={`state-pill ${isLimited ? "bad" : target.state === "busy" ? "live" : ""}`}>
            <span className="state-dot" />{statusLabel(target.state)}
          </span>
        </div>
        <div className="session-subline">
          {showProject && <><span title={target.project}>{shortPath(target.project)}</span><span className="sub-separator" /></>}
          <span>{timeAgo(target.updatedAt)}</span>
          {target.promptOverride && <span className="custom-prompt">Custom prompt</span>}
        </div>
        {isLimited && target.lastError && <p className="session-error">{target.lastError}</p>}
      </div>
      <div className="session-actions">
        <button className="icon-button" title="Edit continuation prompt" aria-label={`Edit prompt for ${target.title}`} onClick={() => onPrompt(target)}><Settings2 size={17} /></button>
        <button className="small-button" disabled={busy} onClick={() => onRecover(target.id)}><RotateCcw size={14} /> Recover</button>
      </div>
    </div>
  );
}

function OrcaProjectGroup({ project, targets, onRecover, onPrompt, busy }: {
  project: string; targets: SessionTarget[];
  onRecover: (id: string) => void; onPrompt: (target: SessionTarget) => void; busy: boolean;
}) {
  const [open, setOpen] = useState(true);
  const limited = targets.filter(target => target.state === "rate-limited").length;
  return <section className="orca-project">
    <button className="project-heading" onClick={() => setOpen(value => !value)} aria-expanded={open} aria-label={`${projectName(project)} project, ${targets.length} ${targets.length === 1 ? "terminal" : "terminals"}`}>
      <span className="project-glyph"><FolderClosed size={17} strokeWidth={1.8} /></span>
      <span className="project-copy"><strong>{projectName(project)}</strong><small title={project}>{project || "Project path unavailable"}</small></span>
      <span className="project-count">{targets.length} {targets.length === 1 ? "terminal" : "terminals"}</span>
      {limited > 0 && <span className="section-warning">{limited} need recovery</span>}
      <ChevronDown size={16} className={`disclosure ${open ? "open" : ""}`} />
    </button>
    <div className={`reveal ${open ? "open" : ""}`} aria-hidden={!open} inert={!open}>
      <div className="reveal-inner">
        <div className="project-terminals">
          {targets.map(target => <SessionRow key={target.id} target={target} onRecover={onRecover} onPrompt={onPrompt} busy={busy} showProject={false} />)}
        </div>
      </div>
    </div>
  </section>;
}

function SessionSection({ title, subtitle, icon, targets, onRecover, onPrompt, busy, groupByProject = false }: {
  title: string; subtitle: string; icon: React.ReactNode; targets: SessionTarget[];
  onRecover: (id: string) => void; onPrompt: (target: SessionTarget) => void; busy: boolean;
  groupByProject?: boolean;
}) {
  const [open, setOpen] = useState(true);
  const limited = targets.filter(target => target.state === "rate-limited").length;
  const projects = useMemo(() => {
    const groups = new Map<string, SessionTarget[]>();
    for (const target of targets) {
      const project = target.project || "";
      groups.set(project, [...(groups.get(project) || []), target]);
    }
    return [...groups.entries()].sort(([a], [b]) => projectName(a).localeCompare(projectName(b)));
  }, [targets]);
  return (
    <section className="panel session-panel">
      <button className="section-heading collapsible" onClick={() => setOpen(value => !value)} aria-expanded={open}>
        <div className="heading-icon">{icon}</div>
        <div className="heading-text"><h2>{title}</h2><p>{subtitle}</p></div>
        <span className="section-count">{targets.length} {groupByProject ? targets.length === 1 ? "terminal" : "terminals" : targets.length === 1 ? "session" : "sessions"}</span>
        {limited > 0 && <span className="section-warning">{limited} need recovery</span>}
        <ChevronDown size={17} className={`disclosure ${open ? "open" : ""}`} />
      </button>
      <div className={`reveal ${open ? "open" : ""}`} aria-hidden={!open} inert={!open}>
        <div className="reveal-inner">
          <div className={groupByProject ? "project-list" : "session-list"}>
            {targets.length ? (groupByProject ?
              projects.map(([project, group]) => <OrcaProjectGroup key={project} project={project} targets={group} onRecover={onRecover} onPrompt={onPrompt} busy={busy} />) :
              targets.map(target => <SessionRow key={target.id} target={target} onRecover={onRecover} onPrompt={onPrompt} busy={busy} />)) :
              <div className="empty-state"><span className="empty-icon"><Terminal size={20} /></span><div><strong>{groupByProject ? "No active Orca terminals" : "No active OpenCode sessions"}</strong><span>{groupByProject ? "Terminals appear here when an Orca project is active." : "Only active shared-service work appears here."}</span></div></div>}
          </div>
        </div>
      </div>
    </section>
  );
}

function PromptDialog({ target, current, global, onClose, onSave }: {
  target: SessionTarget | null; current: string; global: string;
  onClose: () => void; onSave: (value: string | null) => void;
}) {
  const [value, setValue] = useState(current);
  const [closing, setClosing] = useState(false);
  const closeTimer = useRef<number | null>(null);
  useEffect(() => setValue(current), [current, target]);
  useEffect(() => () => { if (closeTimer.current !== null) window.clearTimeout(closeTimer.current); }, []);

  function dismiss() {
    if (closing) return;
    setClosing(true);
    closeTimer.current = window.setTimeout(onClose, 160);
  }

  return (
    <div className={`dialog-backdrop ${closing ? "closing" : ""}`} onMouseDown={dismiss} onKeyDown={event => { if (event.key === "Escape") dismiss(); }}>
      <div className="dialog" role="dialog" aria-modal="true" aria-label="Edit continuation prompt" onMouseDown={event => event.stopPropagation()}>
        <div className="dialog-top"><div><span className="eyebrow">RECOVERY MESSAGE</span><h2>{target ? "Session prompt" : "Default prompt"}</h2></div><button className="icon-button" onClick={dismiss} aria-label="Close"><X size={18} /></button></div>
        <p className="dialog-intro">{target ? `Used when ${target.title} is ready to continue. Leave it empty to use the default prompt.` : "Sent to affected sessions after a verified VPN connection."}</p>
        <textarea value={value} onChange={event => setValue(event.target.value)} rows={5} autoFocus placeholder={target ? global : "Enter a continuation prompt"} />
        <div className="dialog-footer">
          {target && <button className="text-button" onClick={() => onSave(null)}>Use default</button>}
          <div className="dialog-spacer" />
          <button className="secondary-button" onClick={dismiss}>Cancel</button>
          <button className="primary-button" disabled={!target && !value.trim()} onClick={() => onSave(value.trim() || null)}><Save size={15} /> Save prompt</button>
        </div>
      </div>
    </div>
  );
}

function App() {
  const [snapshot, setSnapshot] = useState<Snapshot>(empty);
  const [loaded, setLoaded] = useState(false);
  const [action, setAction] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [promptTarget, setPromptTarget] = useState<SessionTarget | null | "global">(null);
  const [showAllServers, setShowAllServers] = useState(false);
  const desktop = isTauri();

  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    const unlisten = listen<Snapshot>("snapshot", event => { if (!disposed) { setSnapshot(event.payload); setLoaded(true); } })
      .catch(error => { if (!disposed) setNotice(String(error)); return () => {}; });
    invoke<Snapshot>("get_snapshot")
      .then(value => { if (!disposed) { setSnapshot(value); setLoaded(true); } })
      .catch(error => { if (!disposed) setNotice(String(error)); });
    return () => { disposed = true; unlisten.then(fn => fn()).catch(() => {}); };
  }, [desktop]);

  async function call(command: string, args?: Record<string, unknown>) {
    if (!desktop) return;
    setAction(command);
    setNotice(null);
    try {
      const result = await invoke<Snapshot>(command, args);
      setSnapshot(result);
    } catch (error) {
      setNotice(String(error));
    } finally {
      setAction(null);
    }
  }

  const sessions = [...snapshot.orcaSessions, ...snapshot.otherSessions];
  const limitedCount = sessions.filter(target => target.state === "rate-limited").length;
  const busy = snapshot.phase.startsWith("connecting") || snapshot.phase === "resuming sessions" || snapshot.phase.startsWith("waiting");
  const visibleServers = showAllServers ? snapshot.servers.slice(0, 30) : snapshot.servers.slice(0, 7);
  const current = snapshot.connectedServer;
  const recoveryLabel = busy ? snapshot.phase : snapshot.monitoring ? "Watching for rate limits" : "Monitoring is paused";
  const promptDialogTarget = promptTarget && promptTarget !== "global" ? promptTarget : null;
  const promptCurrent = promptTarget === "global" ? snapshot.globalPrompt : promptDialogTarget?.promptOverride ?? "";
  const recentHistory = useMemo(() => snapshot.history.slice(0, 8), [snapshot.history]);

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="topbar-inner">
          <div className="brand">
            <span className="brand-symbol"><ListRestart size={21} strokeWidth={2.2} /></span>
            <span className="brand-copy"><strong>OpenCode Recovery</strong><small>Network continuity</small></span>
          </div>
          <div className="topbar-right">
            <span className="local-badge"><LockKeyhole size={13} /> LOCAL DESKTOP</span>
            <span className="top-divider" />
            <span className="softether-name">SOFTETHER + VPN GATE</span>
          </div>
        </div>
      </header>

      <main>
        <div className="page-heading">
          <div>
            <span className="eyebrow"><span className="eyebrow-line" /> CONTROL CENTER</span>
            <h1>Stay in the flow.</h1>
            <p>Watch active sessions and recover them when a rate limit interrupts the work.</p>
          </div>
          <button
            className={`monitor-toggle ${snapshot.monitoring ? "enabled" : ""}`}
            role="switch"
            aria-checked={snapshot.monitoring}
            disabled={!desktop || !!action}
            onClick={() => call("set_monitoring", { enabled: !snapshot.monitoring })}
          >
            <span className="toggle-track"><span /></span>
            <span className="monitor-copy"><strong>{snapshot.monitoring ? "Monitoring on" : "Start monitoring"}</strong><small>{snapshot.monitoring ? "Watching active work" : "Paused until you start"}</small></span>
            {snapshot.monitoring ? <Radio size={17} /> : <Play size={16} />}
          </button>
        </div>

        {!desktop && <div className="preview-note"><Monitor size={17} /><span>Browser preview. Open the desktop app to see live VPN and session data.</span></div>}
        {notice && <div className="notice" role="alert"><CircleAlert size={18} /><span>{notice}</span><button onClick={() => setNotice(null)} aria-label="Dismiss"><X size={16} /></button></div>}

        <section className="status-card" aria-label="Current connection">
          <div className="status-main">
            <span className={`status-orb ${snapshot.vpnStatus === "connected" ? "connected" : ""}`}><Globe2 size={31} strokeWidth={1.55} /></span>
            <div className="status-copy">
              <span className="status-kicker">CURRENT ROUTE</span>
              <h2>{current ? `${current.country} relay` : "No VPN connection"}</h2>
              <p>{current ? `${current.host} · ${current.ip}:${current.port}` : "A verified route will appear here when recovery begins."}</p>
            </div>
            <span className={`connection-state ${snapshot.vpnStatus}`}><span />{snapshot.vpnStatus}</span>
          </div>
          <div className="status-details">
            <div><span className="detail-label">PUBLIC IP</span><strong className="mono">{snapshot.publicIp || "Checking…"}</strong></div>
            <div><span className="detail-label">RECOVERY STATE</span><strong>{recoveryLabel}</strong></div>
            {current && <div className="status-action"><button className="disconnect-button" disabled={!!action || busy} onClick={() => call("disconnect_vpn")}>Disconnect VPN <ArrowUpRight size={15} /></button></div>}
          </div>
        </section>

        <div className="metrics-grid" aria-label="At a glance">
          <div className="metric"><span className="metric-icon blue"><Monitor size={18} /></span><div><span>ACTIVE SESSIONS</span><strong>{sessions.length}</strong></div></div>
          <div className="metric"><span className="metric-icon amber"><CircleAlert size={18} /></span><div><span>NEED RECOVERY</span><strong>{limitedCount}</strong></div></div>
          <div className="metric"><span className="metric-icon green"><Server size={18} /></span><div><span>AVAILABLE RELAYS</span><strong>{snapshot.servers.length}</strong></div></div>
          <span className={`phase-indicator ${busy ? "working" : ""}`}>{busy ? <LoaderCircle size={17} className="spin" /> : snapshot.monitoring ? <Activity size={17} /> : <Pause size={17} />}{busy ? "Recovery in progress" : snapshot.monitoring ? "Ready to recover" : "Standing by"}</span>
        </div>

        <div className="content-grid">
          <div className="primary-column">
            <div className="column-title"><div><span className="eyebrow">YOUR WORK</span><h2>Active sessions</h2><p>Orca projects and live OpenCode work, in one place.</p></div><button className="secondary-button" disabled={!desktop} onClick={() => setPromptTarget("global")}><Settings2 size={16} /> Default prompt</button></div>
            <SessionSection title="Orca terminals" subtitle="Grouped by project" icon={<Command size={19} />} targets={snapshot.orcaSessions} onRecover={id => call("recover_now", { targetId: id })} onPrompt={setPromptTarget} busy={busy} groupByProject />
            <SessionSection title="Other OpenCode sessions" subtitle="Active shared-service work and affected sessions" icon={<HardDrive size={19} />} targets={snapshot.otherSessions} onRecover={id => call("recover_now", { targetId: id })} onPrompt={setPromptTarget} busy={busy} />

            <section className="panel history-panel">
              <div className="section-heading plain"><span className="heading-icon"><Clock3 size={19} /></span><div className="heading-text"><h2>Recovery history</h2><p>Recent connection and session events</p></div></div>
              <div className="history-list">{recentHistory.length ? recentHistory.map((entry, index) => <div className="history-row" key={`${entry.at}-${index}`}><span className={`history-dot ${entry.level}`} /><span>{entry.message}</span><time>{timeAgo(entry.at)}</time></div>) : <div className="empty-state history-empty"><span className="empty-icon"><Activity size={20} /></span><div><strong>No recovery events yet</strong><span>Connection and continuation activity will appear here.</span></div></div>}</div>
            </section>
          </div>

          <aside className="secondary-column">
            <section className="panel relay-panel">
              <div className="relay-heading"><div><span className="eyebrow">VPN GATE</span><h2>Recommended relays</h2><p>Lower ping and higher line speed rise to the top.</p></div><button className="secondary-button refresh-button" disabled={!desktop || action === "refresh_servers"} onClick={() => call("refresh_servers")}>{action === "refresh_servers" ? <LoaderCircle className="spin" size={16} /> : <RefreshCw size={16} />} Refresh list</button></div>
              <div className="relay-meta"><span><span className="live-dot" /> {snapshot.servers.length} eligible TCP servers</span><span>Updated {timeAgo(snapshot.refreshedAt)}</span></div>
              <div className="server-table">
                <div className="table-header"><span>SERVER</span><span>PING</span><span>SPEED</span></div>
                {visibleServers.length ? visibleServers.map((server, index) => <button key={server.id} className={`server-row ${snapshot.preferredServer === server.id ? "preferred" : ""} ${current?.id === server.id ? "current" : ""}`} aria-pressed={snapshot.preferredServer === server.id} title="Prioritize this server for the next recovery" onClick={() => call("set_preferred_server", { serverId: snapshot.preferredServer === server.id ? null : server.id })}><span className="server-name"><i>{String(index + 1).padStart(2, "0")}</i><span><strong>{server.country}{snapshot.preferredServer === server.id && <em>Preferred</em>}{current?.id === server.id && <em>Connected</em>}</strong><small>{server.host}</small></span></span><span className="ping"><Wifi size={13} />{server.pingMs} ms</span><span className="speed">{Math.round(server.speedMbps)} <small>Mbps</small></span></button>) : <div className="empty-state relay-empty"><span className="empty-icon"><WifiOff size={20} /></span><div><strong>No relay list yet</strong><span>Refresh to find available VPN Gate servers.</span></div></div>}
              </div>
              {snapshot.servers.length > 7 && <button className="show-more" onClick={() => setShowAllServers(value => !value)}>{showAllServers ? "Show fewer relays" : `Show more relays (${Math.min(snapshot.servers.length, 30) - 7})`} <ChevronDown size={15} className={showAllServers ? "up" : ""} /></button>}
            </section>
            <div className="info-card"><ShieldCheck size={20} /><div><strong>Verified before continuing</strong><p>Recovery resumes only after SoftEther connects and your public IP changes.</p></div></div>
          </aside>
        </div>
        <footer><span>OpenCode Recovery · Windows desktop</span><span>{desktop ? loaded ? "Connected to local controller" : "Loading controller…" : "Browser preview"}</span></footer>
      </main>
      {promptTarget && <PromptDialog target={promptDialogTarget} current={promptCurrent} global={snapshot.globalPrompt} onClose={() => setPromptTarget(null)} onSave={value => { if (promptTarget === "global") { call("set_global_prompt", { prompt: value || snapshot.globalPrompt }); } else { call("set_prompt_override", { targetId: promptTarget.id, prompt: value }); } setPromptTarget(null); }} />}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<React.StrictMode><App /></React.StrictMode>);
