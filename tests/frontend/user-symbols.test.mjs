import assert from "node:assert/strict";
import test from "node:test";

const { canChooseAlternatives, symbolChoices } = await import(
  "../../.test-build/user-symbols.js"
);

function token(overrides = {}) {
  return {
    index: 1,
    surface: "beve",
    symbol_id: "s1",
    concept_id: "c1",
    symbol_label: "bere",
    image: "data:image/png;base64,AAAA",
    pack_id: "core",
    approximate: false,
    alternatives: [
      { symbol_id: "s1", label: "bere", concept_id: "c1" },
      { symbol_id: "s2", label: "acqua", concept_id: "c2" },
      { symbol_id: "s2", label: "acqua", concept_id: "c2" },
      { symbol_id: "s3", label: "bicchiere", concept_id: "c3" },
    ],
    ...overrides,
  };
}

test("il pittogramma attribuito resta in testa e i duplicati sono rimossi", () => {
  const choices = symbolChoices(token());
  assert.deepEqual(
    choices.map((item) => item.symbol_id),
    ["s1", "s2", "s3"]
  );
  assert.equal(choices[0].label, "bere");
  assert.equal(choices[0].image, "data:image/png;base64,AAAA");
});

test("senza pittogramma attribuito restano le alternative del motore", () => {
  const choices = symbolChoices(token({ symbol_id: null, image: null, symbol_label: null }));
  assert.deepEqual(
    choices.map((item) => item.symbol_id),
    ["s1", "s2", "s3"]
  );
});

test("senza token non ci sono scelte", () => {
  assert.deepEqual(symbolChoices(null), []);
});

test("le alternative richiedono il permesso del profilo", () => {
  assert.equal(canChooseAlternatives({ features: ["search", "speech"] }), true);
  assert.equal(canChooseAlternatives({ features: ["speech"] }), false);
  assert.equal(canChooseAlternatives(null), false);
});
