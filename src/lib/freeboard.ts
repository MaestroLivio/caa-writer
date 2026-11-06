// Conversione pura tra il foglio libero e la specifica di layout `.caadoc`.
// Vive fuori da App.svelte per poter essere verificata senza DOM.

import type {
  FreeBoardCell,
  FreeBoardData,
  LayoutElementSpec,
  LayoutSpecification,
} from "./types";

export const FREE_BOARD_COLUMNS_MAX = 8;
export const FREE_BOARD_ROWS_MAX = 40;

export function freeBoardSpec(data: FreeBoardData): LayoutSpecification {
  const columns = Math.max(
    1,
    Math.min(FREE_BOARD_COLUMNS_MAX, Math.round(Number(data.columns) || 4))
  );
  const maxCells = columns * FREE_BOARD_ROWS_MAX;
  const cells = data.cells.slice(0, maxCells);
  const rows = Math.max(1, Math.ceil(cells.length / columns));
  const elements: LayoutElementSpec[] = cells.map((cell, index) => ({
    id: cell.id,
    kind: cell.svg || cell.symbolId ? "symbol" : "box",
    label: cell.label,
    concept_id: cell.conceptId,
    symbol_id: cell.symbolId,
    svg: cell.svg,
    page: 0,
    row: Math.floor(index / columns),
    column: index % columns,
    row_span: 1,
    column_span: 1,
    hidden: false,
    locked: false,
    spoken_text: cell.label,
    action: { type: "append", target_page: null },
  }));
  return {
    schema_version: 1,
    document_type: "free_board",
    title: data.title || "Progetto libero",
    page: { size: "A4", orientation: "portrait", margin_mm: 12 },
    layout: { kind: "grid", columns, rows, gap_mm: 3 },
    elements,
    accessibility: { large_symbols: false, high_contrast: false },
  };
}

export function freeBoardFromLayout(
  title: string,
  layout: {
    document_type?: string;
    layout?: { columns?: number };
    elements?: Array<Record<string, unknown>>;
  } | null
): FreeBoardData {
  return {
    title: title || "Progetto libero",
    columns: Math.max(
      1,
      Math.min(FREE_BOARD_COLUMNS_MAX, Number(layout?.layout?.columns ?? 4) || 4)
    ),
    cells: (layout?.elements ?? []).map((element, index): FreeBoardCell => ({
      id: String(element.id ?? `cell-${index}`),
      label: String(element.label ?? ""),
      svg: typeof element.svg === "string" ? element.svg : null,
      symbolId: typeof element.symbol_id === "string" ? element.symbol_id : null,
      conceptId: typeof element.concept_id === "string" ? element.concept_id : null,
      approximate: false,
    })),
  };
}
