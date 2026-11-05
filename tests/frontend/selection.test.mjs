// Test della mappatura selezione → token (modulo TypeScript compilato dal
// comando `npm run test:frontend`). Non richiede browser né rete.

import assert from "node:assert/strict";
import test from "node:test";

import {
  tokenAtOffset,
  tokenForSelection,
  tokenInRange,
} from "../../.test-build/editor/selection.js";

const tokens = [
  { index: 0, surface: "Io", start: 0, end: 2 },
  { index: 1, surface: "vorrei", start: 3, end: 9 },
  { index: 2, surface: "capire", start: 10, end: 16 },
  { index: 3, surface: ".", start: 16, end: 17, is_punct: true },
];

test("il cursore dentro una parola senza pittogramma seleziona la parola", () => {
  assert.equal(tokenAtOffset(tokens, 12)?.surface, "capire");
});

test("il cursore subito dopo la parola resta sulla parola appena scritta", () => {
  assert.equal(tokenAtOffset(tokens, 16)?.surface, "capire");
});

test("uno spazio interno non inventa una selezione", () => {
  const spaced = [
    { index: 0, surface: "Io", start: 0, end: 2 },
    { index: 1, surface: "capire", start: 4, end: 10 },
  ];
  assert.equal(tokenAtOffset(spaced, 3), null);
});

test("una selezione su una parola restituisce quella parola", () => {
  assert.equal(tokenInRange(tokens, 10, 16)?.surface, "capire");
});

test("una selezione multipla restituisce il primo token, non l'ultimo", () => {
  assert.equal(tokenForSelection(tokens, 3, 16)?.surface, "vorrei");
});

test("la punteggiatura non è selezionabile", () => {
  assert.equal(tokenAtOffset(tokens, 16)?.surface, "capire");
  assert.equal(tokenInRange(tokens, 16, 17), null);
});

test("gli accenti non alterano gli offset in caratteri", () => {
  const accented = [
    { index: 0, surface: "città", start: 0, end: 5 },
    { index: 1, surface: "bella", start: 6, end: 11 },
  ];
  assert.equal(tokenAtOffset(accented, 4)?.surface, "città");
  assert.equal(tokenAtOffset(accented, 8)?.surface, "bella");
});
