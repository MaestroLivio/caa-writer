// Rende eseguibili i moduli compilati in `.test-build` con Node ESM.
//
// `tsc` conserva gli import senza estensione (`from "../editor/units"`), che
// Vite risolve ma Node ESM no. Dopo la compilazione aggiungiamo `.js` ai soli
// specificatori relativi: è una riscrittura meccanica, non una modifica al
// codice sorgente.

import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = process.argv[2] ?? ".test-build";
const specifier = /(\bfrom\s*|\bimport\s*\(\s*)(["'])(\.\.?\/[^"']+?)\2/g;

function walk(directory) {
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) {
      walk(path);
    } else if (path.endsWith(".js")) {
      const source = readFileSync(path, "utf8");
      const patched = source.replace(specifier, (match, head, quote, value) => {
        if (/\.(js|json|mjs|cjs)$/.test(value)) return match;
        return `${head}${quote}${value}.js${quote}`;
      });
      if (patched !== source) writeFileSync(path, patched);
    }
  }
}

walk(root);
