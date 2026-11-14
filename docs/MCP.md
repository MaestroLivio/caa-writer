# Server MCP di CAA Writer

## Stato reale

CAA Writer include un **server MCP locale** (`caa-mcp`) con trasporto **stdio**.
Non è un servizio remoto e non richiede account. È stato implementato e
verificato in questa sessione: `initialize`, `tools/list`, `tools/call`,
`resources/list`, `resources/read`, `prompts/list`, `prompts/get` e `ping`
funzionano con messaggi JSON-RPC reali su stdin/stdout.

Il server **non** è un client MCP e **non** collega servizi esterni. I servizi
SVGMaker, CleanVector, SVG.new e Svg/icons citati in passato non sono
configurati né usati.

## Perché esiste

Espone la stessa API applicativa della CLI, senza duplicare la logica:

```text
caa-cli / caa-mcp / GUI
          │
          ▼
  automation::dispatch
          │
          ├── policy (read/write/admin, confirm, dry-run)
          ├── registry (metodi, schemi, tool, catalogo)
          └── engine.rs, database, universal, export...
```

Ogni tool MCP corrisponde a un metodo del registro con campo `tool`. Gli alias
solo CLI (`methods`, `speak`) non sono esposti come tool.

## Avvio

```bash
cd src-tauri
cargo build --bin caa-mcp

./target/debug/caa-mcp \
  --data-dir "/percorso/dati" \
  --resources "../resources" \
  --access write
```

| Flag | Valori | Default |
|---|---|---|
| `--data-dir` | percorso | cartella dati dell'app o `CAA_WRITER_DATA_DIR` |
| `--resources` | percorso con `dictionaries/it-core.json` | `resources/` trovata automaticamente |
| `--access` | `read`, `write`, `admin` | `write` |
| `--transport` | solo `stdio` | `stdio` |

Per un agente in sola lettura usa `--access read`. Le operazioni distruttive
richiedono `admin` **e** `confirm: true` negli argomenti.

## Trasporto

- **stdio**: messaggi JSON-RPC 2.0 delimitati da riga (standard MCP). In
  ingresso è accettato anche il framing `Content-Length`.
- **HTTP**: `caa-mcp --transport http --bind 127.0.0.1:8765` espone
  `POST /mcp` (JSON-RPC, risposta JSON) e `GET /health`. Il protocollo è
  Streamable HTTP minimale: una richiesta per connessione, sessione riportata
  nell'header `Mcp-Session-Id`. Su indirizzi non loopback serve `--token`.
- I log vanno su **stderr**; **stdout** (stdio) contiene solo protocollo.
- Il server non esegue shell e non espone percorsi arbitrari.

## Handshake di esempio

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
```

Risposta di `initialize` (campi principali):

```json
{
  "protocolVersion": "2024-11-05",
  "capabilities": {
    "tools": {"listChanged": false},
    "resources": {"subscribe": false, "listChanged": false},
    "prompts": {"listChanged": false}
  },
  "serverInfo": {"name": "caa-writer", "title": "CAA Writer", "version": "0.15.20"}
}
```

Il server negozia la versione `2024-11-05`. Il client può proporre una versione
diversa; il server risponde con la propria.

## Tool

`tools/list` restituisce 92 tool generati dal registro. Ogni tool ha:

- `name` con underscore (`project_create`);
- `description` con metodo CLI e livello di rischio;
- `inputSchema` JSON Schema;
- `annotations` (`readOnlyHint`, `destructiveHint`, `idempotentHint`,
  `openWorldHint`);
- `x-caa-method` e, quando presente, `x-caa-example`.

`tools/call`:

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call",
 "params":{"name":"project_list","arguments":{}}}
```

Risposta:

```json
{
  "content": [{"type":"text","text":"{...}"}],
  "structuredContent": {"root":"...","items":[],"count":0},
  "isError": false
}
```

Un errore applicativo reale restituisce `isError: true` e il messaggio nel
testo; non viene trasformato in un finto successo.

### Dry-run da MCP

Aggiungi `dry_run: true` agli argomenti del tool:

```json
{"name":"project_create","arguments":{"path":"x.caadoc","title":"X","dry_run":true}}
```

Il campo viene rimosso prima del dispatch e la risposta contiene
`dry_run: true` senza creare file.

## Risorse

| URI | Contenuto |
|---|---|
| `caa://capabilities` | catalogo JSON generato dal registro |
| `caa://status` | stato applicativo reale |
| `caa://instructions` | istruzioni operative e limiti |

Le risorse sono lette con `resources/read`. Non richiedono rete.

## Prompt

| Nome | Argomenti | Uso |
|---|---|---|
| `create_material` | `request`, `template_id?` | pianifica un materiale CAA |
| `edit_document` | `path`, `request` | modifica un progetto |
| `find_symbol` | `word` | cerca simboli senza inventarli |
| `analyze_text` | `text` | simbolizza e segnala i buchi |

I prompt sono testo di orientamento: non eseguono operazioni da soli. L'agente
deve chiamare i tool e riportare i risultati reali.

## Configurazione in un client MCP

```json
{
  "mcpServers": {
    "caa-writer": {
      "command": "/percorso/assoluto/src-tauri/target/debug/caa-mcp",
      "args": [
        "--data-dir", "/percorso/dati",
        "--resources", "/percorso/risorse",
        "--access", "write"
      ]
    }
  }
}
```

La stessa configurazione vale per OpenCode, Codex e Claude Code, purché il
client supporti i server MCP stdio. La Agent Skill è in `agent-skill/`.

## Verifica eseguita

```bash
printf '%s\n%s\n%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"app_get_capabilities","arguments":{}}}' \
  | ./target/debug/caa-mcp --data-dir "$TMP/data" --resources ../resources --access read
```

Esito reale in questa sessione: `initialize` ok, 92 tool, chiamata ok con 94
operazioni nel catalogo. I test di framing sono in `caa-mcp.rs` e si eseguono
con `cargo test --bin caa-mcp`.

## Limiti dichiarati

- Solo stdio; nessun HTTP.
- Nessun bridge con la GUI aperta: non c'è sincronizzazione live del documento
  ProseMirror.
- I job vivono in memoria nel processo MCP e usano una propria istanza del
  motore; l'annullamento è cooperativo.
- Le operazioni di rete dipendono dalla rete e dalle configurazioni locali;
  offline falliscono con un errore esplicito.
- Windows e Linux non sono stati eseguiti in questa sessione.

## Perché i pittogrammi del repository sono originali

Il pacchetto `core` contiene SVG disegnati in casa, con licenza CC BY-NC-SA 4.0
e senza dipendenze da servizi esterni. ARASAAC non è nel repository: le copie
si scaricano online nella cartella dati dell'utente, con licenza CC BY-NC-SA
4.0 e attribuzione a Sergio Palao. Questo mantiene il repository distribuibile
e l'app utilizzabile offline dopo il primo download esplicito.
