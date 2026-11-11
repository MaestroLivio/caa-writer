// Catalogo tipografico dell'interfaccia e dei documenti.
//
// Le famiglie sono raggiungibili per nome: se il carattere non è installato
// nel sistema, la pila di fallback mantiene il documento leggibile. Nessun
// font viene scaricato durante la scrittura. Le voci di alta leggibilità e
// pregrafismo indicano la licenza e la fonte, così una scuola può installarle
// legalmente nella propria copia.

export interface FontChoice {
  label: string;
  stack: string;
  license: string;
  source: string;
}

export interface FontGroup {
  id: string;
  label: string;
  note: string;
  fonts: FontChoice[];
}

const SYSTEM = {
  label: "Carattere di sistema",
  stack: "",
  license: "—",
  source: "",
};

export const interfaceFontGroups: FontGroup[] = [
  {
    id: "interfaccia",
    label: "Interfaccia",
    note: "Il carattere dell'app; il documento può usarne un altro.",
    fonts: [
      SYSTEM,
      { label: "Inter", stack: '"Inter", "Segoe UI", system-ui, sans-serif', license: "SIL OFL 1.1", source: "https://rsms.me/inter/" },
      { label: "Atkinson Hyperlegible", stack: '"Atkinson Hyperlegible", Verdana, sans-serif', license: "SIL OFL 1.1", source: "https://brailleinstitute.org/freefont" },
      { label: "OpenDyslexic", stack: '"OpenDyslexic", "Atkinson Hyperlegible", sans-serif', license: "SIL OFL 1.1", source: "https://opendyslexic.org/" },
      { label: "Verdana", stack: "Verdana, Geneva, sans-serif", license: "Font di sistema", source: "" },
    ],
  },
];

export const documentFontGroups: FontGroup[] = [
  {
    id: "sans",
    label: "Senza grazie",
    note: "Lettura quotidiana, titoli, tabelle.",
    fonts: [
      SYSTEM,
      { label: "Arial / Helvetica", stack: "Arial, Helvetica, sans-serif", license: "Font di sistema", source: "" },
      { label: "Verdana", stack: "Verdana, Geneva, sans-serif", license: "Font di sistema", source: "" },
      { label: "Calibri / Carlito", stack: 'Calibri, Carlito, "Segoe UI", sans-serif', license: "Carlito: SIL OFL 1.1", source: "https://fonts.google.com/specimen/Carlito" },
      { label: "Inter", stack: '"Inter", "Segoe UI", system-ui, sans-serif', license: "SIL OFL 1.1", source: "https://rsms.me/inter/" },
      { label: "Lexend", stack: '"Lexend", "Inter", sans-serif', license: "SIL OFL 1.1", source: "https://fonts.google.com/specimen/Lexend" },
    ],
  },
  {
    id: "serif",
    label: "Con grazie",
    note: "Testi lunghi e letture ad alta voce.",
    fonts: [
      { label: "Georgia", stack: 'Georgia, "Times New Roman", serif', license: "Font di sistema", source: "" },
      { label: "Times New Roman / Liberation Serif", stack: '"Times New Roman", "Liberation Serif", serif', license: "Liberation: SIL OFL 1.1", source: "https://github.com/liberationfonts/liberation-fonts" },
      { label: "Merriweather", stack: 'Merriweather, Georgia, serif', license: "SIL OFL 1.1", source: "https://fonts.google.com/specimen/Merriweather" },
    ],
  },
  {
    id: "leggibilita",
    label: "Alta leggibilità",
    note: "Lettere distinguibili, utile per dislessia e ipovisione.",
    fonts: [
      { label: "Atkinson Hyperlegible", stack: '"Atkinson Hyperlegible", Verdana, sans-serif', license: "SIL OFL 1.1", source: "https://brailleinstitute.org/freefont" },
      { label: "OpenDyslexic", stack: '"OpenDyslexic", "Atkinson Hyperlegible", sans-serif', license: "SIL OFL 1.1", source: "https://opendyslexic.org/" },
      { label: "Andika", stack: 'Andika, "Atkinson Hyperlegible", sans-serif', license: "SIL OFL 1.1", source: "https://software.sil.org/andika/" },
    ],
  },
  {
    id: "scuola",
    label: "Scrittura scolastica",
    note: "Caratteri per leggere e ricalcare le lettere.",
    fonts: [
      { label: "Andika (stampato)", stack: 'Andika, "Century Gothic", sans-serif', license: "SIL OFL 1.1", source: "https://software.sil.org/andika/" },
      { label: "Edu QLD Beginner", stack: '"Edu QLD Beginner", Andika, sans-serif', license: "SIL OFL 1.1", source: "https://fonts.google.com/specimen/Edu+QLD+Beginner" },
      { label: "Caveat (corsivo)", stack: 'Caveat, "Segoe Print", cursive', license: "SIL OFL 1.1", source: "https://fonts.google.com/specimen/Caveat" },
      { label: "Patrick Hand", stack: '"Patrick Hand", Caveat, cursive', license: "SIL OFL 1.1", source: "https://fonts.google.com/specimen/Patrick+Hand" },
    ],
  },
  {
    id: "maiuscolo",
    label: "Stampato maiuscolo e minuscolo",
    note: "Modelli con lettere separate e senza grazie.",
    fonts: [
      { label: "Century Gothic", stack: '"Century Gothic", "URW Gothic", sans-serif', license: "Font di sistema / URW: AGPL con eccezione font", source: "https://www.urwpp.de/" },
      { label: "Andika", stack: 'Andika, "Century Gothic", sans-serif', license: "SIL OFL 1.1", source: "https://software.sil.org/andika/" },
      { label: "Verdana", stack: "Verdana, Geneva, sans-serif", license: "Font di sistema", source: "" },
    ],
  },
  {
    id: "corsivo",
    label: "Corsivo",
    note: "Per firme, didascalie e testi espressivi.",
    fonts: [
      { label: "Caveat", stack: "Caveat, cursive", license: "SIL OFL 1.1", source: "https://fonts.google.com/specimen/Caveat" },
      { label: "Dancing Script", stack: '"Dancing Script", Caveat, cursive', license: "SIL OFL 1.1", source: "https://fonts.google.com/specimen/Dancing+Script" },
    ],
  },
  {
    id: "pregrafismo",
    label: "Pregrafismo",
    note: "Tratteggio e guide: il carattere resta leggibile, il foglio si stampa.",
    fonts: [
      { label: "Andika (guide)", stack: 'Andika, "Century Gothic", sans-serif', license: "SIL OFL 1.1", source: "https://software.sil.org/andika/" },
      { label: "Senza grazie", stack: "Arial, Helvetica, sans-serif", license: "Font di sistema", source: "" },
    ],
  },
  {
    id: "mono",
    label: "Monospazio",
    note: "Allineamenti tecnici, codici, tabelle.",
    fonts: [
      { label: "Monospazio di sistema", stack: "ui-monospace, SFMono-Regular, Menlo, monospace", license: "Font di sistema", source: "" },
      { label: "Courier New / Liberation Mono", stack: '"Courier New", "Liberation Mono", monospace', license: "Liberation: SIL OFL 1.1", source: "https://github.com/liberationfonts/liberation-fonts" },
    ],
  },
];

/// Etichetta leggibile per una pila già salvata; se non è nel catalogo viene
/// mostrata la pila stessa, senza inventare un nome.
export function fontLabel(stack: string): string {
  if (!stack) return SYSTEM.label;
  const all = [...interfaceFontGroups, ...documentFontGroups].flatMap((group) => group.fonts);
  return all.find((font) => font.stack === stack)?.label ?? stack;
}
