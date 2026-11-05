# Changelog

Tutte le date sono in formato ISO (AAAA-MM-GG). Il progetto segue SemVer:
`MAJOR` per cambi incompatibili del formato o delle preferenze, `MINOR` per
funzionalità compatibili, `PATCH` per correzioni compatibili.

## 0.4.0 — 2026-11-05

### Workspace adattivo

- Nuovo `WorkspaceLayoutManager` (`src/lib/workspace/layout.ts`): unico stato
  reattivo e persistito con versione (`caa.workspace.v1`).
- Ribbon completo, compatto e minimizzato; in minimizzato il corpo si apre
  sovrapposto al documento e si chiude tornando a scrivere. Doppio clic sulla
  scheda per bloccare/sbloccare, `Ctrl/Cmd+F1` per comprimere, preferenza
  persistita.
- Barra di accesso rapido personalizzabile, con opzione per mostrare solo le
  icone o anche le etichette.
- Pannello laterale ancorato, sovrapposto o ridotto a una barra verticale di
  icone (Alternative, Ricerca, Impostazioni/IA, Studio). Larghezza 260–640 px,
  trascinamento, frecce da tastiera, pulsanti comprimi/ancora e memoria della
  larghezza.
- Su finestre sotto 1100 px il pannello ancorato diventa temporaneamente
  sovrapposto senza cambiare la preferenza; il binario delle icone è
  sovrapposto e non riduce il documento.
- Area documento realmente espandibile: la griglia si ricalcola, il foglio A4
  mantiene le sue dimensioni, zoom e scorrimento vengono conservati.
- Modalità **Concentrazione** distinta dallo schermo intero: nasconde Ribbon e
  pannelli, lascia il documento centrale, mantiene scorciatoie e una barra
  temporanea in alto; si esce con Esc o dal pulsante.
- Preset incorporati: Scrittura, Ricerca pittogrammi, Impaginazione,
  Presentazione, Concentrazione. Preset personalizzati salvabili e riapplicabili.
- `Ctrl/Cmd+Shift+F` attiva/disattiva la concentrazione; lo schermo intero usa
  le API native Tauri.

### Test

- 13 test frontend (selezione + workspace: limiti, persistenza, preset).
- 151 test Rust invariati; build 0 errori/0 avvisi.
- Verifica nel mock: compressione/espansione Ribbon, apertura temporanea,
  doppio clic, pannello a icona, overlay, chiusura con ritorno al documento,
  concentrazione ed Esc, preset, personalizzazione accesso rapido, persistenza
  dopo ricarica.

### Limiti

- I pannelli flottanti liberamente spostabili non sono inclusi in questa
  versione: sono coperti i modi ancorato, sovrapposto e compresso.
- Pannelli flottanti, verifica con screen reader reali e build Windows/Linux
  restano attività aperte.

## 0.3.0 — 2026-11-05

### Interfaccia Ribbon

- Nuova barra superiore a quattro aree: barra del documento, schede, Ribbon dei
  comandi, area di lavoro. Sostituisce la vecchia sequenza piatta di pulsanti.
- Sette schede: Home, Inserisci, Layout, Pittogrammi, Revisione, Strumenti IA,
  Visualizza. I comandi sono raggruppati con etichette e icone Lucide.
- Barra di accesso rapido con Salva, Annulla, Ripristina, Cerca, Ascolta,
  Esporta; personalizzabile e persistita in `localStorage`.
- Modalità Ribbon completo, compatto e nascosto; riduzione automatica sotto i
  1200 px e schede sempre raggiungibili. La modalità touch mostra etichette e
  bersagli più grandi.
- Schede contestuali «Pittogramma» (quando è selezionata una parola) e
  «Tabella» (quando il cursore è in una tabella).
- Comandi realmente operativi: formattazione, elenchi, rientri, interlinea,
  bordi di paragrafo, tabelle, interruzione di pagina, data/ora, Trova e
  sostituisci, conteggio parole, report semantici e di copertura, strumenti IA,
  zoom, pannello, touch, schermo intero, stampa/PDF.
- I comandi non ancora disponibili sono omessi oppure disabilitati con una
  spiegazione (selezione multipla, pronomi, avverbi).

### Editor e formattazione

- Nuovi mark ProseMirror `strike` e `highlight`.
- Attributi di paragrafo per rientro, interlinea, spaziatura, bordo e
  interruzione di pagina, serializzati nel documento.
- Scorciatoie `Ctrl+B/I/U`, `Ctrl+Shift+X`, `Ctrl+A`, `Ctrl+F`, `Ctrl+H`,
  `Ctrl+S`, `Ctrl+Shift+S`, `Ctrl+O`, `Ctrl+N`, `Ctrl+P`.
- Trova e sostituisci con conteggio, precedente/successivo, sostituzione
  singola e globale.
- Copia/taglia/incolla testo e copia/incolla formattazione.
- Layout: formato A4/A5/A3, margini, colore pagina, dimensione e spaziatura dei
  pittogrammi, allineamento, cinque stili di bordo con colore/spessore/stile/
  raggio/padding/sfondo, sei layout predefiniti.
- Didascalie spente per default; il testo digitato resta sotto il pittogramma.

### Dipendenze

- Aggiunto `lucide` (icone SVG, licenza ISC) e rimosso `lucide-svelte`, non
  compatibile con la modalità runes forzata dal progetto.

### Test

- 151 test Rust, 7 test Node della selezione, build 0 errori/0 avvisi.
- Verifica manuale nella finestra Tauri: schede, comandi del corpo, stato
  attivo, barra di accesso rapido, modalità compatta/nascosta, touch e scheda
  contestuale del pittogramma.

## 0.2.0 — 2026-11-05

### Editor e selezione

- **Qualsiasi parola è selezionabile**: clic o selezione nel testo aprono il
  pannello anche quando la parola non ha un pittogramma. Non serve più un chip
  preesistente.
- **Aggiunta di un pittogramma a una parola senza simbolo**: ricerca manuale,
  gruppi di affinità o alternative del repertorio; il simbolo viene applicato
  subito, il testo originale non cambia.
- **Selezione stabile**: prima di applicare una scelta il motore verifica che
  la parola all'indice selezionato sia ancora quella scelta; dopo una
  rianalisi non si applica più il simbolo alla parola sbagliata.
- **Nessuna terza riga**: la didascalia sotto il pittogramma è spenta per
  default. Restano il pittogramma e il testo digitato; l'etichetta originale
  del repertorio vive nei metadati (pannello, tooltip, licenze). La didascalia
  si può riattivare dalle impostazioni.
- **Stili di bordo persistenti**: nessun bordo, solo immagine, immagine e
  testo, bordi separati, bordo con divisore.

### Ricerca semantica

- **Affinità della parola** (`word_affinities`): gruppi esatto, lemma,
  varianti, sinonimi, concetto generale, affini; punteggio, motivo leggibile e
  indicazione di approssimazione; deduplica per simbolo.
- **Parole Fuori lessico**: «principesse» → «principessa» tramite regole non
  ambigue verificate sul repertorio; nessun concetto inventato.
- **Condizionale**: «assaggerei» viene ricondotto ad «assaggiare».
- **Sinonimi e concetti nuovi**: «capire» → «comprendere», «gelati» →
  «gelato», «non so» come espressione di incertezza, «da sole» come
  espressione (non il Sole).
- **Unità multiword**: le espressioni non ereditano più la funzione
  grammaticale della prima parola; «non so» non propaga la negazione al verbo
  successivo.

### Correzioni

- **Bundle macOS**: Tauri 2 colloca `bundle.resources` in
  `Contents/Resources/_up_/resources`; l'app confezionata si chiudeva
  all'avvio perché non trovava il lessico. La risoluzione delle risorse copre
  ora sviluppo, `_up_` e bundle; 3 test dedicati.
- **Seed idempotente**: concetti, lemmi, significati, flessioni e sinonimi non
  vengono duplicati quando un pacchetto viene reimportato.
- **Parser delle risposte neurali**: accetta elenchi su una riga separati da
  punti, punti esclamativi/interrogativi e barre.
- **Processo `llama-server`** fermato all'uscita dell'app.
- **Cache neurale** legata al modello attivo: cambiare GGUF non restituisce
  più risposte del modello precedente.

### Test

- 151 test di libreria Rust, corpus semantico, prestazioni, diagnostica ed esportazioni.
- 7 test Node della mappatura selezione → token (`npm run test:frontend`).
- Verifica manuale nella finestra Tauri: «capire» selezionato dal testo,
  pittogramma «comprendere» applicato, testo invariato, didascalie spente.

### Limiti noti

- Windows, Linux e macOS Intel non sono compilati su questa macchina; le build
  native sono affidate ai runner GitHub Actions.
- Nessuna firma o notarizzazione: mancano certificati.
- La ricerca online di immagini non è inclusa in questa versione.
- La copertura di sinonimi e relazioni dipende dal lessico incluso.
