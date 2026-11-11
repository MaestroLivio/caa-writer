# CAA Writer 0.15.15 — note di rilascio

## Novità 0.15.15 — editor universale completo e installer snello

### Editor universale

- **Ogni elemento si modifica con il motore CAA condiviso**: testo, caselle,
  forme, fumetti, pulsanti e immagini accettano cursore dentro le parole,
  selezione, alternative per singolo pittogramma, blocco e visibilità. Non c'è
  un secondo editor: si apre lo stesso `CaaEditor` del word processor, e il
  risultato viene salvato in `metadata.caa` (`doc_json`, `plain_text`,
  `tokens`) dentro l'elemento.
- **Clic sul pittogramma**: da qualsiasi elemento si apre il pannello delle
  alternative; la sostituzione non riscrive la frase e si vede subito.
- **Griglie con celle vere**: 4×5 produce 20 celle indipendenti; +/− riga e
  colonna ricostruiscono la griglia, unisci/dividi cella funzionano, e ogni
  cella ha testo, testo pronunciato, pittogramma, sfondo, bordo, angoli e
  spaziatura propri. L'aspetto della cella è usato sia a schermo sia in
  stampa/PDF/PNG.
- **Pulsanti con azioni reali**: aggiungi alla frase, leggi, seleziona per
  esercizio, vai alla pagina, indietro, home, svuota. La stessa azione viene
  eseguita nel player.
- **Duplica elemento**: copia celle, simboli, stile e azione, con un piccolo scostamento per restare visibile.
- **Testo, tabelle e immagini** restano modificabili; il testo CAA di un
  elemento sopravvive a `update_element` e al salvataggio/riapertura
  (test Rust dedicato).

### Scheda «Inserisci»

Audit completo in `docs/AUDIT_INSERT_0_15_15.md`. I comandi del word processor
restano nella modalità testo (interruzione di pagina, data, ora, tabella,
pittogramma). Nella modalità documento la scheda diventa una galleria di
elementi reali: testo, casella, pulsante, forma, fumetto, immagine, segnaposto,
griglia e **pregrafismo**. L'unico comando dichiarato non disponibile è
«Pittogramma sulla selezione», con il motivo scritto nel suggerimento.

### Tipografia e accessibilità

- **Impostazioni → Aspetto e accessibilità → Tipografia**: scala
  dell'interfaccia 75–200 %, dimensione del testo dell'interfaccia, carattere
  dell'interfaccia; per i nuovi documenti carattere, corpo, interlinea e spazio
  dopo il paragrafo; dimensione dei pittogrammi e rapporto pittogramma/testo.
  Le preferenze sono globali e non riscrivono il testo già presente.
- **Catalogo di caratteri per categorie**: senza grazie, con grazie, alta
  leggibilità, scrittura scolastica, stampato, corsivo, pregrafismo,
  monospazio. Ogni voce dichiara licenza e fonte; nessun font viene scaricato
  durante la scrittura.
- **Foglio di pregrafismo**: lettere riempite con un motivo di punti vero,
  guide di scrittura, contorno continuo o tratteggiato, colore, corpo,
  spaziatura e raggio dei punti regolabili. Il foglio è un SVG generato dal
  backend, inserito come immagine modificabile e stampabile.

### Modalità Utilizzatore

- **Pannello Simboli in basso**: sempre presente nell'editor «Scrivo», con le
  stesse funzioni del pannello del testo libero. Selezionando una parola nel
  documento compaiono il pittogramma attribuito e le alternative prodotte dal
  motore, con blocco e visibilità; la ricerca manuale e il vocabolario
  applicano il simbolo alla parola selezionata o lo inseriscono nel punto del
  cursore.
- **Stesse regole CAA del Creator**: modalità, intensità, articoli,
  preposizioni, congiunzioni, repertorio, uso commerciale, disposizione dei
  pittogrammi e debounce arrivano dalle impostazioni condivise, non da una
  configurazione separata della modalità Utente.
- **Tastiera virtuale flottante**: parte ridotta a un indicatore, si apre solo
  su richiesta, si trascina, si ridimensiona, ha tasti ampi e ricorda posizione
  e dimensioni per ogni profilo.
- **Alternative per pittogramma**: disponibili solo se il profilo ha il
  permesso «Ricerca pittogrammi»; la scelta cambia il disegno, non il testo.

### Installer snello e pacchetti

- **L'installer non contiene più** la libreria materiali (~530 MB), il modello
  neurale (~470 MB) e le voci neurali. Contiene le risorse di base, il piccolo
  repertorio originale `core` e il runtime llama.cpp (senza modello).
- **Primo avvio**: una finestra elenca i pacchetti opzionali (libreria,
  ARASAAC, Global Symbols, modello neurale, voce). Si possono selezionare,
  rimandare o chiudere definitivamente. Nessun download parte da solo e il
  modello neurale non è mai preselezionato.
- **Gestore pacchetti** raggiungibile dalla Home Creator accanto alla Libreria:
  categorie per materiali, pittogrammi, modello neurale (con installazione
  reale dal catalogo modelli) e voci (stato reale del motore, senza pulsanti
  finti).
- **Libreria vuota senza errori**: se la libreria non è installata, l'app si
  apre normalmente e la Libreria propone «Apri Pacchetti» o la pagina della
  release. Il download automatico della libreria non è ancora implementato ed
  è dichiarato nell'interfaccia.


## Problemi riprodotti e cause

| Problema | Causa | Correzione |
|---|---|---|
| Il pittogramma di un pulsante/forma non si poteva cambiare senza perdere il tipo | `set_symbol` impostava `kind = "symbol"` | La sostituzione su elementi non-simbolo aggiorna solo i campi del pittogramma e conserva tipo e azione |
| La cella della griglia mostrava sempre un bordo di 1 px | Lo stile della cella era applicato solo in parte nel DOM | Bordo, angoli, spaziatura e misura del testo usano `cell.style`, sia a schermo sia nel renderer SVG |
| L'installer conteneva libreria e modello | `tauri.conf.json` includeva `resources/library/**/*` e `resources/ai/*` | Allowlist ridotta a risorse di base e `ai/runtime`; modello e libreria sono pacchetti opzionali |
| Su macOS la build falliva per il glob `resources/ai/*.dll` | Nessun file corrispondeva al pattern | Globs sostituiti con `resources/ai/runtime/**/*`; la cartella esiste su un clone nuovo grazie a `README.md` |
| La scheda Inserisci offriva etichette che non corrispondevano alle azioni | «Libreria ARASAAC» apriva la ricerca, «Modelli salvati» tornava alla Home | Etichette corrette in «Ricerca pittogrammi» e «Libreria materiali», collegate alle azioni reali |
| Il test frontend pretendeva la libreria nel bundle | Il test era fermo alla policy precedente | Ora verifica l'esclusione di libreria/modello e l'inclusione di core/runtime |


## Aggiornamento editor universale

La stessa build include il lavoro strutturale su testo ed elementi nella stessa pagina:

- **Disposizione rispetto al testo** per ogni elemento: in linea, sopra e sotto, testo intorno a
  sinistra/destra, davanti, dietro, posizione libera. Il campo `disposition` è salvato in
  `.caadoc`; i documenti precedenti restano validi (campo vuoto = comportamento storico).
- **Doppio clic in un'area libera** di una pagina non testuale crea un vero paragrafo di testo,
  non una casella invisibile. Nelle pagine di flusso il paragrafo entra in coda.
- **Frasi CAA nelle celle**: ogni cella può contenere una frase completa simbolizzata, con il
  pittogramma sopra ogni parola. Doppio clic sulla cella apre lo stesso `CaaEditor` del word
  processor; i singoli pittogrammi si sostituiscono con le stesse alternative e lo stesso ordine.
  Il renderer SVG/PDF/PNG disegna la frase con i simboli.
- **Tabelle** create come griglie di celle reali con prima riga di intestazione; **temi pulsante**
  pronti e testo alternativo; elenco degli elementi della pagina per selezionare anche gli
  oggetti coperti.
- **Pacchetti**: il catalogo è indipendente dalle installazioni. Un pacchetto rimosso resta
  visibile, con stato «Non installato», e si può reinstallare.

Dettagli tecnici e limiti in `docs/UNIVERSAL_LAYOUT_0_15_15.md`. Nuove verifiche:

```text
cargo test --lib                       237/237 passati
cargo test --test universal_documents  11/11 passati
npm run test:frontend                  65/65 passati
npm run build                          riuscito
build release macOS arm64              riuscita, app avviata
```

## Dimensioni reali

| Artefatto | 0.15.14 | 0.15.15 | Variazione |
|---|---|---|---|
| macOS arm64 `.dmg` | 813.780.634 byte (776,1 MiB) | 29.360.685 byte (28,0 MiB) | −96,4 % |
| macOS arm64 `.app` | ~896 MB | 74,97 MiB | — |

Misure ottenute dalla build release reale (`npm run tauri:build:mac-arm --
--bundles app,dmg`) e dagli artefatti 0.15.14 in `dist-release/`.

## Verifiche eseguite

```text
npm run check                          0 errori, 44 warning (13 file)
npm run test:frontend                  64/64 passati
npm run build                          riuscito (Vite)
cargo test --lib                       234/234 passati
cargo test --test universal_documents   10/10 passati
cargo build --bin caa-cli              riuscito
caa-cli info                           motore e repertori letti
python3 scripts/check_context.py       Contesto.md conforme (21/21)
```

Build release macOS arm64 riuscita; `.app` avviata (processo attivo) con le sole
risorse di base. La finestra non è stata pilotata a mano in questa sessione.

## Limiti dichiarati

- Windows x64 e macOS Intel non sono stati eseguiti su hardware reale in questa
  sessione; la compatibilità è verificata solo staticamente. Il pacchetto
  Windows 0.15.15 non è stato prodotto.
- macOS non è firmato né notarizzato: al primo avvio può servire «Apri comunque».
- La modifica del testo CAA dentro gli elementi e le sostituzioni di pittogramma
  sono coperte da test unitari, dal controllo tipi e dal round-trip Rust, ma non
  sono state riprovate cliccando nell'app Tauri in questa sessione.
- Il download automatico della libreria materiali e delle voci neurali non è
  implementato: il Gestore pacchetti rimanda alla pagina della release e mostra
  lo stato reale delle voci. Il modello neurale si installa dal catalogo modelli.
- Il corpus semantico non è stato rieseguito: richiede il catalogo ARASAAC
  locale, assente in un'installazione snella.
- Su un clone nuovo `resources/ai/runtime/` esiste grazie al suo `README.md`;
  i binari del runtime si scaricano con `python3 scripts/fetch_llama.py`.
