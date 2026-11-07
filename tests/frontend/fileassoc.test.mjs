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

test("le risorse della biblioteca universale entrano nel bundle", () => {
  const resources = config.bundle.resources ?? [];
  assert.ok(resources.some((pattern) => pattern.includes("resources/library")));
});
