# OpenCode Recovery roadmap

`[x]` means implemented and verified. `[ ]` means pending or unverified. This is a status map, not authorization to start unrelated work.

## 1. Core desktop recovery — In progress

Outcome: the Windows dashboard can recover an affected existing session through a verified VPN switch.

- [x] Display SoftEther state, relay rankings, refresh action, monitoring control, continuation settings, and history. Verified in the Tauri dashboard and `npm run build`.
- [x] Parse VPN Gate CSV and rank relays by ping and speed; refresh on demand and every five minutes. Covered by Rust tests and a live list refresh.
- [x] Try at most five app-owned SoftEther profiles, cancel failed or slow attempts, and confirm a changed public IPv4 address. Covered by mock tests and live recovery that skipped a failed candidate.
- [x] Continue a rate-limited Orca terminal in place once after a verified switch. Confirmed in the existing terminal during live recovery.
- [x] Queue incidents, avoid uncertain duplicate prompt sends, and stop after two verified new IPs for a session. Covered by Rust tests.
- [x] Discover open idle outside terminals as well as responding sessions and observed rate-limit incidents. The local presence plugin reports the current session and shared-service PID; stale leases and private-service reports are excluded. Node and Rust tests cover these cases. Live verification detected the user's idle Thesis research session and the native dashboard listed it as Ready under Other projects → Thesis, without restarting the terminal.
- [x] Deduplicate Orca ownership by exact session ID instead of excluding an entire project directory. A mock API test covers an open idle outside session alongside an Orca-owned ID; historical session enumeration is absent.
- [x] Group outside sessions by project in both the sidebar and session list. Browser fixtures verified two outside projects, two sessions in one project, and independent Orca/outside scopes for the same directory. Native accessibility verified the real Thesis project and session.
- [x] Label the sidebar sections Orca sessions and Other sessions, with project links directly beneath each heading and no separate Other sessions overview row. Verified by `npm run build` and desktop-size browser navigation.
- [x] Separate Orca terminals by project in the dashboard. Verified by the TypeScript/Vite build and the running native app's accessibility tree, which showed two project groups.
- [x] Replace the long dashboard with a desktop workspace: project sidebar, separate Sessions/Relays/Activity views, persistent route bar, grouped session selection, and contextual inspectors. Verified with populated development fixtures at 1280 × 820, 980 × 660, and 390px width, without horizontal page overflow.
- [x] Verify project and status filters, arrow-key selection, prompt editing, focus wrapping and Escape, relay sorting and preferences, search, and activity filters in the browser preview. Fixture interactions stay in memory; production assets contain no fixture identifiers or hostnames.
- [x] Keep a continuation queued on the verified IP when its target is busy; retry prompt delivery after it becomes idle without rotating again. Covered by the recovery scheduling test and controller state review.

## 2. Release verification — In progress

Outcome: a source build and installer work on the target Windows setup.

- [x] Type-check and bundle the web UI with `npm run build`.
- [x] Pass `npm test`: 2 Node presence-plugin tests and 20 Rust tests at the last verified run.
- [x] Build the updated NSIS installer with `npm run tauri -- build` after active-session filtering, queued delivery, the desktop workspace rebuild, and the idle-presence/project-grouping fix.
- [x] Start a second desktop dev run when port 1420 is occupied. The npm Tauri command selected 1421 for both Vite and Tauri; the page returned HTTP 200, and the existing process on 1420 remained active.
- [ ] Visually verify the new workspace with live data in an unobstructed native window. The previous interface's native accessibility tree showed two Orca groups; current layout and interaction checks used labeled development fixtures in the browser.
- [ ] Verify outside OpenCode session continuation during a real rate-limit event. The local shared service has real failed `provider.quota` messages with status 429 and matches the parser; outside-session prompt submission has mock API coverage only.

## Deferred

- [ ] Discover private OpenCode `--standalone` services after explicit server setup is designed.
- [ ] Verify provider egress over IPv6 as well as IPv4; current checks confirm IPv4 only.

## SoftEther setup for downloaded releases — Later

- [ ] Check for the installed SoftEther VPN Client, its running service, `vpncmd`, and an enabled virtual adapter named `VPN` before recovery; show specific setup steps for anything missing.
- [ ] Offer a guided link to the official SoftEther VPN Client installer. Keep SoftEther as a separate installed dependency for the first public release.
- [ ] Revisit a single installer that runs the official SoftEther installer as a prerequisite, after checking its redistribution terms, administrator prompts, existing installations, and update behavior. Do not bundle only the Client Manager or `vpncmd` executable; the service and virtual adapter are also required.

Update this file only after observing the outcome or running the relevant check. Technical details live in source and [CONTEXT.md](CONTEXT.md).
