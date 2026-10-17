# Contributing

Read the [development guide](docs/DEVELOPMENT.md) and [source map](docs/CONTEXT.md). Use small conventional commits and explain the behavior changed, how you checked it, and any remaining limitations.

Before a pull request, run `npm run check:release`, `npm test`, `npm run build`, and `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`. Use recorded responses or mocks for VPN behavior. A live VPN switch affects other work on the machine; run one only when explicitly intended.

Keep existing sessions open. Use the app's exact-session transport, preserve uncertain-delivery protection, and touch only `OPCRotate_` SoftEther profiles. Never commit local settings, service passwords, signing keys, prompts from real work, build output, or dependencies.

For frontend changes, use the labeled development preview and check the desktop minimum of 980 × 660 as well as 1280 × 820. Preserve project grouping, independent session activity/selection states, keyboard access, reduced motion, and readable contrast.

Report bugs in [GitHub Issues](https://github.com/edisonmalasan/opc-rotate/issues), with the app version, Windows/OpenCode versions, steps to reproduce, and relevant redacted Activity entries. Remove credentials, private paths, session contents, and public IPs from logs or screenshots before sharing. For sensitive reports, see [SECURITY.md](SECURITY.md).
