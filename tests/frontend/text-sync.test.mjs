// Test dell'unione tra flusso di testo e elementi liberi (modulo TypeScript
// compilato da `npm run test:frontend`). Il caso che conta è la regressione
// del fumetto: dopo la sincronizzazione del testo non deve sparire.

import assert from "node:assert/strict";
import test from "node:test";
import {
  isFlowElement,
  mergeTextElements,
} from "../../.test-build/universal/text-sync.js";

function element(id, kind = "text") {
  return { id, kind, text: id, style: {} };
}

test("gli elementi con id t- sono flusso, gli altri sono liberi", () => {
  assert.equal(isFlowElement(element("t-abc-0")), true);
  assert.equal(isFlowElement(element("el-abc")), false);
  assert.equal(isFlowElement(element("free-abc")), false);
});

test("la sincronizzazione conserva un fumetto libero", () => {
  const flow = [element("t-1-0"), element("t-1-1")];
  const existing = [element("t-old-0"), element("el-fumetto", "speech")];
  const merged = mergeTextElements(flow, existing);
  assert.deepEqual(
    merged.map((item) => item.id),
    ["t-1-0", "t-1-1", "el-fumetto"]
  );
});

test("i vecchi elementi di flusso vengono sostituiti, non duplicati", () => {
  const flow = [element("t-2-0")];
  const existing = [element("t-1-0"), element("t-1-1"), element("grid-1", "grid")];
  const merged = mergeTextElements(flow, existing);
  assert.deepEqual(
    merged.map((item) => item.id),
    ["t-2-0", "grid-1"]
  );
});

test("l'ordine dei liberi resta quello originale", () => {
  const existing = [
    element("el-a", "speech"),
    element("el-b", "image"),
    element("free-c", "shape"),
  ];
  const merged = mergeTextElements([], existing);
  assert.deepEqual(
    merged.map((item) => item.id),
    ["el-a", "el-b", "free-c"]
  );
});

test("senza elementi liberi resta solo il flusso", () => {
  const merged = mergeTextElements([element("t-3-0")], [element("t-2-0")]);
  assert.deepEqual(
    merged.map((item) => item.id),
    ["t-3-0"]
  );
});
