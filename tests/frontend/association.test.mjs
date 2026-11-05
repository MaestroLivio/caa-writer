// Test della scelta delle occorrenze per l'ambito documento.

import assert from "node:assert/strict";
import test from "node:test";

const { equivalentTargets } = await import("../../.test-build/association/scope.js");

function token(index, surface, concept, locked = false, lemma = null) {
  return {
    index,
    surface,
    concept_id: concept,
    locked,
    lemma: lemma ?? surface,
    is_punct: false,
  };
}

test("l'ambito documento usa il concetto e non la sola stringa", () => {
  const base = token(0, "capire", "action.understand");
  const tokens = new Map([
    ["p1", [base]],
    ["p2", [token(0, "capire", "action.understand"), token(1, "capire", "other.meaning")]],
  ]);
  const targets = equivalentTargets(base, "action.understand", "p1", tokens);
  assert.equal(targets.length, 2);
  assert.ok(targets.every((item) => item.token.concept_id === "action.understand"));
});

test("le associazioni bloccate non vengono sovrascritte", () => {
  const base = token(0, "capire", "action.understand");
  const lockedOther = token(0, "capire", "action.understand", true);
  const tokens = new Map([
    ["p1", [base]],
    ["p2", [lockedOther]],
  ]);
  const targets = equivalentTargets(base, "action.understand", "p1", tokens);
  assert.deepEqual(targets.map((item) => item.pid), ["p1"]);
});

test("senza concetto si usa il lemma, non la sostituzione indiscriminata", () => {
  const base = token(0, "capire", null);
  const tokens = new Map([
    ["p1", [base]],
    ["p2", [token(0, "capito", null, false, "capire"), token(1, "casa", null, false, "casa")]],
  ]);
  const targets = equivalentTargets(base, null, "p1", tokens);
  assert.equal(targets.length, 2);
  assert.ok(targets.every((item) => item.token.lemma === "capire"));
});
