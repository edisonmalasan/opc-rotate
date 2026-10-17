import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const root = new URL("../", import.meta.url);
const read = (path) => readFile(new URL(path, root), "utf8");
const json = async (path) => JSON.parse(await read(path));
const pkg = await json("package.json");
const lock = await json("package-lock.json");
const tauri = await json("src-tauri/tauri.conf.json");
const cargo = await read("src-tauri/Cargo.toml");
const cargoLock = await read("src-tauri/Cargo.lock");
const cargoVersion = cargo.match(/^\[package\][\s\S]*?^version = "([^"]+)"/m)?.[1];
const cargoLockVersion = cargoLock.match(/^name = "opc-rotate"\r?\nversion = "([^"]+)"/m)?.[1];

assert.match(pkg.version, /^\d+\.\d+\.\d+(?:-[\da-z.-]+)?$/, "Invalid release version");
for (const [name, version] of Object.entries({
  "package-lock.json": lock.version,
  "package-lock root package": lock.packages[""].version,
  "tauri.conf.json": tauri.version,
  "Cargo.toml": cargoVersion,
  "Cargo.lock": cargoLockVersion,
})) {
  assert.equal(version, pkg.version, `${name} must match package.json`);
}
assert.deepEqual(tauri.bundle.targets, ["nsis"], "Release packaging expects NSIS");
console.log(`Release versions agree: ${pkg.version}`);

if (process.argv.includes("--assets")) {
  const dist = new URL("dist/", root);
  const files = await readdir(dist, { recursive: true, withFileTypes: true });
  let checked = 0;
  for (const file of files) {
    if (!file.isFile() || !/\.(?:js|html|css|map)$/.test(file.name)) continue;
    const content = await readFile(`${file.parentPath}/${file.name}`, "utf8");
    for (const marker of ["preview-session-", "ses_preview_", "preview-relay-", "vpn723480100.opengw.net"]) {
      assert.ok(!content.includes(marker), `Development fixture found in ${file.name}: ${marker}`);
    }
    checked += 1;
  }
  assert.ok(checked > 0, `No built assets found in ${fileURLToPath(dist)}`);
  console.log(`Checked ${checked} production assets: no preview fixtures`);
}
