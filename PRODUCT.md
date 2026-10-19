# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

A solo operator on their own Windows workstation. The same person runs long-lived OpenCode v2 sessions in Orca terminals and in other terminals, and works alongside them. Their provider applies IP-sensitive limits, so a session that is doing real work can be cut off mid-task by a rate-limit/quota failure or by the free-tier client gate.

Their job is to keep that affected work alive: notice the failure, get a new public IP, and let the exact session continue from where it stopped — without hand-editing terminals, without losing the session, and without a second person watching the machine.

This is a one-operator tool, not a product for a general audience. Density, speed of recognition, and predictable behavior matter more than onboarding, discoverability, or collaborative features.

## Product Purpose

OpenCode Recovery watches live OpenCode sessions and completes a recovery for the ones that were blocked by an IP-sensitive provider failure.

It exists because the failure is invisible: a terminal looks idle, work has stopped, and the operator has no way to tell that a provider blocked the request rather than the model simply finishing. The app makes that failure visible, and then completes the recovery end to end — wait for the machine to go quiet, change the public IP through a VPN relay, verify the change, and deliver a continuation to the exact session that failed.

Success means the affected session resumes work after a verified IP change, with no duplicated prompt and no lost context. When recovery cannot be trusted, success is the app saying so plainly — surfacing the uncertainty for inspection rather than pretending, and never silently resending work.

## Positioning

Recovery is not a VPN client, not a session manager, and not a prompt injector. It is a recovery controller that binds three things that are otherwise separate: live tab presence reported by the app's own OpenCode CLI plugin, a separately installed SoftEther VPN client, and public VPN Gate relay data.

The mechanism a neighboring product could not copy truthfully is exactness of delivery under uncertainty. Continuation is addressed to an exact session ID through that session's live client, with the request claimed once, an incident guard persisted before transport, and a receipt read back from the client. Work lands in the session that failed rather than in whatever tab happens to be focused, and a delivery whose outcome is ambiguous is never resent automatically.

## Operating Context

- The operator's sessions are already running. The app connects to live OpenCode clients; it does not launch, own, close, or restart them, and their terminals stay open.
- A VPN change reroutes the whole machine, not one session. Other live work — including work the operator excluded from monitoring — can be interrupted by a rotation, so busy sessions everywhere delay a change.
- The operator must be able to leave the app running and trust it: a rotation happens on the operator's behalf, on a volunteer-operated relay, in the middle of other work.
- The provider's limits are outside the operator's control. A new IP may still be rejected, IPv6 egress is not verified, and a limit that persists is a normal outcome, not a failure of the app.
- The software lives on one Windows machine alongside separately installed dependencies: SoftEther VPN Client with its service and a virtual adapter, OpenCode v2, and the `orca` CLI for Orca terminals.

## Capabilities and Constraints

**Discovery**
- Live tab presence only, from the app-owned `opc-rotate-presence/tui.js` plugin that it installs under OpenCode's global plugin directory (honoring `OPENCODE_CONFIG_DIR`). It rewrites no OpenCode configuration and no other plugin.
- Historical session lists and saved global tab files are deliberately not discovery sources.
- Leases refresh every two seconds and expire after eight seconds. A closed tab disappears but an already observed incident stays visible, labeled **Closed tab**, with no delivery route until it is reopened.
- Sessions are deduplicated by exact session ID, never by project directory. Orca and outside clients are listed as separate sources.
- The shared service is a fallback for responding sessions, not for idle presence. Unknown execution status is never rendered as Idle.

**Monitoring selection**
- Rules exist per session and per terminal. A session override wins over its terminal rule; a new tab inherits its terminal rule.
- Choices are persisted. Unchecked sessions stay visible as **Not monitored** and cannot queue or receive recovery. Unchecking prunes queued work; pausing the master switch clears pending continuations.
- Busy-work protection covers every live session, monitored or not, because VPN routing is machine-wide. Unknown execution status also blocks a change.

**Recovery**
- Triggers: failed, idle provider events containing a rate-limit or quota error, and the specific message "OpenCode's free tier can only be used from within OpenCode." The client-gate match tolerates wrapped whitespace and curly apostrophes and ignores HTTP status. Ordinary permission errors and assistant prose are excluded.
- Sequence: wait for other detected work to go idle, rank eligible relays by low ping and high advertised speed, try at most five relays with up to twenty seconds per connection, require both a connected SoftEther account and a changed public IPv4 address checked through ipify, then deliver the continuation to the exact affected session while preserving the selected tab.
- A session stops after two verified IP changes. There is no timed reset, and the count survives restarts. A manual action queues another attempt that still waits for busy work and still verifies a changed IP.
- Requests are claimed once, receipts come back from the client, and the incident guard is persisted before transport. An ambiguous submission is shown for inspection and never retried automatically. Unknown execution status blocks VPN changes.
- The 30-second Orca delay is a minimum after a confirmed continuation receipt, not a fixed interval after an error. Outside sources have no such gate.

**Relays and history**
- The relay list refreshes every five minutes and on demand, and can be sorted by recommendation, ping, or line speed. A preferred relay becomes the next first candidate.
- Activity records connection changes, prompt receipts, and failures, and is the place to look when a recovery stops or a delivery is uncertain.

**Terminology** (fixed product language)
- Activity: **Idle**, **Working**, **Rate limited**, **Provider blocked**, **Status unavailable**, **Selected tab**, **Background tab**.
- Recovery: **queued**, **waiting for work**, **connecting**, **verifying IP**, **waiting for idle**, **sending a prompt**, **prompt sent**, **stopped/uncertain**, **Not monitored**, **Stopped · 2-IP limit**, **Eligible in …s**, **Awaiting next check**.
- A session can stay **Rate limited** while its recovery is **Stopped**: the provider failure and the app's recovery process are separate states.
- **Prompt sent** confirms a receipt, not that the provider accepted the next model call.

**Platform and environment**
- Windows x64 desktop app (Tauri 2), with a React 19 + TypeScript + Vite 6 interface and a Rust 2021 controller. The interface is a web UI rendered in a native window.
- Requires SoftEther VPN Client in its default install location, its Windows service running, and an enabled virtual adapter named **VPN**. SoftEther is a separate installed dependency and is not bundled or uninstalled by this app. Custom install paths are not configurable.
- Requires OpenCode v2 with CLI tab-plugin support (integration verified with 2.0.32), and the `orca` CLI on `PATH` for Orca terminals.
- The app touches only SoftEther profiles carrying its `OPCRotate_` prefix, and refuses to replace an unrelated connected account. Failed app-created profiles are removed before the next candidate is tried.
- Background CLI helpers run with Windows `CREATE_NO_WINDOW`; polling must never open a console or activate Windows Terminal.
- Local state lives in the user's config directory: settings, history, and incident guards in `%APPDATA%\opc-rotate\settings.json`; presence, requests, and receipts under the user's local state directory. Presence records carry no credentials and no prompt text.

**Open product facts, deliberately undecided**
- Publisher signing and the update distribution mechanism for a future public release.
- Whether a single installer should also run the official SoftEther installer, pending redistribution terms and administrator-prompt design.
- Whether to verify provider egress over IPv6 in addition to IPv4.
- Whether to support arbitrary private OpenCode services with no live CLI client, which would require a server setup design. Live connected private clients already work through their own SDK.

## Brand Commitments

- Name: **OpenCode Recovery** (window title and installer product name). The repository and package name remain `opc-rotate`.
- The app icon is a real, existing asset (`src/assets/app-icon.png`, `src-tauri/app-icon.svg`) and is preserved as the source of the app's color. It is not redrawn or recolored per surface.
- Voice: plain, precise, operator-facing. Technical names (OpenCode, Orca, SoftEther, VPN Gate) are used as they are. The documentation states limits and uncertainty directly, without promotional language, and prefers naming the real state (**Stopped · 2-IP limit**, **Awaiting next check**) over euphemism.
- Sample and preview data is always labeled as such in the product surface, and preview fixtures are excluded from release builds. A preview is never presented as a live connection.
- MIT licensed, with bundled third-party notices preserved.
- The app is a guest in the operator's environment: it preserves existing OpenCode and Orca sessions, does not launch or own them, and leaves terminals and their selected tabs intact.

## Evidence on Hand

- `docs/images/dashboard.jpg` — labeled development preview of the workspace with sample data and no live connection.
- `src/dev-preview.ts` — development-only fixture data, reached through `?preview=1` in the browser; excluded from release assets by `scripts/check-release.mjs`.
- `release-artifacts/v0.1.0/` — built NSIS installer and matching `SHA256SUMS.txt` (gitignored, not published).
- Automated coverage: 9 Node presence-plugin tests plus Rust unit and mock HTTP tests alongside the source, run by `npm test`.
- Verified live, in the running native app: discovery of idle and background tabs in Orca and outside clients, project grouping, and detection of the free-tier client-gate provider error with monitoring paused.
- Verified live earlier, using the former terminal-send transport: a recovery that skipped a failed relay candidate.

**Absences — future work must not fabricate**
- No public GitHub release has been published; version 0.1.0 is a preview candidate.
- Live continuation through the current exact-session transport is unverified for Orca, outside clients, and background tabs. Live VPN verification is still pending.
- Clean-machine install, upgrade, and uninstall smoke tests have not been run.
- The installer is unsigned and has no automatic updater.
- There are no customers, testimonials, case studies, benchmarks, pricing, or licensing claims beyond the MIT license.

## Product Principles

1. **Verify before acting.** A change of public IP is confirmed against the real address, and a delivery is confirmed by a receipt. Anything less is reported as uncertainty, not as success.
2. **Address work exactly.** Work is identified by session ID, never by directory, never by whatever terminal tab is focused, and never by typing into a terminal.
3. **The machine is shared.** One operator's rotation reroutes everything running on it, so every live session's busy state protects the network even when that session is excluded from recovery.
4. **Preserve the operator's work.** The app connects to existing sessions and leaves them running. It keeps incidents visible after they close, keeps monitoring choices across restarts, and never discards a queued continuation silently.
5. **Say what actually happened.** The app reports the real stage, the real count, and the real uncertainty in plain terms, and it never invents progress or resends work behind the operator's back.

## Accessibility & Inclusion

The operator works in a dark desktop environment, often under time pressure, with one window and dense rows. The confirmed requirements are: inspected text contrast for every non-disabled label, full keyboard selection and navigation, visible focus rings, native expand/collapse disclosures for detail, reduced-motion support in which waiting states use a static indicator instead of a continuous spinner, and no horizontal overflow at the desktop minimum window size of 980 × 660 as well as at wider and narrower widths.

The tool is for a single expert user, so density is a feature: status, counts, and stage belong on the row itself rather than behind a hover, tooltip, or separate screen.
