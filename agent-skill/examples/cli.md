# Esempi CLI copiabili

Imposta prima i percorsi reali:

```bash
BIN=src-tauri/target/debug/caa-cli
DATA=/tmp/caa-writer-data
RES=$PWD/resources
```

```bash
# Stato e catalogo
$BIN --data-dir "$DATA" --resources "$RES" info
$BIN --data-dir "$DATA" --resources "$RES" status
$BIN --data-dir "$DATA" --resources "$RES" capabilities

# Elenco metodi
$BIN --data-dir "$DATA" --resources "$RES" methods

# Simbolizzazione
$BIN --data-dir "$DATA" --resources "$RES" \
  call symbolize '{"text":"La bambina beve acqua."}'

# Ricerca e affinità
$BIN --data-dir "$DATA" --resources "$RES" call search '{"query":"acqua","limit":5}'
$BIN --data-dir "$DATA" --resources "$RES" call affinities '{"word":"beve"}'

# Impostazioni
$BIN --data-dir "$DATA" --resources "$RES" settings list --prefix ai.
$BIN --data-dir "$DATA" --resources "$RES" settings get ai.enabled

# Progetti
$BIN --data-dir "$DATA" --resources "$RES" project list
$BIN --data-dir "$DATA" --resources "$RES" \
  project create --path "demo/acqua.caadoc" --title "Acqua"
$BIN --data-dir "$DATA" --resources "$RES" project open demo/acqua.caadoc
$BIN --data-dir "$DATA" --resources "$RES" project validate demo/acqua.caadoc
$BIN --data-dir "$DATA" --resources "$RES" \
  project export demo/acqua.caadoc --format html

# Dry run: non crea il file
$BIN --data-dir "$DATA" --resources "$RES" --dry-run \
  project create --path "non-creato.caadoc" --title "Mai creato"

# Sola lettura: la scrittura viene rifiutata dal dispatch
$BIN --data-dir "$DATA" --resources "$RES" --access read \
  project create --path "negato.caadoc" --title "Negato"
# exit code 1, errore JSON su stderr

# Eliminazione: solo admin + conferma
$BIN --data-dir "$DATA" --resources "$RES" --access admin \
  project delete demo/acqua.caadoc --confirm
```

## RPC

```bash
printf '%s\n' \
  '{"id":1,"method":"app.capabilities","params":{}}' \
  '{"id":2,"method":"project.list","params":{},"access":"read"}' \
  '{"id":3,"method":"project.create","params":{"path":"dry.caadoc","title":"Dry"},"dry_run":true}' \
  | $BIN --data-dir "$DATA" --resources "$RES" rpc
```

## Exit code

| Codice | Significato |
|---|---|
| `0` | successo |
| `1` | errore applicativo o di ambiente (JSON su stderr) |
| `2` | uso non valido, flag o modalità di accesso errata |

# Aggiornamenti della libreria, per singolo pacchetto (0.16.0)
caa-cli --resources ../../resources call library.packages.status '{}'
caa-cli --resources ../../resources call library.packages.status '{"check_remote":true}'
caa-cli --resources ../../resources call library.package.outline '{"id":"teacch"}'
caa-cli --resources ../../resources call library.update_package '{"id":"storie-caa","source":"inclusa","confirm":true}'
caa-cli --resources ../../resources call library.packages.update_all '{"source":"auto"}'
