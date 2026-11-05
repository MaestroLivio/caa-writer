// Scelta delle occorrenze per l'ambito «tutte le occorrenze nel documento».
//
// L'equivalenza non è una sostituzione testuale: conta il concetto quando è
// noto, altrimenti il lemma. Le unità bloccate manualmente non vengono toccate,
// tranne quella scelta esplicitamente dall'utente.

import type { CaaToken } from "../types";

export interface AssociationTarget {
  pid: string;
  token: CaaToken;
}

export function equivalentTargets(
  base: CaaToken,
  conceptId: string | null,
  basePid: string,
  tokensByPid: Map<string, CaaToken[]>
): AssociationTarget[] {
  const out: AssociationTarget[] = [];
  for (const [pid, list] of tokensByPid) {
    for (const token of list) {
      if (token.is_punct) continue;
      const isSelected = pid === basePid && token.index === base.index;
      if (token.locked && !isSelected) continue;
      let equivalent = isSelected;
      if (!equivalent && conceptId) {
        equivalent = token.concept_id === conceptId;
      }
      if (!equivalent && !conceptId && base.lemma && token.lemma) {
        equivalent = token.lemma === base.lemma;
      }
      if (!equivalent && !conceptId) {
        equivalent =
          token.surface.toLocaleLowerCase() === base.surface.toLocaleLowerCase();
      }
      if (equivalent) out.push({ pid, token });
    }
  }
  return out;
}
