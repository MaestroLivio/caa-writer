// Client del DecisionEngine per il frontend.
//
// Tutte le chiamate sono asincrone e non bloccano la digitazione. Chi chiama
// deve scartare le risposte obsolete tramite un proprio contatore e passare
// `cancel_key`: il backend annulla comunque la richiesta precedente.

import { api, isTauri } from "../api";
import type {
  Alternative,
  DecisionCandidate,
  DecisionRequest,
  DecisionResponse,
  DecisionStatus,
} from "../types";
import { charLength } from "./ranking";

let cachedStatus: DecisionStatus | null = null;
let statusRequest: Promise<DecisionStatus | null> | null = null;

/// Stato del motore, letto una volta per sessione.
export async function decisionStatus(refresh = false): Promise<DecisionStatus | null> {
  if (!isTauri()) return null;
  if (cachedStatus && !refresh) return cachedStatus;
  if (statusRequest && !refresh) return statusRequest;
  statusRequest = api
    .decisionStatus()
    .then((status) => {
      cachedStatus = status;
      return status;
    })
    .catch(() => null)
    .finally(() => {
      statusRequest = null;
    });
  return statusRequest;
}

/// Vero solo se Horizon è realmente disponibile in questa build.
export async function decisionUsable(): Promise<boolean> {
  const status = await decisionStatus();
  return Boolean(status?.neural);
}

export interface RerankParams {
  /// Frase o paragrafo noto al chiamante; se vuoto usa solo la parola.
  text: string;
  word: string;
  alternatives: Alternative[];
  /// Chiave stabile, es. `elemento@revisione`.
  cancelKey: string;
  requestId?: string;
  priority?: "high" | "medium" | "low";
  timeoutMs?: number;
}

function candidatesFromAlternatives(alternatives: Alternative[]): DecisionCandidate[] {
  const seen = new Set<string>();
  const candidates: DecisionCandidate[] = [];
  for (const item of alternatives) {
    if (!item.symbol_id || seen.has(item.symbol_id)) continue;
    seen.add(item.symbol_id);
    const label = (item.meaning || item.label || "").trim() || item.symbol_id;
    candidates.push({ id: item.symbol_id, label });
    if (candidates.length >= 32) break;
  }
  return candidates;
}

/// Riordina le alternative di un pittogramma rispetto alla frase.
export async function rerankPictograms(params: RerankParams): Promise<DecisionResponse | null> {
  if (!isTauri()) return null;
  const candidates = candidatesFromAlternatives(params.alternatives);
  if (candidates.length < 2) return null;
  const word = params.word.trim();
  if (!word) return null;
  const contextText = params.text.trim() || word;
  const index = contextText.indexOf(word);
  const start = index >= 0 ? charLength(contextText.slice(0, index)) : 0;
  const end = start + charLength(word);
  const request: DecisionRequest = {
    request_id: params.requestId,
    task: "pictogram_disambiguation",
    language: "it",
    input: {
      text: contextText,
      target: { text: word, start, end },
    },
    candidates,
    context: {
      element_id: params.cancelKey,
      cancel_key: params.cancelKey,
    },
    options: {
      mode: "suggest",
      multi_label: false,
      priority: params.priority ?? "high",
      timeout_ms: params.timeoutMs ?? 5000,
    },
  };
  try {
    return await api.decisionClassify(request);
  } catch {
    return null;
  }
}

/// Chiamata generica per gli altri task (intenzioni, storie, comandi, ...).
export async function classify(request: DecisionRequest): Promise<DecisionResponse | null> {
  if (!isTauri()) return null;
  try {
    return await api.decisionClassify(request);
  } catch {
    return null;
  }
}

/// Invalida lo stato memorizzato dopo installazione/rimozione del modello.
export function resetDecisionStatus() {
  cachedStatus = null;
  statusRequest = null;
}
