# Releasing OpenCode Recovery

The current version is **0.1.0**, intended as an early Windows x64 preview. Preparing an installer or passing CI does not verify a live VPN recovery. Keep the [known limitations](USAGE.md#preview-limitations) visible in release notes.

## Build a candidate

From a Windows x64 checkout with the [development prerequisites](DEVELOPMENT.md):

```powershell
npm ci
npm run check:release
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm test
npm audit --audit-level=high
npm run release:build
```

`release:build` checks version consistency, runs Tauri's production NSIS build with the Cargo lockfile enforced, rejects stale installer output, checks that development fixtures are absent from the web bundle, and writes:

```text
release-artifacts/v0.1.0/
  OpenCode Recovery_0.1.0_x64-setup.exe
  SHA256SUMS.txt
```

This folder is ignored by Git. The original installer remains under `src-tauri/target/release/bundle/nsis/`. SoftEther is not bundled. There is no configured signing certificate or auto-updater; do not describe the package as signed or automatically updated.

Verify the downloaded installer in PowerShell and compare it with `SHA256SUMS.txt`:

```powershell
Get-FileHash '.\OpenCode Recovery_0.1.0_x64-setup.exe' -Algorithm SHA256
```

A checksum detects a different file; it does not replace publisher signing.

## CI artifacts

The [Windows release check workflow](https://github.com/edisonmalasan/opc-rotate/actions/workflows/windows.yml) runs on pull requests, pushes to `main`, and manual dispatch. It installs the pinned toolchains and locked dependencies, checks formatting and versions, runs tests and the npm audit, then builds the NSIS installer and uploads **opencode-recovery-windows-x64** for 14 days. The artifact includes the version folder and checksum file.

CI has read-only repository permissions and **does not publish a GitHub Release or push a tag**. Review the successful run for the intended commit before downloading its artifact. Local and CI builds can have different hashes; publish the checksum paired with the exact installer you distribute.

## Before public publication

- [ ] Verify the exact commit's Windows CI run succeeded.
- [ ] Install on a clean Windows x64 machine with the documented prerequisites; check WebView2 setup, launch, upgrade, and uninstall.
- [ ] Verify both Orca and outside clients, including idle and background tabs, against the OpenCode version named in the notes.
- [ ] During an authorized live test, verify failed-relay cleanup, changed IPv4, exact-session continuation, and a single receipt. Test a background tab without changing the selected tab. Current automated coverage is not a substitute for this check.
- [ ] Review signing status and known limitations; label the release **Pre-release** while these preview checks remain open.
- [ ] Review [CHANGELOG.md](../CHANGELOG.md), setup instructions, and the dependency license notices accompanying the software you distribute.
- [ ] Attach the tested installer and its matching `SHA256SUMS.txt` to the GitHub release.

The repository is prepared for review before these manual checks; unchecked items remain release acceptance gaps. Publishing is a separate maintainer action.

## Changing the version

Update `package.json`, `package-lock.json` (both root version fields), `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` (the root `opc-rotate` package), and `src-tauri/tauri.conf.json` together. Run `npm run check:release`, update the changelog and version examples, then rebuild. Keep each logical change in its own conventional commit; push requested commits separately to `main`.
