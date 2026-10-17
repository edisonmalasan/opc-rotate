# OpenCode Recovery

A Windows desktop dashboard for recovering OpenCode sessions after an IP-sensitive rate limit. It ranks public VPN Gate relays, connects through the installed SoftEther VPN Client, verifies that the public IP changed, and sends a continuation prompt to affected sessions.

## Requirements

- Windows with SoftEther VPN Client and an enabled virtual adapter named `VPN`.
- The VPN Gate Client plug-in is useful for manual inspection, but the app reads VPN Gate's public CSV itself.
- OpenCode v2 using its default shared background service for sessions outside Orca.
- Orca CLI available as `orca` for Orca terminals.
- Node.js and Rust to build from source.

## Run

```powershell
npm install
npm run tauri -- dev
```

The dev command selects the first available loopback port from 1420 to 1499 and gives the same URL to Vite and Tauri. If another process is using 1420, the app starts on the next free port without stopping that process. Use the npm command above for this coordinated fallback. `npm run dev` starts only Vite and also falls back to the next available port.

To build a Windows installer:

```powershell
npm run tauri -- build
```

The installer is written under `src-tauri/target/release/bundle/nsis/`.

## How it works

The sidebar separates **Sessions**, **VPN relays**, and **Activity**. Orca projects have their own sidebar entries and stay grouped in the session list; **Other sessions** shows OpenCode work from the shared service. Search or filter the list, then select a session to inspect its error, edit its continuation prompt, or choose **Recover session**. Up and Down arrow keys move between session rows.

Monitoring starts **off**. Turn on the **Monitoring** switch in the toolbar when ready. The compact route bar stays visible across all views. In **VPN relays**, use **Refresh list** to fetch servers immediately, sort by recommendation, ping, or line speed, then select a relay to review its details. **Prioritize this relay** makes it the first candidate for the next recovery; **Use automatic selection** removes that preference. The list also refreshes automatically every five minutes. **Activity** records connection changes, prompt receipts, and errors.

When a session reports a rate-limit error, the app waits for other shared-service OpenCode calls and Orca OpenCode terminals to become idle. It tries up to five relays in ranked order, allowing 20 seconds for each connection to establish. A failed or timed-out SoftEther profile is disconnected, deleted, and checked for removal before the next server. Once SoftEther reports a connected account, the app checks its public IPv4 address through ipify. Only a changed IP allows continuation prompts to be sent.

Orca terminals are read and prompted through the Orca CLI and shown under their project paths. Sessions outside Orca are read and prompted through OpenCode's shared local API. The outside list fetches only sessions currently responding, plus a rate-limited session the app observed responding before it failed. Historical shared-service sessions are not fetched. An OpenCode session in the same project directory as an active Orca OpenCode terminal is handled by the Orca section to avoid duplicate prompts.

The sidebar's **Continuation prompt** action edits the default message. The session inspector's **Edit** action sets an override for that session. If an affected session is busy after the IP changes, its continuation waits for that session to become idle; it does not rotate the VPN again for that queued prompt. A prompt with an ambiguous delivery result is not resent automatically. If a new rate-limit error appears after two verified IP changes for the same session, recovery stops and Activity records the failure. **Recover session** can be used manually on any listed session.

Only profiles created with this app's `OPCRotate_` prefix are disconnected or deleted. Incomplete profiles from an interrupted attempt are removed before recovery. If another SoftEther profile is currently connected, automatic recovery stops rather than changing that connection.

## Verification

```powershell
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri -- build
```

The automated tests use simulated SoftEther responses and never change the VPN. To validate live recovery, enable monitoring during a real rate-limit event and confirm the sequence in **Activity**: connection failure or timeout, successful alternate relay, changed public IP, and a single continuation receipt for each affected idle session.

For frontend work, run `npm run dev` and append `?preview=1` to the URL printed by Vite. This explicitly labeled development preview supplies sample sessions, relays, and events. Prompt, preference, and monitoring edits stay in memory; recovery, disconnection, and live refresh are disabled. Append `&empty=1` to inspect empty states. Preview fixtures are excluded from production builds.

## Current limits

- Private `opencode --standalone` servers are not discovered.
- The app checks IPv4 egress. If a provider request uses IPv6 outside the VPN, a changed IPv4 address alone may not restore access.
- Public VPN Gate relays are volunteer operated; advertised speed and ping are estimates. A connection may fail despite a high rank.
- The outside-session monitor polls the active endpoint. A very short response that starts and fails entirely between polls may not be observed.
