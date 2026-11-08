# CAA Writer per un coding agent

Questa è la documentazione operativa per pilotare CAA Writer da un agente o da
uno script. Descrive la CLI headless, i protocolli JSON, i flussi di lavoro e i
limiti reali del repository. Le regole di progetto restano in `AGENTS.md`; lo
stato tecnico aggiornato è in `Contesto.md` (locale, escluso da Git).

## 1. Principi non negoziabili

1. **Nessun risultato inventato.** Se un comando non è eseguito o un test non è
   verde, va dichiarato.
2. **Offline per primo.** Il programma apre, modifica, salva e stampa documenti
   senza rete. La ricerca online è esplicita e disattivata per default.
3. **Licenze separate dal codice.** I pittogrammi ARASAAC sono CC BY-NC-SA 4.0
   (autore Sergio Palao); i simboli originali sono CC BY-NC-SA 4.0. Ogni
   risultato online conserva autore, fonte, URL e licenza.
4. **L'utente decide.** Le scelte manuali (pittogrammi bloccati, associazioni,
   unioni manuali) non vengono sovrascritte dal motore.
5. **Il documento è la fonte di verità.** Le modifiche al testo passano da
   ProseMirror; la CLI non manipola il DOM.

## 2. Compilazione e verifica

```bash
cd "CAA Writer"
npm ci
npm run build                 # svelte-check + Vite
npm run test:frontend         # test Node

cd src-tauri
cargo test --lib              # test Rust
cargo test --test semantic_corpus
cargo build --release --bin caa-cli
```

Il binario della CLI viene creato in:

```text
src-tauri/target/release/caa-cli
```

In debug: `cargo run --bin caa-cli -- ...` da `src-tauri/`.
`default-run = "caa-writer"` mantiene `cargo run` e `tauri dev` sulla GUI.

Installazione locale della CLI:

```bash
cargo install --path src-tauri --bin caa-cli --locked
# oppure copiare src-tauri/target/release/caa-cli nel PATH
```

La CLI non è inclusa nei bundle macOS/Windows: è uno strumento per sviluppo e
automazione e va compilata dal sorgente.

## 3. Avvio e percorsi

```bash
caa-cli --help
caa-cli info
caa-cli methods
```

Flag globali:

| Flag | Significato |
|---|---|
| `--data-dir PATH` | database, asset importati, cache vocale |
| `--resources PATH` | `dictionaries/`, `symbol-packs/`, `templates/`, `voice/` |
| `--compact` | una riga JSON invece di JSON indentato |

Risoluzione predefinita:

- **dati**: cartella dell'app (`~/Library/Application Support/org.caawriter.desktop`
  su macOS, `%APPDATA%\org.caawriter.desktop` su Windows,
  `~/.local/share/org.caawriter.desktop` su Linux). Variabile:
  `CAA_WRITER_DATA_DIR`.
- **risorse**: cerca `resources/` a partire dalla directory corrente,
  `../resources`, accanto all'eseguibile e nelle varianti del bundle. Variabile:
  `CAA_WRITER_RESOURCES`.

**Primo avvio**: se il database è vuoto o i pacchetti cambiano versione, il
motore importa il lessico, i simboli core e il catalogo ARASAAC. Può richiedere
da decine di secondi a qualche minuto. Per script brevi conviene riusare la
cartella dati dell'app già popolata, oppure accettare l'attesa una volta.

`caa-cli info` restituisce versione, percorsi, conteggi, pacchetti e warning di
avvio reali.

## 4. Due modi di chiamare i metodi

### 4.1 Singola chiamata

```bash
caa-cli call symbolize '{"text":"La sintesi vocale legge.","options":{"paragraph_id":"p1"}}'
```

Il secondo argomento è JSON. Con `-` viene letto da stdin:

```bash
echo '{"text":"Il gatto dorme."}' | caa-cli call symbolize -
```

### 4.2 Modalità RPC JSONL (consigliata per agenti)

`caa-cli rpc` inizializza il motore **una sola volta** e legge una richiesta per
riga:

```json
{"id":1,"method":"symbolize","params":{"text":"Il gatto dorme.","options":{"paragraph_id":"p1"}}}
{"id":2,"method":"search","params":{"query":"gatto","limit":5}}
```

Risposta per riga:

```json
{"id":1,"ok":true,"result":{...}}
{"id":2,"ok":false,"error":"..."}
```

L'`id` è restituito invariato (stringa o numero). Una riga non valida produce
`{"id":null,"ok":false,"error":"..."}` e non interrompe il flusso. Il processo
termina a EOF di stdin.

Esempio Python:

```python
import json, subprocess
proc = subprocess.Popen(["caa-cli", "rpc"], stdin=subprocess.PIPE,
                        stdout=subprocess.PIPE, text=True, bufsize=1)
def call(method, params):
    proc.stdin.write(json.dumps({"id": method, "method": method, "params": params}) + "\n")
    proc.stdin.flush()
    return json.loads(proc.stdout.readline())
print(call("info", {}))
```

## 5. Metodi disponibili

`caa-cli methods` restituisce l'elenco aggiornato. I parametri sono JSON con
nomi `snake_case`.

### 5.1 Stato e motore

| Metodo | Parametri | Risultato |
|---|---|---|
| `info` | — | versione, percorsi, statistiche, lessico, template, warning |
| `methods` | — | elenco metodi |
| `ai.status` | — | stato del modello neurale |
| `ai.models` | — | catalogo modelli installabili |
| `ai.tool` | `tool`, `input` | esecuzione di uno strumento IA tipizzato |
| `ai.affinity` | `word`: sinonimi/concetti affini con il modello locale, senza modificare il documento |
| `ai.words` | `word`: parole grezze proposte dal modello (diagnostica) |

Strumenti IA utilizzabili (`tool`): `analyse_text_clarity`, `simplify_sentence`,
`explain_application_feature`, `prepare_svg_description` (vedi
`src-tauri/src/ai_engine/tools.rs`). L'output è sempre validato.

### 5.2 Simbolizzazione

| Metodo | Parametri | Risultato |
|---|---|---|
| `symbolize` / `symbolize_paragraph` | `text`, `options`, `include_svg` | token CAA |
| `analyze.text` | `text`, `document_key`, `options`, `include_svg` | paragrafi separati da riga vuota, ciascuno simbolizzato |
| `analyze.caadoc` | `path`, `options`, `include_svg` | come sopra, leggendo `plain_text` dal `.caadoc` |

`options` è `SymbolizeOptions`:

```json
{
  "intensity": "minima|media|completa",
  "mode": "sopra|solo_simboli|testo",
  "document_key": "doc-123",
  "commercial_only": false,
  "pack_id": null,
  "paragraph_id": "p1",
  "symbolize_articles": true,
  "symbolize_prepositions": true,
  "symbolize_conjunctions": true,
  "manual_units": [{"start": 0, "end": 16}],
  "split_ranges": [{"start": 13, "end": 27}]
}
```

`manual_units` e `split_ranges` usano offset UTF-16, come ProseMirror.
`include_svg: false` (default) rimuove sia gli SVG sia le immagini PNG (campo `image`, data
URL) dalla risposta per alleggerirla. I pittogrammi ARASAAC e quelli importati sono PNG usati
direttamente; `svg` resta per i pittogrammi vettoriali. `symbol.hydrate` scarica i PNG mancanti.

Esempio:

```bash
caa-cli call symbolize '{
  "text": "La sintesi vocale legge il testo.",
  "options": {"paragraph_id": "p1", "mode": "sopra", "intensity": "media"}
}'
```

### 5.3 Ricerca e affinità

| Metodo | Parametri |
|---|---|
| `search` | `query`, `limit`, `offset`, `pack_id`, `collection_id`, `commercial_only` |
| `affinities` | `word`, `pack_id`, `commercial_only` |
| `symbol.get` | `id`: restituisce etichetta, repertorio e lunghezza dell'SVG letto dal file |
| `symbol.hydrate` | `ids` (array): scarica i PNG ARASAAC mancanti nella cartella dati dell'utente e aggiorna il database |
| `symbol.resolve` | `words` (array): pipeline automatica locale → ARASAAC online → sinonimi italiani → Global Symbols → sinonimi inglesi; restituisce le alternative trovate |

La ricerca usa FTS5 e mette i pittogrammi personali prima dei repertori
installati. `query` vuota con `collection_id` elenca la collezione.

### 5.4 Modelli (template)

| Metodo | Parametri | Risultato |
|---|---|---|
| `templates.list` | `category`, `query` | elenco modelli |
| `templates.get` | `id` | modello completo |
| `templates.plan` | `template_id` oppure `prompt` | elementi con pittogrammi risolti |

La biblioteca contiene **300 modelli** in `resources/templates/library/`, oltre
ai quattro storici. I modelli sono `grid`, `schedule`, `sequence`, `tokens`,
`worksheet` o `story`; ogni modello ha contenuti reali, non celle vuote.

### 5.5 Layout

| Metodo | Parametri |
|---|---|
| `layout.build` | `spec` (`LayoutSpecification`) |
| `layout.edit` | `spec`, `prompt` oppure `operations` |

Restituiscono pagine in millimetri con simboli risolti. Le operazioni sono
validate e gli elementi bloccati non vengono modificati.

### 5.5b Profili e sistemi comunicativi

| Metodo | Parametri |
|---|---|
| `profiles.list` | — |
| `boards.list` | — |
| `boards.build` | `id` |

`boards.list` restituisce i 30 sistemi comunicativi con pagine e celle.
`boards.build` risolve i pittogrammi delle celle e restituisce il grafo
navigabile (pagine, celle, azioni `speak`/`append`/`navigate`/`back`/`home`).
La modalità Utilizzatore usa questi sistemi; la console CLI li può ispezionare.

### 5.6 Importazione e collezioni

| Metodo | Parametri |
|---|---|
| `import.scan` | `paths`: array di file o cartelle |
| `import.run` | `entries`, `collection`, `license` |
| `collections.list` | — |
| `collections.rename` | `id`, `name` |
| `collections.delete` | `id` |
| `collections.delete_symbols` | `ids` |
| `collections.move` | `ids`, `collection_id` |

`import.scan` visita ricorsivamente le cartelle, riconosce il formato dai byte,
propone il lemma dal nome, calcola il checksum, segnala duplicati (anche contro
il database) e produce miniature per i primi 160 elementi. Non segue i
collegamenti simbolici.

Formati con decoder incluso: PNG, JPEG, GIF (primo fotogramma), WebP, BMP, TIFF,
ICO (PNG RGBA), AVIF, SVG. **HEIC/HEIF non è supportato**: viene segnalato, non
dichiarato supportato.

`import.run` copia i file in `assets/imported/<collezione>/`, crea un concetto
`custom.<lemma>`, inserisce il simbolo nel pacchetto `personal`, aggiunge FTS,
sinonimi e licenza. Il nome della collezione è quello suggerito dalla cartella,
modificabile. Gli SVG restano vettoriali; PNG/JPEG/WebP/GIF/BMP/TIFF/ICO/AVIF
vengono salvati come PNG diretto (niente wrapper SVG) e restituiti nel campo
`image` come data URL. Il comando Tauri `import_image_symbol` importa una
singola immagine da base64 (usato dalla scheda Simbolo).

Esempio completo:

```bash
caa-cli call import.scan '{"paths":["/percorso/Pittogrammi scuola"]}' > scan.json
python3 - <<'PY'
import json
s=json.load(open('scan.json'))
entries=[{"path":e["path"],"lemma":e["suggested_lemma"],
          "collection":s["suggested_collection"],
          "category":e["relative_path"].split('/')[0] if '/' in e["relative_path"] else None,
          "synonyms":[],"relative_path":e["relative_path"],
          "original_name":e["original_name"]}
         for e in s["entries"] if e["supported"] and not e["duplicate"]]
json.dump({"entries":entries,"collection":s["suggested_collection"],
           "license":{"name":"Uso personale (non commerciale)","url":None,
                      "author":"Autore","source_url":None,"commercial_ok":False}},
          open('import.json','w'), ensure_ascii=False)
PY
caa-cli call import.run - < import.json
```

### 5.7 Ricerca online

| Metodo | Parametri |
|---|---|
| `online.providers` | — |
| `online.search` | `terms` (array), `force_refresh` |
| `online.ensure` | `words` (array): cerca solo ARASAAC e salva le copie offline |
| `online.autofill` | `words` (array): se manca un'etichetta ARASAAC esatta, importa il primo pittogramma disponibile |
| `online.global` | `term`: cerca solo su Global Symbols (utile per verificare la ricerca inglese) |
| `online.english` | `word`, `limit`: traduce in inglese, cerca su Global Symbols, importa le copie e restituisce le alternative (ARASAAC inglese escluso) |
| `symbol.hydrate` | `ids` (array): ripristina gli SVG ARASAAC da `static.arasaac.org`, utile dopo aver rimosso il catalogo locale |
| `online.save` | `result`, `lemma`, `collection` |

Provider attivi: **ARASAAC** (prima scelta, pittogrammi CAA, CC BY-NC-SA 4.0) e
**Wikimedia Commons** (fallback, fotografie e disegni con licenza letta dai
metadati). Openverse e Global Symbols sono elencati come non collegati: non
basta che il servizio esista, servono API e condizioni verificate.

Privacy: `terms` non è una frase. Vengono ammessi al massimo quattro termini
generici; il backend li filtra ulteriormente con il lessico locale
(`known_terms`) e scarta email, URL, numeri e nomi propri non presenti nel
lessico. La frase completa non lascia il dispositivo.

`online.save` scarica l'immagine (ARASAAC/Wikimedia o provider personalizzato su
HTTPS pubblico, mai localhost/IP privati), la importa nella collezione indicata e
restituisce l'`Alternative` pronta per l'applicazione. Dopo il salvataggio il
documento resta utilizzabile offline.

### 5.8 Prompt per il documento

`prompt.interpret`:

```json
{"prompt":"Ingrandisci i pittogrammi del 20% e metti il testo in maiuscolo",
 "use_neural": true}
```

Risultato:

```json
{
  "operations": [
    {"type":"set_uppercase","value":true},
    {"type":"scale_symbol_size","factor":1.2}
  ],
  "descriptions": ["Testo in maiuscolo", "Dimensione dei pittogrammi ×1.20"],
  "warnings": [],
  "source": "deterministico"
}
```

Operazioni ammesse: `set_uppercase`, `scale_font_size`, `set_font_size`,
`scale_symbol_size`, `set_symbol_gap`, `set_chip_border_color`,
`set_alignment`, `insert_table`, `add_table_row`, `add_table_column`,
`generate_board`. Le operazioni sconosciute o fuori intervallo vengono
rifiutate. Il modello locale interviene solo se il parser deterministico non
riconosce nulla; non esegue codice.

### 5.9 Documenti ed esportazione

| Metodo | Parametri |
|---|---|
| `document.open` | `path` (`.caadoc`) |
| `document.save` | `path`, `payload` (`DocumentPayload`) |
| `export.html` | `path`, `document` (`ExportDoc`) |
| `export.docx` | `path`, `document` |
| `document export` | `--format pdf\|png\|html\|svg\|docx\|pptx`, `--out` |

### 5.10 Voce

| Metodo | Parametri |
|---|---|
| `voice.status` | `language` opzionale (`it`, `en`) |
| `voice.speak` | `text`, `language` |
| `speak` | alias |

Usa la voce neurale se disponibile, altrimenti la voce di sistema, e dichiara
quale motore ha risposto.

## 6. Schema degli errori

Ogni errore è una stringa italiana esplicita. In RPC:

```json
{"id":3,"ok":false,"error":"serve il parametro text"}
```

Errori frequenti: risorse non trovate, database non apribile, provider di rete
non raggiungibile, file non supportato, operazione non ammessa, modello non
configurato. La CLI non restituisce mai un risultato parziale come se fosse
completo.

## 7. Cosa la CLI non fa (e perché)

- **Non applica le operazioni del prompt a un documento ProseMirror.** Il
  parsing e la validazione sono nella CLI; l'applicazione usa le transazioni
  dell'editor nella GUI. La CLI può però applicare `layout.edit`, generare
  `templates.plan` e modificare le impostazioni tramite un agente che legge il
  piano.
- **Non pilota drag-and-drop, dialoghi di file o il comunicatore a schermo
  intero.** Sono superfici della GUI.
- **Non firma, non pubblica e non installa** aggiornamenti.
- **Non inventa provider online** non verificati.
- **Non converte HEIC** perché il decoder non è incluso.

## 8. Flussi consigliati per un agente

### 8.1 Analizzare testo e proporre simboli

1. `symbolize` con `include_svg:false`.
2. Leggere `tokens[].surface`, `concept_id`, `symbol_id`, `strategy`, `approximate`.
3. Per le unità mancanti, `search` o `affinities` con il lemma.
4. Mai applicare automaticamente un simbolo approssimativo senza dichiararlo.

### 8.2 Importare una collezione

1. `import.scan` e controllare `supported`, `duplicate`, `reason`.
2. Mostrare all'utente il nome suggerito e i lemmi proposti.
3. `import.run` a blocchi da 20 elementi, riportando `imported/skipped/failed`.
4. `collections.list` per verificare il conteggio.
5. `search` con `collection_id` per controllare l'indicizzazione.

### 8.3 Cercare online senza violare la privacy

1. Costruire `terms` da concetto/lemma, mai dalla frase.
2. `online.search`; mostrare `kind` (pittogramma/fotografia), fonte e licenza.
3. `online.save` solo dopo una scelta esplicita.
4. Non salvare immagini con licenza incompatibile con l'uso previsto.

### 8.4 Generare un materiale

1. `templates.list` con categoria/query.
2. `templates.get` o `templates.plan`.
3. `layout.build`/`layout.edit` per la geometria.
4. Esportare o salvare il documento; non sovrascrivere il modello originale.

### 8.5 Modificare un documento con un prompt

1. `prompt.interpret`.
2. Mostrare `descriptions` e `warnings`.
3. Applicare solo operazioni validate; per la GUI usare il pannello «Modifica
   con IA», che applica le operazioni alle transazioni ProseMirror.

## 8bis. Modalità Utilizzatore e Creator

All'avvio, se esiste un profilo attivo, CAA Writer mostra la home
dell'Utilizzatore: solo schede dei sistemi assegnati e le funzioni abilitate.
Il pulsante **Area Creator** richiede il PIN locale, se configurato, e apre
l'ambiente completo (Ribbon, Studio, gestione profili, CLI).

I profili sono in `profiles` (SQLite) e contengono nome, tema, livello editor,
funzionalità, voce, accessibilità e navigazione. Le assegnazioni sono in
`profile_boards`. La migrazione è la v4; non richiede dati sanitari.

## 8ter. Console CLI nella GUI

In CAA Writer: **Strumenti IA → Modello neurale → Console CLI** (il gruppo può
essere nel menu «Gruppi»). La console esegue gli stessi metodi della CLI con
`--data-dir` e `--resources` dell'app, mostra stdout/stderr e il JSON. Il
la CLI è distribuita come binario separato (non è più inclusa nel bundle); in sviluppo viene cercata anche in
`target/release` e `target/debug`. Non usa una shell: metodo e parametri sono
dati, non comandi di sistema.

## 8quater. Prestazioni, annullamento e diagnostica

La scrittura non aspetta mai l'IA. Il frontend pianifica l'analisi lessicale
dopo un debounce (predefinito 240 ms) e la rifinitura neurale dopo 600 ms.
Ogni richiesta neurale ha un `requestId` (`pid:sequenza`); quando il testo
cambia, il frontend chiama `cancel_refine` e il backend scarta il risultato
anche se llama.cpp ha già risposto. I comandi Tauri pesanti girano su
`spawn_blocking`.

Impostazioni → Prestazioni offre quattro profili: `risparmio`, `bilanciato`,
`elevate`, `personalizzato`. Cambiano thread/contesto del modello e le due
attese di debounce. Il profilo è salvato con le impostazioni dell'interfaccia;
thread, contesto e timeout vengono salvati nella configurazione IA.

Impostazioni → Diagnostica misurata legge dal sistema: CPU/RAM del processo e
del modello, coda neurale, inferenze, risultati obsoleti, tempi di
simbolizzazione, latenza input p95 e blocchi di frame > 120 ms raccolti nel
WebView. Su Windows `Get-Process` espone secondi CPU cumulati, non una
percentuale istantanea: il pannello lo dichiara.

Le preferenze manuali hanno cinque ambiti:

| Ambito | Chiave | Effetto |
|---|---|---|
| occorrenza | `document_overrides` | solo l'unità selezionata |
| documento | `document_overrides` per concetto | occorrenze equivalenti nel documento |
| profilo | `symbol_preferences` scope `profile` | solo il profilo attivo |
| globale | scope `global` | tutti i documenti futuri |
| vocabolario | scope `vocabulary` | ultima risorsa, condivisa dal Creator |

Nessun ambito sovrascrive un blocco manuale; l'ordine è documento > profilo >
contesto > globale > vocabolario.

## 8quinquies. Comunicatore, foglio libero ed editor Utilizzatore

- `build_layout` accetta elementi con `spoken_text` e `action`. Le azioni ammesse sono
  `append`, `speak`, `navigate`, `back`, `home`, `clear`; `navigate` richiede
  `target_page` e la pagina deve esistere. Non vengono eseguiti script o URL.
- `Communicator.svelte` ha una modalità Modifica: cambia etichetta, testo pronunciato,
  pittogramma e azione; aggiunge/rimuove celle e pagine; applica con `build_layout` e salva
  in `.caadoc` con `document_type = "communicator"`.
- `FreeBoard.svelte` salva una specifica `document_type = "free_board"`; la conversione
  è in `src/lib/freeboard.ts` (funzioni pure, testate). La riapertura riporta al foglio.
- `UserEditor.svelte` legge `profile.editor_level`: 1 solo simboli, 2 tastiera visuale,
  3 formattazione essenziale. Non espone comandi Creator.
- Comando Tauri `common_symbols` (GUI): vocabolario di base con soli simboli reali del
  repertorio; nessun simbolo inventato.

## 8sexies. CAA Universal Document Engine e biblioteca (0.9.0)

Tutti i materiali sono `UniversalDocument` (`.caadoc` v3, campo `universal`).
Pagine `flow`, `absolute` e `grid`; elementi con ID stabile, posizione,
dimensione, rotazione, livello, stile, celle, azioni e collegamenti per ID
pagina. La GUI usa i comandi Tauri `document_apply`, `document_render`,
`document_validate`, `universal_open`, `universal_from_plan`; la CLI usa le
stesse `DocumentCommand` tramite `universal::apply_transaction`.

Comandi CLI principali (il binario è `caa-cli`; la GUI è `caa-writer`):

```bash
caa-cli document create --spec templates/sources/...json --out file.caadoc
caa-cli document validate file.caadoc
caa-cli document inspect file.caadoc
caa-cli document render file.caadoc --out cartella
caa-cli document export file.caadoc --format html --out file.html
caa-cli document migrate legacy.caadoc --out nuovo.caadoc
caa-cli document apply file.caadoc --commands comandi.json

caa-cli page add file.caadoc --title "Nuova" --layout grid
caa-cli page remove file.caadoc --id pagina
caa-cli page reorder file.caadoc --id pagina --index 0

caa-cli element add file.caadoc --page home --element elemento.json
caa-cli element update file.caadoc --page home --id cella --patch patch.json
caa-cli element remove file.caadoc --page home --id cella

caa-cli board create --spec specifica.json --out sistema.caadoc
caa-cli board link file.caadoc --page home --id cella --target frutta
caa-cli board validate file.caadoc

caa-cli template build --spec specifica.json --out modello.caadoc
caa-cli template validate --spec specifica.json
caa-cli template preview file.caadoc --out anteprima.svg
caa-cli template package file.caadoc --out modello.caadoc
caa-cli template build-all --sources templates/sources --out resources/library
```

`template build-all` legge le specifiche versionate, risolve i pittogrammi col
motore semantico, applica i comandi, valida, rende, scrive anteprime SVG,
`manifest.json` e `report.json`. Non pubblica se un documento fallisce.

## 8septies. Griglie, PDF e player (0.10.0)

- `create_grid` crea automaticamente `columns × rows` celle; `update_grid` le
  ridimensiona senza perdere il contenuto. La GUI offre un dialogo righe/colonne
  e comandi +/− riga/colonna.
- `document export --format pdf|png` e `document export --format html|svg`
  usano lo stesso renderer dell'editor. Il PDF è multipagina e rasterizza le
  pagine con resvg; il PNG è la prima pagina.
- La stampa usa `document_html` (tutte le pagine) e il dialogo di sistema.
- Il player si apre da `.caadoc`/`.caaplay`, da `take_pending_open` o
  dall'evento `open-document`; «Importa» crea una copia in
  `app_data_dir/library/imported/`.
- Audit aggiornato in `docs/AUDIT_0_10.md`.

## 8octies. Repertori remoti, provider e default (0.11.0)

- Il bundle contiene `resources/symbol-packs/core` (88 simboli offline) e la
  biblioteca `.caadoc` con i pittogrammi incorporati; il catalogo ARASAAC non è
  più duplicato intero. La ricerca remota usa `online_search` (ARASAAC,
  Wikimedia, provider personalizzati con URL contenente `{query}`), salva una
  copia offline e conserva autore/licenza/fonte.
- I provider personalizzati si configurano in Impostazioni → Librerie online;
  download solo HTTPS pubblico, con blocco di localhost/IP privati.
- I risultati ARASAAC non vengono filtrati con il campo `aac` (non indica
  l'idoneità CAA); le miniature usano `_300.png`. La cache è versionata.
- Con l'impostazione «Suggerisci automaticamente nel pannello», la ricerca
  parte quando si seleziona un'unità senza simbolo; «Disattivata» è l'unica
  modalità senza rete. Ricerca, anteprima e salvataggio girano fuori dal thread
  dell'interfaccia.
- `ensure_online_symbols` cerca solo ARASAAC per le parole minuscole fuori
  lessico, salva copie offline e restituisce le parole elaborate. Il frontend
  lo chiama dopo l'analisi se `online_auto_insert` è attivo, poi rianalizza:
  `symbol_by_label` collega il simbolo per etichetta/parola chiave esatta.
- All'avvio `repair_asset_paths` riallinea i file dei simboli alla cartella
  risorse corrente. Senza questo, un database creato da un'altra installazione
  può bloccare `svg_for` (doppio lock della cache) e lasciare l'analisi appesa.
- Default: `speech.enabled=false` nel database e `speech_enabled=false` nelle
  impostazioni UI; profilo `risparmio`, debounce 420/1200 ms; AI 1 thread,
  contesto 512, timeout 4 s.
- Provider IA online: `ai_provider_*`, chiave nel portachiavi di sistema, mai in
  `.caadoc`, log o errori; consenso e privacy obbligatori.
- Griglie: `set_grid_sizes`, `merge_grid_cell`, `split_grid_cell`.
- Export universale: `pdf`, `png`, `docx`, `pptx`, `html`, `svg`.
- Stampa dell'interfaccia: comando Tauri `print_window` (`WebviewWindow::print`).
- Editor visuale: pagine con miniature richiudibili, righello e margini
  trascinabili, guide magnetiche, celle svincolabili, ancoraggio z e
  ripaginazione automatica dei documenti a flusso.
- Comunicatore: modalità «Componi frase» e «Tocca per parlare»; in quest'ultima
  il simbolo viene pronunciato subito.
- Profili: `profile_boards` assegna uno qualunque dei 330 documenti della
  biblioteca; `profile_boards` restituisce titolo/pagine anche per i template.
  La scheda **Utenti** è visibile solo nell'Area Creator. `profileSetActive`
  cambia il profilo attivo della sessione.
- Permessi finestra: `core:window:allow-set-fullscreen` e
  `core:window:allow-is-fullscreen` in `capabilities/default.json`. DOCX e PPTX
  contengono testo, immagini e celle come elementi nativi modificabili; azioni e
  collegamenti del player restano solo in `.caadoc`.
- Lettura automatica durante la scrittura: le chiamate sono silenziose e non
  mostrano avvisi; i pulsanti espliciti spiegano lo stato della voce.

## 9. Sicurezza, rete e licenze

- La CLI non apre connessioni se non si chiamano `online.*` o `voice.*` che
  richiedono download/voce.
- I download sono limitati a `upload.wikimedia.org` e
  `static.arasaac.org`; dimensione massima 10 MB; timeout; MIME verificato.
- Gli SVG importati passano dal sanificatore esistente: niente script,
  `foreignObject`, gestori `on*`, risorse esterne.
- I metadati di licenza sono salvati per simbolo; il codice resta
  CC BY-NC-SA 4.0 e le licenze dei materiali restano separate.
- `Contesto.md` non va pubblicato né incluso negli installer.

## 10. Mappa del codice utile

| Area | File |
|---|---|
| motore | `src-tauri/src/engine.rs` |
| analisi linguistica | `src-tauri/src/linguistics/` |
| database e migrazioni | `src-tauri/src/database/` |
| import e collezioni | `src-tauri/src/pictogram_import.rs` |
| ricerca online | `src-tauri/src/online_search.rs` |
| prompt tipizzati | `src-tauri/src/document_edit.rs` |
| layout | `src-tauri/src/layout.rs`, `layout_edit.rs` |
| modelli | `src-tauri/src/studio.rs`, `resources/templates/` |
| CLI | `src-tauri/src/bin/caa-cli.rs` |
| editor | `src/lib/editor/`, `src/App.svelte` |
| pannelli | `src/lib/components/SidePanel.svelte` |
| Ribbon | `src/lib/components/Ribbon.svelte` |

## 11. Test e definizione di “fatto”

Prima di dichiarare completata una modifica:

```bash
npm run check
npm run build
npm run test:frontend
cd src-tauri && cargo test --lib
cd src-tauri && cargo test --test semantic_corpus
cargo run --bin caa-cli -- info
```

Per una modifica alla CLI, provare almeno `info`, `methods`, una chiamata
`call`, una sessione `rpc` con due richieste e un errore controllato.

Aggiornare `Contesto.md` con: stato reale, decisioni, problemi nuovi, test
eseguiti con risultati, prossime attività. Non dichiarare “fatto” ciò che è
stato solo compilato.

## 12. Stato corrente delle funzionalità collegate

- Import immagini e cartelle: implementato, con formati verificati e HEIC
  dichiarato non supportato.
- Collezioni personali: creazione tramite import, rinomina, eliminazione,
  spostamento, ricerca per collezione.
- Ricerca online: ARASAAC + Wikimedia, cache 24 h, privacy e licenze.
- Biblioteca modelli: 300 modelli validi, categorie e metadati, apertura nello
  Studio, filtri.
- Modifica con prompt: parser deterministico e piano neurale validato;
  applicazione GUI con anteprima, cronologia e annullamento delle impostazioni.
  Le operazioni tabella usano l'editor; le operazioni non supportate sono
  dichiarate.
- Ribbon: i gruppi che non entrano vengono raccolti nel menu «Gruppi (n)», senza
  scorrimento orizzontale oltre la finestra.

## 13. Libreria organizzata, ricerca inglese e controlli smartboard (novembre 2026)

### 13.1 Catalogo

- `library.catalog`, `library.search`, `library.validate` sono metodi CLI/RPC. I comandi Tauri
  corrispondenti sono `library_catalog`, `library_search`, `library_by_ids`, `library_preview`,
  `library_validate`.
- La tassonomia è in `src-tauri/src/template_catalog.rs` (categorie A–L, sottocategorie, sinonimi,
  pubblico, obiettivi). Il manifest della biblioteca espone categoria, sottocategoria, pubblico,
  obiettivo, tag, complessità, formati, anteprima e numero di pittogrammi mancanti.
- `library validate` controlla schema, elementi fuori pagina, celle vuote, griglie incomplete,
  token board senza gettoni, istruzioni/esercizi/soluzioni mancanti, titoli duplicati e simboli
  assenti. Il report distingue `errore` e `avviso`.

Esempi:

```bash
caa-cli library catalog
caa-cli library search --query "token board" --category comportamentali
caa-cli library search --query "sillabe" --category schede --subcategory italiano
caa-cli library validate --only-pilots
caa-cli template build-all --sources templates/sources --out resources/library
```

### 13.2 Ricerca in inglese nel pannello Simbolo

- Il pannello unico si chiama **Simbolo** (le vecchie schede Alternative e Cerca sono confluite).
- Il selettore `IT / EN` traduce la richiesta prima della ricerca locale:
  - glossario deterministico per il vocabolario CAA di base;
  - modello neurale locale per gli altri termini;
  - se il modello non è disponibile, la ricerca non parte e viene mostrato un errore esplicito.
- Metodo CLI: `translate.search` con `{"text":"dog"}` → `{"source":"dog","translated":"cane"}`.
- Il pannello mostra **prima i pittogrammi possibili** per la parola selezionata (alternative
  automatiche + risultati inglesi), poi la ricerca nel repertorio e quindi i dettagli.
- **Ricerca inglese automatica**: alla selezione di una parola il pannello traduce in inglese e
  cerca su **Global Symbols** (language `eng`), aggiungendo i risultati alle alternative con il
  badge `EN`. I symbolset ARASAAC di Global Symbols sono esclusi perché duplicano il catalogo
  italiano.
  - Metodo CLI: `translate.english` con `{"text":"gatto"}` → `{"translated":"cat"}`;
  - `online.global` con `{"term":"cat"}` mostra direttamente i risultati Global Symbols;
  - `online.english` con `{"word":"gatto","limit":3}` traduce, cerca, importa le copie offline e
    restituisce `{"term":"cat","items":[...],"warnings":[]}`;
  - senza modello e senza voce nel glossario la traduzione fallisce in modo esplicito.
- **Sinonimi e concetti affini**: il pannello chiede in background al modello locale parole vicine
  (priorità bassa, debounce 1200 ms) e mostra i pittogrammi trovati con il badge «IA affine».
  `ai.affinity` con `{"word":"ancorare"}` restituisce le alternative; `ai.words` mostra le parole
  grezze proposte dal modello. Se il modello è spento non viene inventato nulla.
- **Attribuzione automatica nel testo**: dopo la ricerca locale e ARASAAC online, il resolver
  prova sinonimi italiani, Global Symbols con traduzione e sinonimi inglesi. Ogni risultato viene
  applicato all'occorrenza scritta, resta approssimativo e viene salvato come override; se nessuna
  fase trova un simbolo la parola resta senza. Comando CLI `symbol.resolve` con
  `{"words":["ancorare"]}`; comando Tauri `resolve_missing_symbols`. La pausa di 1,2 s e la coda
  a priorità bassa tengono il processo fuori dal percorso di digitazione.
- **Nessun pulsante di ricerca online** nel pannello Simbolo: tutte le fasi sono automatiche.
- La ricerca FTS5 privilegia l'etichetta esatta, quindi `cane` restituisce prima i simboli «cane»
  e non parole che iniziano per «cane» (es. «canestro»).

### 13.3 Repertori online

- Provider attivi: ARASAAC, Global Symbols, Wikimedia Commons, Openverse.
- Global Symbols usa `https://globalsymbols.com/api/v1/labels/search` e conserva autore e licenza
  del symbolset; Openverse usa `https://api.openverse.org/v1/images/` e dichiara le immagini come
  fotografie, non come pittogrammi CAA.
- La ricerca online è **federata**: interroga ARASAAC, Global Symbols, Wikimedia e Openverse (più
  provider personalizzati) e unisce i risultati, max 8 per provider e deduplicati. Non si ferma al
  primo provider che risponde. I risultati scelti vengono salvati come copia offline.
- Nel pannello Simbolo i risultati sono divisi in **pertinenti** (etichetta uguale o che inizia con
  la parola) e **altri**, nascosti dietro un pulsante. La ricerca delle alternative parte
  automaticamente quando cambia l'unità selezionata.
- Se la traduzione inglese non risponde, il pannello cerca comunque il termine originale e mostra
  un avviso; il glossario copre il vocabolario CAA di base.

### 13.4 IA: chat, immagini e modelli diffusion

- Pannello → scheda **IA**: chat, generazione immagini, modelli di diffusione e stato del runtime.
  Gli stessi comandi sono raggiungibili da **Strumenti IA → Immagini e chat** nel Ribbon.
- Backend immagini `auto` | `mlx` | `gpu` | `online` | `off`.
  - Su Apple Silicon il motore locale usa **MPS** (GPU Apple); se è presente un runtime **MLX
    Stable Diffusion** completo (`mlx` + `stable_diffusion`) lo preferisce.
  - Il runtime Python (`torch`, `diffusers`, `transformers`, …) e i modelli **non sono nel
    bundle**: si installano da Impostazioni con «Installa runtime immagini» e «Scarica modello»;
    restano nei dati dell'utente (`image-venv`, `models/image`).
  - Il provider **online** è compatibile con `POST {endpoint}/images/generations` (OpenAI e
    compatibili). Endpoint, modello e chiave si configurano nel pannello; la chiave va nel
    portachiavi di sistema, mai nel database o nei documenti.
- Nell'editor, con una parola selezionata, il pannello Simbolo offre **«Genera un pittogramma con
  l'IA»**: il prompt viene inviato solo al motore configurato, l'immagine viene importata come
  simbolo personale e applicata alla parola. Resta una scelta manuale (concept `custom.ai.*`), non
  viene applicata automaticamente ad altre occorrenze.
- Le immagini generate si possono **applicare alla parola selezionata** (pittogramma personale) oppure
  **inserire nel documento universale** aperto come elemento `image` con SVG incorporato.
- **Chat**: la scheda IA usa il modello locale se attivo, altrimenti il provider online
  configurato. Metodo CLI `ai.chat` con
  `{"messages":[{"role":"user","content":"..."}]}` → `{"reply":...,"backend":...,"elapsed_ms":...}`.
- Metodi CLI immagini: `image.status`, `image.generate` (parametri `prompt`, `negative?`, `out?`,
  `include_base64?`), `image.install`, `image.download` (`model`).
- Nessun modello viene dichiarato disponibile se il runtime o i pesi non ci sono: `image.status`
  riporta `mlx`, `torch`, `mps`, `diffusers` e la nota reale.

### 13.5 Editor completo e modelli di documento

- Ogni modello della biblioteca e ogni materiale generato si apre nell'**editor completo**
  (`UniversalEditor`), in modifica, con il selettore di layout `absolute` / `flow` / `grid` sempre
  disponibile. Non esiste più un ripiego silenzioso sull'editor di testo: se il modello non si apre,
  l'errore viene dichiarato.
- L'anteprima dei modelli resta in sola lettura finché non si preme **Modifica layout e contenuti**
  (banner in alto o pulsante nella barra); il salvataggio crea una copia e non sovrascrive il modello.
- Anche i sistemi comunicativi aperti dal Creator sono modificabili; la sola lettura resta per
  l'Utilizzatore.
- Il Ribbon **Strumenti IA → Creazione** contiene **Storia dal testo** e **Materiale dal testo**:
  usano il testo selezionato (o tutto il documento) e aprono il risultato nello stesso editor
  completo, con pagine, testi e pittogrammi modificabili.
- Nell'editor completo la barra offre **IA** (chat/immagini) e **Genera** (Studio), così generazione
  e modifica avvengono nello stesso posto.
- **Miniature delle pagine**: il pannello Pagine a sinistra disegna il contenuto reale di ogni
  pagina (testi, pittogrammi SVG, celle di griglia, forme, sfondi) in scala, non più blocchi grigi.
  Il pannello si riduce con « e si riapre con «Pagine».
- **Un solo interfaccia**: nell'editor completo la barra interna (Home, titolo, undo, salva, export)
  è nascosta; i comandi sono nel Ribbon (scheda **Documento** e gruppo **Elementi** in Inserisci).
  Lo zoom è in basso a destra e si regola anche con Ctrl/⌘+rotella o pinch sul documento.
- **Formato foglio**: A4/A3/A5/Letter/16:9/4:3 o larghezza×altezza in mm; le dimensioni restano
  stabili e valgono anche per la pagina di testo. Il righello (attivo di default) trascina i margini
  e il pannello proprietà permette di inserirli in mm.
- **Stampa**: genera un PDF con il renderer reale (formato e orientamento corretti) e lo apre nel
  visualizzatore di sistema; non usa il dialogo WKWebView, non affidabile su macOS.
- **Ponte dall'editor di testo**: il comando **Apri nell'editor completo** (Strumenti IA →
  Creazione) converte il documento di testo in un documento universale con una pagina per gruppo
  di paragrafi, i pittogrammi disponibili sopra il testo e layout modificabile. Il file originale
  non viene sovrascritto; il nuovo documento si salva con un nome proprio.

### 13.6 Contenuti espliciti e controlli smartboard

- Impostazioni → Rappresentazione CAA → **Mostra contenuti espliciti (adulti)**, disattivato per
  default. Quando è spento, `src/lib/content-filter.ts` esclude termini sessuali/anatomici
  espliciti da alternative, ricerca locale, ricerca online, documento ed esportazioni. I risultati
  esclusi sono conteggiati; non vengono mostrati come elementi attenuati. Le scelte manuali non
  esplicite restano visibili.
- Il vecchio significato «nascondi i simboli applicati a mano» è stato rimosso: non confondere
  l'interruttore con la visibilità delle scelte manuali.
- In modalità touch compare in basso a sinistra il pulsante **Controlli**: apre una barra compatta
  con tastiera a schermo, spazio, cancellazione, invio, annulla/ripeti e ascolto. La tastiera si
  apre solo su richiesta e non riduce stabilmente l'area di lavoro.
- **Layout predefiniti → Pittogrammi sulla parola** (`settings.symbol_layout = "parola"`): ogni
  pittogramma viene centrato sul range DOM della parola a cui si riferisce, invece di stare in una
  striscia unica sopra il paragrafo. Il testo non viene modificato; la scelta si salva con le
  impostazioni del documento. La stampa nativa usa il DOM corrente, quindi rispetta la disposizione;
  le esportazioni DOCX/HTML restano quelle del documento strutturato.

### 13.5 Storie illustrate

- `caa-cli story create --spec story.json --out storia.caadoc` costruisce una storia multipagina
  con scene vettoriali modificabili, testo CAA, striscia di pittogrammi e pulsanti di navigazione.
- Specifica minima: `title`, `kind` (`social`, `routine`, `emotion`, `educational`, `narrative`),
  `layout` (`image_top`, `pictograms_top`, `image_left`, `image_right`), `pages` (10–20),
  `setting`, `characters`, `subject`, `sentences` opzionale.
- `story validate`, `story render --format pdf|png`, `story package` usano lo stesso renderer e lo
  stesso formato `.caadoc` dell'editor.
- Nella GUI: schermata iniziale → **Crea una storia illustrata in CAA** → Story Studio. Il documento
  generato si apre nell'editor universale.
- Il backend immagini disponibile è **vettoriale offline**. I modelli diffusion locali/online non
  sono attivi e non vengono proposti come compatibili senza download, caricamento e benchmark reali.

### 13.6 Avvio

- Con profili esistenti l'app mostra una schermata di scelta: **Utente** (uno dei profili) oppure
  **Modalità Creator** (con PIN se configurato). Non entra più automaticamente nell'ultimo profilo.
- Al primo avvio la procedura guidata permette anche di entrare come Creator senza creare subito un
  profilo.
