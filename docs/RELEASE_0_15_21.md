# CAA Writer 0.15.21 — correzione dei pittogrammi espliciti

Data: 20 novembre 2026

## Correzione principale

Scrivendo «pap» (prima di «papà») non compare più il pittogramma «pap test»
quando i contenuti espliciti sono disattivati.

La causa era nell'import automatico ARASAAC: una ricerca parziale accettava il
primo risultato senza verificare il titolo, quindi «pap» importava «pap test»
come copia personale con etichetta «pap». La correzione agisce su quattro
livelli:

- l'import automatico accetta solo un titolo che corrisponde davvero alla
  parola (con eventuale articolo italiano);
- una migrazione nasconde gli import automatici già sbagliati, senza
  cancellare gli asset;
- i ripieghi automatici di `resolveMissingSymbols` rispettano il filtro dei
  contenuti espliciti;
- l'editor universale applica lo stesso filtro anche ai pittogrammi già
  salvati nelle caselle.

Nello stesso aggiornamento la rianalisi non sostituisce più un pittogramma
bloccato dall'utente.

## Novità incluse

- Motore decisionale locale per disambiguazione, intenzione, categoria, board,
  emozione, azione, layout e obiettivo, con runtime ONNX incluso nella build.
- Suggerimenti Horizon applicati direttamente nell'editor principale, in
  CaaEditor e nel compositore di frasi dell'editor universale.
- Modalità sviluppatore della libreria con anteprima, modifica,
  sostituzione, aggiunta ed eliminazione dei documenti.
- Libreria arricchita con materiale v2, dizionario v8 e catalogo
  pre-indicizzato.

## Limiti

- I pesi del modello neuronale ONNX (268 MB) non sono inclusi nel bundle: si
  installano separatamente dal Gestore pacchetti. Il runtime ONNX è incluso
  nella build e collegato staticamente, quindi non serve installare librerie
  esterne.
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

- **Windows**: usare `CAA-Writer_0.15.21_windows-x64-setup.exe` (NSIS). Il
  pacchetto non è firmato: verificare il checksum SHA-256 e scegliere
  «Ulteriori informazioni» → «Esegui comunque» se SmartScreen avvisa.
- **macOS**: aprire il `.dmg`, trascinare `CAA Writer.app` in Applicazioni.
  Al primo avvio macOS può chiedere di confermare l'apertura (l'app non è
  notarizzata). In alternativa usare il file `.app.zip`.
- **Linux**: usare il pacchetto `.deb`, `.rpm` o `.AppImage` prodotto dal
  workflow.

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
cargo test --lib                   313/313
cargo test --lib --features onnx   318/318 (modello Horizon reale)
cargo build --features onnx --bin caa-cli   ok (runtime ONNX statico)
python3 scripts/check_context.py   21/21 sezioni
```

Windows e Linux non sono stati eseguiti localmente: la verifica reale avviene
nel workflow di release.
