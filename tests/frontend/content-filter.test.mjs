import assert from "node:assert/strict";
import test from "node:test";
import { filterExplicit, isExplicitContent } from "../../.test-build/content-filter.js";

test("i termini espliciti sono riconosciuti", () => {
  assert.equal(isExplicitContent("vagina"), true);
  assert.equal(isExplicitContent("pap test"), true);
  assert.equal(isExplicitContent("testicoli"), true);
  assert.equal(isExplicitContent("vaginal"), true);
  assert.equal(isExplicitContent("Penises"), true);
  assert.equal(isExplicitContent("sexual intercourse"), true);
  assert.equal(isExplicitContent("masturbating"), true);
  assert.equal(isExplicitContent("Pictogram for oral sex"), true);
  assert.equal(isExplicitContent("lavagna"), false);
  assert.equal(isExplicitContent("salvagente"), false);
  assert.equal(isExplicitContent("Essex"), false);
  assert.equal(isExplicitContent("cockatiel"), false);
});

test("il filtro rimuove contenuti espliciti in italiano e in inglese", () => {
  const items = [
    { label: "lavagna" },
    { label: "vagina" },
    { label: "tovagliolo" },
    { label: "sexual intercourse" },
  ];
  const safe = filterExplicit(items, (item) => [item.label], false);
  assert.deepEqual(safe.map((item) => item.label), ["lavagna", "tovagliolo"]);
  assert.equal(filterExplicit(items, (item) => [item.label], true).length, 4);
});
