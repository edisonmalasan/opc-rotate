import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { createServer } from "node:net";

const require = createRequire(import.meta.url);
const tauriCli = require.resolve("@tauri-apps/cli/tauri.js");
const args = process.argv.slice(2);

function portIsAvailable(port) {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", (error) => {
      if (error.code === "EADDRINUSE" || error.code === "EACCES") resolve(false);
      else reject(error);
    });
    server.listen({ host: "127.0.0.1", port }, () => server.close(() => resolve(true)));
  });
}

async function findDevPort() {
  for (let port = 1420; port <= 1499; port += 1) {
    if (await portIsAvailable(port)) return port;
  }
  throw new Error("No free development port found between 1420 and 1499.");
}

async function main() {
  let cliArgs = args;
  let env = process.env;

  if (args[0] === "dev" && !args.includes("--help") && !args.includes("-h")) {
    const port = await findDevPort();
    const devUrl = `http://127.0.0.1:${port}`;
    console.log(`Starting Tauri and Vite at ${devUrl}`);
    cliArgs = ["dev", "--config", JSON.stringify({ build: { devUrl } }), ...args.slice(1)];
    env = { ...process.env, OPC_DEV_PORT: String(port) };
  }

  const child = spawn(process.execPath, [tauriCli, ...cliArgs], { stdio: "inherit", env });
  child.once("error", (error) => {
    console.error(error);
    process.exitCode = 1;
  });
  child.once("exit", (code, signal) => {
    process.exitCode = code ?? (signal ? 1 : 0);
  });
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
