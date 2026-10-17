# OpenCode Recovery: agent guide

## Project

OpenCode Recovery is a Windows desktop dashboard for recovering live OpenCode work from IP-sensitive rate limits. It uses VPN Gate relay data, the installed SoftEther VPN Client, Orca terminal controls, and OpenCode's shared local service. See [README.md](README.md) for use and [docs/CONTEXT.md](docs/CONTEXT.md) for a source map.

The Tauri application, Rust controller, active-session filtering, per-project Orca layout, tests, and Windows installer build exist. [docs/ROADMAP.md](docs/ROADMAP.md) records verified progress and remaining live checks; inspect source and tests before changing its status.

## Source map

| Concern | Source |
| --- | --- |
| Sidebar, session inspector, relay and activity views | `src/main.tsx`, `src/styles.css` |
| Development-only sample data | `src/dev-preview.ts` (`?preview=1`, never a live controller) |
| Dev server port selection | `scripts/tauri.mjs`, `vite.config.ts` |
| State, polling, incident queue, recovery | `src-tauri/src/lib.rs` |
| OpenCode shared service | `src-tauri/src/opencode.rs` |
| Live OpenCode terminal presence | `src-tauri/src/presence.rs`, `integrations/opencode/tui.js` |
| Orca terminals | `src-tauri/src/orca.rs` |
| VPN Gate CSV and ranking | `src-tauri/src/vpngate.rs` |
| SoftEther profile lifecycle | `src-tauri/src/softether.rs` |
| UI data contracts | `src/types.ts`, `src-tauri/src/models.rs` |
| Product intent | `PRODUCT.md`, `docs/ROADMAP.md` |

## Stack and commands

React 19, TypeScript, and Vite 6 provide the UI (`package.json`). Tauri 2 and Rust 2021 provide the desktop controller (`src-tauri/Cargo.toml`). Settings and history are JSON in the user's config directory (`src-tauri/src/lib.rs`). Rust unit and mock HTTP tests live alongside the source.

Run from the repository root:

```powershell
npm install
npm run tauri -- dev
npm run build
npm test
npm run tauri -- build
```

`npm run build` includes TypeScript checking. `npm test` runs the Node presence-plugin tests and Rust tests. The Tauri build creates an NSIS installer under `src-tauri/target/release/bundle/nsis/`.
`npm run tauri -- dev` selects an available port from 1420 to 1499 and passes it to both Vite and Tauri; use this command for desktop development when 1420 may be occupied.
For populated browser UI checks, use the Vite URL with `?preview=1`; add `&empty=1` for empty states. Preview edits are local to the page, and fixture data is excluded from release builds. Keep sample data clearly labeled and isolated from live recovery.

## Boundaries

- Preserve existing Orca/OpenCode sessions; the app connects to them but does not launch or own them.
- Touch only SoftEther profiles with the app's `OPCRotate_` prefix. VPN routing affects the machine; wait for other active work to become idle.
- Require a connected SoftEther account and a changed public IPv4 address before continuing. Bound relay attempts and avoid duplicate prompt submissions after uncertain receipts.
- The first version uses OpenCode's shared loopback service. Private `--standalone` services require explicit support.
- UI session lists represent live work, not historical shared-service records. Preserve an already detected rate-limit incident long enough to recover it.
- An idle terminal is live when its presence lease identifies the current shared-service PID. Saved TUI tabs and session timestamps are not proof of a live client. Deduplicate Orca ownership by session ID, never by the whole project directory. Keep both sources grouped by project.
- The app installs only its own `opc-rotate-presence/tui.js` under OpenCode's global plugin directory; it does not rewrite OpenCode configuration or other plugins. Keep presence records free of credentials and prompt text.
- User requests take precedence over this guide. The current user explicitly requested separate commits pushed to `main`.

## Working method

1. Read `docs/CONTEXT.md`, the relevant roadmap section, and directly affected source.
2. Make a small, reviewable change. Apply the repository's Apple Design and Emil Design Engineering skills for the requested frontend pass.
3. Run relevant builds and tests. Live VPN tests can interrupt the user's connection; run those only when necessary.
4. Update the roadmap only for verified outcomes; label remaining gaps accurately.
5. Stage logical groups, use conventional commits, and push each requested commit separately. Do not commit secrets, local settings, `node_modules`, `dist`, or `src-tauri/target`.

If documentation disagrees with code, report the discrepancy and resolve it against the approved behavior. Template text and generated knowledge are not implementation evidence.
