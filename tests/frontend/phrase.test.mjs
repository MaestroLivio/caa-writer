// Test della composizione di una frase CAA in celle (modulo TypeScript
// compilato da `npm run test:frontend`). Verifica che le parole restino,
// la punteggiatura si unisca e la griglia si disponga su più righe.

import assert from "node:assert/strict";
import test from "node:test";
import { buildPhraseGrid } from "../../.test-build/universal/phrase.js";

function token(surface, extra = {}) {
  return { surface, symbol_id: null, concept_id: null, svg: null, image: null, ...extra };
}

test("ogni parola diventa una cella con il suo pittogramma", () => {
  const grid = buildPhraseGrid(
    [
      token("ciao", { symbol_id: "s1", image: "data:image/png;base64,AAA" }),
      token("mamma", { symbol_id: "s2", image: "data:image/png;base64,BBB" }),
    ],
    { columns: 4 }
  );
  assert.equal(grid.cells.length, 2);
  assert.equal(grid.columns, 2);
  assert.equal(grid.rows, 1);
  assert.equal(grid.cells[0].text, "ciao");
  assert.equal(grid.cells[0].image_asset, "data:image/png;base64,AAA");
  assert.equal(grid.cells[1].symbol_id, "s2");
});

test("la punteggiatura non occupa una cella e si attacca alla parola", () => {
  const grid = buildPhraseGrid(
    [token("Il"), token("gatto"), token("dorme"), token(".", { is_punct: true })],
    { columns: 6 }
  );
  assert.equal(grid.cells.length, 3);
  assert.equal(grid.cells[2].text, "dorme.");
});

test("le parole senza pittogramma restano come celle di testo", () => {
  const grid = buildPhraseGrid([token("e"), token("poi"), token("casa")], { columns: 6 });
  assert.equal(grid.cells.length, 3);
  assert.equal(grid.cells[0].image_asset, null);
  assert.equal(grid.cells[0].text, "e");
});

test("le celle si dispongono su più righe quando superano le colonne", () => {
  const grid = buildPhraseGrid(
    [token("a"), token("b"), token("c"), token("d"), token("e")],
    { columns: 2 }
  );
  assert.equal(grid.rows, 3);
  assert.deepEqual(
    grid.cells.map((cell) => [cell.row, cell.column]),
    [
      [0, 0],
      [0, 1],
      [1, 0],
      [1, 1],
      [2, 0],
    ]
  );
});

test("un pittogramma nascosto lascia la parola ma non il simbolo", () => {
  const grid = buildPhraseGrid(
    [token("segreto", { symbol_id: "s1", image: "data:image/png;base64,AAA", hidden: true })],
    { columns: 4 }
  );
  assert.equal(grid.cells.length, 1);
  assert.equal(grid.cells[0].text, "segreto");
  assert.equal(grid.cells[0].image_asset, null);
  assert.equal(grid.cells[0].symbol_id, null);
});

test("una frase vuota non produce celle", () => {
  const grid = buildPhraseGrid([], { columns: 4 });
  assert.equal(grid.cells.length, 0);
  assert.equal(grid.columns, 1);
  assert.equal(grid.rows, 1);
});
