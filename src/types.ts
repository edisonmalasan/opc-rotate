export type VpnServer = {
  id: string;
  host: string;
  ip: string;
  port: number;
  country: string;
  pingMs: number;
  speedMbps: number;
  sessions: number;
  score: number;
};

export type SessionTarget = {
  id: string;
  source: "orca" | "other";
  title: string;
  project: string;
  state: string;
  lastError: string | null;
  errorKey: string | null;
  promptOverride: string | null;
  updatedAt: number;
  terminalId?: string | null;
  terminalTitle?: string | null;
  selectedTab?: boolean;
  clientId?: string | null;
  sessionId?: string | null;
  orcaHandle?: string | null;
  viaPlugin?: boolean;
  /** Identifies the operator input a stopped session is waiting on. */
  stopKey?: string | null;
  /** Child sessions of this session that are still executing. */
  workingSubagents?: number;
};

export type HistoryEntry = { at: number; level: string; message: string };

export type RecoveryProgress = {
  stage: string;
  detail: string;
  updatedAt: number;
  retryAt: number | null;
  ipChanges: number;
  maxIpChanges: number;
  lastSentAt: number | null;
  /** Whether this recovery changes the machine's public IP. */
  changesIp?: boolean;
};

export type Snapshot = {
  recovery: Record<string, RecoveryProgress>;
  monitoring: boolean;
  sessionMonitoring: Record<string, boolean>;
  terminalMonitoring: Record<string, boolean>;
  sessionTerminals: Record<string, string>;
  phase: string;
  vpnStatus: string;
  publicIp: string | null;
  connectedServer: VpnServer | null;
  servers: VpnServer[];
  refreshedAt: number | null;
  orcaSessions: SessionTarget[];
  otherSessions: SessionTarget[];
  otherSessionsNotice: string | null;
  history: HistoryEntry[];
  globalPrompt: string;
  /** The watcher mode every session follows unless it overrides it. */
  watcher: string;
  sessionWatcher: Record<string, string>;
  /** The prompt sent to a session that simply finished its turn. */
  continuePrompt: string;
  /** Whether anything on screen still depends on the VPN route. */
  vpnNeeded: boolean;
  preferredServer: string | null;
  lastError: string | null;
};

export const DEFAULT_PROMPT =
  "Continue from where you stopped after the rate limit";

export const EMPTY_SNAPSHOT: Snapshot = {
  recovery: {},
  monitoring: false,
  sessionMonitoring: {},
  terminalMonitoring: {},
  sessionTerminals: {},
  phase: "idle",
  vpnStatus: "checking",
  publicIp: null,
  connectedServer: null,
  servers: [],
  refreshedAt: null,
  orcaSessions: [],
  otherSessions: [],
  otherSessionsNotice: null,
  history: [],
  globalPrompt: DEFAULT_PROMPT,
  watcher: "worker-idle",
  sessionWatcher: {},
  continuePrompt: "Continue where you left off.",
  vpnNeeded: false,
  preferredServer: null,
  lastError: null,
};
