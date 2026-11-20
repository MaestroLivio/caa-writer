// Funzioni pure per riordinare le alternative in base al DecisionEngine.
//
// Non chiamano Tauri: sono testabili con Node e non dipendono dal DOM.

import type { DecisionResponse } from "../types";

/// Mappa symbol_id → punteggio, solo per i candidati restituiti.
export function rankMap(response: DecisionResponse): Record<string, number> {
  const scores: Record<string, number> = {};
  for (const item of response.ranked_candidates) {
    scores[item.id] = item.score;
  }
  return scores;
}

/// Vero solo quando Horizon ha realmente partecipato alla decisione.
/// Le regole deterministiche da sole non vengono presentate come semantiche.
export function isNeuralMethod(method: string): boolean {
  return method.startsWith("neural");
}

/// Numero di caratteri Unicode (non unità UTF-16).
export function charLength(text: string): number {
  return [...text].length;
}

/// Riordina mantenendo in testa l'elemento corrente (se presente).
/// Gli elementi senza punteggio restano dopo quelli valutati, nell'ordine
/// originale: non scompaiono durante l'aggiornamento.
export function reorderByScores<T extends { symbol_id: string }>(
  items: T[],
  scores: Record<string, number>,
  keepFirst?: string | null
): T[] {
  if (Object.keys(scores).length === 0) return items;
  const first = keepFirst ? items.find((item) => item.symbol_id === keepFirst) : undefined;
  const rest = first ? items.filter((item) => item !== first) : [...items];
  const evaluated = rest.filter((item) => scores[item.symbol_id] !== undefined);
  const unevaluated = rest.filter((item) => scores[item.symbol_id] === undefined);
  evaluated.sort((a, b) => {
    const delta = scores[b.symbol_id] - scores[a.symbol_id];
    return delta !== 0 ? delta : 0;
  });
  return first ? [first, ...evaluated, ...unevaluated] : [...evaluated, ...unevaluated];
}

/// Etichetta del candidato suggerito, se presente.
export function suggestedLabel(
  response: DecisionResponse,
  items: { symbol_id: string; label: string }[]
): string | null {
  const id = response.selected_candidate_id;
  if (!id) return null;
  return items.find((item) => item.symbol_id === id)?.label ?? null;
}
