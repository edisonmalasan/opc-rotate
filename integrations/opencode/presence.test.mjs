import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import plugin from "./tui.js";

test("reports the current route, follows changes, and removes presence on disposal", async (t) => {
  const directory = mkdtempSync(join(tmpdir(), "opc-presence-"));
  const previous = process.env.OPC_ROTATE_PRESENCE_DIR;
  process.env.OPC_ROTATE_PRESENCE_DIR = directory;
  t.after(() => {
    if (previous === undefined) delete process.env.OPC_ROTATE_PRESENCE_DIR;
    else process.env.OPC_ROTATE_PRESENCE_DIR = previous;
    rmSync(directory, { recursive: true, force: true });
  });
  let tick;
  t.mock.method(globalThis, "setInterval", (callback) => {
    tick = callback;
    return { unref() {} };
  });
  t.mock.method(globalThis, "clearInterval", () => {});
  let route = { type: "session", sessionID: "ses_open" };
  const dispose = await plugin.setup({
    ui: { router: { current: () => route } },
    client: { server: { info: async () => ({ data: { pid: 42 } }) } },
  });
  const file = join(directory, `${process.pid}.json`);
  const read = () => JSON.parse(readFileSync(file, "utf8"));
  assert.equal(read().sessionID, "ses_open");
  assert.equal(read().servicePid, 42);
  assert.deepEqual(Object.keys(read()).sort(), [
    "orca",
    "pid",
    "servicePid",
    "sessionID",
    "updatedAt",
    "version",
  ]);
  route = { type: "session", sessionID: "ses_switched" };
  tick();
  await new Promise(setImmediate);
  assert.equal(read().sessionID, "ses_switched");
  route = { type: "home" };
  tick();
  await new Promise(setImmediate);
  assert.equal(read().sessionID, null);
  dispose();
  assert.deepEqual(readdirSync(directory), []);
  tick();
  await new Promise(setImmediate);
  assert.deepEqual(readdirSync(directory), []);
});

test("an OpenCode startup probe has no side effects", async () => {
  assert.equal(await plugin.setup(undefined), undefined);
});
