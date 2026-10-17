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
3. Check the **Orca sessions** and **Other sessions** project lists. Every live tab appears beneath its project folder, including idle and background tabs.
4. Choose which work to recover. All sessions are included initially; use the row checkboxes or **Monitor this terminal** in the inspector. A terminal rule includes new tabs; individual session choices override it. Choices are saved.
5. Turn on **Monitoring** when ready. It starts off on a fresh install and restores your saved preference on subsequent launches.

**Idle** means the session is live but not responding. **Working** means it is responding. **Needs recovery** means a qualifying provider failure was observed. **Status unavailable** means execution status could not be established. **Selected tab** describes the tab shown in its terminal, independently of whether it is working.

Unchecked sessions stay visible as **Not monitored**. They cannot trigger recovery or receive its prompts. Unchecking removes queued work; a prompt already submitted cannot be recalled. Pausing Monitoring clears pending continuations without changing your selections. **Recover session** is a manual action for included sessions and works while automatic monitoring is paused.

## Recovery sequence

Recovery recognizes failed, idle provider events containing a rate-limit/quota error, or the specific message:

> OpenCode's free tier can only be used from within OpenCode.

The latter match accepts wrapped whitespace and curly apostrophes, regardless of HTTP status. Ordinary permission errors and assistant prose are excluded.

1. Wait for other detected work to become idle. Busy sessions block VPN changes even if you excluded them, because VPN routing affects the whole machine. Unknown execution status also blocks changes.
2. Rank eligible VPN Gate relays using low ping and high advertised speed. Try at most **five relays**, allowing **20 seconds** for each connection. Disconnect and remove failed app-created profiles before trying the next.
3. Require both a connected SoftEther account and a changed **public IPv4** address, checked through ipify.
4. Send the continuation to the exact affected session ID through its live CLI plugin, preserving the currently selected tab. Busy targets wait on the verified connection instead of causing another rotation.
5. Stop if the error persists after **two verified IP changes** for that session. Record the result in **Activity**.

Request claims, SDK receipts, and a persisted incident guard prevent automatic duplicate submissions. An ambiguous submission is not retried automatically; inspect the session before taking another action. The app touches only profiles named with its `OPCRotate_` prefix and refuses to replace an unrelated connected SoftEther profile.

The default continuation prompt is:

> Continue from where you stopped after the rate limit

Edit it with **Continuation prompt** in the sidebar, or set a session override in its inspector. Custom prompts survive updates.

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
- A clean-PC installation and upgrade/uninstall smoke test remain pending.
- The installer is currently unsigned; Windows may show an unknown-publisher warning. No automatic updater is configured.
- Egress verification covers IPv4 only. A provider using IPv6 outside the VPN may still reject requests.
- Without the presence plugin, shared-service fallback sees responding sessions, not idle clients. A response that starts and fails entirely between polls may be missed.
