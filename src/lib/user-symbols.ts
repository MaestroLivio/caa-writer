// Scelte dei pittogrammi nella modalità Utilizzatore.
//
// Il pannello in basso non inventa un ordinamento: parte dalle alternative
// prodotte dal motore CAA per la parola selezionata, mette in testa il
// pittogramma già attribuito e toglie i duplicati. È la stessa regola usata dal
// pannello Simbolo del testo libero.

import type { Alternative, CaaToken, Profile } from "./types";

export function canChooseAlternatives(profile: Pick<Profile, "features"> | null | undefined): boolean {
  return Boolean(profile?.features?.includes("search"));
}

export function symbolChoices(token: CaaToken | null | undefined): Alternative[] {
  if (!token) return [];
  const current: Alternative | null = token.symbol_id
    ? {
        symbol_id: token.symbol_id,
        concept_id: token.concept_id ?? null,
        label: token.symbol_label ?? token.concept_label ?? token.surface,
        meaning: token.meaning ?? "",
        pack_id: token.pack_id ?? "",
        pack_name: "",
        format: token.image ? "png" : "svg",
        svg: token.svg ?? null,
        image: token.image ?? null,
        approximate: token.approximate,
        license: "",
        source_url: null,
        author: null,
      }
    : null;
  const result: Alternative[] = current ? [current] : [];
  const seen = new Set(result.map((item) => item.symbol_id));
  for (const item of token.alternatives ?? []) {
    if (seen.has(item.symbol_id)) continue;
    seen.add(item.symbol_id);
    result.push(item);
  }
  return result;
}
