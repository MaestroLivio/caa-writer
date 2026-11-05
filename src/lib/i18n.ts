// Lingua dell'interfaccia. Le stringhe vivono qui, non sparse nei componenti:
// aggiungere una lingua significa aggiungere un dizionario completo.
//
// Nota onesta: il motore semantico, i modelli e i contenuti dei materiali sono
// in italiano. La lingua inglese copre per ora l'interfaccia della schermata
// iniziale; le altre superfici restano italiane finché non vengono estratte.

import { writable } from "svelte/store";

export type Language = "it" | "en-GB";

const STORAGE_KEY = "caa.ui.language";

export interface UiStrings {
  tagline: string;
  openDocument: string;
  library: string;
  settings: string;
  createTitle: string;
  recentTitle: string;
  clearRecent: string;
  noText: string;
  recentEmpty: string;
  templatesTitle: string;
  templatesMissing: string;
  templatesNoMatch: string;
  searchPlaceholder: string;
  searchLabel: string;
  today: string;
  languageLabel: string;
  communicator: CommunicatorStrings;
  footerStats: (values: { symbols: number; concepts: number; lemmas: number }) => string;
  footerPacks: (values: { packs: number; expressions: number }) => string;
  footerLicense: string;
  cards: {
    word: CardStrings;
    grid: CardStrings;
    schedule: CardStrings;
    aba: CardStrings;
    teacch: CardStrings;
    smart: CardStrings;
    free: CardStrings;
  };
}

export interface CommunicatorStrings {
  dialogLabel: (title: string) => string;
  titleFallback: string;
  home: string;
  back: string;
  exit: string;
  page: (current: number, total: number) => string;
  gridLabel: string;
  add: (label: string) => string;
  unnamedSymbol: string;
  emptyPage: string;
  emptyMessage: string;
  messageLabel: string;
  removeLast: string;
  clear: string;
  speak: string;
}

export interface CardStrings {
  title: string;
  description: string;
  detail: string;
}

const it: UiStrings = {
  tagline: "Scrivere e comunicare con i pittogrammi, anche senza connessione.",
  openDocument: "Apri documento",
  library: "Libreria pittogrammi",
  settings: "Impostazioni",
  createTitle: "Cosa vuoi creare?",
  recentTitle: "Documenti recenti",
  clearRecent: "Svuota elenco",
  noText: "Documento senza testo.",
  recentEmpty:
    "Nessun documento recente. Il primo documento che salvi comparirà qui con un estratto del testo.",
  templatesTitle: "Modelli",
  templatesMissing: "Nessun modello caricato: verifica la cartella resources/templates.",
  templatesNoMatch: "Nessun modello corrisponde alla ricerca.",
  searchPlaceholder: "Cerca un modello…",
  searchLabel: "Cerca fra i modelli",
  today: "oggi",
  languageLabel: "Lingua dell'interfaccia",
  communicator: {
    dialogLabel: (title) => `Comunicatore: ${title}`,
    titleFallback: "Comunicatore",
    home: "Home",
    back: "Indietro",
    exit: "Esci dal comunicatore",
    page: (current, total) => `Pagina ${current} di ${total}`,
    gridLabel: "Celle della pagina",
    add: (label) => `Aggiungi ${label}`,
    unnamedSymbol: "simbolo senza testo",
    emptyPage: "Questa pagina non ha celle visibili.",
    emptyMessage: "Seleziona i pittogrammi per costruire il messaggio.",
    messageLabel: "Messaggio in costruzione",
    removeLast: "Cancella ultimo",
    clear: "Svuota",
    speak: "Parla",
  },
  footerStats: ({ symbols, concepts, lemmas }) =>
    `${symbols} pittogrammi · ${concepts} concetti · ${lemmas} lemmi`,
  footerPacks: ({ packs, expressions }) => `${packs} repertorio/i · ${expressions} espressioni`,
  footerLicense: "Pittogrammi originali e ARASAAC, CC BY-NC-SA 4.0.",
  cards: {
    word: {
      title: "Word Processor CAA",
      description: "Scrivi normalmente: i pittogrammi compaiono mentre digiti.",
      detail: "Simbolizzazione automatica attiva · aggiornamento contestuale",
    },
    grid: {
      title: "Tabelle di comunicazione",
      description: "Crea tabelle con pittogrammi per la comunicazione quotidiana.",
      detail: "Da modello, da descrizione o a mano",
    },
    schedule: {
      title: "Agende e calendari visivi",
      description: "Routine giornaliere, settimane, sequenze prima/dopo.",
      detail: "Organizzatori temporali stampabili",
    },
    aba: {
      title: "Materiali ABA",
      description: "Token board, schede di osservazione, analisi del compito.",
      detail: "Supporti per l'insegnamento strutturato",
    },
    teacch: {
      title: "Materiali TEACCH",
      description: "Sistemi di lavoro, sequenze operative, autonomia.",
      detail: "Organizzatori visivi passo per passo",
    },
    smart: {
      title: "Generazione intelligente",
      description: "Descrivi il materiale che ti serve e il programma lo prepara.",
      detail: "Es. «tabella con 20 attività per l'intervallo»",
    },
    free: {
      title: "Progettazione libera",
      description: "Disponi liberamente pittogrammi e didascalie su un foglio.",
      detail: "Griglia personalizzabile, pronta da stampare",
    },
  },
};

const enGB: UiStrings = {
  tagline: "Write and communicate with pictograms, even without a connection.",
  openDocument: "Open document",
  library: "Pictogram library",
  settings: "Settings",
  createTitle: "What would you like to create?",
  recentTitle: "Recent documents",
  clearRecent: "Clear list",
  noText: "Document without text.",
  recentEmpty:
    "No recent documents. The first document you save will appear here with an excerpt of its text.",
  templatesTitle: "Templates",
  templatesMissing: "No templates loaded: check the resources/templates folder.",
  templatesNoMatch: "No template matches your search.",
  searchPlaceholder: "Search for a template…",
  searchLabel: "Search templates",
  today: "today",
  languageLabel: "Interface language",
  communicator: {
    dialogLabel: (title) => `Communicator: ${title}`,
    titleFallback: "Communicator",
    home: "Home",
    back: "Back",
    exit: "Exit communicator",
    page: (current, total) => `Page ${current} of ${total}`,
    gridLabel: "Cells on this page",
    add: (label) => `Add ${label}`,
    unnamedSymbol: "symbol without text",
    emptyPage: "This page has no visible cells.",
    emptyMessage: "Select pictograms to build your message.",
    messageLabel: "Message being built",
    removeLast: "Delete last",
    clear: "Clear",
    speak: "Speak",
  },
  footerStats: ({ symbols, concepts, lemmas }) =>
    `${symbols} symbols · ${concepts} concepts · ${lemmas} lemmas`,
  footerPacks: ({ packs, expressions }) => `${packs} repertoire(s) · ${expressions} expressions`,
  footerLicense: "Original pictograms and ARASAAC, CC BY-NC-SA 4.0.",
  cards: {
    word: {
      title: "AAC Word Processor",
      description: "Write normally: pictograms appear as you type.",
      detail: "Automatic symbolisation on · contextual update",
    },
    grid: {
      title: "Communication boards",
      description: "Create boards with pictograms for everyday communication.",
      detail: "From a template, from a description, or by hand",
    },
    schedule: {
      title: "Visual schedules and calendars",
      description: "Daily routines, weeks, first/then sequences.",
      detail: "Printable time organisers",
    },
    aba: {
      title: "ABA materials",
      description: "Token boards, observation sheets, task analysis.",
      detail: "Supports for structured teaching",
    },
    teacch: {
      title: "TEACCH materials",
      description: "Work systems, step-by-step sequences, independence.",
      detail: "Step-by-step visual organisers",
    },
    smart: {
      title: "Smart generation",
      description: "Describe the material you need and the program prepares it.",
      detail: "E.g. \"board with 20 break-time activities\"",
    },
    free: {
      title: "Free design",
      description: "Arrange pictograms and captions freely on a sheet.",
      detail: "Customisable grid, ready to print",
    },
  },
};

export const strings: Record<Language, UiStrings> = { it, "en-GB": enGB };

function initial(): Language {
  if (typeof localStorage === "undefined") return "it";
  return localStorage.getItem(STORAGE_KEY) === "en-GB" ? "en-GB" : "it";
}

export const language = writable<Language>(initial());

export function setLanguage(next: Language) {
  language.set(next);
  if (typeof localStorage !== "undefined") localStorage.setItem(STORAGE_KEY, next);
  if (typeof document !== "undefined") {
    document.documentElement.lang = next === "en-GB" ? "en-GB" : "it";
  }
}
