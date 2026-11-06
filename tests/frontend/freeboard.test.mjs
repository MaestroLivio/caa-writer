// Il foglio libero deve diventare una specifica di layout salvabile e
// riapribile senza perdere celle, ordine o pittogrammi.

import assert from "node:assert/strict";
import test from "node:test";

const { freeBoardSpec, freeBoardFromLayout } = await import("../../.test-build/freeboard.js");

function cell(id, label, svg = null, symbolId = null) {
  return {
    id,
    label,
    svg,
    symbolId,
    conceptId: symbolId ? "concept.demo" : null,
    approximate: false,
  };
}

test("il foglio libero diventa una specifica di layout versionata", () => {
  const spec = freeBoardSpec({
    title: "La mia giornata",
    columns: 3,
    cells: [cell("a", "mela"), cell("b", "bicchiere"), cell("c", "casa"), cell("d", "cane")],
  });
  assert.equal(spec.document_type, "free_board");
  assert.equal(spec.schema_version, 1);
  assert.equal(spec.layout.columns, 3);
  assert.equal(spec.layout.rows, 2);
  assert.deepEqual(
    spec.elements.map((element) => [element.row, element.column]),
    [
      [0, 0],
      [0, 1],
      [0, 2],
      [1, 0],
    ]
  );
  assert.equal(spec.elements[0].kind, "box");
});

test("un pittogramma resta incorporato con identificativo e SVG", () => {
  const svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"></svg>';
  const spec = freeBoardSpec({
    title: "Simboli",
    columns: 2,
    cells: [cell("a", "mela", svg, "core.fruit.apple")],
  });
  assert.equal(spec.elements[0].kind, "symbol");
  assert.equal(spec.elements[0].symbol_id, "core.fruit.apple");
  assert.equal(spec.elements[0].svg, svg);
  assert.deepEqual(spec.elements[0].action, { type: "append", target_page: null });
});

test("la riapertura ricostruisce celle e colonne", () => {
  const data = {
    title: "Riaperto",
    columns: 5,
    cells: [cell("a", "uno", null, "core.one"), cell("b", "due")],
  };
  const spec = freeBoardSpec(data);
  const reopened = freeBoardFromLayout("Riaperto", spec);
  assert.equal(reopened.columns, 5);
  assert.equal(reopened.cells.length, 2);
  assert.equal(reopened.cells[0].symbolId, "core.one");
  assert.equal(reopened.cells[1].label, "due");
});

test("le colonne sono limitate e le celle in eccesso non vengono salvate", () => {
  const cells = Array.from({ length: 400 }, (_, index) => cell(`c${index}`, `voce ${index}`));
  const spec = freeBoardSpec({ title: "Troppe", columns: 20, cells });
  assert.equal(spec.layout.columns, 8);
  assert.equal(spec.elements.length, 8 * 40);
});
