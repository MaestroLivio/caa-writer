#!/usr/bin/env node
// Avvia CAA Writer in modalità sviluppatore per i pacchetti della libreria.
// La modalità è attiva solo nelle build di debug e consente di modificare
// i documenti dentro resources/library.
import { spawn } from "node:child_process";

process.env.CAA_WRITER_DEV_LIBRARY = "1";
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const child = spawn(npm, ["run", "tauri:dev"], {
  stdio: "inherit",
  env: process.env,
});
child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  else process.exit(code ?? 0);
});
