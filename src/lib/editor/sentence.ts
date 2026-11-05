// Estrazione delle frasi per la lettura durante la digitazione.
//
// Due operazioni distinte:
// - `completedSentenceBefore` restituisce l'ultima frase già chiusa da un
//   segno di punteggiatura (usata dopo l'inserimento di `. ! ? : ;`);
// - `lastSentence` restituisce l'ultima frase del blocco anche senza segno
//   finale (usata su Invio, prima che il paragrafo venga spezzato).

const TERMINATORS = ".!?:;\n";

/** Ultima frase già terminata nel testo, oppure stringa vuota. */
export function completedSentenceBefore(text: string): string {
  if (!text) return "";
  const pattern = new RegExp(`[^${TERMINATORS}]*[.!?:;]\\s*$`, "u");
  const match = text.match(pattern);
  return match ? match[0].trim() : "";
}

/** Ultima frase del testo, con o senza segno di punteggiatura finale. */
export function lastSentence(text: string): string {
  if (!text) return "";
  const segments = text.match(/[^.!?:;\n]+[.!?:;]?/gu) ?? [];
  const trimmed = segments.map((segment) => segment.trim()).filter(Boolean);
  return trimmed.length > 0 ? trimmed[trimmed.length - 1] : "";
}
