// Mappatura fra la selezione di ProseMirror e i token CAA.
//
// Vive in un modulo senza dipendenze da Svelte o da ProseMirror per poter
// essere verificata con test Node: la selezione testuale è la parte che ha
// causato il bug «capire» (parola senza pittogramma non selezionabile).

export interface TokenRange {
  index: number;
  surface: string;
  start: number;
  /** Offset esclusivo, in caratteri, relativo al paragrafo. */
  end: number;
  is_punct?: boolean;
  is_function?: boolean;
}

/** Token che contiene l'offset del cursore (offset in caratteri). */
export function tokenAtOffset<T extends TokenRange>(tokens: T[], offset: number): T | null {
  const at = Math.max(0, offset);
  const inside = tokens.find(
    (token) => !token.is_punct && at >= token.start && at < token.end
  );
  if (inside) return inside;
  // Cursore subito dopo la parola: appartiene ancora alla parola appena scritta.
  return (
    tokens.find(
      (token) => !token.is_punct && token.end === at && token.surface.trim().length > 0
    ) ?? null
  );
}

/** Primo token che interseca l'intervallo [from, to). */
export function tokenInRange<T extends TokenRange>(
  tokens: T[],
  from: number,
  to: number
): T | null {
  const start = Math.max(0, Math.min(from, to));
  const end = Math.max(start + 1, Math.max(from, to));
  return (
    tokens.find(
      (token) => !token.is_punct && token.end > start && token.start < end
    ) ?? null
  );
}

/**
 * Sceglie il token da mostrare nel pannello per una selezione.
 * Una selezione multipla usa il primo token dell'intervallo: l'interfaccia
 * dichiara il limite e non applica il simbolo a un intervallo sbagliato.
 */
export function tokenForSelection<T extends TokenRange>(
  tokens: T[],
  from: number,
  to: number
): T | null {
  return from === to ? tokenAtOffset(tokens, from) : tokenInRange(tokens, from, to);
}
