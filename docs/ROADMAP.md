# OpenCode Recovery roadmap

`[x]` means implemented and verified. `[ ]` means pending or unverified. This is a status map, not authorization to start unrelated work.

## 1. Core desktop recovery — In progress

Outcome: the Windows dashboard can recover an affected existing session through a verified VPN switch.

- [x] Display SoftEther state, relay rankings, refresh action, monitoring control, continuation settings, and history. Verified in the Tauri dashboard and `npm run build`.
- [x] Parse VPN Gate CSV and rank relays by ping and speed; refresh on demand and every five minutes. Covered by Rust tests and a live list refresh.
- [x] Try at most five app-owned SoftEther profiles, cancel failed or slow attempts, and confirm a changed public IPv4 address. Covered by mock tests and live recovery that skipped a failed candidate.
- [x] Continue a rate-limited Orca terminal in place once after a verified switch. Confirmed in the existing terminal during live recovery.
- [x] Queue incidents, avoid uncertain duplicate prompt sends, and stop after two verified new IPs for a session. Covered by Rust tests.
- [ ] Show only live outside OpenCode sessions while retaining observed rate-limit incidents for recovery. The current build still lists recent historical sessions.
- [ ] Separate Orca terminals by project in the dashboard. The current build uses one flat Orca list.

## 2. Release verification — In progress

Outcome: a source build and installer work on the target Windows setup.

- [x] Type-check and bundle the web UI with `npm run build`.
- [x] Pass the Rust test suite (`npm test`, 15 tests at the last verified run).
- [x] Build the NSIS installer with `npm run tauri -- build` before the latest requested changes.
- [ ] Rebuild and visually verify the installer after active-session filtering and Orca grouping.
- [ ] Verify outside OpenCode session continuation during a real rate-limit event. Existing live evidence covers Orca; outside-session submission has mock API coverage only.

## Deferred

- [ ] Discover private OpenCode `--standalone` services after explicit server setup is designed.
- [ ] Verify provider egress over IPv6 as well as IPv4; current checks confirm IPv4 only.

Update this file only after observing the outcome or running the relevant check. Technical details live in source and [CONTEXT.md](CONTEXT.md).
