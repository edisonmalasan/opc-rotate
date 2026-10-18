# Changelog

## 0.1.0 — Preview candidate

Initial Windows x64 release candidate; a public GitHub release has not yet been published.

### Included

- Project-grouped live OpenCode tabs for Orca and other terminals, with independent activity and selected/background status.
- Saved monitoring choices per session or terminal, prompt overrides, relay preferences, and connection activity.
- Per-session recovery stages with active-work spinners, retry-eligibility countdowns, relay-attempt details, confirmed prompt times, and visible stop/uncertain states.
- VPN Gate relay ranking and bounded SoftEther connection attempts with changed-public-IPv4 verification.
- Exact-session continuation requests, claimed receipts, uncertain-delivery protection, and a two-IP recovery limit.
- Detection of rate-limit/quota failures and the specific OpenCode free-tier client-gate error.
- Neutral charcoal desktop interface, keyboard navigation, hidden background CLI helpers, and development port fallback.
- Reproducible release commands, version/fixture checks, Windows CI artifacts, installer checksums, and MIT licensing.

### Known gaps

- Live recovery with the new exact-session transport still needs verification for Orca, outside clients, and background tabs.
- Clean-machine installation, upgrade, and uninstall checks are pending.
- The installer is unsigned and has no automatic updater. SoftEther must be installed separately.
- IPv6 egress and arbitrary private endpoints without a live OpenCode client are not supported by verification/discovery.

See [usage and limitations](docs/USAGE.md) and the [release checklist](docs/RELEASING.md).
