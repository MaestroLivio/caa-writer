// Lettura delle frasi durante la digitazione: il segno di punteggiatura deve
// essere già nel testo, altrimenti la frase viene scartata.

import assert from "node:assert/strict";
import test from "node:test";

const { completedSentenceBefore, lastSentence } = await import(
  "../../.test-build/editor/sentence.js"
);

test("una frase chiusa dal punto viene riconosciuta", () => {
  assert.equal(
    completedSentenceBefore("La sintesi vocale legge il testo."),
    "La sintesi vocale legge il testo."
  );
});

test("senza segno di punteggiatura non c'è una frase completata", () => {
  assert.equal(completedSentenceBefore("La sintesi vocale legge"), "");
});

test("si legge solo l'ultima frase, non tutto il paragrafo", () => {
  assert.equal(
    completedSentenceBefore("Prima frase. La sintesi vocale legge il testo!"),
    "La sintesi vocale legge il testo!"
  );
});

test("domanda, due punti e punto e virgola chiudono la frase", () => {
  assert.equal(completedSentenceBefore("Legge tutto?"), "Legge tutto?");
  assert.equal(completedSentenceBefore("Ho finito:"), "Ho finito:");
  assert.equal(completedSentenceBefore("Ho finito;"), "Ho finito;");
});

test("su Invio si legge l'ultima frase anche senza punto finale", () => {
  assert.equal(lastSentence("Prima frase. Uso un computer portatile"), "Uso un computer portatile");
});

test("su Invio non si ripetono le frasi precedenti", () => {
  assert.equal(lastSentence("Prima frase. Seconda frase."), "Seconda frase.");
});

test("testo vuoto non produce frasi", () => {
  assert.equal(lastSentence(""), "");
  assert.equal(completedSentenceBefore(""), "");
});
