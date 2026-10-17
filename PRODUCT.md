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

- Separate views for Orca terminals grouped by project and live outside OpenCode sessions.
- Show VPN connection state, ranked public relays, manual list refresh, monitoring switch, recovery history, and editable continuation prompts.
- Use only app-created SoftEther connection profiles; do not alter unrelated profiles.
- Recover ordinary OpenCode sessions through the shared local service. Private standalone servers are outside the initial release.
- Stop after bounded connection and rate-limit retries.

## Evidence on Hand

The user supplied screenshots of an OpenCode rate-limit error, the VPN Gate relay list, and SoftEther connection error code 2. The user confirmed that changing VPN IP usually restores the session.

## Product Principles

- Make the current connection and affected sessions easy to scan.
- Never send a duplicate continuation prompt after an ambiguous submission.
- Prefer a verified new public IP before resuming work.
- Preserve existing agent sessions and unrelated VPN profiles.
