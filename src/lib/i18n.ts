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
  packages: string;
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
  libraryCardTitle: string;
  libraryCardDescription: string;
  libraryCardAction: string;
  today: string;
  languageLabel: string;
  communicator: CommunicatorStrings;
  footerStats: (values: { symbols: number; concepts: number; lemmas: number }) => string;
  footerPacks: (values: { packs: number; expressions: number }) => string;
  footerLicense: string;
  downloadCenter: DownloadCenterStrings;
  traceSheet: TraceSheetStrings;
  profileManager: ProfileManagerStrings;
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
  suggestIntent: string;
  intentLabel: (intent: string) => string;
  noIntent: string;
  suggestBoard: string;
  boardLabel: (board: string) => string;
  noBoard: string;
  suggestEmotion: string;
  emotionLabel: (emotion: string) => string;
  noEmotion: string;
}

export interface CardStrings {
  title: string;
  description: string;
  detail: string;
}

export interface DownloadCenterStrings {
  title: string;
  description: string;
  doNotClose: string;
  close: string;
  percent: string;
  doneOfTotal: (done: string, total: string) => string;
  active: string;
  completed: string;
  noDownloads: string;
  retry: string;
  allDone: string;
  allDoneHint: string;
  waitHint: string;
  continueOffline: string;
  continue: string;
  errorPrefix: string;
}

export interface TraceSheetStrings {
  title: string;
  description: string;
  close: string;
  textToTrace: string;
  sheetTitle: string;
  font: string;
  fontSize: string;
  dotRadius: string;
  dotSpacing: string;
  outline: string;
  color: string;
  margin: string;
  guides: string;
  dashedOutline: string;
  uppercase: string;
  pageWidth: string;
  pageHeight: string;
  hint: string;
  previewLabel: string;
  generating: string;
  noPreview: string;
  cancel: string;
  updatePreview: string;
  insertIntoDocument: string;
  sheetDimensions: (width: string, height: string, chars: string) => string;
  errorPrefix: string;
}

export interface ProfileManagerStrings {
  title: string;
  close: string;
  profileListLabel: string;
  newProfile: string;
  name: string;
  theme: string;
  editorLevel: string;
  enabledFeatures: string;
  features: {
    boards: string;
    editor: string;
    speech: string;
    compose: string;
    search: string;
    edit_symbols: string;
    save: string;
    export: string;
    print: string;
    agenda: string;
    calendar: string;
    social: string;
    didactic: string;
    online: string;
    ai: string;
  };
  editorLevels: {
    1: string;
    2: string;
    3: string;
  };
  themes: {
    infanzia: string;
    primaria: string;
    secondaria: string;
    adulto: string;
    contrasto: string;
    minimal: string;
  };
  assignedMaterials: (count: string) => string;
  assignedHint: string;
  noAssignedMaterials: string;
  removeFromProfile: string;
  addMaterials: string;
  closeMaterialsList: string;
  searchLibrary: string;
  addToProfile: string;
  allAssigned: string;
  saveProfile: string;
  deleteProfile: string;
  setAsDefault: string;
  confirmDelete: (name: string) => string;
  delete: string;
  cancel: string;
  creatorPinTitle: string;
  creatorPinHint: string;
  newPin: string;
  confirmPin: string;
  updatePin: string;
  pinRemoved: string;
  pinUpdated: string;
  pinTooShort: string;
  pinsMismatch: string;
  profileSaved: string;
  profileDeleted: string;
  materialRemoved: (title: string) => string;
  materialAdded: (title: string) => string;
  errorPrefix: string;
}

const it: UiStrings = {
  tagline: "Scrivere e comunicare con i pittogrammi, anche senza connessione.",
  openDocument: "Apri documento",
  library: "Libreria dei materiali",
  packages: "Pacchetti",
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
  libraryCardTitle: "Libreria dei materiali",
  libraryCardDescription:
    "Sfoglia tabelle, quaderni, agende e materiali per categoria, pubblico e obiettivo. Il browser apre anteprime e editor.",
  libraryCardAction: "Apri il browser della libreria",
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
    suggestIntent: "Suggerisci intenzione",
    intentLabel: (intent) => `Intenzione suggerita: ${intent}`,
    noIntent: "Nessuna intenzione con confidenza sufficiente.",
    suggestBoard: "Suggerisci tabella",
    boardLabel: (board) => `Tabella suggerita: ${board}`,
    noBoard: "Nessuna tabella con confidenza sufficiente.",
    suggestEmotion: "Suggerisci emozione",
    emotionLabel: (emotion) => `Emozione espressa: ${emotion}`,
    noEmotion: "Nessuna emozione con confidenza sufficiente.",
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
  downloadCenter: {
    title: "Download dei pacchetti",
    description: "Puoi avviare più download contemporaneamente.",
    doNotClose: "<strong>Non chiudere la finestra</strong> finché la barra non arriva al 100%: i file vengono salvati nella cartella dati dell'utente e restano disponibili offline.",
    close: "Chiudi",
    percent: "{value}%",
    doneOfTotal: (done, total) => `· ${done} / ${total}`,
    active: "· download in corso",
    completed: "· completato",
    noDownloads: "Nessun download in corso.",
    retry: "Riprova",
    allDone: "Tutti i download sono terminati. Puoi continuare a usare il programma offline.",
    allDoneHint: "Tutti i download sono terminati. Puoi continuare a usare il programma offline.",
    waitHint: "Puoi continuare a usare le funzioni già installate, ma non chiudere la finestra.",
    continueOffline: "Continua offline",
    continue: "Continua",
    errorPrefix: "Errore: ",
  },
  traceSheet: {
    title: "Pregrafismo e lettere a punti",
    description: "Lettere riempite con punti, guide di scrittura e contorno regolabile.",
    close: "Chiudi",
    textToTrace: "Testo da tracciare",
    sheetTitle: "Titolo del foglio",
    font: "Carattere",
    fontSize: "Corpo (mm)",
    dotRadius: "Raggio punto (mm)",
    dotSpacing: "Distanza punti (mm)",
    outline: "Contorno (mm)",
    color: "Colore",
    margin: "Margine (mm)",
    guides: "Guide di scrittura",
    dashedOutline: "Contorno tratteggiato",
    uppercase: "Solo maiuscole",
    pageWidth: "Larghezza foglio (mm)",
    pageHeight: "Altezza foglio (mm)",
    hint: "Le lettere sono disegnate con il carattere scelto: se non è installato nel sistema, il foglio usa il ripiego e resta leggibile.",
    previewLabel: "Anteprima del foglio",
    generating: "Generazione del foglio…",
    noPreview: "Nessuna anteprima disponibile.",
    cancel: "Annulla",
    updatePreview: "Aggiorna anteprima",
    insertIntoDocument: "Inserisci nel documento",
    sheetDimensions: (width, height, chars) => `Foglio ${width}×${height} mm · ${chars} caratteri`,
    errorPrefix: "Errore: ",
  },
  profileManager: {
    title: "Gestione utenti",
    close: "Chiudi",
    profileListLabel: "Profili",
    newProfile: "+ Nuovo profilo",
    name: "Nome o pseudonimo",
    theme: "Tema",
    editorLevel: "Livello editor",
    enabledFeatures: "Funzionalità abilitate",
    features: {
      boards: "Tabelle comunicative",
      editor: "Editor libero",
      speech: "Sintesi vocale",
      compose: "Componi frase",
      search: "Ricerca pittogrammi",
      edit_symbols: "Modifica pittogrammi",
      save: "Salvataggio personale",
      export: "Esportazione",
      print: "Stampa",
      agenda: "Agenda",
      calendar: "Calendario",
      social: "Storie sociali",
      didactic: "Attività didattiche",
      online: "Ricerca online",
      ai: "Strumenti IA",
    },
    editorLevels: {
      1: "1 · Essenziale",
      2: "2 · Guidato",
      3: "3 · Avanzato",
    },
    themes: {
      infanzia: "Infanzia",
      primaria: "Scuola primaria",
      secondaria: "Scuola secondaria",
      adulto: "Adulto",
      contrasto: "Alto contrasto",
      minimal: "Minimalista",
    },
    assignedMaterials: (count) => `Materiali assegnati (${count})`,
    assignedHint: "Il profilo vede nella propria schermata solo questi documenti. Puoi rimuoverli dall'elenco in qualsiasi momento; gli originali restano nella libreria.",
    noAssignedMaterials: "Nessun materiale assegnato a questo profilo.",
    removeFromProfile: "Rimuovi",
    addMaterials: "Aggiungi materiali",
    closeMaterialsList: "Chiudi l'elenco dei materiali",
    searchLibrary: "Cerca nella libreria…",
    addToProfile: "Aggiungi",
    allAssigned: "Tutti i materiali trovati sono già assegnati.",
    saveProfile: "Salva profilo",
    deleteProfile: "Elimina",
    setAsDefault: "Imposta come predefinito",
    confirmDelete: (name) => `Eliminare il profilo «${name}»? I materiali originali restano.`,
    delete: "Elimina",
    creatorPinTitle: "Protezione Area Creator",
    creatorPinHint: "Il PIN protegge le operazioni amministrative e l'accesso all'Area Creator.",
    newPin: "Nuovo PIN (vuoto = rimuovi)",
    confirmPin: "Conferma PIN",
    updatePin: "Aggiorna PIN",
    pinRemoved: "PIN rimosso.",
    pinUpdated: "PIN aggiornato.",
    pinTooShort: "Il PIN deve avere almeno 4 caratteri.",
    pinsMismatch: "I due PIN non coincidono.",
    profileSaved: "Profilo salvato.",
    profileDeleted: "Profilo eliminato.",
    materialRemoved: (title) => `«${title}» rimosso dal profilo.`,
    materialAdded: (title) => `«${title}» aggiunto: salva il profilo per confermare.`,
    errorPrefix: "Errore: ",
    cancel: "Annulla",
  },
};

const enGB: UiStrings = {
  tagline: "Write and communicate with pictograms, even without a connection.",
  openDocument: "Open document",
  library: "Materials library",
  packages: "Packages",
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
  libraryCardTitle: "Materials library",
  libraryCardDescription:
    "Browse tables, notebooks, schedules and materials by category, audience and goal. The browser opens previews and the editor.",
  libraryCardAction: "Open the library browser",
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
    suggestIntent: "Suggest intent",
    intentLabel: (intent) => `Suggested intent: ${intent}`,
    noIntent: "No intent with sufficient confidence.",
    suggestBoard: "Suggest board",
    boardLabel: (board) => `Suggested board: ${board}`,
    noBoard: "No board with sufficient confidence.",
    suggestEmotion: "Suggest emotion",
    emotionLabel: (emotion) => `Expressed emotion: ${emotion}`,
    noEmotion: "No emotion with sufficient confidence.",
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
  downloadCenter: {
    title: "Package Downloads",
    description: "You can start multiple downloads at once.",
    doNotClose: "<strong>Do not close the window</strong> until the bar reaches 100%: files are saved to the user data folder and remain available offline.",
    close: "Close",
    percent: "{value}%",
    doneOfTotal: (done, total) => `· ${done} / ${total}`,
    active: "· downloading",
    completed: "· completed",
    noDownloads: "No downloads in progress.",
    retry: "Retry",
    allDone: "All downloads are finished. You can continue using the program offline.",
    allDoneHint: "All downloads are finished. You can continue using the program offline.",
    waitHint: "You can continue using installed features, but do not close the window.",
    continueOffline: "Continue offline",
    continue: "Continue",
    errorPrefix: "Error: ",
  },
  traceSheet: {
    title: "Pre-writing and dotted letters",
    description: "Letters filled with dots, writing guides and adjustable outline.",
    close: "Close",
    textToTrace: "Text to trace",
    sheetTitle: "Sheet title",
    font: "Font",
    fontSize: "Font size (mm)",
    dotRadius: "Dot radius (mm)",
    dotSpacing: "Dot spacing (mm)",
    outline: "Outline (mm)",
    color: "Color",
    margin: "Margin (mm)",
    guides: "Writing guides",
    dashedOutline: "Dashed outline",
    uppercase: "Uppercase only",
    pageWidth: "Page width (mm)",
    pageHeight: "Page height (mm)",
    hint: "Letters are drawn with the chosen font: if it is not installed on the system, the sheet uses the fallback and remains readable.",
    previewLabel: "Sheet preview",
    generating: "Generating sheet…",
    noPreview: "No preview available.",
    cancel: "Cancel",
    updatePreview: "Update preview",
    insertIntoDocument: "Insert into document",
    sheetDimensions: (width, height, chars) => `Sheet ${width}×${height} mm · ${chars} characters`,
    errorPrefix: "Error: ",
  },
  profileManager: {
    title: "User management",
    close: "Close",
    profileListLabel: "Profiles",
    newProfile: "+ New profile",
    name: "Name or pseudonym",
    theme: "Theme",
    editorLevel: "Editor level",
    enabledFeatures: "Enabled features",
    features: {
      boards: "Communication boards",
      editor: "Free editor",
      speech: "Speech synthesis",
      compose: "Compose sentence",
      search: "Pictogram search",
      edit_symbols: "Edit pictograms",
      save: "Personal save",
      export: "Export",
      print: "Print",
      agenda: "Agenda",
      calendar: "Calendar",
      social: "Social stories",
      didactic: "Didactic activities",
      online: "Online search",
      ai: "AI tools",
    },
    editorLevels: {
      1: "1 · Essential",
      2: "2 · Guided",
      3: "3 · Advanced",
    },
    themes: {
      infanzia: "Early childhood",
      primaria: "Primary school",
      secondaria: "Secondary school",
      adulto: "Adult",
      contrasto: "High contrast",
      minimal: "Minimal",
    },
    assignedMaterials: (count) => `Assigned materials (${count})`,
    assignedHint: "The profile only sees these documents on its screen. You can remove them from the list at any time; the originals remain in the library.",
    noAssignedMaterials: "No materials assigned to this profile.",
    removeFromProfile: "Remove",
    addMaterials: "Add materials",
    closeMaterialsList: "Close materials list",
    searchLibrary: "Search library…",
    addToProfile: "Add",
    allAssigned: "All found materials are already assigned.",
    saveProfile: "Save profile",
    deleteProfile: "Delete",
    setAsDefault: "Set as default",
    confirmDelete: (name) => `Delete profile «${name}»? Original materials remain.`,
    delete: "Delete",
    creatorPinTitle: "Creator Area Protection",
    creatorPinHint: "The PIN protects administrative operations and access to the Creator Area.",
    newPin: "New PIN (empty = remove)",
    confirmPin: "Confirm PIN",
    updatePin: "Update PIN",
    pinRemoved: "PIN removed.",
    pinUpdated: "PIN updated.",
    pinTooShort: "The PIN must be at least 4 characters long.",
    pinsMismatch: "The two PINs do not match.",
    profileSaved: "Profile saved.",
    profileDeleted: "Profile deleted.",
    materialRemoved: (title) => `«${title}» removed from profile.`,
    materialAdded: (title) => `«${title}» added: save the profile to confirm.`,
    errorPrefix: "Error: ",
    cancel: "Cancel",
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
