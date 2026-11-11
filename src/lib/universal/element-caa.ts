// Testo CAA dentro gli elementi dell'editor universale.
//
// Gli elementi (caselle, celle, pulsanti, fumetti) conservano il testo e i
// pittogrammi scelti parola per parola. Il motore è lo stesso del word
// processor: qui vivono solo le regole pure per immagazzinare i token in
// `metadata.caa` e per trasformarli in segmenti di testo da disegnare.
//
// Tenere questo modulo senza Svelte permette di verificarlo con Node: le
// regole di visibilità sono le stesse di `editor/units.ts`, così un elemento
// non può mostrare un pittogramma che il testo libero nasconderebbe.

import { unitsForParagraph, unitTitle, symbolKey } from "../editor/units";
import type { UnitTokenLike, UnitView } from "../editor/units";
import type { CaaToken } from "../types";
import type { UniversalElement } from "../types";

/** Token ridotto salvato nel documento: niente elenchi di alternative, che
 *  sarebbero grandi e cambiano a ogni ricerca. */
export interface StoredCaaToken {
  index: number;
  surface: string;
  start: number;
  end: number;
  lemma?: string | null;
  pos?: string;
  concept_id?: string | null;
  concept_label?: string | null;
  meaning?: string | null;
  symbol_id?: string | null;
  symbol_label?: string | null;
  caption?: string;
  svg?: string | null;
  image?: string | null;
  composition?: Array<{ svg?: string | null; image?: string | null; label?: string | null }>;
  strategy?: string;
  confidence?: number;
  approximate?: boolean;
  locked?: boolean;
  hidden?: boolean;
  key?: boolean;
  is_punct?: boolean;
  is_function?: boolean;
  span?: number;
  sentence_index?: number;
  modifiers?: CaaToken["modifiers"];
}

export interface StoredCaaText {
  /** ProseMirror JSON del testo dell'elemento: serve a riaprire la stessa
   *  modifica con selezione, paragrafi e formattazione. */
  doc_json: unknown;
  plain_text: string;
  tokens: StoredCaaToken[];
}

/** Converte i token completi del motore nella forma compatta da salvare. */
export function compactTokens(tokens: CaaToken[]): StoredCaaToken[] {
  return tokens.map((token) => ({
    index: token.index,
    surface: token.surface,
    start: token.start,
    end: token.end,
    lemma: token.lemma,
    pos: token.pos,
    concept_id: token.concept_id,
    concept_label: token.concept_label,
    meaning: token.meaning,
    symbol_id: token.symbol_id,
    symbol_label: token.symbol_label ?? null,
    caption: token.caption,
    svg: token.svg,
    image: token.image ?? null,
    composition: (token.composition ?? []).map((part) => ({
      svg: part.svg ?? null,
      image: part.image ?? null,
      label: part.label ?? null,
    })),
    strategy: token.strategy,
    confidence: token.confidence,
    approximate: token.approximate,
    locked: token.locked,
    hidden: token.hidden,
    key: token.key,
    is_punct: token.is_punct,
    is_function: token.is_function,
    span: token.span,
    sentence_index: token.sentence_index,
    modifiers: token.modifiers,
  }));
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

/** Testo CAA salvato su un elemento, se esiste ed è coerente col testo. */
export function storedCaaText(entity: { metadata?: unknown }): StoredCaaText | null {
  const metadata = asRecord(entity.metadata);
  const caa = metadata ? asRecord(metadata.caa) : null;
  if (!caa) return null;
  const tokens = Array.isArray(caa.tokens) ? (caa.tokens as StoredCaaToken[]) : [];
  const plain = typeof caa.plain_text === "string" ? caa.plain_text : "";
  return { doc_json: caa.doc_json ?? null, plain_text: plain, tokens };
}

/** Un elemento ha una modifica CAA salvata? */
export function hasStoredCaa(entity: { metadata?: unknown }): boolean {
  return storedCaaText(entity) !== null;
}

export interface ElementSegment {
  text: string;
  unit: UnitView<UnitTokenLike> | null;
}

/**
 * Segmenti da disegnare per un testo con token CAA.
 *
 * Il testo resta la fonte autorevole: si cammina sui range dei token visibili
 * e si conservano gli spazi, le parole senza pittogramma e la punteggiatura.
 * Un token non valido (range fuori dal testo) viene ignorato, non inventato.
 */
export function elementSegments(
  text: string,
  tokens: StoredCaaToken[] | UnitTokenLike[],
  options?: { showExplicitSymbols?: boolean; explicit?: (token: UnitTokenLike) => boolean }
): ElementSegment[] {
  const source = text ?? "";
  if (!source) return [];
  const units = unitsForParagraph(
    tokens as UnitTokenLike[],
    {
      mode: "sopra",
      autoEnabled: true,
      showExplicitSymbols: options?.showExplicitSymbols ?? true,
      explicit: options?.explicit,
    },
    source.length
  );
  const segments: ElementSegment[] = [];
  let cursor = 0;
  for (const unit of units) {
    const start = Math.max(cursor, Math.min(source.length, unit.start));
    const end = Math.max(start, Math.min(source.length, unit.end));
    if (start > cursor) segments.push({ text: source.slice(cursor, start), unit: null });
    segments.push({ text: source.slice(start, end), unit });
    cursor = end;
  }
  if (cursor < source.length) segments.push({ text: source.slice(cursor), unit: null });
  return segments;
}

/** URL CSS fra apici doppi, con i caratteri che romperebbero la regola. */
export function cssImageUrl(value: string): string {
  return `url("${value.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "")}")`;
}

/** Regola inline per un'unità disegnata fuori dal registro del word processor. */
export function unitInlineStyle(unit: UnitView<UnitTokenLike>): string {
  const url = mediaUrl(unit.token);
  if (!url) return "";
  const width =
    (unit.token.composition?.length ?? 0) > 1
      ? `--caa-symbol-width:calc(var(--symbol-size) * ${unit.token.composition!.length});`
      : "";
  return `--caa-symbol-image:${cssImageUrl(url)};${width}`;
}

/** Unità da disegnare, con la stessa identità usata dal registro CSS. */
export function unitKey(unit: UnitView<UnitTokenLike>): string {
  return symbolKey(unit.token);
}

export function unitAccessibleTitle(unit: UnitView<UnitTokenLike>): string {
  return unitTitle(unit.token);
}

function mediaUrl(token: UnitTokenLike): string | null {
  if (token.image) return token.image;
  if (token.svg) return svgCompatibleUrl(token.svg);
  const parts = token.composition ?? [];
  if (parts.length === 0) return null;
  if (parts.length === 1) {
    const part = parts[0];
    if (part.image) return part.image;
    if (part.svg) return svgCompatibleUrl(part.svg);
    return null;
  }
  // Composizione multipla: solo SVG possono essere accostati in una sola
  // immagine; per i raster si usa il primo, dichiarando che è parziale.
  const svgs = parts.map((part) => part.svg).filter((value): value is string => Boolean(value));
  if (svgs.length !== parts.length) return parts[0]?.image ?? null;
  return null;
}

function svgCompatibleUrl(svg: string): string {
  if (svg.startsWith("data:")) return svg;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

/** Vero quando l'elemento può contenere testo CAA modificabile. */
export function supportsCaaText(kind: string): boolean {
  return [
    "text",
    "caa_text",
    "text_box",
    "button",
    "shape",
    "speech",
    "symbol",
    "communication_cell",
    "table",
    "grid",
    "placeholder",
  ].includes(kind);
}
