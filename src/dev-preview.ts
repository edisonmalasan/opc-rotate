import { DEFAULT_PROMPT, type Snapshot, type VpnServer } from "./types";

// Loaded only by Vite's explicit development preview. No controller commands run.
export function createPreviewSnapshot(): Snapshot {
  const now = Date.now();
  const servers: VpnServer[] = [
    ["Japan", "203.0.113.42", 12, 842, 14],
    ["Japan", "203.0.113.43", 18, 1240, 28],
    ["Korea Republic of", "198.51.100.12", 24, 925, 19],
    ["Japan", "203.0.113.44", 21, 640, 8],
    ["Singapore", "198.51.100.22", 29, 780, 31],
    ["Korea Republic of", "198.51.100.13", 32, 1050, 16],
    ["Japan", "203.0.113.45", 26, 425, 22],
    ["United States", "192.0.2.12", 94, 1120, 46],
    ["Taiwan", "198.51.100.32", 39, 540, 11],
    ["Japan", "203.0.113.46", 41, 380, 9],
  ].map(([country, ip, pingMs, speedMbps, sessions], index) => ({
    id: `preview-relay-${index}`,
    country: String(country),
    ip: String(ip),
    host: `vpn${723480100 + index}.opengw.net`,
    port: 443,
    pingMs: Number(pingMs),
    speedMbps: Number(speedMbps),
    sessions: Number(sessions),
    score: 100 - index,
  }));
  return {
    monitoring: false,
    phase: "idle",
    vpnStatus: "connected",
    publicIp: "203.0.113.42",
    connectedServer: servers[0],
    servers,
    refreshedAt: now - 98000,
    orcaSessions: [
      {
        id: "preview-session-1",
        source: "orca",
        title: "Desktop parity and theme",
        project: "C:\\Projects\\opc-rotate",
        state: "rate-limited",
        lastError: "Rate limit exceeded. Please try again later.",
        errorKey: "preview-429",
        promptOverride: null,
        updatedAt: now - 42000,
      },
      {
        id: "preview-session-2",
        source: "orca",
        title: "Connection retry handling",
        project: "C:\\Projects\\opc-rotate",
        state: "idle",
        lastError: null,
        errorKey: null,
        promptOverride: null,
        updatedAt: now - 160000,
      },
      {
        id: "preview-session-3",
        source: "orca",
        title: "Navigation and accessibility",
        project: "C:\\Projects\\website",
        state: "busy",
        lastError: null,
        errorKey: null,
        promptOverride:
          "Continue the accessibility review. Check existing changes first.",
        updatedAt: now - 7000,
      },
    ],
    otherSessions: [
      {
        id: "preview-session-4",
        source: "other",
        title: "API integration tests",
        project: "C:\\Projects\\api-service",
        state: "busy",
        lastError: null,
        errorKey: null,
        promptOverride: null,
        updatedAt: now - 12000,
      },
    ],
    history: [
      {
        at: now - 42000,
        level: "warning",
        message: "Desktop parity and theme reported a rate limit.",
      },
      {
        at: now - 98000,
        level: "info",
        message: "Refreshed 10 eligible VPN Gate relays.",
      },
      {
        at: now - 260000,
        level: "success",
        message: "Continuation delivered to Connection retry handling.",
      },
      {
        at: now - 262000,
        level: "success",
        message: "Verified new public IP: 203.0.113.42.",
      },
      {
        at: now - 267000,
        level: "info",
        message: "Connected to vpn723480100.opengw.net.",
      },
      {
        at: now - 285000,
        level: "error",
        message:
          "Relay connection failed with SoftEther error code 2. Trying the next server.",
      },
    ],
    globalPrompt: DEFAULT_PROMPT,
    preferredServer: null,
    lastError: null,
  };
}
