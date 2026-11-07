# CAA Writer

Videoscrittore con **simbolizzazione CAA assistita in italiano** e **Visual Resource Studio** per
materiali CAA, ABA e TEACCH. Applicazione desktop Tauri 2 (Rust + Svelte), pensata per funzionare
**offline** su Windows 10/11 e macOS, anche su computer con 4 GB di RAM, senza GPU dedicata.

## Stato attuale (verificare sempre su questa macchina)

Ultima release locale: **0.14.0** (7 novembre 2026), non pubblicata su GitHub.

Il software è in sviluppo attivo. La documentazione tecnica di stato — cosa è completo, cosa è
parziale, quali test sono stati eseguiti e con quali risultati — vive in `Contesto.md`, che è
**locale e non versionato**. In questo repository trovi solo la documentazione pubblica.

Sintesi onesta di ciò che esiste nel codice:

| Area | Stato |
|---|---|
| Editor ProseMirror con formattazione, elenchi, tabelle, annulla/ripristina | implementato |
| Interfaccia Ribbon a schede (Home, Inserisci, Layout, Pittogrammi, Revisione, Strumenti IA, Visualizza) | implementato |
| Workspace adattivo: Ribbon minimizzabile, pannello a icone/overlay, concentrazione, preset | implementato |
| Ribbon orizzontale con gruppi raccolti in «Gruppi (n)», senza scorrimento orizzontale | implementato |
| Applicazione immediata dei pittogrammi con ambito occorrenza/documento/globale | implementato |
| Unità multiparola automatiche (sintesi vocale, computer portatile, …) con un solo pittogramma | implementato |
| Unione e separazione manuale delle unità CAA, persistenti e annullabili | implementato |
| Lettura facoltativa di parola e frase durante la digitazione | implementato |
| Importazione di immagini e cartelle con collezioni personali | implementato |
| Ricerca online ARASAAC/Wikimedia con privacy, cache e licenze | implementato |
| Biblioteca di 300 modelli CAA filtrabili e apribili | implementato |
| Modifica del documento con prompt e anteprima | implementato |
| CLI headless `caa-cli` con `call`/`rpc` per agenti | implementato |
| Selezione di qualsiasi parola e aggiunta di un pittogramma senza simbolo preesistente | implementato |
| Pittogramma + testo digitato, senza didascalia duplicata (didascalie opzionali) | implementato |
| Stili di bordo del pittogramma (nessuno, immagine, immagine+testo, separato, divisore) | implementato |
| Motore semantico a 5 livelli (analisi, ricerca, contesto, scelta, validazione) | implementato |
| Repertorio offline originale (SVG, CC BY-NC-SA 4.0) con metadati di licenza | 88 pittogrammi |
| Fallback A–G (sinonimo, forma base, concetto generale, composizione, scomposizione, vicino, segnaposto) | implementato |
| Affinità della parola: esatto, lemma, varianti, sinonimi, concetto generale, affini | implementato (deterministico, senza IA) |
| Preferenze globale / contestuale / documento con blocchi manuali | implementato |
| Pannello alternative, ricerca simboli, importazione SVG | implementato |
| Formato nativo `.caadoc` (ZIP, manifest versionato, salvataggio atomico) | implementato |
| Export HTML, PDF, PNG, DOCX e PPTX modificabili per elemento | implementato |
| Visual Resource Studio (modelli + prompt → specifica JSON → layout) | implementato (layout essenziale) |
| Sintesi vocale di sistema (macOS `say`, Windows SAPI) | implementato |
| IA locale via llama.cpp + GGUF | **inclusa**: runtime e Qwen2.5-0.5B-Instruct nel pacchetto; 1.5B, 3B e SmolLM2 1.7B scaricabili da un catalogo con URL e sha256 verificati, oppure importabili come file GGUF locali |
| Installer firmati/notarizzati | **non disponibili**: mancano certificati |
| Test su Windows con 4 GB di RAM | **non eseguiti** in questo ambiente |
| Pittogrammi ARASAAC (pacchetto `arasaac`) | **inclusi**: intero catalogo italiano, 13.906 pittogrammi, CC BY-NC-SA 4.0, autore Sergio Palao / ARASAAC |

Nessuna funzione dell'interfaccia è un pulsante finto: se il backend non è disponibile (per esempio
nell'anteprima browser) l'applicazione lo dichiara e non simula risultati.

## Requisiti di sviluppo

- Node.js 22 e npm
- Rust stabile (edizione 2021) con i target delle piattaforme desiderate
- macOS: Xcode Command Line Tools; Windows: WebView2 e Visual Studio Build Tools

## Avvio in sviluppo

```bash
npm install
npm run tauri dev        # applicazione desktop completa
npm run dev              # solo interfaccia web, in modalità anteprima (senza motore)
```

## Verifiche

```bash
npm run build                       # controllo tipi Svelte + build Vite
cd src-tauri && cargo test          # test unitari e d'integrazione, corpus semantico
python3 scripts/check_context.py    # conformità di Contesto.md (vedi sotto)
```

Il test `semantic_corpus_accuracy` misura il motore su `tests/semantic_corpus.json` (205 frasi
italiane annotate a mano) e scrive i risultati reali in `tests/last_report.json`. Il confronto con
il sistema ibrido (LLM) non viene misurato se non è configurato un modello: il report lo dichiara.

## Compilazione degli installer

```bash
npm run tauri build                                  # piattaforma corrente
npm run tauri:build:mac-arm                          # macOS Apple Silicon
npm run tauri:build:mac-intel                        # macOS Intel
npx tauri build --target x86_64-pc-windows-msvc      # Windows (da Windows)
```

Gli installer prodotti localmente non sono firmati. Prima di una build di release servono gli asset
non versionati (modello GGUF, runtime llama.cpp, catalogo ARASAAC):

```bash
python3 scripts/fetch_release_assets.py
```

La pipeline GitHub Actions (`.github/workflows/build.yml`) compila e testa su macOS e Windows e
allega gli artefatti non firmati. La release multipiattaforma è in
`.github/workflows/release.yml`: parte da un tag `v*`, compila macOS arm64/x64, Windows x64 e Linux
x64 su runner nativi, calcola gli SHA-256 e pubblica la GitHub Release. **Non è ancora stata
eseguita**: richiede un repository e il push del tag.

## Struttura

```text
src/                 interfaccia Svelte + editor ProseMirror
src-tauri/src/       backend Rust: database, linguistics, symbol_search, ai_engine,
                     asset_manager, speech, export, studio
resources/           dizionari, pacchetti simboli SVG (core, arasaac), modello neurale, template
scripts/             icone, verifica Contesto.md, import simboli, indici, benchmark
tests/               corpus semantico annotato e report generati
docs/                manuale utente, guida sviluppatori, note MCP, licenze
```

## Licenze

Il progetto è distribuito, per scelta del committente, con **CC BY-NC-SA 4.0** (Attribuzione,
Non commerciale, Condividi allo stesso modo): vale per codice, documentazione, template e
pittogrammi originali. Vedi `LICENSES/README.md` e `docs/LICENSES.md` per il dettaglio componente
per componente.

I **pittogrammi ARASAAC** sono inclusi nel pacchetto `arasaac` e restano sotto **CC BY-NC-SA 4.0**
(autore Sergio Palao, titolare dei diritti ARASAAC — Governo di Aragona); i PNG ufficiali sono
incorporati in contenitori SVG e l'attribuzione è registrata nel manifest e nell'applicazione. Il
pacchetto contiene **l'intero catalogo italiano** (13.906 pittogrammi, ~270 MB): è ricercabile per
parola chiave dall'interfaccia. Gli altri repertori di terzi non sono inclusi e si importano solo
localmente.

Per rigenerare il pacchetto (serve rete, una volta sola):

```bash
python3 scripts/fetch_arasaac.py                # tutto il catalogo
python3 scripts/fetch_arasaac.py --manifest-only  # solo il manifest dai file presenti
```

## Memoria del progetto

`Contesto.md` è la memoria tecnica condivisa tra agenti di coding: è obbligatorio leggerla prima di
modifiche significative e aggiornarla prima di terminare una sessione. È esclusa da Git
(`/Contesto.md`), non viene pubblicata e non entra negli installer. `AGENTS.md` e `CLAUDE.md`
rimandano ad essa; `scripts/check_context.py` verifica che esista, che contenga le 21 sezioni
richieste e che resti fuori dal repository.

## CLI per agenti e automazione

`cargo run --bin caa-cli -- info` avvia il motore headless. Documentazione:
[`docs/AGENT.md`](docs/AGENT.md).
