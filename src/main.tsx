import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity, ArrowDownRight, ArrowUpRight, ChevronDown, ChevronRight, CircleAlert,
  Clock3, Command, Globe2, HardDrive, ListRestart, LoaderCircle,
  LockKeyhole, Monitor, Pause, Play, Radio, RefreshCw, RotateCcw, Save,
  Server, Settings2, ShieldCheck, Terminal, Wifi, WifiOff, X
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
        <button className="icon-button" title="Edit continuation prompt" onClick={() => onPrompt(target)}><Settings2 size={17} /></button>
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
    <button className="project-heading" onClick={() => setOpen(value => !value)} aria-expanded={open} aria-label={`${projectName(project)} project, ${targets.length} terminals`}>
      <span className="project-glyph"><Command size={16} strokeWidth={1.8} /></span>
      <span className="project-copy"><strong>{projectName(project)}</strong><small title={project}>{project || "Project path unavailable"}</small></span>
      <span className="project-count">{targets.length} {targets.length === 1 ? "terminal" : "terminals"}</span>
      {limited > 0 && <span className="section-warning">{limited} need recovery</span>}
      {open ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
    </button>
    {open && <div className="project-terminals">
      {targets.map(target => <SessionRow key={target.id} target={target} onRecover={onRecover} onPrompt={onPrompt} busy={busy} showProject={false} />)}
    </div>}
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
        <span className="section-count">{targets.length} sessions</span>
        {limited > 0 && <span className="section-warning">{limited} need recovery</span>}
        {open ? <ChevronDown size={17} /> : <ChevronRight size={17} />}
      </button>
      {open && <div className={groupByProject ? "project-list" : "session-list"}>
        {targets.length ? (groupByProject ?
          projects.map(([project, group]) => <OrcaProjectGroup key={project} project={project} targets={group} onRecover={onRecover} onPrompt={onPrompt} busy={busy} />) :
          targets.map(target => <SessionRow key={target.id} target={target} onRecover={onRecover} onPrompt={onPrompt} busy={busy} />)) :
          <div className="empty-state"><Terminal size={21} /><span>{groupByProject ? "No active Orca terminals found." : "No active OpenCode sessions found."}</span></div>}
      </div>}
    </section>
  );
}

function PromptDialog({ target, current, global, onClose, onSave }: {
  target: SessionTarget | null; current: string; global: string;
  onClose: () => void; onSave: (value: string | null) => void;
}) {
  const [value, setValue] = useState(current);
  useEffect(() => setValue(current), [current, target]);
  return (
    <div className="dialog-backdrop" onMouseDown={onClose}>
      <div className="dialog" role="dialog" aria-modal="true" aria-label="Edit continuation prompt" onMouseDown={event => event.stopPropagation()}>
        <div className="dialog-top"><div><span className="eyebrow">RECOVERY MESSAGE</span><h2>{target ? "Session prompt" : "Default prompt"}</h2></div><button className="icon-button" onClick={onClose} aria-label="Close"><X size={18} /></button></div>
        <p className="dialog-intro">{target ? `Used when ${target.title} is ready to continue. Leave it empty to use the default prompt.` : "Sent to affected sessions after a verified VPN connection."}</p>
        <textarea value={value} onChange={event => setValue(event.target.value)} rows={5} autoFocus placeholder={target ? global : "Enter a continuation prompt"} />
        <div className="dialog-footer">
          {target && <button className="text-button" onClick={() => onSave(null)}>Use default</button>}
          <div className="dialog-spacer" />
          <button className="secondary-button" onClick={onClose}>Cancel</button>
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

  useEffect(() => {
    let disposed = false;
    const unlisten = listen<Snapshot>("snapshot", event => { if (!disposed) { setSnapshot(event.payload); setLoaded(true); } });
    invoke<Snapshot>("get_snapshot").then(value => { if (!disposed) { setSnapshot(value); setLoaded(true); } }).catch(error => setNotice(String(error)));
    return () => { disposed = true; unlisten.then(fn => fn()); };
  }, []);

  async function call(command: string, args?: Record<string, unknown>) {
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

  return <div className="app-shell">
    <header className="topbar">
      <div className="brand"><div className="brand-symbol"><ListRestart size={21} strokeWidth={2.2} /></div><div><strong>OpenCode Recovery</strong><span>Network continuity console</span></div></div>
      <div className="topbar-right"><span className="local-badge"><LockKeyhole size={13} /> LOCAL APP</span><span className="top-divider" /><span className="softether-name">SOFTETHER / VPN GATE</span></div>
    </header>

    <main>
      <div className="page-heading"><div><span className="eyebrow">OPERATIONS OVERVIEW</span><h1>Keep your sessions moving<span className="heading-period">.</span></h1><p>One connection, every project. Recovery starts only after the new route is verified.</p></div><button className={`monitor-toggle ${snapshot.monitoring ? "enabled" : ""}`} disabled={!!action} onClick={() => call("set_monitoring", { enabled: !snapshot.monitoring })}><span className="toggle-track"><span /></span><span>{snapshot.monitoring ? "Monitoring on" : "Start monitoring"}</span>{snapshot.monitoring ? <Radio size={16} /> : <Play size={15} />}</button></div>

      {notice && <div className="notice"><CircleAlert size={18} /><span>{notice}</span><button onClick={() => setNotice(null)} aria-label="Dismiss"><X size={16} /></button></div>}

      <section className="status-card">
        <div className="status-main"><div className={`status-orb ${snapshot.vpnStatus === "connected" ? "connected" : ""}`}><Globe2 size={33} strokeWidth={1.5} /></div><div><span className="eyebrow">CURRENT CONNECTION</span><h2>{current ? current.host : "No VPN connection"}</h2><p>{current ? `${current.country} · ${current.ip}:${current.port}` : "Ready to find a reliable relay when recovery is needed."}</p></div></div>
        <div className="status-details"><div><span className="detail-label">PUBLIC IP</span><strong className="mono">{snapshot.publicIp || "Checking…"}</strong></div><div><span className="detail-label">VPN STATUS</span><strong className={`connection-state ${snapshot.vpnStatus}`}><span />{snapshot.vpnStatus}</strong></div></div>
        <div className="status-footer"><span className={`phase-indicator ${busy ? "working" : ""}`}>{busy ? <LoaderCircle size={16} className="spin" /> : snapshot.monitoring ? <Activity size={16} /> : <Pause size={16} />}{recoveryLabel}</span>{current && <button className="text-button disconnect" disabled={!!action || busy} onClick={() => call("disconnect_vpn")}>Disconnect VPN <ArrowUpRight size={14} /></button>}</div>
      </section>

      <div className="metrics-grid"><div className="metric"><div className="metric-icon blue"><Monitor size={19} /></div><div><span>MONITORED SESSIONS</span><strong>{sessions.length}</strong></div><ArrowUpRight size={15} className="metric-arrow" /></div><div className="metric"><div className="metric-icon amber"><CircleAlert size={19} /></div><div><span>NEED RECOVERY</span><strong>{limitedCount}</strong></div><ArrowDownRight size={15} className="metric-arrow" /></div><div className="metric"><div className="metric-icon green"><Server size={19} /></div><div><span>AVAILABLE RELAYS</span><strong>{snapshot.servers.length}</strong></div><ArrowUpRight size={15} className="metric-arrow" /></div></div>

      <div className="content-grid">
        <div className="primary-column">
          <div className="column-title"><div><span className="eyebrow">SESSION WATCH</span><h2>Active workspaces</h2></div><button className="secondary-button" onClick={() => setPromptTarget("global")}><Settings2 size={16} /> Default prompt</button></div>
          <SessionSection title="Orca terminals" subtitle="OpenCode terminals grouped by project" icon={<Command size={19} />} targets={snapshot.orcaSessions} onRecover={id => call("recover_now", { targetId: id })} onPrompt={setPromptTarget} busy={busy} groupByProject />
          <SessionSection title="Other OpenCode sessions" subtitle="Live shared-service work and affected sessions" icon={<HardDrive size={19} />} targets={snapshot.otherSessions} onRecover={id => call("recover_now", { targetId: id })} onPrompt={setPromptTarget} busy={busy} />

          <section className="panel history-panel"><div className="section-heading plain"><div className="heading-icon"><Clock3 size={19} /></div><div className="heading-text"><h2>Recovery history</h2><p>Recent connection and session events</p></div></div><div className="history-list">{recentHistory.length ? recentHistory.map((entry, index) => <div className="history-row" key={`${entry.at}-${index}`}><span className={`history-dot ${entry.level}`} /><span>{entry.message}</span><time>{timeAgo(entry.at)}</time></div>) : <div className="empty-state"><Activity size={20} /><span>Events will appear here as monitoring runs.</span></div>}</div></section>
        </div>

        <aside className="secondary-column">
          <section className="panel relay-panel"><div className="relay-heading"><div><span className="eyebrow">VPN GATE NETWORK</span><h2>Recommended relays</h2><p>Ranked by latency and line speed</p></div><button className="secondary-button refresh-button" disabled={action === "refresh_servers"} onClick={() => call("refresh_servers")}>{action === "refresh_servers" ? <LoaderCircle className="spin" size={16} /> : <RefreshCw size={16} />} Refresh list</button></div><div className="relay-meta"><span><span className="live-dot" /> {snapshot.servers.length} eligible TCP servers</span><span>Updated {timeAgo(snapshot.refreshedAt)}</span></div><div className="server-table"><div className="table-header"><span>SERVER</span><span>PING</span><span>SPEED</span></div>{visibleServers.length ? visibleServers.map((server, index) => <button key={server.id} className={`server-row ${snapshot.preferredServer === server.id ? "preferred" : ""} ${current?.id === server.id ? "current" : ""}`} title="Prioritize this server for the next recovery" onClick={() => call("set_preferred_server", { serverId: snapshot.preferredServer === server.id ? null : server.id })}><span className="server-name"><i>{String(index + 1).padStart(2, "0")}</i><span><strong>{server.country}</strong><small>{server.host}</small></span></span><span className="ping"><Wifi size={13} />{server.pingMs} ms</span><span className="speed">{Math.round(server.speedMbps)} <small>Mbps</small></span></button>) : <div className="empty-state"><WifiOff size={20} /><span>Waiting for a server list.</span></div>}</div>{snapshot.servers.length > 7 && <button className="show-more" onClick={() => setShowAllServers(value => !value)}>{showAllServers ? "Show fewer relays" : `Show more relays (${Math.min(snapshot.servers.length, 30) - 7})`} <ChevronDown size={15} className={showAllServers ? "up" : ""} /></button>}</section>
          <div className="info-card"><ShieldCheck size={19} /><div><strong>Connection check</strong><p>A relay is used only after SoftEther reports an established session and the public IP changes.</p></div></div>
        </aside>
      </div>
      <footer><span>OpenCode Recovery · Windows desktop</span><span>{loaded ? "Connected to local controller" : "Loading controller…"}</span></footer>
    </main>
    {promptTarget && <PromptDialog target={promptDialogTarget} current={promptCurrent} global={snapshot.globalPrompt} onClose={() => setPromptTarget(null)} onSave={value => { if (promptTarget === "global") { call("set_global_prompt", { prompt: value || snapshot.globalPrompt }); } else { call("set_prompt_override", { targetId: promptTarget.id, prompt: value }); } setPromptTarget(null); }} />}
  </div>;
}

createRoot(document.getElementById("root")!).render(<React.StrictMode><App /></React.StrictMode>);
