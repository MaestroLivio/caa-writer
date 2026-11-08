import assert from "node:assert/strict";
import test from "node:test";
import { filterExplicit, isExplicitContent } from "../../.test-build/content-filter.js";

test("i termini espliciti sono riconosciuti", () => {
  assert.equal(isExplicitContent("vagina"), true);
  assert.equal(isExplicitContent("pap test"), true);
  assert.equal(isExplicitContent("testicoli"), true);
  assert.equal(isExplicitContent("lavagna"), false);
  assert.equal(isExplicitContent("salvagente"), false);
});

test("il filtro rimuove solo i contenuti espliciti", () => {
  const items = [
    { label: "lavagna" },
    { label: "vagina" },
    { label: "tovagliolo" },
  ];
  const safe = filterExplicit(items, (item) => [item.label], false);
  assert.deepEqual(safe.map((item) => item.label), ["lavagna", "tovagliolo"]);
  assert.equal(filterExplicit(items, (item) => [item.label], true).length, 3);
});
