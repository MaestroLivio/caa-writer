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
`include_svg: false` (default) rimuove gli SVG dalla risposta per alleggerirla.

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
modificabile.

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
| `online.save` | `result`, `lemma`, `collection` |

Provider attivi: **ARASAAC** (prima scelta, pittogrammi CAA, CC BY-NC-SA 4.0) e
**Wikimedia Commons** (fallback, fotografie e disegni con licenza letta dai
metadati). Openverse e Global Symbols sono elencati come non collegati: non
basta che il servizio esista, servono API e condizioni verificate.

Privacy: `terms` non è una frase. Vengono ammessi al massimo quattro termini
generici; il backend li filtra ulteriormente con il lessico locale
(`known_terms`) e scarta email, URL, numeri e nomi propri non presenti nel
lessico. La frase completa non lascia il dispositivo.

`online.save` scarica l'immagine (solo host ARASAAC/Wikimedia), la importa nella
collezione indicata e restituisce l'`Alternative` pronta per l'applicazione.
Dopo il salvataggio il documento resta utilizzabile offline.

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
