import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
function run(script, args = []) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [script, ...args], { cwd: root, stdio: "inherit" });
    child.once("error", reject);
    child.once("exit", (code) => code === 0 ? resolve() : reject(new Error(`${script} exited with ${code}`)));
  });
}

assert.equal(process.platform, "win32", "Build the Windows release on Windows");
assert.equal(process.arch, "x64", "This release supports Windows x64 only");
await run("scripts/check-release.mjs");
const { version } = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const installerName = `OpenCode Recovery_${version}_x64-setup.exe`;
const installer = new URL(`../src-tauri/target/release/bundle/nsis/${installerName}`, import.meta.url);
const startedAt = Date.now();
await run("scripts/tauri.mjs", ["build", "--ci", "--", "--locked"]);
assert.ok((await stat(installer)).mtimeMs >= startedAt - 2000, "Installer was not rebuilt; refusing stale output");
await run("scripts/check-release.mjs", ["--assets"]);

const output = new URL(`../release-artifacts/v${version}/`, import.meta.url);
await mkdir(output, { recursive: true });
await copyFile(installer, new URL(installerName, output));
const digest = createHash("sha256").update(await readFile(installer)).digest("hex");
await writeFile(new URL("SHA256SUMS.txt", output), `${digest}  ${installerName}\n`);
console.log(`Release files: ${fileURLToPath(output)}\nSHA-256: ${digest}`);
