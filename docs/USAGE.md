# Setup and use

## Install prerequisites

OpenCode Recovery targets Windows x64. Node.js and Rust are needed only when building from source.

1. Install [SoftEther VPN Client](https://www.softether-download.com/en.aspx?product=softether), including its Windows service. Open **SoftEther VPN Client Manager** and create or enable a virtual adapter named **VPN**.
2. Keep SoftEther in its default installation directory. Recovery looks for `vpncmd_x64.exe` or `vpncmd.exe` under `C:\Program Files\SoftEther VPN Client`, or `vpncmd.exe` under `C:\Program Files (x86)\SoftEther VPN Client`. Custom paths are not configurable yet.
3. Install OpenCode v2 with CLI tab-plugin support; the integration was verified with **2.0.32**. For Orca terminals, the `orca` CLI must also be on `PATH`.
4. Install OpenCode Recovery using the Windows x64 installer. Its default Tauri installer can download Microsoft WebView2 when it is missing, so installation may require internet access.

SoftEther is a **separate installed dependency** in this preview. Its service and adapter do the networking; Recovery controls it with `vpncmd`. The VPN Gate Client plug-in that adds the public relay list to Client Manager is optional: Recovery fetches the public list itself. Client Manager does not need to stay open once the service and adapter are running.

## First run

1. Open your existing OpenCode terminals, with `opencode` or `opencode --auto`, inside Orca or another terminal.
2. Start Recovery. It installs its own small OpenCode CLI plugin; your terminals remain open. If a terminal is missing, reopen that OpenCode client once to load the plugin.
3. Check the **Orca projects** and **Other projects** lists in the rail. Every live tab appears as a row in the session list, including idle and background tabs, and each row names its own project and source.
4. Choose which work to recover. All sessions are included initially; use the row checkboxes or **Monitor this terminal** in the inspector. A terminal rule includes new tabs; individual session choices override it. Choices are saved.
5. Turn on **Monitoring** when ready. It starts off on a fresh install and restores your saved preference on subsequent launches.

**Idle** means the session is live but not responding. **Working** means it is responding. **Rate limited** or **Provider blocked** describes the latest qualifying provider failure. **Worker Idle** means the session finished its turn and is waiting for you. This appears beneath the session title; the separate **Recovery** column shows what Recovery is doing. **Status unavailable** means execution status could not be established. **Selected tab** describes the tab shown in its terminal, independently of whether it is working. The **Provider errors** filter includes affected sessions even if recovery is stopped or they are not monitored.

Unchecked sessions stay visible as **Not monitored**. They cannot trigger recovery or receive its prompts. Unchecking removes queued work; a prompt already submitted cannot be recalled. Pausing Monitoring clears pending continuations without changing your selections. **Recover session** and **Continue session** are manual actions for included sessions and work while automatic monitoring is paused.

## Watcher modes

Every session is watched for one of two things, and the mode is chosen in two places.

- The **Recovery** control in the top bar, immediately to the left of the **Monitoring** switch, sets the mode that every session follows unless it overrides it. It reads **Recovery** followed by the current mode, so it shows **Recovery  Worker Idle** or **Recovery  Rate limits**, and clicking it switches to the other mode. It is the only app-wide control for the mode.
- **Watch this session for** in the session inspector's **Monitoring** section sets the mode of the selected session. It has three options: **Rate limits**, **Worker Idle**, and **Default**. **Default** means the session follows the app-wide mode. The line under it reads **Following the app-wide mode: Worker Idle** or **Following the app-wide mode: Rate limits** while the session is on **Default**, and reads **Overridden for this session** once the session has a choice of its own.

All choices are saved. The app-wide mode starts on **Worker Idle**, so a fresh install watches finished turns rather than rate limits. Changing the app-wide mode changes every session that is on **Default**; a session with its own choice keeps it. Changing the app-wide mode also clears the work queued for every session.

**Rate limits** is the provider-failure watcher. When a session hits a rate-limit or quota failure, or the free-tier client gate, Recovery waits for the machine to go quiet, changes the public IP through a VPN relay, verifies the new address, and only then sends the continuation prompt. The limit of two verified IP changes per session applies to this mode.

**Worker Idle** watches for a session that finished its turn and is waiting for you. Recovery sends the continuation prompt to that session and does nothing else: it connects no relay, changes no public IP, and uses none of the two IP changes. The session's activity is shown as **Worker Idle**.

The two modes are exclusive per session. A session watched for finished turns that hits a rate limit is still shown as **Rate limited**, but it is not recovered automatically, because a prompt cannot clear a rate limit. A manual recovery follows the session's mode: **Continue session** queues a prompt-only continuation, and **Recover session** queues a route change.

Changing one session's mode cancels the other kind of work queued for it. A session switched to **Worker Idle** no longer keeps a queued route change, and one switched to **Rate limits** no longer keeps a queued continuation.

Both modes send the same default continuation prompt:

> Continue

Edit it with **Continuation prompt** in the sidebar, or set a session override in the inspector. A session's own prompt override wins over the default. The editor on screen always shows the prompt that will actually be sent, so there is no separate prompt per mode to keep track of.

**Subagents.** The live OpenCode plugin reports how many child sessions of a session are still executing. The session row shows the count, for example **2 subagents working**. A session with a live subagent is not treated as finished, so its parent is not continued while a child session is still working.

**Worker Idle** and the subagent count come from the live OpenCode CLI plugin only, because only that client can see a session's child sessions and can tell your own message apart from one Recovery delivered. The shared-service fallback, used when no live client is present, reports rate limits, busy, and idle; it never reports a finished turn, and a session it reports is never continued automatically.

### What Worker Idle hides

When no session on screen is watched for rate limits, the rate-limit parts of the interface are hidden, so the window does not imply that a network change is involved.

- **The route bar** — the connection state, public IP, **Disconnect**, and **View relays** — is hidden. It reappears when a relay is connected, when a recovery has to change the public IP, or when any session still watches rate limits. The relay list stays reachable from the sidebar.
- The **Provider errors** filter is not offered, because no session is being watched for a provider failure.
- The inspector's primary action reads **Continue session** instead of **Recover session**, and its helper line reads **Sends a prompt to this session. The public IP is not changed.** instead of **Verifies a new IP before sending a continuation.**

### The route line

A recovering session shows a route line of six stations: **Queued**, **Idle wait**, **New route**, **Verify IP**, **Deliver**, **Receipt**. In **Rate limits** mode the whole route is traversed. In **Worker Idle** mode the two relay stations, **New route** and **Verify IP**, are drawn as short dashes and the line is broken across them, so the route that was not taken is visible. A failure breaks the line where it happened: a failed route change breaks at **New route**, a failed continuation breaks at **Deliver**. The live station breathes while a recovery is moving; reduced motion stops the pulse.

**Preview.** The browser preview (`npm run dev`, then `?preview=1`) opens on the shipped default: **Worker Idle**, no route bar, a continuation in flight and one queued. Add `&rate=1` for **Rate limits** with the route bar and the **Provider errors** filter visible, `&recovery=1` for the rate-limit route diagram in its four states — connecting, waiting, receipt confirmed, and stopped — `&empty=1` for empty states, and `&view=relays` or `&view=activity` for the other two views. Preview data is in-memory and never reaches a live client.

## Recovery sequence

The **Rate limits** watcher recognizes failed, idle provider events containing a rate-limit/quota error, or the specific message:

> OpenCode's free tier can only be used from within OpenCode.

The latter match accepts wrapped whitespace and curly apostrophes, regardless of HTTP status. Ordinary permission errors and assistant prose are excluded.

1. Wait for other detected work to become idle. Busy sessions block VPN changes even if you excluded them, because VPN routing affects the whole machine. Unknown execution status also blocks changes.
2. Rank eligible VPN Gate relays using low ping and high advertised speed. Try at most **five relays**, allowing **20 seconds** for each connection. Disconnect and remove failed app-created profiles before trying the next.
3. Require both a connected SoftEther account and a changed **public IPv4** address, checked through ipify.
4. Send the continuation to the exact affected session ID through its live CLI plugin, preserving the currently selected tab. Busy targets wait on the verified connection instead of causing another rotation.
5. Stop if the error persists after **two verified IP changes** for that session. Record the result in **Activity**.

The **Worker Idle** mode runs only the delivery step: it sends the continuation prompt to the exact session and leaves the relay, the public IP, and the IP-change count alone.

Request claims, SDK receipts, and a persisted incident guard prevent automatic duplicate submissions. An ambiguous submission is not retried automatically; inspect the session before taking another action. The app touches only profiles named with its `OPCRotate_` prefix and refuses to replace an unrelated connected SoftEther profile.

The default continuation prompt is:

> Continue

Edit it with **Continuation prompt** in the sidebar, or set a session override in its inspector. Custom prompts survive updates.

### Retry timing and per-session progress

The **30-second Orca delay is a minimum after a confirmed continuation receipt**, not a fixed interval after an error. Outside sessions do not have this delay. Discovery/polling, other busy work, relay failures, profile cleanup, and public-IP checks can extend the gap. OpenCode's terminal duration labels are not a recovery event timeline.

Each affected row shows its recovery stage alongside its OpenCode activity: queued, waiting for work, connecting, verifying IP, waiting for idle, sending a prompt, prompt sent, or stopped/uncertain. Active connection and delivery stages use a small spinner; waiting stages use a static clock. Reduced motion keeps the stage text without continuous rotation. Only sessions in that recovery show progress; unrelated sessions retain their normal activity status.

The session inspector's **Recovery** section shows the controller's current explanation. Open **Attempts and timing** for the verified IP-change count, stage start time, and last confirmed prompt time. A cooldown countdown says **Eligible in …s**, then **Awaiting next check**; it does not promise an immediate retry at zero. Relay details show the current candidate number and connection timeout. **Prompt sent** confirms a receipt, not that the provider accepted the next model call.

The automatic limit is two verified IP changes per session. There is no timed reset, and the count survives app restarts. **Recover session** explicitly queues another manual attempt; it still waits for busy work and verifies a changed IP. Pausing monitoring or excluding queued work stops its progress indicator. Ambiguous delivery stays visible for inspection instead of triggering an automatic resend.

**Stopped · 2-IP limit** means automatic recovery has stopped; it is not still retrying. The inspector explains this and labels the manual action **Retry manually**. A session can remain **Rate limited** while its recovery is **Stopped**, because the provider failure and the app's recovery process are different states. Unchecked sessions show **Not monitored** in the recovery column.

## Relays and history

**VPN relays** refreshes every five minutes and on **Refresh list**. Sort by recommendation, ping, or line speed. **Prioritize this relay** makes a selected relay the next first candidate; **Use automatic selection** clears the preference. Public relays are volunteer operated; advertised speed and ping do not guarantee availability.

**Activity** records connection changes, prompt receipts, and failures. Review it when a recovery stops or delivery is uncertain.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| No idle or background sessions | Use OpenCode v2 with CLI plugin support. Reopen the missing client once after starting Recovery; ensure local plugins are allowed. Saved session history is intentionally absent. |
| An Orca terminal is unavailable | Check that `orca` works on `PATH` and the OpenCode presence plugin loaded. Unknown execution status blocks VPN changes. |
| SoftEther is unavailable | Check the installed service, default install location, `vpncmd`, and enabled adapter named `VPN`. The VPN Gate plug-in alone is not the VPN service. |
| SoftEther error code 2 or connection timeout | A relay can fail despite its rank. Recovery cleans up its profile and tries another candidate, up to five per attempt. |
| Recovery waits despite excluded sessions | Working sessions still protect the machine's network connection. Wait for them to become idle. |
| IP changed but the provider still rejects work | Review Activity and the original provider error. IPv4 verification does not verify IPv6 egress; changing IP does not guarantee the provider will accept the request. |
| Prompt delivery is uncertain | Inspect the affected OpenCode session. The app avoids an automatic retry that could duplicate work. |
| A finished turn is never continued | Worker Idle detection needs the live OpenCode CLI plugin. The shared-service fallback reports rate limits, busy, and idle only, and never continues a session automatically. |

Leases refresh every two seconds and expire after eight seconds. Closed tabs disappear; already observed recovery incidents can remain labeled **Closed tab**, with no delivery route until reopened. Connected private-service clients work through their live plugin; arbitrary private endpoints without a live client are not discovered.

## Local data and removal

| Location | Contents |
| --- | --- |
| `%APPDATA%\opc-rotate\settings.json` | Preferences, monitoring rules, prompts, history, and incident guards |
| `%USERPROFILE%\.config\opencode\plugins\opc-rotate-presence\tui.js` | App-owned OpenCode CLI plugin; under `OPENCODE_CONFIG_DIR` instead when configured |
| `%USERPROFILE%\.local\state\opc-rotate\presence` | Live tab IDs, titles, project paths, status, service PID, and timestamps; no credentials or prompt text |
| Sibling `requests` and `receipts` folders | Short-lived commands, including continuation text when needed, and durable delivery claims/receipts |

Requests are removed on claim or timeout. OpenCode service credentials are read locally for API access and are not copied into the presence channel. Network requests include VPN Gate relay data, ipify's public IP check, and your selected VPN server. Public relay operators carry the machine's routed traffic.

To remove Recovery, first disconnect its VPN through the dashboard, quit Recovery, and uninstall it in Windows Settings. The app-owned plugin and local state may remain. If you want to remove those too, delete only the locations listed above after quitting Recovery and OpenCode. Keep other plugins and SoftEther profiles. SoftEther is installed separately and is not removed by Recovery's uninstaller.

## Preview limitations

- The new exact-session continuation transport has automated coverage but still needs a live recovery check for Orca, outside clients, and background tabs. Earlier live recovery used the previous terminal-send transport.
- The **Worker Idle** watcher has automated coverage and a browser preview check, but no live check against a real OpenCode client yet.
- The shared-service fallback never reports a finished turn and never continues a session automatically. Worker Idle recovery only happens for sessions reported by a live CLI client.
- A clean-PC installation and upgrade/uninstall smoke test remain pending.
- The installer is currently unsigned; Windows may show an unknown-publisher warning. No automatic updater is configured.
- Egress verification covers IPv4 only. A provider using IPv6 outside the VPN may still reject requests.
- Without the presence plugin, shared-service fallback sees responding sessions, not idle clients. A response that starts and fails entirely between polls may be missed.
