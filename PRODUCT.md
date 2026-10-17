# Product

<!-- impeccable:product-schema 1 -->

## Platform

Windows desktop

## Stack

Tauri 2, React, TypeScript, and Rust. The app is packaged for Windows.

## Users

The primary user runs several OpenCode sessions across Orca projects and ordinary Windows terminals.

## Product Purpose

Recover sessions that stop on an IP-sensitive rate limit by selecting a working VPN Gate relay through SoftEther and sending a continuation prompt to affected sessions.

## Operating Context

The app runs beside existing Orca and OpenCode sessions. SoftEther VPN Client and its VPN Gate plug-in are installed. VPN routing affects the whole machine, so connection changes need to wait for active agent work to become idle.

## Capabilities and Constraints

- Separate Orca and outside OpenCode session lists, with open session tabs directly beneath folder-marked project headings. Omit redundant terminal headings and counts. Use live client presence rather than saved history. Show Idle, Working, Needs recovery, or unavailable status independently of which tab is selected.
- Show VPN connection state, ranked public relays, manual list refresh, monitoring switch, recovery history, and editable continuation prompts.
- Choose monitored sessions with row checkboxes or whole terminals in the inspector. Save selections, apply terminal choices to new tabs, and support individual overrides. Unchecked work stays visible and cannot trigger recovery or receive prompts. Busy work in any session still delays VPN changes.
- Use only app-created SoftEther connection profiles; do not alter unrelated profiles.
- Recover exact OpenCode sessions through their live CLI plugin's connected SDK, including connected private services. Keep the shared local service as a fallback for responding sessions. Arbitrary private-server connection setup remains outside this release.
- Stop after bounded connection and rate-limit retries.
- Recognize the specific OpenCode free-tier client-gate provider error as a recovery trigger for monitored sessions, with the same bounded retry and verified-IP rules.

## Evidence on Hand

The user supplied screenshots of an OpenCode rate-limit error, the VPN Gate relay list, and SoftEther connection error code 2. The user confirmed that changing VPN IP usually restores the session.

## Product Principles

- Make the current connection and affected sessions easy to scan.
- Never send a duplicate continuation prompt after an ambiguous submission.
- Prefer a verified new public IP before resuming work.
- Preserve existing agent sessions and unrelated VPN profiles.

## Visual Direction

Use the existing lime-green recovery icon with dark forest surfaces, green selection states, and lime action/focus accents. Keep amber for errors and rate limits. Preserve the compact desktop layout, readable type, project folder headings, and restrained motion.
