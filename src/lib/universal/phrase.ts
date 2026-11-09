// Costruzione di una frase CAA dentro un elemento libero (forma, fumetto,
// casella). Ogni parola diventa una cella di una griglia interna: il
// pittogramma sopra e la parola scritta sotto, esattamente come nel rigo CAA.
//
// La griglia interna è il formato già renderizzato da editor, PDF, PNG, DOCX,
// PPTX e HTML: così la frase resta modificabile (testo, simbolo, righe e
// colonne) e non è un'immagine piatta.

import type { UniversalGridCell, UniversalGridData, UniversalStyle } from "../types";

/** Campi minimi di un token CAA usati per comporre la frase. */
export interface PhraseToken {
  surface: string;
  spoken_text?: string | null;
  symbol_id?: string | null;
  concept_id?: string | null;
  svg?: string | null;
  image?: string | null;
  hidden?: boolean;
  is_punct?: boolean;
}

export interface PhraseGridOptions {
  /** Numero massimo di parole per riga. */
  columns?: number;
  gapMm?: number;
  fontMm?: number;
}

const CELL_STYLE: UniversalStyle = {
  font_size: 3.2,
  background: "transparent",
  border_width: 0,
  border_color: "transparent",
  border_style: "none",
  align: "center",
  valign: "middle",
};

/**
 * Trasforma i token simbolizzati in celle: la punteggiatura non occupa una
 * cella propria, viene aggiunta alla parola precedente. Le parole senza
 * pittogramma restano come celle di solo testo: la frase non perde nulla.
 */
export function buildPhraseGrid(
  tokens: PhraseToken[],
  options: PhraseGridOptions = {}
): UniversalGridData {
  const gap = Math.max(0, options.gapMm ?? 1.2);
  const cells: UniversalGridCell[] = [];
  for (const token of tokens) {
    const surface = (token.surface ?? "").trim();
    if (!surface) continue;
    if (token.is_punct) {
      const previous = cells[cells.length - 1];
      if (previous) {
        previous.text = `${previous.text}${surface}`;
        previous.spoken_text = previous.text;
      }
      continue;
    }
    cells.push({
      id: `u-${cells.length + 1}`,
      row: 0,
      column: cells.length,
      row_span: 1,
      column_span: 1,
      text: surface,
      spoken_text: token.spoken_text?.trim() || surface,
      symbol_id: token.hidden ? null : token.symbol_id ?? null,
      concept_id: token.hidden ? null : token.concept_id ?? null,
      svg: token.hidden ? null : token.svg ?? null,
      image_asset: token.hidden ? null : token.image ?? null,
      action: null,
      style: { ...CELL_STYLE, font_size: options.fontMm ?? CELL_STYLE.font_size },
      hidden: false,
    });
  }
  const maxColumns = Math.max(1, Math.min(20, Math.round(options.columns ?? 6)));
  const columns = Math.min(maxColumns, Math.max(1, cells.length));
  const rows = Math.max(1, Math.ceil(cells.length / columns));
  cells.forEach((cell, index) => {
    cell.row = Math.floor(index / columns);
    cell.column = index % columns;
  });
  return { columns, rows, gap_mm: gap, cells };
}
