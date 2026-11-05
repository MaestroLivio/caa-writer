# CAA Writer — Guida per sviluppatori

## 1. Architettura

```text
┌──────────────────────────── frontend (Svelte 5 + TS) ────────────────────────────┐
│ App.svelte            stato documento, comandi, esportazione                     │
│ lib/editor/           ProseMirror: schema con `pid`, strisce CAA, formattazione   │
│ lib/components/       pannello laterale: alternative, ricerca, impostazioni, studio│
│ lib/api.ts            ponte tipizzato verso i comandi Tauri                       │
│ lib/raster.ts         SVG → PNG (solo per DOCX)                                   │
└───────────────────────────────────┬───────────────────────────────────────────────┘
                                    │ invoke (camelCase → snake_case)
┌───────────────────────────────────▼───────────────────────────────────────────────┐
│ commands.rs           31 comandi Tauri                                            │
│ engine.rs             orchestrazione: cache, preferenze, fallback, rerank IA      │
│ linguistics/          tokenizer, lessico, morfologia a regole, analizzatore       │
│ symbol_search/        punteggio dei concetti e scala di fallback A–G              │
│ database/             schema, migrazioni versionate, seed, FTS5, preferenze       │
│ asset_manager/        SVG: validazione, sanitizzazione, cache LRU                 │
│ ai_engine/            llama.cpp via processo CLI, risposte JSON validate          │
│ export/               .caadoc, DOCX, HTML, scrittura atomica                      │
│ speech/               sintesi vocale di sistema                                   │
│ studio.rs             prompt → specifica JSON deterministica → layout             │
└───────────────────────────────────────────────────────────────────────────────────┘
```

Il modello documentale è **ProseMirror JSON**: il testo vive nell'editor, la rappresentazione
pittografica vive in `CaaToken` calcolati per paragrafo e legati a un `pid` stabile. Nessun simbolo
è scritto dentro il testo: cambiare modalità non altera il contenuto.

## 2. Pipeline semantica

1. **Analisi** (`linguistics/analyzer.rs`): frasi, token, lemmi, POS, negazioni, domini lessicali,
   espressioni multiword.
2. **Ricerca** (`engine::resolve_token`): concetti del lemma → sinonimi → concetti correlati.
3. **Contesto** (`symbol_search::rank_concepts`): punteggio da cues, dominio della frase, specificità,
   frequenza d'uso.
4. **Selezione**: preferenze documento > contesto > globale; quindi punteggio.
5. **Validazione**: il modello riceve solo `concept_id` candidati e può solo riordinarli; la risposta
   è JSON validato, altrimenti si usa la scelta deterministica.

Fallback: `exact` → `synonym` → `base_form` → `broader` → `composition` → `decomposition` →
`related` (marcato approssimativo) → nessun simbolo. Le approssimazioni sono sempre dichiarate
(`approximate: true`, bordo tratteggiato, `≈` nella didascalia).

La `confidence` restituita è un **indice interno** derivato dal punteggio: non è una probabilità
calibrata e non va presentata come tale.

## 3. Database

SQLite con FTS5 (`unicode61 remove_diacritics 2`). Tabelle principali: `symbols`, `concepts`,
`meanings`, `lemmas`, `inflections`, `synonyms`, `semantic_relations`, `translations`,
`symbol_assets`, `symbol_licenses`, `symbol_preferences`, `document_overrides`,
`generated_symbols`, `symbol_packs`, `search_statistics`, `settings`, e le tabelle FTS.

- Migrazioni versionate in `database/migrations.rs` (`schema_version` in `settings`).
- Preferenze: `symbol_preferences(scope, lemma, context_key, symbol_id, locked)`.
- Override di documento: `document_overrides(doc_key, unit_key, symbol_id, concept_id, locked)`;
  `unit_key` è `"{paragraph_id}:{token_index}"`, coerente con il frontend.
- Il database utente vive in `app_data_dir`; le risorse sono in sola lettura accanto al binario.
- Per ripristinare la cartella dati eliminare `caa-writer.sqlite3`: viene ricreato al successivo
  avvio con il repertorio incluso.

## 4. Frontend

- Svelte 5 con rune (`$state`, `$props`, `$effect`); TypeScript rigoroso; nessuna libreria UI esterna.
- Editor: schema ProseMirror in `lib/editor/schema.ts`. Ogni paragrafo/titolo ha `pid`; il testo
  viene analizzato per paragrafo con debounce (~240 ms) e cache per firma `testo + opzioni`.
- Le strisce CAA sono elementi DOM (`div.caa-strip`) inseriti prima o dopo il blocco, con chip
  `button` accessibili da tastiera. Nessuna immagine viene caricata da rete: l'SVG arriva dal backend.
- La modalità anteprima browser (`npm run dev`) mostra l'interfaccia reale ma dichiara che il motore
  nativo non è disponibile; i token dimostrativi non hanno pittogrammi.
- Comandi Tauri: gli argomenti viaggiano in camelCase (`commercialOnly`, `packId`, `documentKey`).
- **Selezione testuale**: `lib/editor/selection.ts` mappa l'offset del cursore o
  dell'intervallo ai token CAA; `CaaEditor` lo invoca su `mouseup` e sulle frecce,
  mai a ogni tasto. Una parola senza pittogramma è quindi selezionabile come
  qualsiasi altra. Prima di applicare una scelta, `App.svelte` confronta la
  `surface` corrente con quella selezionata: una rianalisi non può applicare il
  simbolo alla parola sbagliata.
- **Didascalie**: `show_captions` è spento per default. Il chip mostra il
  pittogramma e la parola scritta resta nel testo sotto la striscia; l'etichetta
  del repertorio vive in `CaaToken.symbol_label` (tooltip, pannello, licenze).
  Le esportazioni ricevono `include_captions` e omettono la didascalia quando è
  falsa. `border_style` sceglie fra nessun bordo, solo immagine, immagine e
  testo, bordi separati e divisore.
- **Morfologia e lessico**: il condizionale presente è generato da
  `morphology::verb_forms` (`assaggerei` → `assaggiare`). Le unità multiword non
  ereditano `is_function` dalla prima parola e aggiornano `normalized`: «non so»
  e «da sole» sono unità lessicali, non negazioni o preposizioni.

### 4.0 Ribbon

`src/lib/components/Ribbon.svelte` rende un registro dichiarativo
(`src/lib/ribbon/types.ts`): schede → gruppi → comandi. I comandi sono costruiti in
`App.svelte` (`ribbonTabs()`, `fileCommands()`, `quickCatalog()`) con le stesse azioni del resto
dell'app. Tipi di controllo: pulsante, `select`, `color`; stati `active`, `enabled`, `shortcut`,
`tooltip`.

- Sette schede: Home, Inserisci, Layout, Pittogrammi, Revisione, Strumenti IA, Visualizza.
- Schede contestuali: Pittogramma (selezione nel testo) e Tabella (`isInTable`).
- Accesso rapido in `localStorage` (`ui.quickAccess`); modalità `ui.ribbonMode`.
- Sotto 1200 px il Ribbon completo diventa compatto; sotto 980 px si nasconde, le schede restano e
  un clic riapre il corpo. In compatto le etichette si nascondono ma restano i tooltip; in touch
  restano visibili.
- Le icone sono nodi SVG del pacchetto `lucide` (ISC), resi da `RibbonIcon.svelte`. Non usare
  `lucide-svelte`: non è compatibile con `compilerOptions.runes = true`.
- Un comando non implementato non va aggiunto come pulsante attivo: va omesso oppure disabilitato
  con `tooltip` esplicativo.

### 4.1 Affinità della parola (`word_affinities`)

`Engine::word_affinities(word, pack_id, commercial_only)` risponde alla domanda «quali pittogrammi
sono legati a questa parola?» senza usare il modello neurale. Restituisce un `WordAffinity` con i
soli gruppi non vuoti, in ordine di forza:

| Livello | Punteggio | Criterio | Approssimativo |
|---|---|---|---|
| `exact` | 100 | etichetta del pittogramma uguale alla parola | no |
| `lemma` | 90 / 85 | forma base riconosciuta dal lessico, o concetto collegato dal lessico | no |
| `variant` | 80 / 75 | forma flessa del paradigma; fuori lessico solo regole non ambigue (`-esse`, `-iste`, `-trici`, `-che`, `-ghe`) | no / sì |
| `synonym` | 70 / 65 | sinonimo dichiarato nel lessico o concetto equivalente | no / sì |
| `general` | 60 | concetto più generale (`broader`), un solo livello | sì |
| `related` | 30–55 | concetto collegato o somiglianza FTS su etichetta/significato | sì |

Regole:

- deduplica per `symbol_id`: un pittogramma compare solo nel gruppo più forte;
- i dati vengono da `lexicon.lookup`, `morphology::generate_forms`, `symbols_for_concept`,
  `broader_of`, `equivalent_concepts`, `related_concepts` e `search_symbols` (FTS5);
- per le parole fuori lessico non si inventano lemmi o concetti: si usano etichette che esistono
  davvero nel repertorio e si dichiarano approssimative;
- la regola generica `-e → -a` è esclusa: «pesce» diventerebbe «pesca»;
- il comando non modifica il documento: l'applicazione passa da `onApplyAlternative`, cioè dallo
  stesso `set_document_override` + `applyTokenSymbol` delle alternative normali.

L'interfaccia mostra i gruppi nel pannello Alternative quando non ci sono alternative automatiche e
nella scheda Cerca sopra i risultati FTS; ogni candidato porta punteggio, motivo leggibile e badge
«≈» se approssimativo. `mock.html` contiene un caso dimostrativo per provare la sola resa grafica:
non è il motore reale e lo dichiara.

## 5. IA locale (facoltativa)

Il motore è un **servizio unico nel processo Rust** (`ai_engine::service`): `LocalModel` avvia
`llama-server` su `127.0.0.1` con una porta effimera, `Service` gestisce coda con priorità, cache e
annullamento. Le richieste usano il formato conversazionale (`/v1/chat/completions`, che applica il
template Instruct del GGUF) e ricadono su `/completion` se il modello non ha un template o la
richiesta fallisce: con la sola continuazione i modelli Instruct tendevano a ripetere la prima
parola. Se `llama-server` non è disponibile si ricade su `llama-cli`, che resta un ponte più lento
ma funzionante. All'uscita dell'app il processo del modello viene fermato esplicitamente
(`RunEvent::Exit` → `Engine::shutdown`), perché Tauri non esegue i distruttori dello stato gestito.
Il modello riceve solo candidati già presenti nel database: non inventa identificativi.

Il pacchetto include `resources/ai/` (runtime llama.cpp e Qwen2.5-0.5B-Instruct Q4_K_M). Gli altri
modelli del catalogo (`ai_engine::catalog`) si scaricano su richiesta con
`ai_install_model` → `ai_engine::installer`: il download usa il `curl` di sistema, riprende un file
parziale `.part`, **ricalcola lo sha256** e rifiuta il file se l'impronta non coincide con quella
pubblicata. `ai_import_model` accetta un GGUF già presente sul disco dopo averne verificato la firma
`GGUF`. Nessun modello viene scaricato in modo implicito.

Durante la scrittura la passata neurale (`Engine::refine`) usa il campo semantico della frase e le
parole affini per aggiungere alternative marcate `neural`, senza mai sostituire una preferenza
bloccata. La scheda «Cerca» espone gli stessi suggerimenti con il pulsante «Affini con l'IA».

Configurazione: Impostazioni → *Gestione modelli* (download, uso, rimozione, importazione) e
*Intelligenza artificiale locale* (percorsi manuali, thread, contesto, timeout). Il valore
predefinito è 2 thread e 1024 token di contesto.

## 6. Formato `.caadoc`

Archivio ZIP con:

```text
manifest.json        versione formato, titolo, impostazioni, data
document.json        ProseMirror JSON + testo semplice
assets/<id>.svg      pittogrammi usati (vettoriali, incorporati)
licenses.json        attribuzioni delle risorse
```

Scrittura atomica: file temporaneo nella stessa cartella + rename. Il caricamento valida la versione
del manifest e tollera asset mancanti segnalandolo.

## 7. Aggiungere un pacchetto di simboli

```bash
python3 scripts/import_symbols.py --source ~/mio-repertorio --pack-id mio \
  --name "Mio repertorio" --license "CC BY 4.0" \
  --license-url https://creativecommons.org/licenses/by/4.0/ --author "Autore"
```

Lo script valida gli SVG (XML, `viewBox`, niente script né riferimenti esterni), copia i file e scrive
il manifest con `status: "da verificare"`. La revisione semantica resta manuale. I simboli importati
compaiono al successivo avvio; per forzare la reimportazione eliminare il database.

## 8. Verifiche

```bash
npm run build                         # svelte-check + Vite
cd src-tauri && cargo test            # unit + integrazione + corpus semantico
cd src-tauri && cargo clippy --all-targets
python3 scripts/check_context.py
python3 scripts/build_indexes.py --db <percorso>/caa-writer.sqlite3
```

Il corpus `tests/semantic_corpus.json` contiene 205 frasi annotate a mano; il test produce
`tests/last_report.json` con accuratezza, approssimazioni, tempi e l'elenco dei casi falliti. Il
confronto con il sistema ibrido è dichiarato non misurato se manca il modello.

### Test manuali consigliati su Windows (4 GB)

1. Documento da una pagina con 40 pittogrammi: scrittura fluida, nessun blocco.
2. Documento da 10 e 50 pagine: tempi di apertura e di analisi.
3. Inserimento rapido di testo con IA attiva: l'editor deve restare reattivo.
4. Salvataggio, chiusura forzata, riapertura con recupero della bozza.
5. Export DOCX e PDF con simboli e didascalie.
6. Uso con touch su smartboard e con tastiera virtuale.
7. Funzionamento con IA disattivata o modello assente.

Registrare memoria residente (Task Manager), tempi percepiti e comportamenti anomali. Questi test
**non** sono stati eseguiti nell'ambiente di sviluppo macOS e non vanno dichiarati superati.

## 9. Build e distribuzione

```bash
npm run tauri build
npm run tauri:build:mac-arm
npm run tauri:build:mac-intel
```

La pipeline `.github/workflows/build.yml` esegue test e build su macOS e Windows e carica gli
artefatti. Non produce installer firmati né notarizzati: servono certificati e credenziali che il
repository non contiene.

Su macOS, Tauri 2 colloca `bundle.resources` in `Contents/Resources/_up_/resources` (non in
`Contents/Resources`). `AppPaths::resolve_resources` prova entrambe le posizioni e le varianti
accanto all'eseguibile; i test `paths::tests` coprono sviluppo, bundle e `_up_`. Dopo ogni build
release aprire davvero il `.app`: senza questa verifica una risorsa mancante fa terminare l'app
all'avvio prima di mostrare la finestra.

Il modello GGUF, il runtime llama.cpp e il catalogo ARASAAC non sono versionati (dimensioni oltre i
limiti GitHub). `scripts/fetch_release_assets.py` li prepara e verifica l'impronta del modello; per
la build Intel da host ARM usare `--llama-asset bin-macos-x64 --force-llama`. Il workflow
`release.yml` esegue questa preparazione su ogni runner, compila i bundle nativi e pubblica la
release solo con tag `v*` e versione allineata.
