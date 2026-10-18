# Development

## Toolchain

Use Windows x64 with [Tauri's Windows prerequisites](https://v2.tauri.app/start/prerequisites/), including Visual Studio C++ build tools and Microsoft WebView2. The tested toolchain is pinned in `.node-version` (**Node 26.10.0**) and `rust-toolchain.toml` (**Rust 1.99.0**, MSVC host, rustfmt). Rustup reads the toolchain file automatically.

```powershell
git clone https://github.com/edisonmalasan/opc-rotate.git
cd opc-rotate
npm ci
npm run tauri -- dev
```

The desktop dev wrapper selects an available loopback port from **1420–1499**, then passes the same URL to Vite and Tauri. It leaves any process already occupying a port alone. `npm run dev` starts the frontend only and also falls back when its port is busy.

SoftEther/OpenCode/Orca are needed for live integration, not unit tests or building the installer. Running the desktop app installs its app-owned OpenCode plugin; browser preview does not.

## Frontend preview

Run `npm run dev`, then append `?preview=1` to Vite's printed URL. Add `&empty=1` to check empty states. The preview is clearly labeled, uses in-memory fixtures, and disables recovery, disconnection, and live refresh. It never invokes the desktop controller. Fixtures are excluded from production builds; normal browser access asks for the desktop app.

Add `&recovery=1` to inspect sample connecting, cooldown, sent, and stopped session indicators. These are presentation fixtures; they never rotate a VPN or submit a prompt.

## Verification

```powershell
npm run check:release
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm test
npm run build
node scripts/check-release.mjs --assets
npm audit --audit-level=high
```

`npm test` runs the Node presence-plugin suite and locked Rust unit/mock HTTP tests. Mocked VPN tests do not change the network. The Windows background-process regression test checks that a helper can capture output without owning a console window.

For an installer plus checksums, use `npm run release:build`; see [Releasing](RELEASING.md). Builds use the committed npm and Cargo lockfiles. If `npm ci` reports a locked `esbuild.exe`, stop the repository's running Vite dev server before retrying.

Live VPN checks affect the machine's routing. Preserve existing terminals and monitoring preferences; do not enable monitoring or send continuation prompts just to inspect the UI. Keep real service credentials, local settings, and prompt text out of commits and test fixtures.

## Project references

- [Source map and current status](CONTEXT.md)
- [Verified progress and pending checks](ROADMAP.md)
- [Agent guide](../AGENTS.md)
- [Contributing](../CONTRIBUTING.md)
