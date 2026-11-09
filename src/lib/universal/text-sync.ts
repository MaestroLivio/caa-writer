// Unione tra il testo CAA e gli elementi liberi di una pagina.
//
// La pagina di testo tiene in `elements` due famiglie di elementi:
// - il flusso generato dalle parole (`t-…`), copia tecnica usata da miniature,
//   stampa ed esportazioni, ricostruito a ogni sincronizzazione;
// - fumetti, immagini, forme e griglie inseriti dall'utente, che devono
//   sopravvivere alla sincronizzazione del testo.
//
// Questa funzione è pura per poter essere verificata senza WebView: il bug
// storico era proprio la sostituzione dell'intero elenco con il solo flusso,
// che faceva sparire il fumetto 600 ms dopo l'inserimento.

import type { UniversalElement } from "../types";

/** Vero per gli elementi di flusso generati dal testo, non per quelli liberi. */
export function isFlowElement(element: UniversalElement): boolean {
  return element.id.startsWith("t-");
}

/**
 * Restituisce il nuovo elenco della pagina: prima il flusso appena
 * ricostruito, poi gli elementi liberi già presenti, nell'ordine originale.
 */
export function mergeTextElements(
  flow: UniversalElement[],
  existing: UniversalElement[]
): UniversalElement[] {
  const floating = existing.filter((element) => !isFlowElement(element));
  return [...flow, ...floating];
}
