# Changelog

Tutte le date sono in formato ISO (AAAA-MM-GG). Il progetto segue SemVer:
`MAJOR` per cambi incompatibili del formato o delle preferenze, `MINOR` per
funzionalità compatibili, `PATCH` per correzioni compatibili.

## 0.5.1 — 2026-11-05

### Lettura automatica delle frasi

- La frase veniva cercata durante `keydown`, quando il segno di
  punteggiatura non era ancora nel documento: il testo risultava incompleto e
  veniva scartato. Ora la lettura avviene subito dopo l'inserimento di
  `. ! ? : ;` e usa l'ultima frase chiusa prima del cursore.
- Su Invio si legge l'ultima frase del blocco corrente prima che ProseMirror
  lo divida; non vengono ripetute le frasi precedenti dello stesso paragrafo.
- L'ascolto degli eventi di digitazione è in fase di cattura, così Invio non
  va perso se ProseMirror lo gestisce.
- Se una frase viene chiusa con un segno e subito dopo si preme Invio, non
  viene letta due volte; due frasi uguali digitate di seguito restano due
  letture distinte.
- Estrattori in `src/lib/editor/sentence.ts` con 7 test dedicati
  (`tests/frontend/sentence.test.mjs`).

### Verifica

- Build 0 errori/0 avvisi; 23 test frontend passati.
- Nella finestra Tauri di sviluppo, con «Leggi la frase completata» attivo e
  la pronuncia delle singole parole disattivata, digitando «La sintesi vocale
  legge il testo.» il processo `say` è partito subito dopo il punto ed è
  rimasto attivo per circa 5 secondi.
- La lettura su Invio è coperta dai test di estrazione della frase; la
  pressione sintetica di Invio nel WebView non è risultata affidabile, quindi
  non è dichiarata come prova manuale riuscita.

### Unità semantiche multiparola

- Nuovi concetti e espressioni: computer portatile, sintesi vocale, telefono
  cellulare, carta di credito, mezzi di trasporto, sala da pranzo, macchina
  fotografica, scuola primaria, fare colazione, dare una mano, andare a
  dormire, da sole, non so, essere in grado di, avere bisogno di, prendere in
  giro. Ogni espressione è una sola unità CAA con un solo pittogramma e la
  didascalia è il testo scritto.
- «Computer portatile» usa un nuovo pittogramma originale del pacchetto core
  (`object.laptop`): ARASAAC non ha un simbolo laptop e «portatile» non viene
  più interpretato come telefono.
- I concetti con un simbolo ARASAAC esatto sono legati al pittogramma giusto
  (sintesi vocale, cellulare, carta di credito, mezzi di trasporto, sala da
  pranzo, macchina fotografica, far colazione, prendere in giro, bisogno,
  potere, stare da solo). «Scuola primaria» usa il concetto generale «scuola»
  con la nota dichiarata, perché il simbolo specifico non esiste.
- «Fare colazione» non mostra più il pittogramma di «pranzo»: il concetto è
  `food.breakfast`.
- Comandi **Unisci in un'unica unità CAA** e **Separa unità CAA** nel gruppo
  Gestione della scheda Pittogrammi. Unione e separazione sono transazioni
  ProseMirror: entrano in undo/redo, vengono salvate nel `.caadoc` e
  sopravvivono alla riapertura. La separazione manuale impedisce al motore di
  rifondere automaticamente quella singola occorrenza.

### Altre correzioni

- Apertura di un documento dalla schermata iniziale o dai recenti: il
  contenuto veniva impostato prima che l'editor fosse montato e il documento
  appariva vuoto. Ora si passa alla vista documento e si attende il mount.
- Dopo un'unione, una separazione o un annullamento il pannello si
  risincronizza sull'unità reale invece di mostrare una copia vecchia.
- Build: ProseMirror separato in un chunk dedicato (nessun avviso di dimensione).

### Test

- 154 test Rust, 23 frontend.
- Corpus semantico esteso a 212 frasi: prima scelta 98,11% (208/212),
  0 approssimazioni fuorvianti, 20/20 assenze rilevate; restano i 4 casi noti
  su «porta», «monopattino» e «succo».
- Verifica Tauri: «La sintesi vocale legge il testo. Uso un computer
  portatile.» mostra un solo pittogramma per espressione; separazione manuale
  di «sintesi vocale», unione di più parole, undo; salvataggio e riapertura
  conservano la separazione e l'unità laptop.

## 0.5.0 — 2026-11-05

### Ribbon orizzontale

- Il corpo del Ribbon è una fascia orizzontale a altezza fissa (86 px full,
  62 px compatto, 104 px touch): i gruppi non vanno più a capo e non
  sottraggono righe al documento.
- Ogni gruppo mostra i comandi principali; gli altri sono in un menu a
  comparsa «Altri», ancorato al pulsante, con chiusura su Esc/click esterno e
  navigazione da tastiera.
- Scorrimento orizzontale solo quando la larghezza non basta, con pulsanti
  laterali discreti, rotella/trackpad, frecce da tastiera e scrollbar nascosta.
- Stili con selettore compatto; menu di overflow per Appunti, Carattere,
  Paragrafo, Gestione, ecc.
- Modalità completa, compatta e minimizzata; in minimizzato il corpo è
  sovrapposto e non sposta il documento. Restano doppio clic per bloccare,
  `Ctrl/Cmd+F1` e la barra superiore minimizzabile con `Ctrl/Cmd+Shift+F1`.

### Associazioni dei pittogrammi

- Clic su un risultato applica subito il pittogramma; non serve una seconda
  conferma.
- Tre ambiti espliciti, mostrati prima dell'applicazione:
  - solo questa occorrenza (predefinito);
  - tutte le occorrenze equivalenti nel documento, per concetto/lemma, senza
    toccare le associazioni bloccate;
  - preferenza per i documenti futuri, legata al concetto (`concept:<id>`),
    così significati diversi dello stesso lemma non si contaminano.
- La verifica della selezione non blocca più l'applicazione se la rianalisi ha
  sostituito l'oggetto token: l'override per indice viene salvato e applicato
  appena possibile.
- Undo/redo delle associazioni: dopo un'applicazione, `Ctrl/Cmd+Z` annulla e
  `Ctrl/Cmd+Shift+Z` riapplica; un'eventuale modifica del testo riporta
  l'annullamento a ProseMirror. Comandi dedicati nel gruppo Gestione.
- Toast solo dopo il completamento; conferma «Associazione annullata» dopo
  l'undo.

### Animazioni e accessibilità

- Microanimazioni per menu, cambio scheda, pannelli, toast, pulsanti e
  comparsa del pittogramma appena applicato; la digitazione normale non anima
  i chip.
- Preferenza Animazioni complete / ridotte / disattivate; rispetto di
  `prefers-reduced-motion`.
- Suoni di interfaccia facoltativi, lettura della parola premendo spazio o
  Invio e lettura della frase completata (punto, punto e virgola, due punti,
  ritorno a capo), disattivati per default.

### Test

- 152 test Rust (incluso il test delle preferenze legate al concetto).
- 16 test frontend (selezione, workspace, equivalenza delle occorrenze).
- Build 0 errori/0 avvisi.
- Verifica Tauri: applicazione immediata di «comprendere» su «capire», seconda
  occorrenza invariata nell'ambito «solo questa», undo con Cmd+Z; Ribbon
  orizzontale con menu «Altri».
- Verifica mock: menu, overflow, altezza fissa, ambito documento, persistenza.

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
