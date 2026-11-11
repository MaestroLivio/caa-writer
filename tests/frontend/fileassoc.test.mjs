import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const config = JSON.parse(readFileSync(new URL("../../src-tauri/tauri.conf.json", import.meta.url), "utf8"));

test("il bundle associa i documenti CAA Writer al player", () => {
  const extensions = (config.bundle.fileAssociations ?? []).flatMap((entry) => entry.ext);
  assert.ok(extensions.includes("caadoc"));
  assert.ok(extensions.includes("caaplay"));
  assert.equal(new Set(extensions).size, extensions.length);
});

test("l'installer include i modelli ma non anteprime, libreria completa e modello neurale", () => {
  const resources = config.bundle.resources ?? [];
  assert.ok(resources.length > 0);
  assert.ok(!resources.some((pattern) => pattern.endsWith("library/**/*")));
  assert.ok(!resources.some((pattern) => pattern.includes("library/previews")));
  assert.ok(resources.some((pattern) => pattern.includes("library/templates")));
  assert.ok(resources.some((pattern) => pattern.includes("library/boards")));
  assert.ok(!resources.some((pattern) => pattern.endsWith("model.gguf")));
  assert.ok(resources.some((pattern) => pattern.includes("resources/symbol-packs/core")));
  assert.ok(resources.some((pattern) => pattern.includes("resources/ai/runtime")));
});
