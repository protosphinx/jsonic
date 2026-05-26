import assert from "node:assert/strict";
import { spawn } from "node:child_process";

const child = spawn(process.execPath, ["dist/server.js"], {
  cwd: new URL("..", import.meta.url),
  env: {
    ...process.env,
    JSONIC_RPC_URL: "http://127.0.0.1:9"
  },
  stdio: ["pipe", "pipe", "pipe"]
});

let stdout = "";
let stderr = "";
child.stdout.on("data", (chunk) => {
  stdout += chunk;
});
child.stderr.on("data", (chunk) => {
  stderr += chunk;
});

await new Promise((resolve) => setTimeout(resolve, 300));
assert.equal(child.exitCode, null, `server exited early\nstdout=${stdout}\nstderr=${stderr}`);

child.kill("SIGTERM");
await new Promise((resolve) => child.once("exit", resolve));
