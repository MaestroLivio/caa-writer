# CAA Writer 0.15.22 — pittogrammi legati alla parola e controlli touch

Data: 22 novembre 2026

## Correzione principale

Scrivendo «casa è bella» e poi sostituendo la frase con «famiglia felice», il
primo pittogramma non resta più «casa». Ogni scelta memorizzata registra ora la
parola a cui si riferisce: una scelta fatta per «casa» non viene applicata a
«famiglia». I suggerimenti automatici Horizon non creano più override bloccati,
quindi il motore riparte dalla parola nuova a ogni modifica del testo.

Le scelte manuali restano valide per la stessa parola e non vengono applicate
a una parola diversa.

## Modello di diffusione su Windows

Il runtime immagini cerca ora `py -3`, `python`, `python3` e le versioni
numeriche. Prima veniva interrogato solo `python3`, quindi su Windows
l'installazione poteva dichiarare Python assente anche quando `python` era
disponibile. Se Python non è installato, il messaggio indica di installarlo da
python.org o dal Microsoft Store.

## Interfaccia touch

- La modalità touch è attiva per impostazione predefinita: rimosse le opzioni
  «Modalità tocco» e «Controlli smartboard e tastiera a schermo».
- Il pulsante **Controlli** resta sempre visibile in basso a sinistra. Dal
  pannello si regola la dimensione del testo (10–32 px) e si attiva
  **TUTTO MAIUSCOLE**; il controllo duplicato è stato tolto dalle impostazioni.
- Il pannello mantiene tastiera a schermo, spazio, cancellazione, invio,
  annulla/ripeti e ascolto. Non compare in stampa.

## Modelli

Il modello Qwen2.5-0.5B-Instruct è stato rimosso dal pacchetto e dal catalogo.
Il runtime llama.cpp resta incluso; i modelli Qwen2.5-1.5B, Qwen2.5-3B e
SmolLM2-1.7B si scaricano dal catalogo o si importano come file GGUF.

## Novità della 0.15.21 mantenute

- Runtime ONNX incluso e collegato staticamente per il motore decisionale.
- Correzione del pittogramma esplicito «pap test».
- Modalità sviluppatore della libreria.
- Libreria arricchita con materiale v2, dizionario v8 e catalogo
  pre-indicizzato.

## Limiti

- Il modello neurale ONNX (268 MB) non è incluso: si installa dal Gestore
  pacchetti. Il runtime è incluso.
- Windows e Linux sono compilati dal workflow di release ma non sono stati
  eseguiti su hardware reale in questa sessione.
- La stampa usa il dialogo nativo; l'apertura reale del dialogo non è
  automatizzata nei test.

## Requisiti

- Windows 10/11 a 64 bit con WebView2.
- macOS 10.15 o successivo, Apple Silicon o Intel.
- Linux x64 con WebKitGTK 4.1 e le librerie indicate nel pacchetto `.deb`.
- 4 GB di RAM per l'uso senza modello neurale.

## Installazione

- **Windows**: usare `CAA-Writer_0.15.22_windows-x64-setup.exe` (NSIS). Il
  pacchetto non è firmato: verificare il checksum SHA-256 e scegliere
  «Ulteriori informazioni» → «Esegui comunque» se SmartScreen avvisa.
- **macOS**: aprire il `.dmg`, trascinare `CAA Writer.app` in Applicazioni.
  Al primo avvio macOS può chiedere di confermare l'apertura (l'app non è
  notarizzata). In alternativa usare il file `.app.zip`.
- **Linux**: usare il pacchetto `.deb`, `.rpm` o `.AppImage` prodotto dal
  workflow.

## Artefatti di questa release

| File | Dimensione |
|---|---|
| `CAA-Writer_0.15.22_macos-arm64.dmg` | 263.755.492 byte |
| `CAA-Writer_0.15.22_macos-arm64.app.zip` | 257.104.599 byte |
| `CAA-Writer_0.15.22_cli-macos-arm64` | 22.219.984 byte |
| `CAA-Writer_0.15.22_windows-x64-setup.exe` | 169.889.985 byte |

## Verifica dei file

Ogni artefatto ha un file `SHA256SUMS-*.txt` nella release. Esempio:

```bash
sha256sum -c SHA256SUMS-macos-arm64.txt   # Linux
shasum -a 256 -c SHA256SUMS-macos-arm64.txt   # macOS
```

## Verifiche eseguite su questa macchina

```text
npm run build                      ok, 0 errori TypeScript
npm run test:frontend              75/75
cargo test --lib                   315/315
cargo test --lib --features onnx   320/320 (modello Horizon reale)
python3 scripts/check_context.py   21/21 sezioni
```

Windows e Linux non sono stati eseguiti localmente: la verifica reale avviene
nel workflow di release.
