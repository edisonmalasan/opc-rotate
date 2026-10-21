import assert from "node:assert/strict";
import {
  mkdtempSync,
  readFileSync,
  writeFileSync,
  readdirSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
import { test } from "node:test";
import plugin, { finishedTurn, operatorMessageID } from "./tui.js";

async function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "opc-tabs-"));
  const previous = process.env.OPC_ROTATE_PRESENCE_DIR;
  process.env.OPC_ROTATE_PRESENCE_DIR = join(root, "presence");
  const timers = new Map();
  t.mock.method(globalThis, "setInterval", (callback, interval) => {
    timers.set(interval, callback);
    return { unref() {} };
  });
  t.mock.method(globalThis, "clearInterval", () => {});
  let route = { type: "session", sessionID: "ses_one" };
  let ids = ["ses_one", "ses_two", "ses_failed"];
  let active = { ses_two: { type: "running" } };
  let unavailable = false;
  const prompts = [];
  const context = {
    ui: {
      router: {
        current: () => route,
        navigate: (value) => {
          route = value;
        },
      },
      tabs: {
        enabled: () => true,
        list: () => ids.map((sessionID) => ({ sessionID })),
        focus: (sessionID) => {
          route = { type: "session", sessionID };
        },
      },
    },
    client: {
      server: { info: async () => ({ pid: 42 }) },
      session: {
        active: async () => {
          if (unavailable) throw new Error("offline");
          return active;
        },
        get: async ({ sessionID }) => ({
          id: sessionID,
          title: sessionID,
          location: { directory: "C:/project" },
          time: { updated: 100 },
        }),
        prompt: async (input) => {
          prompts.push(input);
          return { id: "inbox-receipt" };
        },
      },
      message: {
        list: async ({ sessionID }) => ({
          data:
            sessionID === "ses_failed"
              ? [
                  {
                    id: "failure-1",
                    type: "idle",
                    outcome: "failed",
                    error: {
                      type: "provider.quota",
                      status: 429,
                      message: "Rate limit exceeded",
                    },
                  },
                ]
              : [],
        }),
      },
    },
  };
  const dispose = await plugin.setup(context);
  t.after(() => {
    dispose();
    if (previous === undefined) delete process.env.OPC_ROTATE_PRESENCE_DIR;
    else process.env.OPC_ROTATE_PRESENCE_DIR = previous;
    rmSync(root, { recursive: true, force: true });
  });
  const file = join(root, "presence", `${process.pid}.json`);
  const read = () => JSON.parse(readFileSync(file, "utf8"));
  const tick = async (interval) => {
    timers.get(interval)();
    await new Promise(setImmediate);
  };
  async function request(
    sessionID,
    action = "prompt",
    extra = {},
    id = randomUUID(),
  ) {
    const client = read();
    writeFileSync(
      join(root, "requests", client.clientId, `${id}.json`),
      JSON.stringify({
        id,
        clientId: client.clientId,
        servicePid: 42,
        sessionID,
        action,
        prompt: "Continue once",
        expiresAt: Date.now() + 5000,
        ...extra,
      }),
    );
    await tick(250);
    return {
      id,
      reply: JSON.parse(
        readFileSync(
          join(root, "receipts", client.clientId, `${id}.json`),
          "utf8",
        ),
      ),
    };
  }
  return {
    root,
    read,
    tick,
    request,
    prompts,
    dispose,
    context,
    route: () => route,
    setRoute: (value) => {
      route = value;
    },
    setIds: (value) => {
      ids = value;
    },
    setActive: (value) => {
      active = value;
    },
    setUnavailable: (value) => {
      unavailable = value;
    },
  };
}

test("the free-tier client gate is a recovery incident, while ordinary errors and assistant text are not", async (t) => {
  const f = await fixture(t);
  const text =
    "OpenCode's free tier can only be used from within OpenCode. Download it at [https://opencode.ai](https://opencode.ai) and keep building.";
  let latest = {
    id: "gate-1",
    type: "idle",
    outcome: "failed",
    error: { type: "provider.unknown", status: 403, message: text },
  };
  f.context.client.message.list = async ({ sessionID }) => ({
    data: sessionID === "ses_failed" ? [latest] : [],
  });
  await f.tick(2000);
  assert.equal(f.read().sessions[2].state, "rate-limited");
  assert.equal(f.read().sessions[2].lastError, text);
  assert.equal(f.read().sessions[2].errorKey, "api:ses_failed:gate-1");
  latest = {
    ...latest,
    error: {
      message:
        "OpenCode’s free tier can only be\nused from within OpenCode. Download it at https://opencode.ai",
    },
  };
  await f.tick(2000);
  assert.equal(f.read().sessions[2].state, "rate-limited");
  latest = { ...latest, type: "assistant" };
  await f.tick(2000);
  assert.equal(f.read().sessions[2].state, "idle");
  latest = {
    ...latest,
    type: "idle",
    error: { status: 403, message: "Permission denied" },
  };
  await f.tick(2000);
  assert.equal(f.read().sessions[2].state, "idle");
});

test("reports every live tab with independent idle, working, failed, and selected states", async (t) => {
  const f = await fixture(t),
    data = f.read();
  assert.equal(data.version, 2);
  assert.equal(data.sessions.length, 3);
  assert.deepEqual(
    data.sessions.map((tab) => [tab.sessionID, tab.state, tab.selected]),
    [
      ["ses_one", "idle", true],
      ["ses_two", "busy", false],
      ["ses_failed", "rate-limited", false],
    ],
  );
  assert.equal(data.sessions[2].errorKey, "api:ses_failed:failure-1");
  assert.ok(!JSON.stringify(data).includes("Continue once"));
  f.setRoute({ type: "session", sessionID: "ses_two" });
  await f.tick(2000);
  assert.equal(f.read().sessions.length, 3);
  assert.equal(
    f.read().sessions.find((tab) => tab.selected).sessionID,
    "ses_two",
  );
});

test("continues the requested background tab once without changing the selected tab", async (t) => {
  const f = await fixture(t);
  const result = await f.request("ses_failed");
  assert.equal(result.reply.status, "accepted");
  assert.deepEqual(f.prompts, [
    { sessionID: "ses_failed", text: "Continue once" },
  ]);
  assert.equal(f.route().sessionID, "ses_one");
  await f.request("ses_failed", "prompt", {}, result.id);
  assert.equal(f.prompts.length, 1);
});

test("a busy target is deferred without submitting a prompt", async (t) => {
  const f = await fixture(t);
  assert.equal((await f.request("ses_two")).reply.status, "busy");
  assert.equal(f.prompts.length, 0);
});

test("rejects closed, expired, and replacement-client requests", async (t) => {
  const f = await fixture(t);
  for (const [sessionID, extra] of [
    ["ses_closed", {}],
    ["ses_one", { expiresAt: Date.now() - 1 }],
    ["ses_one", { clientId: randomUUID() }],
  ]) {
    assert.equal(
      (await f.request(sessionID, "prompt", extra)).reply.status,
      "refused",
    );
  }
  assert.equal(f.prompts.length, 0);
});







test("the stop key survives a long turn and the turn is reported", async (t) => {
  const f = await fixture(t);
  f.context.client.message.list = async ({ sessionID }) => ({
    data:
      sessionID === "ses_one"
        ? [
            { id: "turn_1", type: "idle", outcome: "succeeded" },
            { id: "msg_operator", type: "user" },
          ]
        : [],
  });
  await f.tick(2000);
  const first = f
    .read()
    .sessions.find((session) => session.sessionID === "ses_one");
  assert.ok(first.stopKey, "a stopped session reports a stop key");
  assert.ok(first.turnKey, "a stopped session reports its turn");

  // This client delivers a prompt, so the message it created is not the
  // operator's own.
  f.context.client.session.prompt = async (input) => {
    f.prompts.push(input);
    return { id: "msg_delivered" };
  };
  await f.request("ses_one", "prompt");

  // The next turn ends and the operator's own message is no longer in the
  // recent list. The key must not disappear with it.
  f.context.client.message.list = async ({ sessionID }) => ({
    data:
      sessionID === "ses_one"
        ? [
            { id: "turn_2", type: "idle", outcome: "succeeded" },
            { id: "msg_delivered", type: "user" },
          ]
        : [],
  });
  await f.tick(2000);
  const second = f
    .read()
    .sessions.find((session) => session.sessionID === "ses_one");
  assert.equal(second.stopKey, first.stopKey);
  assert.notEqual(second.turnKey, first.turnKey);
});

test("focusing one tab preserves the other open tabs", async (t) => {
  const f = await fixture(t);
  assert.equal(
    (await f.request("ses_failed", "focus")).reply.status,
    "accepted",
  );
  assert.equal(f.route().sessionID, "ses_failed");
  await f.tick(2000);
  assert.equal(f.read().sessions.length, 3);
});

test("closed tabs disappear; disconnected services remain live with unknown status", async (t) => {
  const f = await fixture(t);
  f.setIds(["ses_one"]);
  await f.tick(2000);
  assert.equal(f.read().sessions.length, 1);
  f.setUnavailable(true);
  await f.tick(2000);
  assert.equal(f.read().statusKnown, false);
  assert.equal(f.read().sessions[0].state, "unknown");
  assert.equal((await f.request("ses_one")).reply.status, "refused");
  assert.equal(f.prompts.length, 0);
});

test("uncertain prompt receipts are claimed and never resubmitted", async (t) => {
  const f = await fixture(t);
  f.context.client.session.prompt = async (input) => {
    f.prompts.push(input);
    throw new Error("Response lost");
  };
  const first = await f.request("ses_one");
  assert.equal(first.reply.status, "rejected");
  await f.request("ses_one", "prompt", {}, first.id);
  assert.equal(f.prompts.length, 1);
});

test("disposal removes presence and startup probes have no side effects", async (t) => {
  const f = await fixture(t);
  f.dispose();
  assert.deepEqual(readdirSync(join(f.root, "presence")), []);
  await f.tick(2000);
  assert.deepEqual(readdirSync(join(f.root, "presence")), []);
  assert.equal(await plugin.setup(undefined), undefined);
});

test("only a cleanly finished turn counts as a finished turn", () => {
  assert.equal(finishedTurn({ type: "idle", outcome: "succeeded" }), true);
  assert.equal(finishedTurn({ type: "idle", outcome: "failed" }), false);
  assert.equal(finishedTurn({ type: "idle", outcome: "interrupted" }), false);
  assert.equal(finishedTurn({ type: "assistant" }), false);
  assert.equal(finishedTurn(undefined), false);
});

test("the stop key is the newest operator message, never the app's own continuation", () => {
  const messages = [
    { id: "app-1", type: "user" },
    { id: "operator-2", type: "user" },
    { id: "assistant-3", type: "assistant" },
  ];
  // The newest message wins when it was written by the operator.
  assert.equal(operatorMessageID(messages, new Set()), "app-1");
  // The app's own continuation prompt is skipped, so the operator's message
  // is used as the stop key.
  assert.equal(operatorMessageID(messages, new Set(["app-1"])), "operator-2");
  // The wrapped list shape the client returns is accepted as well.
  assert.equal(
    operatorMessageID({ data: messages }, new Set(["app-1"])),
    "operator-2",
  );
  // Without any operator message there is no stop key.
  assert.equal(operatorMessageID([], new Set()), null);
  assert.equal(
    operatorMessageID([{ id: "assistant-3", type: "assistant" }], new Set()),
    null,
  );
  assert.equal(
    operatorMessageID([{ id: "app-1", type: "user" }], new Set(["app-1"])),
    null,
  );
});
