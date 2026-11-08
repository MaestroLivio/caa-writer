// Filtro dei contenuti espliciti/adulti per l'uso in classe.
// Non è un filtro linguistico generale: interviene solo su termini sessuali,
// anatomici espliciti e materiali per adulti. L'utente può riattivarli dalle
// impostazioni quando il contesto è appropriato.

const EXPLICIT_TERMS = [
  "vagina",
  "vaginale",
  "vulva",
  "clitoride",
  "clitoris",
  "pene",
  "penis",
  "testicolo",
  "testicoli",
  "scroto",
  "sperma",
  "spermatozoi",
  "erezione",
  "orgasmo",
  "sesso",
  "sessuale",
  "sessualità",
  "porno",
  "pornografia",
  "pornografico",
  "masturbazione",
  "masturbarsi",
  "nudo",
  "nuda",
  "nudi",
  "nude",
  "seno",
  "capezzolo",
  "capezzoli",
  "prostata",
  "utero",
  "mestruazioni",
  "mestruale",
  "pap test",
  "preservativo",
  "profilattico",
  "transessuale",
  "erotico",
  "erotica",
  "strip",
  "spogliarello",
  "prostituta",
  "prostituto",
  "bordello",
];

function normalize(value: string): string {
  return value
    .toLocaleLowerCase("it")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9àèéìòù\s']/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function isExplicitContent(...values: Array<string | null | undefined>): boolean {
  const text = normalize(values.filter(Boolean).join(" "));
  if (!text) return false;
  return EXPLICIT_TERMS.some((term) => {
    const normalized = normalize(term);
    if (normalized.includes(" ")) return text.includes(normalized);
    // Parola intera o inizio di parola (es. «vagina», «vaginale»).
    return new RegExp(`(^|\\s)${normalized}(\\s|$|')`).test(text);
  });
}

export function filterExplicit<T>(
  items: T[],
  textOf: (item: T) => Array<string | null | undefined>,
  includeExplicit: boolean,
): T[] {
  if (includeExplicit) return items;
  return items.filter((item) => !isExplicitContent(...textOf(item)));
}
