# Project context

## At a glance

**OpenCode Recovery** is a Windows Tauri dashboard for IP-sensitive OpenCode rate limits. It reads public VPN Gate relays, connects with the installed SoftEther client, verifies a changed public IPv4 address, then continues affected existing sessions. The primary user has OpenCode work in Orca terminals across projects and in the shared OpenCode service.

**Current focus:** verify the updated project groups visually in the native window and observe outside-session continuation during a real rate-limit event. Live-session filtering and the per-project layout are implemented; 17 Rust tests, the frontend build, and the updated NSIS installer build pass. An Orca live recovery succeeded earlier. Native screenshot capture was covered by another fullscreen application.

The desktop dev command now selects an available port from 1420 to 1499 and passes it to both Vite and Tauri. With an existing Vite process on 1420, a second desktop dev run launched on 1421 and served the app there. Direct `npm run dev` also falls back to the next available port.

Read-only inspection of the local OpenCode service confirmed the expected `idle` / `failed` / `provider.quota` / HTTP 429 message shape. No live outside-session continuation has been sent by this app yet.

## Where to look

| Need | Canonical source |
| --- | --- |
| User behavior and limitations | `README.md`, `PRODUCT.md` |
| Milestones and verification | `docs/ROADMAP.md` |
| Dashboard and presentation | `src/main.tsx`, `src/styles.css` |
| Desktop dev port handoff | `scripts/tauri.mjs`, `vite.config.ts`, `package.json` |
| Polling, recovery, settings and history | `src-tauri/src/lib.rs` |
| Session discovery and prompts | `src-tauri/src/opencode.rs`, `src-tauri/src/orca.rs` |
| Relay selection and VPN connection | `src-tauri/src/vpngate.rs`, `src-tauri/src/softether.rs` |
| Commands and repository rules | `AGENTS.md`, `package.json`, `src-tauri/Cargo.toml` |
| Frontend design guidance | `.agents/skills/apple-design/SKILL.md`, `.agents/skills/emil-design-eng/SKILL.md` |

## Important constraints

- OpenCode's shared service session list includes historical records. The dashboard uses active IDs and direct session lookups; a just-failed, previously observed session stays recoverable after leaving that endpoint. A response that starts and fails entirely between polls may be missed.
- Orca supplies existing terminal handles and worktree paths. Display project groups using these paths, without launching terminals.
- SoftEther changes the machine's route. Touch only app-created profiles and wait for other responding sessions before rotation.
- Prompt receipts and incident keys prevent duplicate continuation after ambiguous results.
- A verified IP can leave a continuation queued while its target is busy. The queue retries delivery after idle without starting another VPN switch.
- Local settings and connected VPN profiles are outside Git; never copy credentials or service passwords into documentation.

Start with the user's request and this map, then inspect the directly affected source and tests. The roadmap records verified status; source code defines actual behavior.
