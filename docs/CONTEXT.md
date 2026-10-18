# Project context

## Current state

OpenCode Recovery is a Windows x64 Tauri dashboard for live OpenCode sessions. Version **0.1.0** is a preview candidate. Read the [user guide](USAGE.md) for behavior and the [roadmap](ROADMAP.md) for verified milestones. Source and tests define implementation; this file maps them.

The dashboard lists every live OpenCode tab directly beneath a project folder, separated into Orca and other sources. Idle/Working/Needs recovery/unavailable describe activity; selected/background describe terminal selection. Historical session lists and saved global tabs are not discovery sources. Previously observed failures remain visible after closure with no delivery route.

Saved monitoring rules include individual sessions and whole terminals. Session overrides win; new tabs inherit a terminal rule. A cached ownership binding preserves exclusions during API fallback. Unchecking prunes queued work. Pausing clears queues, and an epoch guard prevents raced busy receipts from restoring canceled work. Every busy session still protects machine-wide routing, regardless of monitoring selection.

Failed rate-limit/quota events and the specific OpenCode free-tier client-gate provider message can queue recovery. Recovery bounds relay attempts to five, waits up to 20 seconds per connection, requires a connected SoftEther account and changed public IPv4, and limits a session to two verified IP changes. Exact-session plugin requests, claimed receipts, and persisted incident guards avoid uncertain automatic resubmissions. Busy targets can wait on the already verified route.

The UI uses charcoal surfaces, neutral text/selections, the real green icon, and restrained lime for enabled actions, checks, and focus. Preview fixtures remain development-only. Background CLI helpers use Windows `CREATE_NO_WINDOW`.

Per-session recovery progress is exposed as `Snapshot.recovery`, separate from OpenCode activity/selected-tab state. The controller reports queue/wait/connect/verify/send/receipt/stop stages, details, verified IP-change count, and timing. The 30-second Orca gate starts at a confirmed send receipt and is a minimum, not a fixed retry interval; outside sessions have no such gate. The UI uses a spinner only for active connection/delivery work, a static clock for waits, and a retry-eligibility countdown. Exclusions/pause clear queued indicators; ambiguous receipts never appear as success. Persisted attempt counts remain visible after restart; transient stage details are not persisted.

## Source map

| Concern | Canonical source |
| --- | --- |
| Setup, behavior, limitations, local data, removal | `README.md`, `docs/USAGE.md` |
| Verified milestones and remaining live checks | `docs/ROADMAP.md` |
| Development and publication checklist | `docs/DEVELOPMENT.md`, `docs/RELEASING.md`, `CHANGELOG.md` |
| Workspace, inspectors, activity, prompt editor | `src/main.tsx`, `src/styles.css` |
| Frontend contracts and monitoring presentation | `src/types.ts`, `src/monitoring.ts` |
| Icon and isolated development fixtures | `src/assets/app-icon.png`, `src/dev-preview.ts`, `src/vite-env.d.ts` |
| Desktop dev port handoff | `scripts/tauri.mjs`, `vite.config.ts` |
| Polling, recovery queues, persistence, commands | `src-tauri/src/lib.rs` |
| Settings and monitoring rule precedence | `src-tauri/src/models.rs` |
| Shared-service fallback | `src-tauri/src/opencode.rs` |
| Orca terminal identity and outer-terminal selection | `src-tauri/src/orca.rs` |
| Live leases, plugin install, request/receipt bridge | `src-tauri/src/presence.rs`, `integrations/opencode/tui.js` |
| Tab discovery and exact-delivery tests | `integrations/opencode/presence.test.mjs`; Rust tests beside source |
| Relay parsing/ranking and SoftEther profiles | `src-tauri/src/vpngate.rs`, `src-tauri/src/softether.rs` |
| Hidden helper processes and timeout handling | `src-tauri/src/process.rs` |
| Version/fixture gate and release packaging | `scripts/check-release.mjs`, `scripts/release.mjs` |
| Windows verification and artifact upload | `.github/workflows/windows.yml` |
| Toolchain and dependency pins | `.node-version`, `rust-toolchain.toml`, `package-lock.json`, `src-tauri/Cargo.lock` |
| Repository rules and frontend guidance | `AGENTS.md`, `.agents/skills/apple-design`, `.agents/skills/emil-design-eng`, `.agents/skills/impeccable` |

## Integration constraints

- The live plugin reports all open tabs, selected route, service identity, and per-session execution/error state. The shared `/api/session/active` endpoint supplies a responding-session fallback, not idle presence. Unknown status must not become Idle.
- V2 reports support shared and private connected clients through their existing SDK. Legacy V1 leases require the current shared PID. Do not discover arbitrary private endpoints or copy credentials.
- Install only `opc-rotate-presence/tui.js` under OpenCode's plugin directory, respecting `OPENCODE_CONFIG_DIR`. Heartbeats refresh every two seconds and expire after eight seconds; presence contains no prompts or credentials.
- Match Orca's `ORCA_PANE_KEY` to `tabId:leafId`, not the PTY ID. Deduplicate by exact session ID, preferring Orca ownership; never suppress an entire directory's outside sessions.
- Address requests to a live exact session/client/service identity. Claim before submitting, persist the incident guard before transport, clean expired requests, and never retry an ambiguous receipt automatically. Do not type prompts into the currently selected terminal tab.
- The exact built-in prompt is “Continue from where you stopped after the rate limit”. Migration replaces only the former built-in wording, preserving custom prompts.
- SoftEther is separately installed. Control only `OPCRotate_` profiles and refuse an unrelated active account. Do not enable monitoring or perform a live VPN test just to inspect the UI.
- Local settings, state, credentials, and generated release files stay outside Git. Source is MIT licensed; preserve third-party notices.

## Verification baseline and gaps

The recovery-progress update passed **9 Node and 34 Rust tests**, the production frontend build, NSIS packaging, and the release fixture-exclusion check. New tests cover the Orca send-based minimum delay, outside-source timing, per-session progress/counts, exclusion cleanup, saved limits, and stable stage timestamps. Browser UI verification was blocked by the browser tool's URL policy; no visual or live VPN check is claimed for these indicators.

The pre-release baseline passed **9 Node plugin tests and 31 Rust tests**, TypeScript/Vite build, and NSIS packaging. Windows helper coverage verifies no console handle while output is captured. Browser preview checks covered the requested project hierarchy, monitoring controls, keyboard behavior, empty states, desktop sizes, and inspected text contrast. Native checks confirmed live idle/background tabs and the free-tier provider error without triggering recovery.

Earlier live VPN recovery used the former terminal-send transport. **Live continuation using the current exact-session bridge remains unverified**, including Orca, outside clients, and background tabs. Clean-PC install/upgrade/uninstall and IPv6 egress remain open. Signing and automatic updates are not configured. Use the release checklist rather than treating a successful installer build as end-to-end acceptance.

The Windows workflow checks locked dependencies, versions, formatting, tests, npm audit, production fixture exclusion, and NSIS output. It uploads an installer/checksum artifact only; publication is a separate maintainer action.
