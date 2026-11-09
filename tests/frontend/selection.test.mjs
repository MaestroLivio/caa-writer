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

test("ogni riga di un testo complesso ha offset indipendenti", () => {
  const first = [
    { index: 0, surface: "Oggi", start: 0, end: 4 },
    { index: 1, surface: "scrivo", start: 5, end: 11 },
  ];
  const second = [
    { index: 0, surface: "una", start: 0, end: 3 },
    { index: 1, surface: "tabella", start: 4, end: 11 },
  ];
  assert.equal(tokenAtOffset(first, 7)?.surface, "scrivo");
  assert.equal(tokenAtOffset(second, 7)?.surface, "tabella");
});

test("una selezione all'indietro restituisce comunque il primo token", () => {
  assert.equal(tokenForSelection(tokens, 16, 3)?.surface, "vorrei");
});

test("gli spazi finali di una riga non spostano la parola precedente", () => {
  const line = [{ index: 0, surface: "fine", start: 0, end: 4 }];
  assert.equal(tokenAtOffset(line, 4)?.surface, "fine");
  assert.equal(tokenAtOffset(line, 5), null);
});

test("i caratteri fuori dal BMP mantengono gli offset UTF-16", () => {
  const emoji = [
    { index: 0, surface: "ciao", start: 0, end: 4 },
    { index: 1, surface: "mondo", start: 6, end: 11 },
  ];
  assert.equal(tokenAtOffset(emoji, 7)?.surface, "mondo");
  assert.equal(tokenAtOffset(emoji, 5), null);
});
