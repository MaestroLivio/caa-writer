# CAA Writer 0.14.0 — note di rilascio

Data: 7 novembre 2026.

## Libreria

- 511 documenti totali (481 template + 30 sistemi), 0 errori di validazione.
- 181 modelli nuovi: 12 piloti, 56 secondo lotto, 56 terzo lotto, 57 quarto lotto.
- 45 schede didattiche con esercizio e soluzioni, 30 storie multipagina, 0 titoli duplicati.
- Restano 2.154 avvisi storici, soprattutto 1.098 pittogrammi mancanti dichiarati.

## Editor e ricerca

- Pannello unico **Simbolo** con ricerca italiana/inglese, traduzione con glossario e modello locale,
  fallback al termine originale se il modello non risponde.
- Risultati pertinenti separati dagli altri.
- Ricerca online federata: ARASAAC, Global Symbols, Wikimedia, Openverse.
- **ARASAAC ha la precedenza su CAA Writer Core**: la scelta del simbolo ordina `personal` →
  `arasaac` → `core` → altri e il costruttore prova prima un'etichetta esatta ARASAAC. Core resta
  il ripiego quando ARASAAC non ha il pittogramma.
- **Correzione**: il pittogramma scelto a mano compariva subito ma spariva alla rianalisi quando la
  parola non era nel lessico (per esempio «riguarda»). Ora la scelta manuale vince sempre e resta
  nel documento.
- Il pannello **Simbolo** mostra per primi i pittogrammi possibili della parola selezionata, poi la
  ricerca e gli altri strumenti.
- **Alternative anche in inglese**: la parola viene tradotta automaticamente (modello locale o
  glossario), i pittogrammi ARASAAC trovati in inglese vengono importati e mostrati con il badge
  `EN`. Senza modello né voce di glossario la funzione lo dichiara, senza inventare risultati.
- **Interfaccia unica**: barra interna dell'editor rimossa; comandi nel Ribbon (scheda Documento,
  gruppo Elementi), proprietà del documento nella scheda **Documento** della barra laterale destra,
  zoom flottante in basso a destra con Ctrl/⌘+rotella e pinch, formato foglio personalizzabile e
  stabile (larghezza/altezza in mm), righello attivo che trascina i margini, pannello pagine
  minimizzabile.
- **Stampa corretta**: il PDF ora è un PDF valido (oggetti, stream e xref); si apre nel
  visualizzatore di sistema con formato e orientamento reali.
- **Miniature reali delle pagine** nel pannello a sinistra: testi, pittogrammi, griglie, forme e
  sfondi sono disegnati in scala.
- **Apri nell'editor completo** converte un documento di testo in un documento impaginato
  modificabile (paragrafi, pittogrammi, layout), senza sovrascrivere l'originale.
- **Editor completo per ogni modello**: i 511 documenti della libreria e i materiali generati si
  aprono nell'editor universale in modifica, con layout `absolute`/`flow`/`grid` sempre
  modificabile; l'anteprima ha un pulsante esplicito «Modifica layout e contenuti» e il salvataggio
  crea una copia. Nessun ripiego silenzioso sull'editor di testo.
- **Generazione dentro l'editor**: *Storia dal testo* e *Materiale dal testo* (Strumenti IA →
  Creazione) usano il testo scritto e aprono il risultato nello stesso editor completo.
- **Scheda IA nel pannello** (e gruppo *Strumenti IA → Immagini e chat* nel Ribbon): chat con il
  modello locale o con un provider online, generazione di immagini e gestione dei modelli di
  diffusione.
- **Motore immagini neurale facoltativo**: GPU Apple (MPS) con torch/diffusers, MLX quando il
  runtime MLX Stable Diffusion è presente, oppure provider online compatibile con
  `/images/generations`. Runtime e modelli si installano o scaricano dalla scheda IA e restano nei
  dati dell'utente. Le immagini generate si applicano alla parola selezionata come pittogramma
  oppure si inseriscono nel documento universale come elemento immagine.
- Pittogrammi sulla parola, controlli smartboard e tastiera a schermo.
- Filtro dei contenuti espliciti attivo per default: esclusi da alternative, ricerca locale,
  ricerca online, documento e stampa finché l'adulto non li riattiva.

## Storie

- Story Studio e `caa-cli story create|validate|render|package`.
- Scene vettoriali modificabili, testo CAA, strisce di pittogrammi, navigazione e PDF.
- Backend immagini disponibile: vettoriale offline. I modelli diffusion locali/online non sono
  dichiarati compatibili senza download, caricamento e benchmark reali.

## Test

```text
npm run check      0 errori
npm run build      Vite riuscita
npm run test:frontend 31 passati
cargo test --lib   222 passati
library validate   511 documenti, 0 errori
image.generate     sd-turbo, MPS, 512×512, 2 passi → ~21 s (immagine reale)
image.status       torch 2.15 + diffusers 0.42 + MPS presenti; mlx assente
```

## Installer

- macOS arm64: prodotto localmente, non firmato/notarizzato.
  - `dist-release/0.14.0/CAA-Writer_0.14.0_macos-arm64.dmg`
  - `dist-release/0.14.0/CAA-Writer_0.14.0_macos-arm64.app.zip`
  - `dist-release/0.14.0/CAA-Writer_0.14.0_cli-macos-arm64`
  - checksum in `dist-release/0.14.0/SHA256SUMS.txt`
- Windows x64: **prodotto** con `cargo-xwin` e NSIS su macOS.
  - `dist-release/0.14.0/CAA-Writer_0.14.0_windows-x64-setup.exe` (172 MB)
  - `dist-release/0.14.0/CAA-Writer_0.14.0_cli-windows-x64.exe`
  - Contiene `caa-writer.exe` (PE32+ x86-64), le risorse offline (2656 file, 414 MB), la CLI,
    il bootstrapper WebView2 e il runtime Visual C++ (installati solo se mancanti).
  - **Verificato sotto Wine 8.0.1 (motore Wineskin/CabriPlus)**: installazione silenziosa
    riuscita (exit 0) e disinstallazione riuscita con rimozione completa della cartella.
  - **Non eseguito su Windows**: Wine 8 non fornisce `bcryptprimitives.dll` (presente su
    Windows 10/11), quindi l'eseguibile non parte nell'ambiente di prova. Il pacchetto non è
    firmato.
  - Il runtime neurale Windows non è incluso: le release ufficiali di llama.cpp non offrivano
    un binario CPU Windows al momento della build. Su Windows l'IA usa il glossario e un
    eventuale runtime esterno configurato; senza di esso la funzione lo dichiara.
  - Anche il runtime immagini (Python + torch/diffusers) e i modelli non sono inclusi: su
    Windows vanno installati dall'utente; `image.status` dichiara cosa manca.
