// Runs only in a live OpenCode CLI. Presence has no prompts, tokens, or API passwords.
import {
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  writeFileSync,
  unlinkSync,
  existsSync,
  statSync,
} from "node:fs";
import { randomUUID } from "node:crypto";
import { homedir } from "node:os";
import { join, dirname } from "node:path";

const payload = (value) => value?.data ?? value;
const validID = (id) =>
  typeof id === "string" && /^[A-Za-z0-9_-]{1,128}$/.test(id);
const validUUID = (id) => typeof id === "string" && /^[0-9a-f-]{36}$/.test(id);
const options = () => ({ signal: AbortSignal.timeout(3000) });

function openTabs(context) {
  const route = context.ui.router.current();
  const current = route?.type === "session" ? route.sessionID : null;
  const tabs = context.ui.tabs?.enabled?.() ? context.ui.tabs.list() : [];
  const ids = [
    ...new Set(
      [
        ...tabs.map((tab) =>
          typeof tab === "string" ? tab : (tab.sessionID ?? tab.id),
        ),
        current,
      ].filter(validID),
    ),
  ];
  return { ids: ids.slice(0, 128), current };
}

function recoveryError(message) {
  const error = message?.error;
  if (message?.type !== "idle" || message.outcome !== "failed" || !error)
    return null;
  const text = String(error.message ?? "")
    .replaceAll("’", "'")
    .replace(/\s+/g, " ")
    .toLowerCase();
  const freeTierGate = text.includes(
    "opencode's free tier can only be used from within opencode.",
  );
  return freeTierGate ||
    (error.status === 429 &&
      (error.type === "provider.rate-limit" ||
        error.type === "provider.quota" ||
        /rate limit/i.test(text)))
    ? String(error.message || "Rate limit exceeded").slice(0, 2000)
    : null;
}

// A turn that finished cleanly: the agent is done and waiting for the operator.
export function finishedTurn(message) {
  return message?.type === "idle" && message?.outcome === "succeeded";
}

// The newest user message that the operator wrote, skipping any prompt
// this client delivered. Pure: the message list and the app-created ids are
// passed in, so the same rule can be checked without a live client.
export function operatorMessageID(messages, appMessageIds) {
  const items = Array.isArray(messages) ? messages : (messages?.data ?? []);
  for (const message of items) {
    if (message?.type !== "user") continue;
    if (appMessageIds.has(message?.id)) continue;
    return typeof message.id === "string" ? message.id : null;
  }
  return null;
}

export default {
  id: "opc-rotate.presence",
  async setup(context) {
    if (typeof context?.ui?.router?.current !== "function") return;
    const directory =
      process.env.OPC_ROTATE_PRESENCE_DIR ||
      join(homedir(), ".local", "state", "opc-rotate", "presence");
    const clientId = randomUUID();
    const requests = join(dirname(directory), "requests", clientId);
    const receipts = join(dirname(directory), "receipts", clientId);
    mkdirSync(requests, { recursive: true });
    mkdirSync(receipts, { recursive: true });
    const file = join(directory, `${process.pid}.json`);
    const temporary = `${file}.tmp`;
    let disposed = false,
      writing = false,
      processing = false,
      servicePid = 0;
    let previousSessions = [];
    // Message ids this client created, so an automatic continuation is never
    // mistaken for the operator typing something new.
    const appMessages = new Map();
    // The last operator message seen for a session, kept per finished turn.
    const stopKeys = new Map();
    // The last operator message id seen for a session, kept across turns. A
    // long turn can push the operator's own message out of the recent list,
    // and the key must not disappear with it.
    const operatorKeys = new Map();

    const appMessageIds = (sessionID) => {
      let ids = appMessages.get(sessionID);
      if (!ids) {
        ids = new Set();
        appMessages.set(sessionID, ids);
      }
      return ids;
    };

    async function childSessions(sessionID) {
      try {
        const children = payload(
          await context.client.session.children({ sessionID }, options()),
        );
        return Array.isArray(children) ? children : (children?.data ?? []);
      } catch {
        return [];
      }
    }

    // The newest user message that the operator wrote, skipping any prompt
    // this client delivered.
    async function latestOperatorMessageID(sessionID, skip) {
      try {
        const list = payload(
          await context.client.message.list(
            { sessionID, limit: 100, order: "desc" },
            options(),
          ),
        );
        return operatorMessageID(list, skip);
      } catch {
        /* The stop key is best effort; without it the session is not auto-continued. */
      }
      return null;
    }

    async function publish() {
      if (disposed || writing) return;
      const { ids, current } = openTabs(context);
      writing = true;
      let sessions = previousSessions
        .filter((session) => ids.includes(session.sessionID))
        .map((session) => ({
          ...session,
          state: "unknown",
          selected: session.sessionID === current,
        }));
      let statusKnown = false,
        busySessionIDs = [];
      try {
        const info = payload(await context.client.server.info(options()));
        if (!Number.isInteger(info?.pid) || info.pid <= 0)
          throw new Error("No server identity");
        servicePid = info.pid;
        const active = payload(await context.client.session.active(options()));
        if (!active || typeof active !== "object" || Array.isArray(active))
          throw new Error("No execution status");
        busySessionIDs = Object.keys(active).filter(validID);
        sessions = await Promise.all(
          ids.map(async (sessionID) => {
            try {
              const info = payload(
                await context.client.session.get({ sessionID }, options()),
              );
              if (info?.id !== sessionID) throw new Error("Wrong session");
              const messages = await context.client.message.list(
                { sessionID, limit: 1, order: "desc" },
                options(),
              );
              const latest = (
                Array.isArray(messages) ? messages : messages?.data
              )?.[0];
              const error = active[sessionID] ? null : recoveryError(latest);
              const children = await childSessions(sessionID);
              const workingSubagents = children.filter(
                (child) => child?.id && active[child.id],
              ).length;
              const stopped =
                !active[sessionID] &&
                !error &&
                workingSubagents === 0 &&
                finishedTurn(latest);
              let stopKey = null;
              if (stopped) {
                const cached = stopKeys.get(sessionID);
                if (cached && cached.turn === latest.id) stopKey = cached.key;
                else {
                  stopKey = await latestOperatorMessageID(
                    sessionID,
                    appMessageIds(sessionID),
                  );
                  if (stopKey) operatorKeys.set(sessionID, stopKey);
                  else stopKey = operatorKeys.get(sessionID) ?? null;
                  stopKeys.set(sessionID, { turn: latest.id, key: stopKey });
                }
              } else stopKeys.delete(sessionID);
              return {
                sessionID,
                title: info.title || "Untitled session",
                project: info.location?.directory || "",
                state: active[sessionID]
                  ? "busy"
                  : error
                    ? "rate-limited"
                    : stopped
                      ? "stopped"
                      : "idle",
                selected: sessionID === current,
                lastError: error,
                errorKey:
                  error && latest?.id ? `api:${sessionID}:${latest.id}` : null,
                stopKey: stopKey ? `stop:${sessionID}:${stopKey}` : null,
                turnKey:
                  stopped && latest?.id
                    ? `turn:${sessionID}:${latest.id}`
                    : null,
                workingSubagents,
                updatedAt: info.time?.updated ?? Date.now(),
              };
            } catch {
              const old = previousSessions.find(
                (session) => session.sessionID === sessionID,
              );
              return {
                ...old,
                sessionID,
                title: old?.title || "Loading session…",
                project: old?.project || "",
                selected: sessionID === current,
                state: active[sessionID] ? "busy" : "unknown",
                stopKey: null,
                workingSubagents: old?.workingSubagents ?? 0,
              };
            }
          }),
        );
        statusKnown = true;
      } catch {
        /* Keep a live but unknown heartbeat when the connected service is unavailable. */
      }
      try {
        if (disposed || !servicePid) return;
        previousSessions = sessions;
        mkdirSync(directory, { recursive: true });
        writeFileSync(
          temporary,
          JSON.stringify({
            version: 2,
            clientId,
            pid: process.pid,
            servicePid,
            sessions,
            busySessionIDs,
            statusKnown,
            orca: Boolean(
              process.env.ORCA_AGENT_HOOK_ENDPOINT ||
              process.env.ORCA_AGENT_HOOK_PORT,
            ),
            paneKey: process.env.ORCA_PANE_KEY || null,
            updatedAt: Date.now(),
          }),
        );
        renameSync(temporary, file);
      } finally {
        writing = false;
      }
    }

    async function processRequests() {
      if (disposed || processing) return;
      processing = true;
      try {
        for (const name of readdirSync(requests)
          .filter(
            (name) => validUUID(name.slice(0, -5)) && name.endsWith(".json"),
          )
          .slice(0, 32)) {
          if (disposed) break;
          const path = join(requests, name),
            receiptFile = join(receipts, name);
          let request;
          try {
            if (statSync(path).size > 65536)
              throw new Error("Oversized request");
            request = JSON.parse(readFileSync(path, "utf8"));
          } catch {
            try {
              unlinkSync(path);
            } catch {}
            continue;
          }
          if (existsSync(receiptFile)) {
            try {
              unlinkSync(path);
            } catch {}
            continue;
          }
          // Claim durably before acting: repeated polls and ambiguous responses never resubmit.
          try {
            writeFileSync(receiptFile, JSON.stringify({ status: "started" }), {
              flag: "wx",
            });
          } catch {
            continue;
          }
          try {
            unlinkSync(path);
          } catch {}
          let reply;
          let attempted = false;
          try {
            if (
              request.clientId !== clientId ||
              request.servicePid !== servicePid ||
              !validID(request.sessionID) ||
              !Number.isFinite(request.expiresAt) ||
              Date.now() > request.expiresAt ||
              !openTabs(context).ids.includes(request.sessionID)
            )
              throw new Error(
                "The requested session is no longer open in this client",
              );
            if (request.action === "focus") {
              if (context.ui.tabs?.enabled?.())
                context.ui.tabs.focus(request.sessionID);
              else
                context.ui.router.navigate({
                  type: "session",
                  sessionID: request.sessionID,
                });
              if (context.ui.router.current()?.sessionID !== request.sessionID)
                throw new Error("Tab focus was not confirmed");
              reply = { status: "accepted", receipt: request.id };
            } else if (
              request.action === "prompt" &&
              typeof request.prompt === "string" &&
              request.prompt.trim()
            ) {
              const active = payload(
                await context.client.session.active(options()),
              );
              if (
                !active ||
                typeof active !== "object" ||
                Array.isArray(active)
              )
                throw new Error("Execution status is unavailable");
              if (active[request.sessionID]) reply = { status: "busy" };
              else {
                if (
                  disposed ||
                  Date.now() > request.expiresAt ||
                  !openTabs(context).ids.includes(request.sessionID)
                )
                  throw new Error("The requested session is no longer open");
                attempted = true;
                const receipt = payload(
                  await context.client.session.prompt(
                    { sessionID: request.sessionID, text: request.prompt },
                    options(),
                  ),
                );
                if (!receipt?.id)
                  throw new Error(
                    "Prompt receipt was not confirmed; delivery is uncertain",
                  );
                // Remember the message this client created so the session's
                // next stop is not mistaken for new operator input.
                appMessageIds(request.sessionID).add(receipt.id);
                reply = { status: "accepted", receipt: receipt.id };
              }
            } else throw new Error("Unsupported request");
          } catch (error) {
            // A request refused before the prompt was attempted is not a
            // delivery, so the app may try again. A failure after the attempt
            // is uncertain and is never retried on its own.
            reply = {
              status: attempted ? "rejected" : "refused",
              error: String(error.message).slice(0, 500),
            };
          }
          writeFileSync(`${receiptFile}.tmp`, JSON.stringify(reply));
          renameSync(`${receiptFile}.tmp`, receiptFile);
        }
      } finally {
        processing = false;
      }
    }

    await publish();
    const timer = setInterval(() => void publish().catch(() => {}), 2000);
    const commands = setInterval(
      () => void processRequests().catch(() => {}),
      250,
    );
    timer.unref?.();
    commands.unref?.();
    return () => {
      disposed = true;
      clearInterval(timer);
      clearInterval(commands);
      for (const path of [file, temporary]) {
        try {
          unlinkSync(path);
        } catch {}
      }
    };
  },
};
