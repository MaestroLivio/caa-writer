# Flussi di lavoro reali

Tutti gli esempi presuppongono:

```bash
export CAA_DATA=/tmp/caa-writer-data
export CAA_RES=/percorso/assoluto/risorse   # cartella con dictionaries/it-core.json
BIN=src-tauri/target/debug/caa-cli
```

Il primo avvio importa il lessico e il pacchetto core: può richiedere qualche
secondo. Le operazioni online richiedono rete e non sono necessarie per aprire,
salvare o stampare.

## 1. Ispezione iniziale

```bash
$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" info
$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" status
$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" capabilities --format markdown
```

## 2. Creare un progetto e salvarlo

```bash
$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" \
  project create --path "esercizi/cane.caadoc" --title "Il cane" --kind document

$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" \
  project validate esercizi/cane.caadoc

$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" \
  project export esercizi/cane.caadoc --format pdf
```

La destinazione predefinita dell'export è `exports/<nome>.<formato>` dentro la
radice progetti. Per scegliere un altro percorso relativo usa `--out`.

## 3. Simbolizzare e controllare i buchi

```bash
$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" \
  call symbolize '{"text":"Il gatto dorme sul divano."}'

$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" \
  call content.validate '{"text":"Il gatto dorme sul divano."}'
```

`content.validate` restituisce `missing` con parola e strategia. Non riempire i
buchi con simboli arbitrari: usa `search`/`affinities` e, se serve la rete,
`online.search`; dichiara i warning.

## 4. Aggiungere e modificare elementi

```bash
$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" \
  call element.add '{
    "path":"esercizi/cane.caadoc",
    "page":"page-1",
    "element":{"id":"e1","kind":"text","text":"Ciao","x_mm":10,"y_mm":10,"w_mm":40,"h_mm":12}
  }'

$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" \
  call element.update '{
    "path":"esercizi/cane.caadoc","page":"page-1","id":"e1","patch":{"text":"Ciao mondo"}
  }'

# Distruttivo: richiede accesso admin e conferma esplicita.
$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" --access admin \
  call element.delete '{"path":"esercizi/cane.caadoc","page":"page-1","id":"e1","confirm":true}'
```

## 5. Modifiche in transazione

`document.update` applica `DocumentCommand` tipizzati e salva in modo atomico:

```bash
$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" call document.update '{
  "path":"esercizi/cane.caadoc",
  "commands":[
    {"type":"SetText","page":"page-1","id":"e1","text":"Nuovo testo"},
    {"type":"AddPage","title":"Pagina 2","layout":"flow"}
  ]
}'
```

Con `out` diverso da `path` non tocchi l'originale.

## 6. Generare un materiale

```bash
$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" call content.generate '{
  "kind":"material",
  "prompt":"agenda visiva della giornata a scuola",
  "out":"/tmp/agenda.caadoc"
}'
```

`kind` può essere `material`, `story` (richiede `spec`), `layout` (richiede
`spec`) o `image` (richiede prompt e motore immagini configurato). Se il motore
manca, il comando restituisce l'errore reale: non simulare l'immagine.

## 7. Job lunghi

I job vivono **nel processo che li avvia**. Usa `caa-cli rpc` o il server MCP;
una singola invocazione `caa-cli call job.start ...` termina subito e il job non
sopravvive alla chiusura del processo.

```bash
$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" rpc
```

```json
{"id":1,"method":"job.start","params":{"method":"library.validate","params":{"max_issues":20}}}
{"id":2,"method":"job.status","params":{"id":"job-..."}}
{"id":3,"method":"job.result","params":{"id":"job-..."}}
{"id":4,"method":"job.cancel","params":{"id":"job-..."}}
```

Il job apre una propria istanza del motore sugli stessi percorsi. Evita di
lanciare job che scrivono sugli stessi file contemporaneamente alla GUI.

## 8. RPC JSONL

```bash
$BIN --data-dir "$CAA_DATA" --resources "$CAA_RES" rpc
```

```json
{"id":1,"method":"app.status","params":{}}
{"id":2,"method":"project.list","params":{}}
{"id":3,"method":"symbolize","params":{"text":"Il cane corre."},"access":"read"}
{"id":4,"method":"project.create","params":{"path":"dry.caadoc","title":"Dry"},"dry_run":true}
```

Ogni riga produce una risposta `{"id":...,"ok":true|false,...}`. Il campo
`access` e `dry_run` della singola richiesta sovrascrivono i flag globali.

## 9. MCP

Configurazione tipica di un client MCP:

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

Sequenza:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":"caa://capabilities"}}
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"app_get_status","arguments":{}}}
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"project_list","arguments":{}}}
```

`tools/call` restituisce `content` con testo JSON e `structuredContent` con lo
stesso oggetto. `isError: true` indica un errore applicativo reale.
