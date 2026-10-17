# Project context

## At a glance

**OpenCode Recovery** is a Windows Tauri dashboard for IP-sensitive OpenCode rate limits. It reads public VPN Gate relays, connects with the installed SoftEther client, verifies a changed public IPv4 address, then continues affected existing sessions. The primary user has OpenCode work in Orca terminals across projects and in the shared OpenCode service.

**Current focus:** observe outside-session continuation during a real rate-limit event and finish unobstructed native visual checks. Outside discovery now includes open idle terminals through a small OpenCode CLI presence plugin, alongside the busy-session API and observed failures. The user's `opencode --auto` terminal in Thesis loaded the plugin automatically on OpenCode v2.0.32. Its exact research session appeared as Ready under Other projects → Thesis in the native accessibility tree. No terminal restart, VPN change, or continuation prompt was used for this check. `npm test` passed 2 Node plugin tests and 20 Rust tests. The frontend and NSIS installer were rebuilt successfully after this fix.

The prior UI pass checked populated browser fixtures at 1280 × 820, the native minimum of 980 × 660, and 390px width. Session filters, project navigation, arrow-key selection, prompt saves, modal focus wrapping and Escape, relay sorting and preferences, searches, and activity filters passed. This discovery pass added browser checks for two outside project groups, multiple sessions per project, and separate Orca/outside scopes for the same directory. Native screenshots remain occluded by another foreground app, so accessibility confirmation is not recorded as an unobstructed visual check.

The app uses a dark desktop split view: a sidebar for Sessions, VPN relays, Activity, Orca sessions, and Other sessions; a compact persistent route bar; project groups labeled by source; and a contextual inspector containing the error, prompt, and recovery action. Both session sections list projects directly; the separate Other sessions overview row was removed. The label change and outside project navigation passed the frontend build and a browser check at 1280 × 820. Windows path casing and separator differences share one group within each source. Relay preferences require an explicit inspector action. Activity has its own searchable view. Native modal prompt editing preserves drafts across controller updates and contains keyboard focus. Motion is limited to prompt surfaces, the monitoring switch, and press feedback; repeated navigation and data updates stay immediate. The native title bar uses the dark theme.

For browser verification, `?preview=1` explicitly loads in-memory sample data during Vite development; `&empty=1` clears the fixtures. The preview never invokes the controller, and recovery, disconnection, and live refresh are disabled. Production assets were checked to contain no fixture identifiers or relay hostnames. Normal browser access reports that the desktop app is required.

The desktop dev command now selects an available port from 1420 to 1499 and passes it to both Vite and Tauri. With an existing Vite process on 1420, a second desktop dev run launched on 1421 and served the app there. Direct `npm run dev` also falls back to the next available port.

Read-only inspection of the local OpenCode service confirmed the expected `idle` / `failed` / `provider.quota` / HTTP 429 message shape. No live outside-session continuation has been sent by this app yet.

## Where to look

| Need | Canonical source |
| --- | --- |
| User behavior and limitations | `README.md`, `PRODUCT.md` |
| Milestones and verification | `docs/ROADMAP.md` |
| Sidebar, session and relay inspectors, activity, prompt editor | `src/main.tsx`, `src/styles.css` |
| Frontend contracts and development fixtures | `src/types.ts`, `src/dev-preview.ts`, `src/vite-env.d.ts` |
| Desktop dev port handoff | `scripts/tauri.mjs`, `vite.config.ts`, `package.json` |
| Polling, recovery, settings and history | `src-tauri/src/lib.rs` |
| Session discovery and prompts | `src-tauri/src/opencode.rs`, `src-tauri/src/orca.rs` |
| Live terminal presence and app-owned plugin install | `src-tauri/src/presence.rs`, `integrations/opencode/tui.js` |
| Plugin route, heartbeat, and cleanup tests | `integrations/opencode/presence.test.mjs` |
| Relay selection and VPN connection | `src-tauri/src/vpngate.rs`, `src-tauri/src/softether.rs` |
| Commands and repository rules | `AGENTS.md`, `package.json`, `src-tauri/Cargo.toml` |
| Frontend design guidance | `.agents/skills/apple-design/SKILL.md`, `.agents/skills/emil-design-eng/SKILL.md`, `.agents/skills/find-animation-opportunities/SKILL.md` |

## Important constraints

- OpenCode's `/api/session/active` reports executing drains, not idle terminal presence. The dashboard combines those IDs with fresh reports from the app-owned CLI plugin and performs direct session lookups. Saved tabs, historical lists, and recency thresholds are not used for discovery. A just-failed, previously observed session stays recoverable after its client disappears. Without plugin presence, a response entirely between polls may be missed.
- The CLI plugin is installed under OpenCode's global `plugins/opc-rotate-presence` directory without changing configuration or other plugins. Its records in `~/.local/state/opc-rotate/presence` identify the current session, client PID, connected server PID, Orca ownership, and time. Require the registered shared-service PID and a lease at most eight seconds old. Private standalone reports are ignored. Missing service/plugin status is shown in the session pane; stale idle/busy rows are removed after API failure.
- Deduplicate Orca ownership by session ID, not directory. The same project can contain separate Orca and outside sessions. Only the current CLI route counts as open; globally persisted tab lists are historical state.
- Orca supplies existing terminal handles and worktree paths. Display project groups using these paths, without launching terminals.
- SoftEther changes the machine's route. Touch only app-created profiles and wait for other responding sessions before rotation.
- Prompt receipts and incident keys prevent duplicate continuation after ambiguous results.
- A verified IP can leave a continuation queued while its target is busy. The queue retries delivery after idle without starting another VPN switch.
- Local settings and connected VPN profiles are outside Git; never copy credentials or service passwords into documentation.

Start with the user's request and this map, then inspect the directly affected source and tests. The roadmap records verified status; source code defines actual behavior.
