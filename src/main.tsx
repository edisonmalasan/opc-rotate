import React, { useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  ArrowDown,
  ArrowUpRight,
  Check,
  ChevronRight,
  CircleAlert,
  Clock3,
  Command,
  Folder,
  Globe2,
  LoaderCircle,
  PanelLeft,
  Radio,
  RefreshCw,
  RotateCcw,
  Search,
  Settings2,
  ShieldCheck,
  Terminal,
  X,
} from "lucide-react";
import {
  EMPTY_SNAPSHOT,
  type RecoveryProgress,
  type SessionTarget,
  type Snapshot,
  type VpnServer,
} from "./types";
import "./styles.css";
import appIcon from "./assets/app-icon.png";
import { changeMonitoring, isMonitored, terminalKey } from "./monitoring";

type View = "sessions" | "relays" | "activity";
type SessionFilter = "all" | "limited" | "busy" | "idle";
type PromptEditor = { target: SessionTarget | null; instant: boolean };
type RunCommand = (
  command: string,
  args?: Record<string, unknown>,
) => Promise<string | null>;

function timeAgo(at: number | null) {
  if (!at) return "Not yet";
  const seconds = Math.max(0, Math.floor((Date.now() - at) / 1000));
  if (seconds < 60) return "Just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return new Date(at).toLocaleDateString();
}

function projectName(path: string) {
  return (
    path.replaceAll("\\", "/").split("/").filter(Boolean).at(-1) ||
    "Unknown project"
  );
}

function projectKey(path: string) {
  return path
    .replaceAll("\\", "/")
    .replace(/\/+/g, "/")
    .replace(/\/$/, "")
    .toLowerCase();
}

function groupProjects(targets: SessionTarget[]) {
  const groups = new Map<
    string,
    { project: string; targets: SessionTarget[] }
  >();
  for (const target of targets) {
    const key = projectKey(target.project);
    const group = groups.get(key) ?? { project: target.project, targets: [] };
    group.targets.push(target);
    groups.set(key, group);
  }
  return [...groups.values()].sort(
    (a, b) =>
      projectName(a.project).localeCompare(projectName(b.project)) ||
      a.project.localeCompare(b.project),
  );
}

function sessionTitle(title: string) {
  return title.replace(/^OC\s*\|\s*/i, "");
}
function stateLabel(state: string) {
  return state === "rate-limited"
    ? "Needs recovery"
    : state === "busy"
      ? "Working"
      : state === "idle"
        ? "Idle"
        : state === "completed"
          ? "Completed"
          : "Status unavailable";
}

function tabLabel(target: SessionTarget) {
  return target.viaPlugin
    ? !target.clientId
      ? "Closed tab"
      : target.selectedTab
        ? "Selected tab"
        : "Background tab"
    : target.source === "orca"
      ? "Waiting for tab plugin"
      : "Shared service";
}
function countryName(country: string) {
  return country === "Korea Republic of" ? "South Korea" : country;
}
function countryCode(country: string) {
  return (
    (
      {
        Japan: "JP",
        "Korea Republic of": "KR",
        Singapore: "SG",
        "United States": "US",
        Taiwan: "TW",
        Germany: "DE",
        Thailand: "TH",
        Vietnam: "VN",
      } as Record<string, string>
    )[country] || country.slice(0, 2).toUpperCase()
  );
}
function lineSpeed(speed: number) {
  return speed >= 1000
    ? `${(speed / 1000).toFixed(2)} Gbps`
    : `${Math.round(speed)} Mbps`;
}

function StateLabel({ state }: { state: string }) {
  return (
    <span className={`session-state ${state}`}>
      <i />
      {stateLabel(state)}
    </span>
  );
}

const recoveryLabels: Record<string, string> = {
  queued: "Queued", waiting: "Waiting for work", cooldown: "Retry eligible",
  checking: "Checking route", disconnecting: "Switching VPN", connecting: "Connecting VPN",
  verifying: "Verifying IP", "waiting-target": "Waiting for idle", blocked: "Waiting for client",
  sending: "Sending prompt", sent: "Prompt sent", stopped: "Auto limit reached",
  failed: "Recovery failed", uncertain: "Check delivery", paused: "Recovery paused", history: "Recovery history",
  unavailable: "Client unavailable",
};
const animatedRecovery = new Set(["checking", "disconnecting", "connecting", "verifying", "sending"]);
const activeRecovery = new Set(["queued", "waiting", "checking", "disconnecting", "connecting", "verifying", "waiting-target", "blocked", "sending"]);

function RecoveryStatus({ progress, detailed = false }: { progress?: RecoveryProgress; detailed?: boolean }) {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    if (!progress?.retryAt) return;
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [progress?.retryAt]);
  if (!progress) return null;
  const remaining = progress.retryAt ? Math.max(0, Math.ceil((progress.retryAt - now) / 1000)) : 0;
  const label = progress.stage === "cooldown"
    ? remaining > 0 ? `Eligible in ${remaining}s` : "Awaiting next check"
    : recoveryLabels[progress.stage] || "Recovery update";
  return (
    <span className={`recovery-status ${progress.stage} ${detailed ? "detailed" : ""}`} title={progress.detail}>
      {animatedRecovery.has(progress.stage) ? <LoaderCircle className="spin" size={13} aria-hidden="true" />
        : progress.stage === "sent" ? <Check size={13} aria-hidden="true" />
          : ["stopped", "failed", "uncertain"].includes(progress.stage) ? <CircleAlert size={13} aria-hidden="true" />
            : <Clock3 size={13} aria-hidden="true" />}
      <span role={progress.stage === "cooldown" ? "timer" : undefined} aria-live={progress.stage === "cooldown" ? "off" : undefined}>{label}</span>
    </span>
  );
}

function EmptyState({
  icon,
  title,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="empty-state">
      <span className="empty-symbol">{icon}</span>
      <h2>{title}</h2>
      <p>{children}</p>
    </div>
  );
}

function SessionItem({
  target,
  progress,
  selected,
  monitored,
  monitoringDisabled,
  onMonitoringChange,
  onSelect,
}: {
  target: SessionTarget;
  progress?: RecoveryProgress;
  selected: boolean;
  monitored: boolean;
  monitoringDisabled: boolean;
  onMonitoringChange: (enabled: boolean) => void;
  onSelect: () => void;
}) {
  return (
    <div className="session-entry">
      <label
        className="session-monitor"
        title={
          monitored
            ? "Included when monitoring is on"
            : "Excluded from automatic recovery"
        }
      >
        <input
          type="checkbox"
          aria-label={`Monitor ${sessionTitle(target.title)}`}
          checked={monitored}
          disabled={monitoringDisabled}
          onChange={(event) => onMonitoringChange(event.target.checked)}
        />
      </label>
      <button
        className={`session-item ${selected ? "selected" : ""}`}
        data-session-id={target.id}
        aria-pressed={selected}
        onClick={onSelect}
      >
        <span className={`session-symbol ${target.state}`}>
          <Terminal size={18} strokeWidth={1.6} />
        </span>
        <span className="session-item-copy">
          <strong title={sessionTitle(target.title)}>
            {sessionTitle(target.title)}
          </strong>
          <small>
            {tabLabel(target)}
            {!monitored && (
              <>
                <span>·</span>Not monitored
              </>
            )}
            {target.promptOverride && (
              <>
                <span>·</span>Custom prompt
              </>
            )}
          </small>
        </span>
        <span className="session-status-stack">
          <StateLabel state={target.state} />
          <RecoveryStatus progress={progress} />
        </span>
        <time dateTime={new Date(target.updatedAt).toISOString()}>
          {timeAgo(target.updatedAt)}
        </time>
        <ChevronRight className="row-chevron" size={15} />
      </button>
    </div>
  );
}

function SessionInspector({
  target,
  snapshot,
  canRecover,
  pending,
  onClose,
  onEdit,
  run,
}: {
  target: SessionTarget | null;
  snapshot: Snapshot;
  canRecover: boolean;
  pending: string | null;
  onClose: () => void;
  onEdit: (target: SessionTarget, instant: boolean) => void;
  run: RunCommand;
}) {
  if (!target)
    return (
      <aside className="inspector empty-inspector">
        <EmptyState
          icon={<PanelLeft size={27} strokeWidth={1.3} />}
          title="Session details"
        >
          Select a session to review its status and continuation prompt.
        </EmptyState>
      </aside>
    );
  const monitored = isMonitored(snapshot, target);
  const progress = snapshot.recovery?.[target.id];
  const siblings = [...snapshot.orcaSessions, ...snapshot.otherSessions].filter(
    (item) => terminalKey(item) === terminalKey(target),
  );
  const terminalIncluded =
    snapshot.terminalMonitoring[terminalKey(target) || ""] ?? true;
  const terminalMixed = siblings.some(
    (item) => isMonitored(snapshot, item) !== terminalIncluded,
  );
  return (
    <aside className="inspector" aria-label="Selected session details">
      <div className="inspector-toolbar">
        <span>Session details</span>
        <button
          className="icon-button"
          aria-label="Close session details"
          onClick={onClose}
        >
          <X size={16} />
        </button>
      </div>
      <div className="inspector-scroll">
        <h2 className="inspector-title">{sessionTitle(target.title)}</h2>
        <div className="inspector-session-meta">
          <StateLabel state={target.state} />
          <span>{target.source === "orca" ? "Orca terminal" : "OpenCode"}</span>
        </div>
        {target.viaPlugin && (
          <div className="tab-context-actions">
            <span>{tabLabel(target)}</span>
            <button
              className="text-button"
              disabled={
                !target.clientId || target.selectedTab || pending !== null
              }
              onClick={() => run("select_session_tab", { targetId: target.id })}
            >
              {target.selectedTab ? (
                <>
                  <Check size={13} /> Selected
                </>
              ) : (
                "Select tab"
              )}
            </button>
          </div>
        )}
        <section className="inspector-section recovery-panel" aria-label="Session recovery progress">
          <h3>Recovery progress</h3>
          <div role="status" aria-live="polite" aria-atomic="true">
            {progress ? <RecoveryStatus progress={progress} detailed /> : <span className="detail-caption">No recovery queued</span>}
          </div>
          {progress && <p className="recovery-detail">{progress.detail}</p>}
          <dl className="recovery-facts">
            <div><dt>Verified IP changes</dt><dd>{progress?.ipChanges ?? 0} <span>· auto limit {progress?.maxIpChanges ?? 2}</span></dd></div>
            {!!progress?.updatedAt && <div><dt>Stage started</dt><dd><time dateTime={new Date(progress.updatedAt).toISOString()}>{new Date(progress.updatedAt).toLocaleTimeString()}</time></dd></div>}
            {progress?.lastSentAt && <div><dt>Last prompt sent</dt><dd><time dateTime={new Date(progress.lastSentAt).toISOString()}>{new Date(progress.lastSentAt).toLocaleTimeString()}</time></dd></div>}
          </dl>
          <p className="detail-caption">Orca waits at least 30 seconds after a confirmed prompt. Busy work and VPN checks can take longer. Outside sessions have no fixed retry delay.</p>
        </section>
        <section className="inspector-section">
          <h3>Automatic recovery</h3>
          <label className="monitor-setting">
            <span>Monitor this session</span>
            <input
              type="checkbox"
              checked={monitored}
              disabled={pending !== null}
              onChange={(event) =>
                run("set_session_monitoring", {
                  targetId: target.id,
                  enabled: event.target.checked,
                  wholeTerminal: false,
                })
              }
            />
          </label>
          {target.terminalId && (
            <label className="monitor-setting">
              <span>
                Monitor this terminal
                <small>Applies to all tabs, including new ones.</small>
              </span>
              <input
                type="checkbox"
                checked={terminalIncluded && !terminalMixed}
                ref={(input) => {
                  if (input) input.indeterminate = terminalMixed;
                }}
                disabled={pending !== null}
                onChange={(event) =>
                  run("set_session_monitoring", {
                    targetId: target.id,
                    enabled: event.target.checked,
                    wholeTerminal: true,
                  })
                }
              />
            </label>
          )}
          <p className="monitor-help">
            {snapshot.monitoring
              ? "Monitoring is on for checked sessions."
              : "Monitoring is paused. Your selection is saved."}{" "}
            Recovery waits for all working sessions before changing the VPN.
          </p>
        </section>
        <section className="inspector-section">
          <h3>Project</h3>
          <div className="project-location">
            <Folder size={15} />
            <strong>{projectName(target.project)}</strong>
          </div>
          <p className="path-text">
            {target.project || "Project path unavailable"}
          </p>
        </section>
        {target.lastError && (
          <section className="inspector-section">
            <h3>Latest error</h3>
            <p className="error-detail">
              <CircleAlert size={15} />
              <span>{target.lastError}</span>
            </p>
          </section>
        )}
        <section className="inspector-section">
          <div className="section-label">
            <h3>Continuation prompt</h3>
            <button
              className="text-button"
              onClick={(event) => onEdit(target, event.detail === 0)}
            >
              Edit
            </button>
          </div>
          <p className="prompt-preview">
            {target.promptOverride || snapshot.globalPrompt}
          </p>
          <span className="detail-caption">
            {target.promptOverride
              ? "Custom for this session"
              : "Using the default prompt"}
          </span>
        </section>
      </div>
      <div className="inspector-bottom">
        <button
          className="primary-button full-width"
          disabled={
            !canRecover ||
            !monitored ||
            (progress && activeRecovery.has(progress.stage)) ||
            pending !== null ||
            (target.viaPlugin && !target.clientId) ||
            (target.source === "orca" && !target.clientId)
          }
          onClick={() => run("recover_now", { targetId: target.id })}
        >
          {pending === "recover_now" || (progress && animatedRecovery.has(progress.stage)) ? (
            <LoaderCircle className="spin" size={16} />
          ) : (
            <RotateCcw size={16} />
          )}
          {progress && activeRecovery.has(progress.stage) ? "Recovery in progress" : "Recover session"}
        </button>
        <p>
          {monitored
            ? "A new route is verified before continuing. Recovery waits for other responding sessions."
            : "Enable monitoring for this session to recover it."}
        </p>
      </div>
    </aside>
  );
}

function RelayInspector({
  server,
  snapshot,
  pending,
  available,
  run,
}: {
  server: VpnServer | null;
  snapshot: Snapshot;
  pending: string | null;
  available: boolean;
  run: RunCommand;
}) {
  if (!server)
    return (
      <aside className="inspector empty-inspector">
        <EmptyState
          icon={<Globe2 size={27} strokeWidth={1.3} />}
          title="Relay details"
        >
          Refresh the list to find available VPN Gate servers.
        </EmptyState>
      </aside>
    );
  const preferred = snapshot.preferredServer === server.id;
  const connected = snapshot.connectedServer?.id === server.id;
  return (
    <aside className="inspector" aria-label="Selected relay details">
      <div className="inspector-toolbar">
        <span>Relay details</span>
        {connected && (
          <span className="connected-note">
            <i />
            Connected
          </span>
        )}
      </div>
      <div className="inspector-scroll">
        <span className="relay-country-large">
          {countryCode(server.country)}
        </span>
        <h2 className="inspector-title">{countryName(server.country)}</h2>
        <p className="relay-host">{server.host}</p>
        <div className="relay-measures">
          <div>
            <strong>
              {server.pingMs}
              <small> ms</small>
            </strong>
            <span>Ping</span>
          </div>
          <div>
            <strong>
              {lineSpeed(server.speedMbps).split(" ")[0]}
              <small> {lineSpeed(server.speedMbps).split(" ")[1]}</small>
            </strong>
            <span>Line speed</span>
          </div>
        </div>
        <dl className="detail-list">
          <div>
            <dt>IP address</dt>
            <dd className="mono">{server.ip}</dd>
          </div>
          <div>
            <dt>TCP port</dt>
            <dd>{server.port}</dd>
          </div>
          <div>
            <dt>VPN sessions</dt>
            <dd>{server.sessions}</dd>
          </div>
        </dl>
        <section className="inspector-section">
          <h3>Next recovery</h3>
          <p className="inspector-description">
            {preferred
              ? "This relay will be tried first."
              : "Recovery picks a relay by ping and line speed. You can prioritize this one."}{" "}
            Failed connections are skipped automatically.
          </p>
        </section>
      </div>
      <div className="inspector-bottom">
        <button
          className={`${preferred ? "secondary-button" : "primary-button"} full-width`}
          disabled={!available || pending !== null}
          onClick={() =>
            run("set_preferred_server", {
              serverId: preferred ? null : server.id,
            })
          }
        >
          {preferred ? <Check size={16} /> : <ArrowUpRight size={16} />}
          {preferred ? "Use automatic selection" : "Prioritize this relay"}
        </button>
        <p>Advertised ping and speed are estimates from VPN Gate.</p>
      </div>
    </aside>
  );
}

function PromptDialog({
  editor,
  globalPrompt,
  onClose,
  run,
}: {
  editor: PromptEditor;
  globalPrompt: string;
  onClose: () => void;
  run: RunCommand;
}) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const closing = useRef(false);
  const [value, setValue] = useState(
    editor.target ? (editor.target.promptOverride ?? "") : globalPrompt,
  );
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const dialog = dialogRef.current!;
    const origin = document.activeElement as HTMLElement | null;
    dialog.showModal();
    dialog.querySelector("textarea")?.focus();
    if (
      !editor.instant &&
      !matchMedia("(prefers-reduced-motion: reduce)").matches
    ) {
      dialog.animate(
        [
          { opacity: 0, transform: "translateY(8px) scale(.98)" },
          { opacity: 1, transform: "translateY(0) scale(1)" },
        ],
        { duration: 190, easing: "cubic-bezier(.23,1,.32,1)" },
      );
    }
    return () => {
      dialog.getAnimations().forEach((animation) => animation.cancel());
      dialog.close();
      origin?.focus();
    };
  }, [editor.instant]);

  function dismiss(instant = false) {
    if (closing.current || saving) return;
    closing.current = true;
    const dialog = dialogRef.current;
    if (
      instant ||
      editor.instant ||
      matchMedia("(prefers-reduced-motion: reduce)").matches ||
      !dialog
    ) {
      onClose();
      return;
    }
    dialog
      .animate(
        [
          { opacity: 1, transform: "translateY(0) scale(1)" },
          { opacity: 0, transform: "translateY(6px) scale(.985)" },
        ],
        {
          duration: 120,
          easing: "cubic-bezier(.23,1,.32,1)",
          fill: "forwards",
        },
      )
      .finished.then(onClose)
      .catch(() => {});
  }

  async function save(useDefault = false) {
    setSaving(true);
    const message = editor.target
      ? await run("set_prompt_override", {
          targetId: editor.target.id,
          prompt: useDefault ? null : value.trim() || null,
        })
      : await run("set_global_prompt", { prompt: value.trim() });
    setSaving(false);
    if (message) setError(message);
    else onClose();
  }

  return (
    <dialog
      className="prompt-dialog"
      ref={dialogRef}
      aria-labelledby="prompt-title"
      onCancel={(event) => {
        event.preventDefault();
        dismiss(true);
      }}
      onKeyDown={(event) => {
        if (event.key !== "Tab") return;
        const controls = [
          ...event.currentTarget.querySelectorAll<HTMLElement>(
            "button:not(:disabled), textarea:not(:disabled)",
          ),
        ];
        const first = controls[0];
        const last = controls.at(-1);
        if (!first || !last) {
          event.preventDefault();
        } else if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }}
      onClick={(event) => {
        if (event.target !== event.currentTarget) return;
        const bounds = event.currentTarget.getBoundingClientRect();
        if (
          event.clientX < bounds.left ||
          event.clientX > bounds.right ||
          event.clientY < bounds.top ||
          event.clientY > bounds.bottom
        )
          dismiss();
      }}
    >
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (!saving) void save();
        }}
      >
        <div className="dialog-header">
          <span className="dialog-symbol">
            <Settings2 size={21} />
          </span>
          <button
            type="button"
            className="icon-button"
            aria-label="Close prompt editor"
            disabled={saving}
            onClick={(event) => dismiss(event.detail === 0)}
          >
            <X size={17} />
          </button>
        </div>
        <h2 id="prompt-title">
          {editor.target ? "Session prompt" : "Continuation prompt"}
        </h2>
        <p>
          {editor.target
            ? `Customize the message for “${sessionTitle(editor.target.title)}”.`
            : "Sent once to each affected session after the new connection is verified."}
        </p>
        <label htmlFor="continuation-prompt">
          {editor.target ? "Custom prompt" : "Default prompt"}
        </label>
        <textarea
          id="continuation-prompt"
          value={value}
          onChange={(event) => setValue(event.target.value)}
          placeholder={
            editor.target ? globalPrompt : "Enter a continuation prompt"
          }
          rows={6}
          disabled={saving}
          autoFocus
        />
        {editor.target && <small>Leave blank to use the default prompt.</small>}
        {error && (
          <p className="dialog-error" role="alert">
            {error}
          </p>
        )}
        <div className="dialog-actions">
          {editor.target && (
            <button
              type="button"
              className="text-button"
              disabled={saving}
              onClick={() => save(true)}
            >
              Use default
            </button>
          )}
          <span />
          <button
            type="button"
            className="secondary-button"
            disabled={saving}
            onClick={(event) => dismiss(event.detail === 0)}
          >
            Cancel
          </button>
          <button
            type="submit"
            className="primary-button"
            disabled={saving || (!editor.target && !value.trim())}
          >
            {saving && <LoaderCircle className="spin" size={14} />}Save prompt
          </button>
        </div>
      </form>
    </dialog>
  );
}

function App() {
  const desktop = isTauri();
  const preview =
    import.meta.env.DEV &&
    !desktop &&
    new URLSearchParams(location.search).get("preview") === "1";
  const [snapshot, setSnapshot] = useState<Snapshot>(EMPTY_SNAPSHOT);
  const [loaded, setLoaded] = useState(false);
  const [view, setView] = useState<View>("sessions");
  const [scope, setScope] = useState("all");
  const [filter, setFilter] = useState<SessionFilter>("all");
  const [search, setSearch] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedRelayId, setSelectedRelayId] = useState<string | null>(null);
  const [relaySort, setRelaySort] = useState("recommended");
  const [activityFilter, setActivityFilter] = useState("all");
  const [editor, setEditor] = useState<PromptEditor | null>(null);
  const [pending, setPending] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [dismissedError, setDismissedError] = useState<string | null>(null);
  const inFlight = useRef(false);
  const initialSelection = useRef(false);

  useEffect(() => {
    let disposed = false;
    if (preview) {
      import("./dev-preview").then((module) => {
        if (disposed) return;
        const data = module.createPreviewSnapshot();
        if (new URLSearchParams(location.search).get("empty") === "1") {
          data.orcaSessions = [];
          data.otherSessions = [];
          data.servers = [];
          data.history = [];
          data.recovery = {};
          data.connectedServer = null;
          data.publicIp = null;
          data.vpnStatus = "disconnected";
        }
        setSnapshot(data);
        setLoaded(true);
      });
      return () => {
        disposed = true;
      };
    }
    if (!desktop) return;
    let eventReceived = false;
    const unlisten = listen<Snapshot>("snapshot", (event) => {
      if (!disposed) {
        eventReceived = true;
        setSnapshot(event.payload);
        setLoaded(true);
      }
    }).catch((error) => {
      if (!disposed) setNotice(String(error));
      return () => {};
    });
    invoke<Snapshot>("get_snapshot")
      .then((data) => {
        if (!disposed && !eventReceived) {
          setSnapshot(data);
          setLoaded(true);
        }
      })
      .catch((error) => {
        if (!disposed) setNotice(String(error));
      });
    return () => {
      disposed = true;
      void unlisten.then((stop) => stop());
    };
  }, [desktop, preview]);

  const sessions = useMemo(
    () => [...snapshot.orcaSessions, ...snapshot.otherSessions],
    [snapshot.orcaSessions, snapshot.otherSessions],
  );
  const projects = useMemo(
    () => groupProjects(snapshot.orcaSessions),
    [snapshot.orcaSessions],
  );
  const otherProjects = useMemo(
    () => groupProjects(snapshot.otherSessions),
    [snapshot.otherSessions],
  );
  useEffect(() => {
    if (!initialSelection.current && sessions.length) {
      setSelectedId(
        (
          sessions.find((session) => session.state === "rate-limited") ??
          sessions[0]
        ).id,
      );
      initialSelection.current = true;
    } else if (
      selectedId &&
      !sessions.some((session) => session.id === selectedId)
    )
      setSelectedId(null);
  }, [sessions, selectedId]);

  const recovering =
    snapshot.phase.startsWith("connecting") ||
    snapshot.phase.startsWith("waiting") ||
    snapshot.phase === "resuming sessions";
  const scopedSessions = scope.startsWith("other-project:")
    ? snapshot.otherSessions.filter(
        (session) =>
          projectKey(session.project) === projectKey(scope.slice(14)),
      )
    : scope.startsWith("project:")
      ? snapshot.orcaSessions.filter(
          (session) =>
            projectKey(session.project) === projectKey(scope.slice(8)),
        )
      : sessions;
  const limited = scopedSessions.filter(
    (session) => session.state === "rate-limited",
  ).length;
  const shownSessions = scopedSessions.filter(
    (session) =>
      (filter === "all" ||
        (filter === "limited"
          ? session.state === "rate-limited"
          : session.state === filter)) &&
      `${session.title} ${session.project}`
        .toLowerCase()
        .includes(search.toLowerCase()),
  );
  const selected =
    shownSessions.find((session) => session.id === selectedId) ?? null;
  const shownGroups = groupProjects(
    shownSessions.filter((session) => session.source === "orca"),
  );
  const shownOtherGroups = groupProjects(
    shownSessions.filter((session) => session.source === "other"),
  );
  const relays = useMemo(() => {
    const result = snapshot.servers.filter((server) =>
      `${countryName(server.country)} ${server.host} ${server.ip}`
        .toLowerCase()
        .includes(search.toLowerCase()),
    );
    if (relaySort === "ping") result.sort((a, b) => a.pingMs - b.pingMs);
    if (relaySort === "speed") result.sort((a, b) => b.speedMbps - a.speedMbps);
    return result;
  }, [snapshot.servers, search, relaySort]);
  const relay =
    relays.find((server) => server.id === selectedRelayId) ??
    relays.find((server) => server.id === snapshot.preferredServer) ??
    relays.find((server) => server.id === snapshot.connectedServer?.id) ??
    relays[0] ??
    null;
  const events = snapshot.history.filter(
    (event) =>
      (activityFilter === "all" || event.level === activityFilter) &&
      event.message.toLowerCase().includes(search.toLowerCase()),
  );
  const error =
    notice ??
    (snapshot.lastError !== dismissedError ? snapshot.lastError : null);
  const controlsAvailable = desktop || preview;

  async function run(
    command: string,
    args?: Record<string, unknown>,
  ): Promise<string | null> {
    if (inFlight.current) return "Another action is in progress.";
    if (preview) {
      setSnapshot((current) => {
        const pausePreview = (data: Snapshot, excluded: (id: string) => boolean): Snapshot => ({
          ...data,
          recovery: Object.fromEntries(Object.entries(data.recovery).map(([id, progress]) => [id,
            excluded(id) && progress.stage !== "sending" && (activeRecovery.has(progress.stage) || progress.stage === "cooldown")
              ? { ...progress, stage: "paused", detail: "Queued recovery canceled in this preview", retryAt: null, updatedAt: Date.now() }
              : progress,
          ])),
        });
        if (command === "set_monitoring") {
          const next = { ...current, monitoring: Boolean(args?.enabled) };
          return next.monitoring ? next : pausePreview(next, () => true);
        }
        if (command === "set_session_monitoring") {
          const target = [
            ...current.orcaSessions,
            ...current.otherSessions,
          ].find((item) => item.id === args?.targetId);
          if (!target) return current;
          const next = changeMonitoring(
                current,
                target,
                Boolean(args?.enabled),
                Boolean(args?.wholeTerminal),
              );
          return pausePreview(next, (id) => {
            const item = [...next.orcaSessions, ...next.otherSessions].find((session) => session.id === id);
            return !!item && !isMonitored(next, item);
          });
        }
        if (command === "set_preferred_server")
          return {
            ...current,
            preferredServer: args?.serverId as string | null,
          };
        if (command === "set_global_prompt")
          return { ...current, globalPrompt: args?.prompt as string };
        if (command === "set_prompt_override") {
          const update = (target: SessionTarget) =>
            target.id === args?.targetId
              ? { ...target, promptOverride: args?.prompt as string | null }
              : target;
          return {
            ...current,
            orcaSessions: current.orcaSessions.map(update),
            otherSessions: current.otherSessions.map(update),
          };
        }
        if (command === "select_session_tab") {
          const selected = [
            ...current.orcaSessions,
            ...current.otherSessions,
          ].find((target) => target.id === args?.targetId);
          const update = (target: SessionTarget) =>
            selected && target.terminalId === selected.terminalId
              ? { ...target, selectedTab: target.id === selected.id }
              : target;
          return {
            ...current,
            orcaSessions: current.orcaSessions.map(update),
            otherSessions: current.otherSessions.map(update),
          };
        }
        return current;
      });
      return null;
    }
    if (!desktop) return "Open the desktop app to use this control.";
    inFlight.current = true;
    setPending(command);
    setNotice(null);
    setDismissedError(null);
    try {
      setSnapshot(await invoke<Snapshot>(command, args));
      return null;
    } catch (failure) {
      const message = String(failure);
      setNotice(message);
      return message;
    } finally {
      inFlight.current = false;
      setPending(null);
    }
  }

  function navigate(next: View, nextScope = "all") {
    setView(next);
    setScope(nextScope);
    setSearch("");
    setFilter("all");
    if (nextScope !== "all") setSelectedId(null);
  }

  function selectSession(id: string) {
    setSelectedId(id);
    if (matchMedia("(max-width: 760px)").matches) {
      requestAnimationFrame(() =>
        document
          .querySelector(".inspector")
          ?.scrollIntoView({ block: "start", behavior: "auto" }),
      );
    }
  }

  function moveSession(event: React.KeyboardEvent<HTMLDivElement>) {
    if (!["ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
    const buttons = [
      ...event.currentTarget.querySelectorAll<HTMLButtonElement>(
        ".session-item",
      ),
    ];
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (index < 0) return;
    event.preventDefault();
    const nextIndex =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? buttons.length - 1
          : Math.max(
              0,
              Math.min(
                buttons.length - 1,
                index + (event.key === "ArrowDown" ? 1 : -1),
              ),
            );
    buttons[nextIndex].focus();
    const id = buttons[nextIndex].dataset.sessionId;
    if (id) selectSession(id);
  }

  function renderGroup(
    title: string,
    subtitle: string,
    targets: SessionTarget[],
    other = false,
  ) {
    return (
      <section
        className="session-group"
        key={`${other ? "other" : "orca"}:${subtitle}`}
        aria-label={`${title} · ${other ? "Other OpenCode sessions" : "Orca terminals"}`}
      >
        <div className="group-heading">
          <span className="group-title">
            <Folder size={14} />
            <strong>{title}</strong>
            <em className="group-source">{other ? "OpenCode" : "Orca"}</em>
            <span>
              {targets.length} {targets.length === 1 ? "session" : "sessions"}
            </span>
          </span>
          <small title={subtitle}>
            {subtitle || "Project directory unavailable"}
          </small>
        </div>
        {targets.map((target) => (
          <SessionItem
            key={target.id}
            target={target}
            progress={snapshot.recovery?.[target.id]}
            selected={target.id === selectedId}
            monitored={isMonitored(snapshot, target)}
            monitoringDisabled={!controlsAvailable || pending !== null}
            onMonitoringChange={(enabled) =>
              run("set_session_monitoring", {
                targetId: target.id,
                enabled,
                wholeTerminal: false,
              })
            }
            onSelect={() => selectSession(target.id)}
          />
        ))}
      </section>
    );
  }

  const viewTitle =
    view === "relays"
      ? "VPN relays"
      : view === "activity"
        ? "Activity"
        : scope.startsWith("other-project:")
          ? projectName(scope.slice(14))
          : scope.startsWith("project:")
            ? projectName(scope.slice(8))
            : "Sessions";

  return (
    <div className="app-shell">
      <aside className="sidebar" aria-label="Main navigation">
        <div className="app-identity">
          <span className="app-mark">
            <img src={appIcon} alt="" width={36} height={36} />
          </span>
          <span>
            <strong>Recovery</strong>
            <small>for OpenCode</small>
          </span>
        </div>
        <nav className="main-nav">
          <button
            className={view === "sessions" && scope === "all" ? "active" : ""}
            onClick={() => navigate("sessions")}
            title="Sessions"
            aria-current={
              view === "sessions" && scope === "all" ? "page" : undefined
            }
          >
            <PanelLeft size={17} />
            <span>Sessions</span>
            <small>{sessions.length}</small>
          </button>
          <button
            className={view === "relays" ? "active" : ""}
            onClick={() => navigate("relays")}
            title="VPN relays"
            aria-current={view === "relays" ? "page" : undefined}
          >
            <Globe2 size={17} />
            <span>VPN relays</span>
            <small>{snapshot.servers.length}</small>
          </button>
          <button
            className={view === "activity" ? "active" : ""}
            onClick={() => navigate("activity")}
            title="Activity"
            aria-current={view === "activity" ? "page" : undefined}
          >
            <Activity size={17} />
            <span>Activity</span>
            {snapshot.history.some((event) => event.level === "error") && (
              <i className="nav-indicator" />
            )}
          </button>
        </nav>
        <div className="project-nav-scroll">
          <div className="nav-section-label">
            <span>Orca sessions</span>
            <Command size={12} />
          </div>
          <nav className="project-nav">
            {projects.length ? (
              projects.map(({ project, targets }) => (
                <button
                  key={project}
                  className={
                    view === "sessions" && scope === `project:${project}`
                      ? "active"
                      : ""
                  }
                  title={project}
                  aria-current={
                    view === "sessions" && scope === `project:${project}`
                      ? "page"
                      : undefined
                  }
                  onClick={() => navigate("sessions", `project:${project}`)}
                >
                  <Folder size={15} />
                  <span>{projectName(project)}</span>
                  {targets.some((target) => target.state === "rate-limited") ? (
                    <i className="attention-dot" />
                  ) : (
                    <small>{targets.length}</small>
                  )}
                </button>
              ))
            ) : (
              <p className="nav-empty">Active projects appear here.</p>
            )}
          </nav>
          <div className="nav-section-label second">
            <span>Other sessions</span>
            <Terminal size={12} />
          </div>
          <nav className="project-nav">
            {otherProjects.map(({ project, targets }) => (
              <button
                key={project}
                title={`${project}\nOutside Orca`}
                className={
                  view === "sessions" && scope === `other-project:${project}`
                    ? "active"
                    : ""
                }
                aria-current={
                  view === "sessions" && scope === `other-project:${project}`
                    ? "page"
                    : undefined
                }
                onClick={() => navigate("sessions", `other-project:${project}`)}
              >
                <Folder size={15} />
                <span>{projectName(project)}</span>
                {targets.some((target) => target.state === "rate-limited") ? (
                  <i className="attention-dot" />
                ) : (
                  <small>{targets.length}</small>
                )}
              </button>
            ))}
          </nav>
        </div>
        <div className="sidebar-bottom">
          <button
            className="prompt-settings"
            title="Continuation prompt"
            disabled={!controlsAvailable}
            onClick={(event) =>
              setEditor({ target: null, instant: event.detail === 0 })
            }
          >
            <Settings2 size={16} />
            <span>Continuation prompt</span>
          </button>
          <div className="sidebar-caption">
            <ShieldCheck size={13} />
            <span>SoftEther · VPN Gate</span>
          </div>
        </div>
      </aside>

      <main className="workspace">
        <header className="toolbar">
          <div className="toolbar-title">
            <h1>{viewTitle}</h1>
            {preview && <span className="preview-label">Preview data</span>}
          </div>
          <div className="toolbar-actions">
            <label className="search-field">
              <Search size={15} />
              <input
                aria-label={`Search ${view}`}
                placeholder={`Search ${view}…`}
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
              {search && (
                <button aria-label="Clear search" onClick={() => setSearch("")}>
                  <X size={13} />
                </button>
              )}
            </label>
            <button
              className={`monitor-control ${snapshot.monitoring ? "enabled" : ""}`}
              role="switch"
              aria-label="Monitoring"
              aria-checked={snapshot.monitoring}
              disabled={!controlsAvailable || pending !== null}
              onClick={() =>
                run("set_monitoring", { enabled: !snapshot.monitoring })
              }
            >
              <span>Monitoring</span>
              <span className="switch-track">
                <span />
              </span>
            </button>
          </div>
        </header>

        <div className="route-bar">
          <span
            className={`route-symbol ${snapshot.vpnStatus === "connected" ? "connected" : ""}`}
          >
            <Globe2 size={20} strokeWidth={1.5} />
          </span>
          <div className="route-copy">
            <strong>
              <i className={`route-dot ${snapshot.vpnStatus}`} />
              {snapshot.vpnStatus === "connected"
                ? `Connected${snapshot.connectedServer ? ` · ${countryName(snapshot.connectedServer.country)}` : ""}`
                : snapshot.vpnStatus === "checking"
                  ? "Checking VPN connection"
                  : "VPN disconnected"}
            </strong>
            <span>
              {snapshot.connectedServer?.host ||
                "Recovery chooses a relay when a session needs it."}
            </span>
          </div>
          <div className="route-address">
            <span>Public IP</span>
            <strong className="mono">{snapshot.publicIp || "—"}</strong>
          </div>
          {snapshot.connectedServer && (
            <button
              className="quiet-button disconnect-button"
              disabled={!desktop || recovering || pending !== null}
              onClick={() => run("disconnect_vpn")}
            >
              Disconnect
            </button>
          )}
          <button className="route-link" onClick={() => navigate("relays")}>
            View relays
            <ChevronRight size={15} />
          </button>
        </div>
        {!desktop && !preview && (
          <div className="connection-notice">
            <CircleAlert size={15} />
            <span>Open the desktop app for live VPN and session data.</span>
          </div>
        )}
        {error && (
          <div className="error-banner" role="alert">
            <CircleAlert size={16} />
            <span>{error}</span>
            <button
              className="icon-button"
              aria-label="Dismiss error"
              onClick={() => {
                setNotice(null);
                setDismissedError(snapshot.lastError);
              }}
            >
              <X size={15} />
            </button>
          </div>
        )}

        {view === "sessions" && (
          <>
            <div className="view-controls">
              <div className="segmented-control" aria-label="Filter sessions">
                {(
                  [
                    ["all", "All sessions"],
                    ["limited", "Needs recovery"],
                    ["busy", "Working"],
                    ["idle", "Idle"],
                  ] as const
                ).map(([value, label]) => (
                  <button
                    key={value}
                    aria-pressed={filter === value}
                    className={filter === value ? "active" : ""}
                    onClick={() => setFilter(value)}
                  >
                    {label}
                    {value === "limited" && limited > 0 && (
                      <span>{limited}</span>
                    )}
                  </button>
                ))}
              </div>
              <span className="view-count">
                {
                  scopedSessions.filter((item) => isMonitored(snapshot, item))
                    .length
                }{" "}
                of {scopedSessions.length} monitored
              </span>
            </div>
            <div className="workspace-body">
              <div className="session-list-pane">
                <div className="list-column-labels">
                  <span>Session / project</span>
                  <span>Status</span>
                  <span>Updated</span>
                </div>
                <div className="session-groups" onKeyDown={moveSession}>
                  {shownSessions.length ? (
                    <>
                      {shownGroups.map((group) =>
                        renderGroup(
                          projectName(group.project),
                          group.project,
                          group.targets,
                        ),
                      )}
                      {shownOtherGroups.map((group) =>
                        renderGroup(
                          projectName(group.project),
                          group.project,
                          group.targets,
                          true,
                        ),
                      )}
                    </>
                  ) : (
                    <EmptyState
                      icon={<Terminal size={29} strokeWidth={1.3} />}
                      title={
                        search || filter !== "all"
                          ? "No matching sessions"
                          : "No active sessions"
                      }
                    >
                      {search || filter !== "all"
                        ? "Try another search or session filter."
                        : scope.startsWith("other-project:")
                          ? "Open a session with opencode or opencode --auto. Live shared-service sessions appear under their project."
                          : "Existing OpenCode work in Orca and the shared service will appear here."}
                    </EmptyState>
                  )}
                </div>
                {snapshot.otherSessionsNotice &&
                  (scope === "all" || scope.startsWith("other-project:")) && (
                    <div className="session-discovery-notice" role="status">
                      <CircleAlert size={14} />
                      <span>{snapshot.otherSessionsNotice}</span>
                    </div>
                  )}
                <div className="list-footnote">
                  <ShieldCheck size={13} />
                  <span>
                    Only live work and observed rate-limit incidents are listed.
                  </span>
                </div>
              </div>
              <SessionInspector
                target={selected}
                snapshot={snapshot}
                canRecover={desktop && !recovering}
                pending={pending}
                onClose={() => setSelectedId(null)}
                onEdit={(target, instant) => setEditor({ target, instant })}
                run={run}
              />
            </div>
          </>
        )}

        {view === "relays" && (
          <>
            <div className="view-controls">
              <div className="relay-sort">
                <label htmlFor="relay-sort">Sort by</label>
                <select
                  id="relay-sort"
                  value={relaySort}
                  onChange={(event) => setRelaySort(event.target.value)}
                >
                  <option value="recommended">Recommended</option>
                  <option value="ping">Lowest ping</option>
                  <option value="speed">Highest line speed</option>
                </select>
              </div>
              <button
                className="secondary-button compact"
                disabled={!desktop || pending === "refresh_servers"}
                onClick={() => run("refresh_servers")}
              >
                {pending === "refresh_servers" ? (
                  <LoaderCircle className="spin" size={14} />
                ) : (
                  <RefreshCw size={14} />
                )}
                Refresh list
              </button>
            </div>
            <div className="workspace-body">
              <div className="relay-list-pane">
                <div className="relay-list-scroll">
                  <table className="relay-table">
                    <thead>
                      <tr>
                        <th>Location / server</th>
                        <th className="ip-column">IP address</th>
                        <th>
                          Ping{relaySort === "ping" && <ArrowDown size={10} />}
                        </th>
                        <th>
                          Line speed
                          {relaySort === "speed" && <ArrowDown size={10} />}
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      {relays.map((server) => (
                        <tr
                          key={server.id}
                          className={relay?.id === server.id ? "selected" : ""}
                          onClick={() => setSelectedRelayId(server.id)}
                        >
                          <td>
                            <button
                              aria-pressed={relay?.id === server.id}
                              className="relay-select"
                              onClick={() => setSelectedRelayId(server.id)}
                            >
                              <span className="country-code">
                                {countryCode(server.country)}
                              </span>
                              <span>
                                <strong>
                                  {countryName(server.country)}
                                  {snapshot.preferredServer === server.id && (
                                    <Check
                                      size={12}
                                      aria-label="Preferred relay"
                                    />
                                  )}
                                </strong>
                                <small>{server.host}</small>
                              </span>
                            </button>
                          </td>
                          <td className="ip-column mono">{server.ip}</td>
                          <td className={server.pingMs < 50 ? "low-ping" : ""}>
                            {server.pingMs}
                            <span className="unit"> ms</span>
                          </td>
                          <td>{lineSpeed(server.speedMbps)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  {!relays.length && (
                    <EmptyState
                      icon={<Globe2 size={29} strokeWidth={1.3} />}
                      title={
                        search ? "No matching relays" : "No relay list yet"
                      }
                    >
                      {search
                        ? "Search by location, hostname, or IP address."
                        : "Refresh the list to load available VPN Gate servers."}
                    </EmptyState>
                  )}
                </div>
                <div className="relay-list-footer">
                  <span>
                    {relays.length} relays <i />
                    Updated {timeAgo(snapshot.refreshedAt)}
                  </span>
                  <span>
                    {snapshot.preferredServer
                      ? "Preferred relay set"
                      : "Automatic selection"}
                  </span>
                </div>
              </div>
              <RelayInspector
                server={relay}
                snapshot={snapshot}
                pending={pending}
                available={controlsAvailable}
                run={run}
              />
            </div>
          </>
        )}

        {view === "activity" && (
          <>
            <div className="view-controls">
              <div className="segmented-control" aria-label="Filter activity">
                {[
                  ["all", "All events"],
                  ["warning", "Warnings"],
                  ["error", "Errors"],
                ].map(([value, label]) => (
                  <button
                    key={value}
                    className={activityFilter === value ? "active" : ""}
                    aria-pressed={activityFilter === value}
                    onClick={() => setActivityFilter(value)}
                  >
                    {label}
                  </button>
                ))}
              </div>
              <span className="view-count">{events.length} events</span>
            </div>
            <div className="activity-view">
              <div className="activity-heading">
                <Clock3 size={17} />
                <div>
                  <h2>Connection & recovery log</h2>
                  <p>
                    VPN changes, continuation receipts, and connection errors.
                  </p>
                </div>
              </div>
              {events.length ? (
                <div className="event-list">
                  {events.map((event, index) => (
                    <div
                      className={`event-row ${event.level}`}
                      key={`${event.at}-${index}`}
                    >
                      <span className="event-icon">
                        {event.level === "success" ? (
                          <Check size={14} />
                        ) : event.level === "error" ||
                          event.level === "warning" ? (
                          <CircleAlert size={14} />
                        ) : (
                          <Activity size={14} />
                        )}
                      </span>
                      <div>
                        <span className="event-level">{event.level}</span>
                        <p>{event.message}</p>
                      </div>
                      <time title={new Date(event.at).toLocaleString()}>
                        {new Date(event.at).toLocaleTimeString([], {
                          hour: "2-digit",
                          minute: "2-digit",
                          second: "2-digit",
                        })}
                      </time>
                    </div>
                  ))}
                </div>
              ) : (
                <EmptyState
                  icon={<Clock3 size={29} strokeWidth={1.3} />}
                  title={
                    search || activityFilter !== "all"
                      ? "No matching events"
                      : "No activity yet"
                  }
                >
                  {search || activityFilter !== "all"
                    ? "Try another search or event filter."
                    : "Connections, continuation receipts, and errors will appear here."}
                </EmptyState>
              )}
            </div>
          </>
        )}

        <footer className="statusbar">
          <span>
            <i className={loaded ? "online" : ""} />
            {preview
              ? "Preview data · no live connections"
              : desktop
                ? loaded
                  ? "Local controller connected"
                  : "Connecting to controller…"
                : "Desktop app required"}
          </span>
          <span className={recovering ? "recovery-status" : ""}>
            {recovering ? (
              <LoaderCircle className="spin" size={12} />
            ) : snapshot.monitoring ? (
              <Radio size={12} />
            ) : null}
            {recovering
              ? snapshot.phase
              : snapshot.monitoring
                ? "Watching for rate limits"
                : "Monitoring paused"}
          </span>
        </footer>
      </main>
      {editor && (
        <PromptDialog
          editor={editor}
          globalPrompt={snapshot.globalPrompt}
          onClose={() => setEditor(null)}
          run={run}
        />
      )}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
