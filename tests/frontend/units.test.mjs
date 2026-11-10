// Test delle unità CAA nel flusso del testo.
//
// Le regole verificate qui sono quelle che prima dipendevano dai chip
// posizionati per misura: range, visibilità, sovrapposizioni e identità del
// pittogramma. Il rendering inline è solo la conseguenza di queste regole.

import assert from "node:assert/strict";
import test from "node:test";

const { unitsForParagraph, symbolKey, markerFor, tokenHasMedia, unitTitle } = await import(
  "../../.test-build/editor/units.js"
);

const base = { mode: "sopra", autoEnabled: true, showExplicitSymbols: true };

function token(overrides = {}) {
  return {
    index: 0,
    surface: "gatto",
    start: 0,
    end: 5,
    span: 5,
    symbol_id: "arasaac:1",
    image: "data:image/png;base64,AAAA",
    composition: [],
    is_punct: false,
    hidden: false,
    locked: false,
    approximate: false,
    strategy: "exact",
    modifiers: { plural: false, negation: false, past: false, future: false, manual: false },
    alternatives: [],
    ...overrides,
  };
}

test("la decorazione copre esattamente la parola, non la punteggiatura", () => {
  const units = unitsForParagraph(
    [token({ index: 0, start: 0, end: 5 }), token({ index: 1, surface: ".", start: 5, end: 6, is_punct: true })],
    base,
    6
  );
  assert.equal(units.length, 1);
  assert.equal(units[0].start, 0);
  assert.equal(units[0].end, 5);
});

test("un'espressione multiword resta una sola unità", () => {
  const phrase = token({ index: 0, surface: "non vedo l'ora", start: 0, end: 13, span: 13 });
  const units = unitsForParagraph([phrase, token({ index: 1, surface: "bene", start: 14, end: 18 })], base, 18);
  assert.deepEqual(
    units.map((unit) => [unit.start, unit.end]),
    [
      [0, 13],
      [14, 18],
    ]
  );
});

test("la CAA automatica spenta lascia solo le scelte manuali", () => {
  const automatic = token({ index: 0, start: 0, end: 5 });
  const manual = token({ index: 1, surface: "casa", start: 6, end: 10, strategy: "manual", locked: true });
  const units = unitsForParagraph([automatic, manual], { ...base, autoEnabled: false }, 10);
  assert.equal(units.length, 1);
  assert.equal(units[0].token.surface, "casa");
});

test("un simbolo nascosto non viene disegnato", () => {
  const units = unitsForParagraph([token({ hidden: true })], base, 5);
  assert.equal(units.length, 0);
});

test("le parole senza pittogramma restano fuori dal flusso decorato", () => {
  const units = unitsForParagraph(
    [token({ index: 0, symbol_id: null, image: null, svg: null, composition: [] })],
    base,
    5
  );
  assert.equal(units.length, 0);
  assert.equal(tokenHasMedia(token({ symbol_id: null, image: null, svg: null, composition: [] })), false);
});

test("i simboli espliciti si nascondono solo quando richiesto", () => {
  const sex = token({ surface: "sesso" });
  const visible = unitsForParagraph([sex], base, 5);
  assert.equal(visible.length, 1);
  const filtered = unitsForParagraph([sex], { ...base, showExplicitSymbols: false, explicit: () => true }, 5);
  assert.equal(filtered.length, 0);
});

test("i range non più validi dopo una modifica vengono scartati", () => {
  const units = unitsForParagraph([token({ start: 0, end: 20 })], base, 8);
  assert.equal(units.length, 1);
  assert.equal(units[0].end, 8, "il range viene tagliato alla lunghezza reale del testo");
  const outOfRange = unitsForParagraph([token({ start: 12, end: 20 })], base, 8);
  assert.equal(outOfRange.length, 0);
});

test("le sovrapposizioni parziali non producono decorazioni ambigue", () => {
  const first = token({ index: 0, surface: "non vedo", start: 0, end: 8 });
  const second = token({ index: 1, surface: "vedo l'ora", start: 4, end: 14 });
  const units = unitsForParagraph([second, first], base, 20);
  assert.deepEqual(
    units.map((unit) => [unit.start, unit.end]),
    [[0, 8]]
  );
});

test("la modalità testo spegne ogni pittogramma", () => {
  assert.equal(unitsForParagraph([token()], { ...base, mode: "testo" }, 5).length, 0);
});

test("la chiave del simbolo è stabile per lo stesso disegno e cambia con l'immagine", () => {
  const a = token();
  const b = token({ index: 3, surface: "gatti", start: 10, end: 15 });
  assert.equal(symbolKey(a), symbolKey(b));
  const different = token({ image: "data:image/png;base64,BBBB" });
  assert.notEqual(symbolKey(a), symbolKey(different));
  const otherId = token({ symbol_id: "arasaac:2" });
  assert.notEqual(symbolKey(a), symbolKey(otherId));
});

test("i modificatori producono segni leggibili anche in stampa", () => {
  const marker = markerFor(
    token({
      modifiers: { plural: true, negation: true, past: false, future: false, manual: true },
      approximate: true,
    })
  );
  assert.match(marker, /¬/);
  assert.match(marker, /×2/);
  assert.match(marker, /≈/);
});

test("il tooltip non ripete la didascalia uguale alla parola", () => {
  const title = unitTitle(token({ symbol_label: "gatto", concept_label: "gatto" }));
  assert.equal(title, "gatto");
  assert.match(unitTitle(token({ symbol_label: "felino" })), /felino/);
});
