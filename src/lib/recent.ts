// Documenti recenti e modelli preferiti: solo preferenze locali del client.
// Nessun contenuto viene inviato altrove; l'anteprima è un estratto del testo.

export interface RecentDoc {
  path: string;
  title: string;
  excerpt: string;
  at: number;
  kind: "editor" | "board";
}

const RECENT_KEY = "caa.recent";
const FAVORITES_KEY = "caa.favorite-templates";
const MAX_RECENT = 8;

function readJson<T>(key: string, fallback: T): T {
  if (typeof localStorage === "undefined") return fallback;
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

function writeJson(key: string, value: unknown) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* quota piena: le preferenze non devono bloccare l'uso */
  }
}

export function loadRecent(): RecentDoc[] {
  const items = readJson<RecentDoc[]>(RECENT_KEY, []);
  return Array.isArray(items)
    ? items
        .filter((item) => item && typeof item.path === "string" && item.path.length > 0)
        .sort((a, b) => b.at - a.at)
        .slice(0, MAX_RECENT)
    : [];
}

export function recordRecent(doc: { path: string; title: string; excerpt: string; kind?: "editor" | "board" }) {
  const items = loadRecent().filter((item) => item.path !== doc.path);
  const excerpt = doc.excerpt.replace(/\s+/g, " ").trim().slice(0, 180);
  items.unshift({
    path: doc.path,
    title: doc.title || doc.path.split(/[\\/]/).pop() || "Documento",
    excerpt,
    at: Date.now(),
    kind: doc.kind ?? "editor",
  });
  writeJson(RECENT_KEY, items.slice(0, MAX_RECENT));
}

export function removeRecent(path: string) {
  writeJson(
    RECENT_KEY,
    loadRecent().filter((item) => item.path !== path)
  );
}

export function clearRecent() {
  writeJson(RECENT_KEY, []);
}

export function loadFavorites(): string[] {
  const ids = readJson<string[]>(FAVORITES_KEY, []);
  return Array.isArray(ids) ? ids.filter((id) => typeof id === "string") : [];
}

export function toggleFavorite(id: string): string[] {
  const ids = loadFavorites();
  const next = ids.includes(id) ? ids.filter((item) => item !== id) : [...ids, id];
  writeJson(FAVORITES_KEY, next);
  return next;
}
