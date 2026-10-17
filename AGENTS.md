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
| Live tab presence, exact-session requests and receipts | `src-tauri/src/presence.rs`, `integrations/opencode/tui.js` |
| Orca terminals | `src-tauri/src/orca.rs` |
| VPN Gate CSV and ranking | `src-tauri/src/vpngate.rs` |
| SoftEther profile lifecycle | `src-tauri/src/softether.rs` |
| Background CLI execution and Windows console suppression | `src-tauri/src/process.rs` |
| UI data contracts | `src/types.ts`, `src-tauri/src/models.rs` |
| Monitoring selection and terminal inheritance | `src/monitoring.ts`, `Settings` in `src-tauri/src/models.rs`, queue/commands in `src-tauri/src/lib.rs` |
| User setup and release limits | `README.md`, `docs/USAGE.md`, `docs/ROADMAP.md` |
| Release packaging and CI | `scripts/release.mjs`, `scripts/check-release.mjs`, `.github/workflows/windows.yml`, `docs/RELEASING.md` |

## Stack and commands

React 19, TypeScript, and Vite 6 provide the UI (`package.json`). Tauri 2 and Rust 2021 provide the desktop controller (`src-tauri/Cargo.toml`). Settings and history are JSON in the user's config directory (`src-tauri/src/lib.rs`). Rust unit and mock HTTP tests live alongside the source.

Run from the repository root:

```powershell
npm ci
npm run tauri -- dev
npm run build
npm test
npm run tauri -- build
npm run release:build
```

`npm run build` includes TypeScript checking. `npm test` runs the Node presence-plugin tests and locked Rust tests. The Tauri build creates an NSIS installer under `src-tauri/target/release/bundle/nsis/`. `npm run release:build` checks versions and fixture exclusion, builds with the Cargo lockfile, then copies the installer and SHA-256 checksum into ignored `release-artifacts/v<version>/`. Toolchain versions are pinned in `.node-version` and `rust-toolchain.toml`. See `docs/RELEASING.md` for manual acceptance gaps before publication.
`npm run tauri -- dev` selects an available port from 1420 to 1499 and passes it to both Vite and Tauri; use this command for desktop development when 1420 may be occupied.
For populated browser UI checks, use the Vite URL with `?preview=1`; add `&empty=1` for empty states. Preview edits are local to the page, and fixture data is excluded from release builds. Keep sample data clearly labeled and isolated from live recovery.

## Boundaries

- Preserve existing Orca/OpenCode sessions; the app connects to them but does not launch or own them.
- Run background CLI helpers through `process::run`, which sets Windows `CREATE_NO_WINDOW`. Polling must never open a console or activate Windows Terminal.
- Touch only SoftEther profiles with the app's `OPCRotate_` prefix. VPN routing affects the machine; wait for other active work to become idle.
- Require a connected SoftEther account and a changed public IPv4 address before continuing. Bound relay attempts and avoid duplicate prompt submissions after uncertain receipts.
- Use the shared service for API fallback. A live CLI plugin can report and control its own connected private service through the SDK; do not discover arbitrary private endpoints or copy their credentials.
- UI session lists represent live work, not historical shared-service records. Preserve an already detected rate-limit incident long enough to recover it.
- Persist session and terminal monitoring choices. Unchecked sessions remain visible but cannot queue or receive recovery. Terminal rules cover new tabs, with individual session overrides; preserve known terminal ownership through API fallback. Keep busy-work protection for every live session, including unchecked ones, because VPN routing is machine-wide. Pausing the master switch clears queued continuations.
- Live client leases report their tab strip and connected service PID. Saved global TUI tabs and session timestamps are not proof of a live client. Deduplicate ownership by session ID, never by the whole project directory. Show sessions directly under folder-marked project headings for both sources; omit redundant terminal heading/count rows. Keep terminal identity for exact routing. Selection is independent of Idle/Working status. Closed incidents stay visible without a delivery route until reopened.
- Address continuation by exact session ID through the live plugin request/receipt channel. Do not type a recovery prompt into an Orca terminal, since its selected tab can change. Persist the incident guard before delivery, claim each request once, and never retry ambiguous receipts automatically. Unknown execution status blocks VPN changes.
- The app installs only its own `opc-rotate-presence/tui.js` under OpenCode's global plugin directory; it does not rewrite OpenCode configuration or other plugins. Keep presence records free of credentials and prompt text.
- User requests take precedence over this guide. The current user explicitly requested separate commits pushed to `main`.

## Working method

1. Read `docs/CONTEXT.md`, the relevant roadmap section, and directly affected source.
2. Make a small, reviewable change. Apply the repository's Apple Design and Emil Design Engineering skills for the requested frontend pass.
3. Run relevant builds and tests. Live VPN tests can interrupt the user's connection; run those only when necessary.
4. Update the roadmap only for verified outcomes; label remaining gaps accurately.
5. Stage logical groups, use conventional commits, and push each requested commit separately. Do not commit secrets, local settings, `node_modules`, `dist`, or `src-tauri/target`.

If documentation disagrees with code, report the discrepancy and resolve it against the approved behavior. Template text and generated knowledge are not implementation evidence.
