// Esegue i test frontend elencando i file uno per uno.
//
// `node --test tests/frontend` funziona con Node recenti, ma in CI (Node 22) il
// percorso della cartella viene interpretato come modulo da caricare e la
// suite fallisce con «Cannot find module». Passare i file esplicitamente
// funziona su tutte le versioni e su Windows, dove la shell non espande i glob.

import { readdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const directory = fileURLToPath(new URL("../tests/frontend/", import.meta.url));
const files = readdirSync(directory)
  .filter((name) => name.endsWith(".test.mjs"))
  .sort()
  .map((name) => `${directory}${name}`);

if (files.length === 0) {
  console.error(`Nessun test trovato in ${directory}`);
  process.exit(1);
}

const result = spawnSync(process.execPath, ["--test", ...files], { stdio: "inherit" });
process.exit(result.status ?? 1);
