// Unità CAA nel flusso del testo.
//
// Un'unità è un intervallo del paragrafo che riceve **un solo** pittogramma:
// una parola oppure un'espressione multiword già riconosciuta dal motore.
// Questo modulo è puro (niente Svelte, niente ProseMirror) perché le regole
// di visibilità, i range e l'identità del simbolo sono la parte che va
// verificata con test Node.
//
// Il rendering non usa più una striscia sopra il paragrafo: ogni unità viene
// decorata inline da ProseMirror, quindi il pittogramma resta ancorato alla
// parola anche quando il testo va a capo o viene selezionato con il mouse.

export interface UnitPartLike {
  svg?: string | null;
  image?: string | null;
  label?: string | null;
}

export interface UnitModifiersLike {
  plural?: boolean;
  negation?: boolean;
  past?: boolean;
  future?: boolean;
  manual?: boolean;
}

export interface UnitTokenLike {
  index: number;
  surface: string;
  start: number;
  end: number;
  span?: number;
  is_punct?: boolean;
  is_function?: boolean;
  symbol_id?: string | null;
  symbol_label?: string | null;
  concept_label?: string | null;
  meaning?: string | null;
  svg?: string | null;
  image?: string | null;
  composition?: UnitPartLike[] | null;
  hidden?: boolean;
  locked?: boolean;
  approximate?: boolean;
  strategy?: string;
  modifiers?: UnitModifiersLike | null;
  alternatives?: { neural?: boolean }[] | null;
}

export interface UnitOptions {
  /** Modalità di rappresentazione attiva (`testo` spegne ogni pittogramma). */
  mode: string;
  autoEnabled: boolean;
  showExplicitSymbols: boolean;
  /** Filtro dei contenuti espliciti, passato dal chiamante per restare puro. */
  explicit?: (token: UnitTokenLike) => boolean;
}

export interface UnitView<T extends UnitTokenLike = UnitTokenLike> {
  token: T;
  start: number;
  end: number;
  /** Identità stabile del disegno: due occorrenze uguali condividono la regola CSS. */
  key: string;
  marker: string;
  neural: boolean;
  locked: boolean;
  approximate: boolean;
  composition: boolean;
}

/** Un simbolo è visibile se ha davvero un'immagine o una composizione. */
export function tokenHasMedia(token: UnitTokenLike): boolean {
  return Boolean(token.image) || Boolean(token.svg) || (token.composition?.length ?? 0) > 0;
}

/** Una scelta manuale o un blocco non vengono spenti con la CAA automatica. */
export function isManualSymbol(token: UnitTokenLike): boolean {
  return Boolean(token.locked) || token.strategy === "manual";
}

/**
 * Unità visibili di un paragrafo.
 *
 * - le parole senza pittogramma restano testo normale, senza spazio riservato;
 * - i token nascosti non vengono disegnati (il testo resta selezionabile);
 * - con la CAA automatica spenta restano solo le scelte manuali;
 * - i range non validi (testo cambiato prima della rianalisi) vengono scartati;
 * - in caso di sovrapposizione parziale vince il token che inizia prima, così
 *   ProseMirror non deve gestire decorazioni parzialmente sovrapposte.
 */
export function unitsForParagraph<T extends UnitTokenLike>(
  tokens: T[],
  options: UnitOptions,
  textLength?: number
): UnitView<T>[] {
  if (options.mode === "testo") return [];
  const limit = Number.isFinite(textLength) && (textLength ?? 0) > 0 ? Number(textLength) : Infinity;
  const candidates: UnitView<T>[] = [];
  for (const token of tokens) {
    if (!token || token.is_punct) continue;
    if (token.hidden) continue;
    if (!tokenHasMedia(token)) continue;
    if (!options.autoEnabled && !isManualSymbol(token)) continue;
    if (!options.showExplicitSymbols && options.explicit?.(token)) continue;
    const start = Math.max(0, Math.floor(token.start));
    const end = Math.min(limit, Math.floor(token.end));
    if (!Number.isFinite(end) || end <= start) continue;
    candidates.push({
      token,
      start,
      end,
      key: symbolKey(token),
      marker: markerFor(token),
      neural: hasNeural(token),
      locked: Boolean(token.locked),
      approximate: Boolean(token.approximate),
      composition: (token.composition?.length ?? 0) > 0,
    });
  }
  candidates.sort((a, b) => a.start - b.start || b.end - a.end || a.token.index - b.token.index);
  const out: UnitView<T>[] = [];
  let coveredUntil = -1;
  for (const unit of candidates) {
    if (unit.start < coveredUntil) continue;
    out.push(unit);
    coveredUntil = unit.end;
  }
  return out;
}

export function hasNeural(token: UnitTokenLike): boolean {
  return Boolean(token.alternatives?.some((alternative) => alternative.neural));
}

/** Segni brevi mostrati accanto al pittogramma; il dettaglio resta nel tooltip. */
export function markerFor(token: UnitTokenLike): string {
  const marks: string[] = [];
  if (token.modifiers?.negation) marks.push("¬");
  if (token.modifiers?.plural) marks.push("×2");
  if (token.modifiers?.past) marks.push("⟲");
  if (token.modifiers?.future) marks.push("⟳");
  if (token.approximate) marks.push("≈");
  if (hasNeural(token)) marks.push("✦");
  return marks.join(" ");
}

/** Firma breve e stabile del disegno, usata come chiave della regola CSS. */
export function symbolKey(token: UnitTokenLike): string {
  const media = token.image
    ? `i${token.image.length}:${token.image.slice(0, 24)}:${token.image.slice(-32)}`
    : token.svg
      ? `s${token.svg.length}:${token.svg.slice(0, 32)}`
      : (token.composition ?? [])
          .map((part) =>
            part.image
              ? `i${part.image.length}:${part.image.slice(-24)}`
              : part.svg
                ? `s${part.svg.length}:${part.svg.slice(0, 24)}`
                : "?"
          )
          .join("|");
  return hash(`${token.symbol_id ?? ""}\u0001${media}`);
}

function hash(value: string): string {
  // FNV-1a a 32 bit: veloce, deterministico e sufficiente per i nomi delle
  // regole CSS. Non è una firma crittografica e non deve esserlo.
  let result = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    result ^= value.charCodeAt(index);
    result = Math.imul(result, 0x01000193) >>> 0;
  }
  return result.toString(36).padStart(7, "0");
}

/** Testo leggibile del tooltip, senza didascalie duplicate nel foglio. */
export function unitTitle(token: UnitTokenLike): string {
  const parts: string[] = [token.surface];
  const label = token.symbol_label ?? token.concept_label;
  if (label && label.toLocaleLowerCase("it") !== token.surface.toLocaleLowerCase("it")) {
    parts.push(label);
  }
  if (token.strategy && token.strategy !== "exact" && token.strategy !== "none") {
    parts.push(`strategia: ${token.strategy}`);
  }
  const modifiers = token.modifiers ?? {};
  const mods: string[] = [];
  if (modifiers.negation) mods.push("negazione");
  if (modifiers.plural) mods.push("plurale");
  if (modifiers.past) mods.push("passato");
  if (modifiers.future) mods.push("futuro");
  if (mods.length) parts.push(mods.join(", "));
  if (token.approximate) parts.push("pittogramma approssimativo");
  if (hasNeural(token)) parts.push("simboli affini suggeriti dal modello locale");
  if (token.locked) parts.push("pittogramma bloccato");
  return parts.filter(Boolean).join(" — ");
}
