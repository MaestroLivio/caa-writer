# Rapporto automazione: CLI, MCP e Agent Skill

Data: 14 novembre 2026. Versione di sviluppo: 0.15.20 (non pubblicata).

## 1. Copertura

Il registro `src-tauri/src/automation/registry.rs` è la fonte unica. Contiene
**94 operazioni**; **92** sono esposte anche come tool MCP. Le due eccezioni
sono alias CLI volutamente non duplicati come tool: `methods` e `speak`.

| Dominio | Operazioni | CLI | MCP | Test automatici |
|---|---|---|---|---|
| applicazione | `app.capabilities`, `app.status`, `app.diagnostics`, `info`, `methods` | sì | 4 tool + 1 alias | routing, catalogo |
| impostazioni | `settings.list/get/set` | sì | sì | roundtrip reale |
| progetti | `project.list/create/open/validate/save/duplicate/rename/delete/export` | sì | sì | ciclo di vita, traversal, symlink, conferma |
| documenti | `document.create/get/update/export`, `document.open/save/pdf`, `export.html/docx` | sì | sì | workflow documento/elementi |
| contenuti | `content.generate/transform/validate`, `element.add/update/delete/move/duplicate` | sì | sì | validazione, duplicazione, eliminazione confermata |
| job | `job.start/status/result/cancel/list` | sì (in `rpc`) | sì | job reale `queued → running → done` |
| simboli | `symbolize`, `analyze.text/caadoc`, `search`, `affinities`, `symbol.*` | sì | sì | routing + test esistenti del motore |
| materiali | `templates.*`, `layout.*`, `library.*`, `prompt.interpret` | sì | sì | routing + test libreria |
| repertori | `import.*`, `collections.*`, `profiles.*`, `boards.*` | sì | sì | routing + test esistenti |
| rete/IA/voce | `online.*`, `ai.*`, `image.*`, `voice.*`, `translate.*` | sì | sì | routing; rete dipendente dall'ambiente |
| pacchetti | `packages.*` | sì | sì | routing; `packages.updates` richiede rete |

La copertura non significa che ogni operazione di rete sia stata eseguita in
questa sessione: significa che è instradata, schemata e soggetta alla stessa
politica. I test che dipendono da rete, modelli o hardware sono dichiarati.

## 2. Architettura

```text
caa-cli ─┐
caa-mcp ─┼─► automation::dispatch_with_options ─► registry + policy ─► engine
GUI ─────┘                                  │
                                            └─► jobs (processo persistente)
```

- `dispatch` e `dispatch_with_options` in `automation/mod.rs`.
- `registry.rs`: metodi, schemi, tool, catalogo, esempi.
- `policy.rs`: `read`/`write`/`admin`, `confirm`, dry-run.
- `jobs.rs`: lavoro in background, stato in memoria.
- `bootstrap.rs`: percorsi condivisi.
- `bin/caa-cli.rs`, `bin/caa-mcp.rs`: solo trasporto e presentazione.

## 3. Test eseguiti realmente

| Comando | Esito |
|---|---|
| `cargo test --lib` | **258/258 ok** |
| `cargo test --bin caa-mcp` | **3/3 ok** |
| `cargo test --test universal_documents` | **11/11 ok** |
| `npm run build` | ok (warning già noti) |
| `npm run test:frontend` | **69/69 ok** |
| `python3 scripts/check_context.py` | 21/21 sezioni, non tracciato |
| `python3 agent-skill/tests/check_skill.py --resources resources` | 94 operazioni, 94 documentate |

Harness reale con dati isolati:

- `info`, `capabilities`, `capabilities --format markdown`;
- creazione progetto con spazio nel percorso, elenco, validazione, export HTML;
- dry-run senza creazione del file;
- `--access read` che rifiuta la scrittura con exit 1;
- rifiuto di `../evil.caadoc`;
- job via `rpc`: `queued → running → done` con risultato reale;
- MCP: `initialize`, 92 tool, `resources/read`, `prompts/get`, `ping`, tool call.

Non eseguiti/dichiarati:

- `cargo test --test semantic_corpus` non è verde senza il catalogo ARASAAC
  locale, assente dal repository.
- Windows e Linux non sono stati eseguiti su hardware reale.
- Le operazioni online non sono state tutte esercitate: dipendono dalla rete.

## 4. Installazione

```bash
cd "CAA Writer/src-tauri"
cargo build --release --bin caa-cli --bin caa-mcp
```

CLI:

```bash
./target/release/caa-cli --data-dir "$HOME/.caa-data" --resources ../resources info
```

MCP (stdio):

```json
{
  "mcpServers": {
    "caa-writer": {
      "command": "/percorso/src-tauri/target/release/caa-mcp",
      "args": ["--data-dir", "/percorso/dati", "--resources", "/percorso/risorse", "--access", "write"]
    }
  }
}
```

Agent Skill: copia `agent-skill/` nella directory skill del client, con
`SKILL.md` nella radice. Dettagli in `agent-skill/SKILL.md`.

## 5. Rigenerare il catalogo della skill

```bash
cd src-tauri
cargo build --bin caa-cli
cd ..
TMP=$(mktemp -d)
src-tauri/target/debug/caa-cli --data-dir "$TMP/data" --resources resources \
  capabilities --format markdown > agent-skill/references/capabilities.md
python3 agent-skill/tests/check_skill.py --resources resources
```

## 6. Limiti residui

1. **Bridge GUI**: implementato su `127.0.0.1` con token in
   `<data-dir>/automation-bridge.json`, ricaricamento live e rilevamento
   conflitti nel frontend. Un osservatore del filesystem notifica anche le
   modifiche fatte da processi esterni (CLI headless, MCP, job), compresi i
   file modificati prima che la GUI si accorga del cambiamento. Resta un
   servizio locale: se la GUI è chiusa CLI e MCP continuano in headless.
2. **MCP**: stdio e Streamable HTTP (`--transport http`, endpoint `POST /mcp`)
   condividono lo stesso dispatch. Il trasporto HTTP è pensato per client
   locali; su indirizzi non loopback richiede `--token`.
3. **Job** in memoria, annullamento cooperativo, motore separato.
4. **Windows/Linux** non verificati in questa sessione.
5. **ARASAAC** non nel repository; il corpus semantico che lo richiede non è
   stato dichiarato verde.
6. Le operazioni di rete falliscono esplicitamente offline: non vengono
   simulate.
