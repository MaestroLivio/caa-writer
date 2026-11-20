// Test delle funzioni pure di riordino delle alternative.

import assert from "node:assert/strict";
import test from "node:test";

const { rankMap, isNeuralMethod, charLength, reorderByScores, suggestedLabel } = await import(
  "../../.test-build/decision/ranking.js"
);

function response(overrides = {}) {
  return {
    request_id: "r1",
    status: "completed",
    selected_candidate_id: "b",
    ranked_candidates: [
      { id: "b", label: "b", score: 0.9, rank: 1, score_kind: "hybrid", locked: false },
      { id: "a", label: "a", score: 0.1, rank: 2, score_kind: "hybrid", locked: false },
    ],
    decision: { action: "suggest", requires_confirmation: false, reason: "test" },
    metadata: {
      model: "m",
      model_revision: "r",
      method: "neural+rules",
      cached: false,
      latency_ms: 1,
      hypothesis_version: "it-1",
      taxonomy_version: "it-core-1",
      truncated: false,
      candidates_considered: 2,
    },
    ...overrides,
  };
}

const alternatives = [
  { symbol_id: "a", label: "A" },
  { symbol_id: "b", label: "B" },
  { symbol_id: "c", label: "C" },
];

test("rankMap legge i punteggi per id", () => {
  assert.deepEqual(rankMap(response()), { b: 0.9, a: 0.1 });
});

test("solo i metodi neurali sono presentati come semantici", () => {
  assert.equal(isNeuralMethod("neural"), true);
  assert.equal(isNeuralMethod("neural+rules"), true);
  assert.equal(isNeuralMethod("rules"), false);
  assert.equal(isNeuralMethod("rules_fallback"), false);
});

test("charLength conta caratteri Unicode", () => {
  assert.equal(charLength("pesca"), 5);
  assert.equal(charLength("àèì"), 3);
  assert.equal(charLength("😀a"), 2);
});

test("riordina per punteggio e tiene in testa il corrente", () => {
  const ordered = reorderByScores(alternatives, { b: 0.9, a: 0.1 }, "a");
  assert.deepEqual(
    ordered.map((item) => item.symbol_id),
    ["a", "b", "c"]
  );
  const withoutKeep = reorderByScores(alternatives, { b: 0.9, a: 0.1 }, null);
  assert.deepEqual(
    withoutKeep.map((item) => item.symbol_id),
    ["b", "a", "c"]
  );
});

test("senza punteggi l'ordine resta invariato", () => {
  const ordered = reorderByScores(alternatives, {}, "a");
  assert.deepEqual(
    ordered.map((item) => item.symbol_id),
    ["a", "b", "c"]
  );
});

test("suggestedLabel trova l'etichetta del candidato suggerito", () => {
  assert.equal(suggestedLabel(response(), alternatives), "B");
  assert.equal(
    suggestedLabel(response({ selected_candidate_id: null }), alternatives),
    null
  );
});
