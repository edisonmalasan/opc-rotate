# OpenCode Recovery roadmap

`[x]` means implemented and verified. `[ ]` means pending or unverified. This is a status map, not authorization to start unrelated work.

## 1. Core desktop recovery — In progress

Outcome: the Windows dashboard can recover an affected existing session through a verified VPN switch.

- [x] Display SoftEther state, relay rankings, refresh action, monitoring control, continuation settings, and history. Verified in the Tauri dashboard and `npm run build`.
- [x] Parse VPN Gate CSV and rank relays by ping and speed; refresh on demand and every five minutes. Covered by Rust tests and a live list refresh.
- [x] Try at most five app-owned SoftEther profiles, cancel failed or slow attempts, and confirm a changed public IPv4 address. Covered by mock tests and live recovery that skipped a failed candidate.
- [x] Continue a rate-limited Orca terminal in place once after a verified switch. The earlier terminal-send implementation was confirmed live. It is now replaced by exact-session plugin delivery; live verification of that new transport remains below.
- [x] Queue incidents, avoid uncertain duplicate prompt sends, and stop after two verified new IPs for a session. Covered by Rust tests.
- [x] Discover all open tabs in live Orca and outside clients, including idle/background tabs and connected private services. Fresh V2 plugin reports showed two Vise tabs, two Thesis tabs, and one spectre-mail tab without restarting their terminals. Saved global tab files and historical session lists remain excluded. Expired leases disappear; closed incidents retain their error without an active delivery route.
- [x] Display all session tabs directly beneath folder-marked project headings for both sources, with no redundant terminal heading/count rows. Keep per-tab Idle/Working/Needs recovery/unavailable status, selected/background labels, and an Idle filter. Browser fixtures verified folder icons for all four project groups, all six session rows, no terminal headings, and no overflow at 1280 × 820 and 980 × 660.
- [x] Shorten the default prompt to “Continue from where you stopped after the rate limit”. Existing settings migrate only the former built-in default, preserving custom global prompts and session overrides. Browser prompt-editor verification and a Rust migration test cover the change.
- [x] Route continuation to the exact session through a claimed local request and SDK receipt, preserving the selected tab. Node and Rust mocks cover duplicate/uncertain delivery, busy/closed/expired/replaced clients, private clients, and timeout cleanup. The controller persists its incident guard before sending. Live inspection corrected the Orca pane key to `tabId:leafId`.
- [x] Deduplicate Orca ownership by exact session ID instead of excluding an entire project directory. A mock API test covers an open idle outside session alongside an Orca-owned ID; historical session enumeration is absent.
- [x] Group outside sessions by project in both the sidebar and session list. Browser fixtures verified two outside projects, two sessions in one project, and independent Orca/outside scopes for the same directory. Native accessibility verified the real Thesis project and session.
- [x] Label the sidebar sections Orca sessions and Other sessions, with project links directly beneath each heading and no separate Other sessions overview row. Verified by `npm run build` and desktop-size browser navigation.
- [x] Improve desktop readability with shared type tokens, stronger secondary text, larger controls, wrapped session titles, and clearer inspector details. Browser checks passed at 1280 × 820, 980 × 660, and 390 × 844, including inspected text contrast, sorting, filtering, empty results, keyboard selection, and prompt focus/Escape. Impeccable's detector returned no findings.
- [x] Separate Orca terminals by project in the dashboard. Verified by the TypeScript/Vite build and the running native app's accessibility tree, which showed two project groups.
- [x] Replace the long dashboard with a desktop workspace: project sidebar, separate Sessions/Relays/Activity views, persistent route bar, grouped session selection, and contextual inspectors. Verified with populated development fixtures at 1280 × 820, 980 × 660, and 390px width, without horizontal page overflow.
- [x] Verify project and status filters, arrow-key selection, prompt editing, focus wrapping and Escape, relay sorting and preferences, search, and activity filters in the browser preview. Fixture interactions stay in memory; production assets contain no fixture identifiers or hostnames.
- [x] Keep a continuation queued on the verified IP when its target is busy; retry prompt delivery after it becomes idle without rotating again. Covered by the recovery scheduling test and controller state review.

## 2. Release verification — In progress

Outcome: a source build and installer work on the target Windows setup.

- [x] Type-check and bundle the web UI with `npm run build`.
- [x] Pass `npm test`: 8 Node presence-plugin tests and 26 Rust tests at the last verified run, including the prompt migration and a Windows no-console regression check.
- [x] Suppress console popups from background Orca/SoftEther CLI helpers with Windows `CREATE_NO_WINDOW`. A real PowerShell child reports no console handle while command output is still captured.
- [x] Build the updated NSIS installer with `npm run tauri -- build` after adding Windows console suppression for background CLI helpers.
- [x] Start a second desktop dev run when port 1420 is occupied. The npm Tauri command selected 1421 for both Vite and Tauri; the page returned HTTP 200, and the existing process on 1420 remained active.
- [x] Visually verify the workspace with live data in an unobstructed native window. The rebuilt release displayed both Thesis tabs under Other sessions and both Vise tabs under Orca sessions, with separate selected/background labels and Idle status. The sidebar counted five live sessions with no duplicate legacy terminal rows. Native project navigation also passed.
- [ ] Verify the new exact-session plugin continuation during a real rate-limit event for Orca and outside clients, including a background tab. Parser, receipt, and busy/closed-tab behavior have mock coverage; no live prompt or VPN change was performed for this tab-discovery update.

## Deferred

- [ ] Connect to arbitrary private OpenCode services without a live CLI plugin after explicit server setup is designed. Live connected private clients already work through their own SDK.
- [ ] Verify provider egress over IPv6 as well as IPv4; current checks confirm IPv4 only.

## SoftEther setup for downloaded releases — Later

- [ ] Check for the installed SoftEther VPN Client, its running service, `vpncmd`, and an enabled virtual adapter named `VPN` before recovery; show specific setup steps for anything missing.
- [ ] Offer a guided link to the official SoftEther VPN Client installer. Keep SoftEther as a separate installed dependency for the first public release.
- [ ] Revisit a single installer that runs the official SoftEther installer as a prerequisite, after checking its redistribution terms, administrator prompts, existing installations, and update behavior. Do not bundle only the Client Manager or `vpncmd` executable; the service and virtual adapter are also required.

Update this file only after observing the outcome or running the relevant check. Technical details live in source and [CONTEXT.md](CONTEXT.md).
