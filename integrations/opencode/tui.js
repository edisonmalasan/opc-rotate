// Loaded inside the local OpenCode CLI. Reports presence, never prompts or VPN actions.
import { mkdirSync, renameSync, writeFileSync, unlinkSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

export default {
  id: "opc-rotate.presence",
  async setup(context) {
    // OpenCode can probe a plugin before supplying its TUI context.
    if (typeof context?.ui?.router?.current !== "function") return;
    const directory =
      process.env.OPC_ROTATE_PRESENCE_DIR ||
      join(homedir(), ".local", "state", "opc-rotate", "presence");
    const file = join(directory, `${process.pid}.json`);
    const temporary = `${file}.tmp`;
    let disposed = false;
    let writing = false;
    let warned = false;

    async function publish() {
      if (disposed || writing) return;
      writing = true;
      try {
        // Read identity from the connected server, including after a reconnect.
        const info = await context.client.server.info({
          signal: AbortSignal.timeout(2000),
        });
        if (disposed) return;
        const server = info.data ?? info;
        if (!Number.isInteger(server.pid) || server.pid <= 0) return;
        const route = context.ui.router.current();
        const sessionID = route?.type === "session" ? route.sessionID : null;
        mkdirSync(directory, { recursive: true });
        writeFileSync(
          temporary,
          JSON.stringify({
            version: 1,
            pid: process.pid,
            servicePid: server.pid,
            sessionID,
            orca: Boolean(
              process.env.ORCA_AGENT_HOOK_ENDPOINT ||
                process.env.ORCA_AGENT_HOOK_PORT,
            ),
            updatedAt: Date.now(),
          }),
        );
        renameSync(temporary, file);
        warned = false;
      } catch {
        if (!warned && !disposed) {
          console.warn("[opc-rotate] Unable to report live session presence");
          warned = true;
        }
      } finally {
        writing = false;
      }
    }

    await publish();
    const timer = setInterval(() => void publish(), 2000);
    timer.unref?.();
    return () => {
      disposed = true;
      clearInterval(timer);
      for (const path of [file, temporary]) {
        try {
          unlinkSync(path);
        } catch {
          /* May already be removed. */
        }
      }
    };
  },
};
