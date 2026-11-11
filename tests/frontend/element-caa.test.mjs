// Test del testo CAA dentro gli elementi dell'editor universale.
//
// Verificano che il testo resti la fonte autorevole, che i token compatti
// conservino solo ciò che serve a disegnare, e che i segmenti non perdano
// spazi, parole senza pittogramma o punteggiatura.

import assert from "node:assert/strict";
import test from "node:test";

const { compactTokens, elementSegments, storedCaaText, hasStoredCaa, supportsCaaText } =
  await import("../../.test-build/universal/element-caa.js");

function fullToken(overrides = {}) {
  return {
    index: 0,
    surface: "gatto",
    start: 0,
    end: 5,
    lemma: "gatto",
    pos: "sost",
    concept_id: "c1",
    concept_label: "gatto",
    meaning: "felino",
    category: null,
    symbol_id: "s1",
    pack_id: "core",
    caption: "gatto",
    symbol_label: "gatto",
    svg: null,
    image: "data:image/png;base64,AAAA",
    composition: [],
    strategy: "exact",
    confidence: 1,
    approximate: false,
    locked: false,
    negated: false,
    key: true,
    is_punct: false,
    is_function: false,
    span: 5,
    sentence_index: 0,
    hidden: false,
    modifiers: { plural: false, negation: false, past: false, future: false, manual: false },
    alternatives: [{ symbol_id: "s1" }],
    ...overrides,
  };
}

test("compactTokens scarta le alternative ma conserva il disegno", () => {
  const [stored] = compactTokens([fullToken()]);
  assert.equal(stored.alternatives, undefined);
  assert.equal(stored.image, "data:image/png;base64,AAAA");
  assert.equal(stored.symbol_id, "s1");
  assert.equal(stored.surface, "gatto");
});

test("elementSegments conserva spazi e parole senza pittogramma", () => {
  const text = "Il gatto dorme sul divano.";
  const tokens = [fullToken({ start: 3, end: 8 })];
  const segments = elementSegments(text, tokens);
  assert.equal(segments.map((segment) => segment.text).join(""), text);
  const decorated = segments.filter((segment) => segment.unit);
  assert.equal(decorated.length, 1);
  assert.equal(decorated[0].text, "gatto");
});

test("una parola nascosta resta testo, senza pittogramma", () => {
  const tokens = [fullToken({ start: 3, end: 8, hidden: true })];
  const segments = elementSegments("Il gatto dorme.", tokens);
  assert.equal(segments.some((segment) => segment.unit), false);
  assert.equal(segments.map((segment) => segment.text).join(""), "Il gatto dorme.");
});

test("un range non valido non inventa un'unità", () => {
  const segments = elementSegments("corto", [fullToken({ start: 0, end: 40 })]);
  assert.equal(segments.map((segment) => segment.text).join(""), "corto");
  assert.equal(segments.filter((segment) => segment.unit).length, 1);
  assert.equal(segments[0].text, "corto");
});

test("storedCaaText legge metadata.caa e hasStoredCaa è coerente", () => {
  const tokens = compactTokens([fullToken()]);
  const element = {
    metadata: {
      caa: { doc_json: { type: "doc" }, plain_text: "gatto", tokens },
    },
  };
  const stored = storedCaaText(element);
  assert.ok(stored);
  assert.equal(stored.plain_text, "gatto");
  assert.equal(stored.tokens.length, 1);
  assert.equal(hasStoredCaa(element), true);
  assert.equal(hasStoredCaa({ metadata: {} }), false);
  assert.equal(storedCaaText({ metadata: {} }), null);
});

test("storedCaaText funziona anche su una cella di griglia", () => {
  const tokens = compactTokens([fullToken({ start: 3, end: 8 })]);
  const cell = {
    id: "grid-1-c2",
    text: "Il gatto dorme.",
    metadata: {
      caa: { doc_json: { type: "doc" }, plain_text: "Il gatto dorme.", tokens },
    },
  };
  const stored = storedCaaText(cell);
  assert.ok(stored);
  assert.equal(stored.tokens.length, 1);
  const segments = elementSegments(cell.text, stored.tokens);
  assert.equal(segments.map((segment) => segment.text).join(""), cell.text);
  assert.equal(segments.find((segment) => segment.unit)?.text, "gatto");
});

test("supportsCaaText riconosce gli elementi testuali", () => {
  for (const kind of ["text", "text_box", "button", "shape", "speech", "caa_text"]) {
    assert.equal(supportsCaaText(kind), true, kind);
  }
  assert.equal(supportsCaaText("image"), false);
  assert.equal(supportsCaaText("audio"), false);
});
