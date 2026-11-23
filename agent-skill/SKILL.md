---
name: caa-writer
description: Pilotare CAA Writer (videoscrittore CAA italiano offline) tramite la CLI headless caa-cli o il server MCP caa-mcp. Usa questa skill per creare, analizzare, modificare ed esportare documenti .caadoc, simbolizzare testo italiano, cercare pittogrammi, costruire materiali CAA e gestire job lunghi, senza inventare risultati.
---

# CAA Writer per agenti

CAA Writer è un'app desktop italiana per la Comunicazione Aumentativa e
Alternativa. Questa skill insegna a pilotarla con la **stessa API applicativa**
esposta in due modi:

- **CLI** `caa-cli`: headless, JSON, adatta a script, CI e debug;
- **MCP** `caa-mcp`: server stdio con tool, risorse e prompt, adatto a client
  agentici che supportano il Model Context Protocol.

Le due interfacce chiamano lo stesso dispatch: stessi metodi, stessi errori,
stesse regole di accesso. Non esistono due implementazioni da tenere allineate.

## Caricamento progressivo

1. Leggi solo questa pagina per orientarti.
2. Apri `references/capabilities.md` (generato dal registro reale) per schemi,
   categorie e nomi dei tool. Non fidarti di elenchi copiati a mano.
3. Apri `references/workflows.md` per i flussi completi.
4. Apri `references/security.md` prima di qualsiasi scrittura o eliminazione.
5. Gli esempi eseguibili sono in `examples/`.

## Installazione nella skill del client

Copia l'intera cartella `agent-skill/` nella directory delle skill del tuo
client, mantenendo `SKILL.md` nella radice della cartella. I percorsi cambiano
tra versioni e client: verifica la documentazione del client.

```bash
# Esempi di destinazione (adatta il percorso alla tua installazione)
cp -R agent-skill ~/.config/opencode/skills/caa-writer
cp -R agent-skill ~/.claude/skills/caa-writer
cp -R agent-skill ~/.codex/skills/caa-writer
```

Per il server MCP aggiungi la configurazione descritta in
`docs/MCP.md`. La skill e il server sono indipendenti: puoi usare la CLI senza
MCP, oppure MCP senza skill.

## Scelta rapida: CLI o MCP

| Situazione | Usa |
|---|---|
| Script, shell, CI, debug di una singola chiamata | CLI |
| Client agentico con supporto MCP | MCP stdio |
| Molte chiamate brevi | `caa-cli rpc` (JSONL, motore avviato una volta) |
| Lavoro lungo (immagini, validazione biblioteca) | `job.start` + polling, solo in `caa-cli rpc` o MCP |
| Serve isolamento totale dei dati | CLI/MCP con `--data-dir` e `--resources` |

In entrambi i casi vale la regola: **leggi prima, scrivi dopo, conferma le
operazioni distruttive**.

## Avvio

```bash
# dalla radice del repository
cd src-tauri
cargo build --bin caa-cli --bin caa-mcp

# CLI
./target/debug/caa-cli --data-dir /tmp/caa-data --resources ../resources info
./target/debug/caa-cli --data-dir /tmp/caa-data --resources ../resources capabilities

# MCP (stdio, una richiesta JSON per riga)
./target/debug/caa-mcp --data-dir /tmp/caa-data --resources ../resources --access write
```

Flag importanti:

| Flag | CLI | MCP | Significato |
|---|---|---|---|
| `--data-dir` | sì | sì | database, asset, progetti |
| `--resources` | sì | sì | lessico, pacchetti, template |
| `--access read\|write\|admin` | sì | sì | limita le operazioni |
| `--dry-run` | sì | no (usa `dry_run: true` negli argomenti del tool) | mostra senza eseguire |
| `--compact` | sì | no | JSON su una riga |

La **radice progetti** è sempre `<data-dir>/projects`. I tool `project_*`
accettano solo percorsi relativi e rifiutano `..`, percorsi assoluti e link
simbolici che escono dalla radice.

## Metodi e tool

- I metodi CLI usano il nome puntato, per esempio `project.create`.
- I tool MCP usano il nome con underscore, per esempio `project_create`.
- La corrispondenza è nel catalogo (`tool` e `x-caa-method`); non indovinarla.
- Due alias sono solo CLI: `methods` e `speak`.

```bash
caa-cli methods
caa-cli call app.capabilities '{}'
caa-cli call project.create '{"path":"demo/cane.caadoc","title":"Il cane"}'
caa-cli call symbolize '{"text":"Il gatto dorme."}'
```

## Flusso minimo sicuro

1. `app.capabilities` → verifica metodi, schemi e modalità.
2. `app.status` → controlla percorsi, database, modello e avvisi.
3. `project.list` → elenca i progetti esistenti.
4. `project.create` oppure `project.open`.
5. `content.validate` / `project.validate` → controlla prima di salvare.
6. `project.export` → PDF, PNG, HTML, SVG, DOCX, PPTX o JSON.
7. Se un pittogramma manca: `search`, `affinities`, poi eventualmente
   `online.search`. **Non inventare simboli** e riporta i warning.

## Libreria: aggiornamenti per pacchetto

La libreria è divisa in pacchetti (categorie) con una versione propria. Prima di
aggiornare qualcosa, leggi lo stato reale: non dedurre che un pacchetto sia
vecchio perché l'app lo dice una volta.

```bash
# stato offline di tutti i pacchetti
caa-cli call library.packages.status '{}'
# stato con confronto sul repository (richiede rete; se manca, arriva un avviso)
caa-cli call library.packages.status '{"check_remote":true}'
# scheda didattica: obiettivi, pubblico, sottocategorie, documenti
caa-cli call library.package.outline '{"id":"teacch"}'
# aggiorna un solo pacchetto dalla copia inclusa
caa-cli call library.update_package '{"id":"schede","source":"inclusa","confirm":true}'
```

Tre versioni per pacchetto: `installed_version` (profilo dell'utente),
`bundled_version` (inclusa nell'app), `remote_version` (repository). `status` è
`aggiornato` o `aggiornabile`; `action` è `inclusa`, `repository` o `nessuna`.

Regole:

- `source`: `inclusa` funziona sempre offline; `repository` scarica dal branch
  `main`; `auto` prova il repository e ripiega sulla copia inclusa.
- L'aggiornamento scrive solo nella cartella dati dell'utente: non tocca i file
  dell'app e non modifica gli altri pacchetti.
- Se il pacchetto è già installato in una versione **diversa**, il dispatch
  rifiuta senza `confirm: true`. Chiedere all'utente prima di confermare un
  downgrade: `confirm` è una decisione umana, non un default.
- `library.update_package` non è distruttivo per gli altri materiali, ma
  sostituisce i documenti del pacchetto scelto: dichiaralo prima di eseguirlo.
- Non esiste un aggiornamento della libreria intera come singola operazione:
  `library.packages.update_all` aggiorna solo i pacchetti indietro.

## Regole non negoziabili

- Non dichiarare un test eseguito se non lo hai eseguito.
- Non descrivere come funzionante ciò che il dispatch rifiuta o che richiede
  rete/credenziali non configurate.
- Non sovrascrivere le preferenze manuali dell'utente.
- Non usare `confirm: true` per aggirare una richiesta dell'utente: la conferma
  è una decisione umana, non un default dell'agente.
- Con `--access read` ogni scrittura è rifiutata dal dispatch, non solo dal
  prompt del modello.
- Il testo originale del documento resta la fonte di verità.

## Limiti dichiarati di questa versione

- **Nessun bridge GUI**: CLI e MCP lavorano in headless. Se la GUI è aperta, le
  modifiche ai file non vengono ricaricate nell'editor aperto. Non simulare
  digitazione o sincronizzazione live.
- **Nessun server HTTP MCP**: è implementato solo stdio. Non esiste un endpoint
  Streamable HTTP.
- **Job**: girano in un thread con una propria istanza del motore; lo stato è in
  memoria e si perde alla chiusura del processo. L'annullamento è cooperativo.
- **Rete**: le operazioni online falliscono esplicitamente offline. ARASAAC è
  CC BY-NC-SA 4.0 (Sergio Palao): conserva attribuzione e licenza.
- **Piattaforme**: il codice è multipiattaforma, ma in questa sessione sono
  stati eseguiti test solo su macOS. Windows e Linux non sono stati verificati.
- **Corpus semantico**: `cargo test --test semantic_corpus` richiede il catalogo
  ARASAAC locale, assente nel repository; senza catalogo non è un test verde.

## Riferimenti

- `references/capabilities.md` — catalogo completo generato.
- `references/workflows.md` — ricette CLI e MCP.
- `references/security.md` — accesso, conferme, path safety, licenze.
- `examples/cli.md` — comandi copiabili.
- `examples/mcp.jsonl` — sequenza MCP di esempio.
- `docs/AGENT.md` e `docs/MCP.md` nel repository — documentazione estesa.
