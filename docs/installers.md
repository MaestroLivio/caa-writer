# Release e installer

## Stato reale 0.15.16 (11 novembre 2026)

- Corretto l'avvio dell'editor universale e inclusa la libreria materiali
  (manifest, 505 modelli, 30 sistemi) nell'installer.
- Download in background con barra globale; ARASAAC completo, Global Symbols
  per tutti i lemmi del lessico, voce Piper installabile dalla GUI.
- macOS arm64: `.app` 225,45 MiB, `.dmg` 190.918.403 byte (182,1 MiB), firmato ad-hoc sull'intero bundle.
- Windows x64: installer NSIS cross-compilato con `cargo-xwin` e `makensis` (142.742.012 byte, 136,1 MiB), non eseguito su hardware Windows.

## Stato reale 0.15.15 (11 novembre 2026)

L'installer è stato ridotto togliendo i pacchetti opzionali: libreria materiali,
modello GGUF e voci neurali non sono più inclusi. Restano le risorse di base, il
repertorio originale `core` e il runtime llama.cpp senza modello.

| Piattaforma | Costruito | Dimensione |
|---|---|---|
| macOS arm64 | **sì**, non firmato, avviato | `.dmg` 29.360.685 byte (28,0 MiB), `.app` 74,97 MiB |
| macOS x86-64 | non ricostruito in questa sessione | — |
| Windows x64 | non ricostruito in questa sessione | — |
| Linux x64 | no su questa macchina | workflow CI |

Confronto con la 0.15.14: `.dmg` macOS arm64 da 813.780.634 byte (776,1 MiB) a
29.360.685 byte (28,0 MiB), **−96,4 %**. Gli artefatti della 0.15.14 restano in
`dist-release/0.15.14/`.

La build 0.15.15 è stata prodotta con:

```bash
npm run tauri:build:mac-arm -- --bundles app,dmg
```

Il bundle contiene `resources/image`, `dictionaries`, `symbol-packs/core`,
`templates`, `boards` e `ai/runtime`. Non contiene `resources/library`,
`model.gguf` né le voci. All'avvio l'app funziona senza pacchetti opzionali: la
finestra del primo avvio propone cosa installare e il Gestore pacchetti mostra lo
stato reale.

## Stato storico (versione 0.15.2, 8 novembre 2026)

| Piattaforma | Costruito | Come |
|---|---|---|
| macOS arm64 | **sì**, non firmato, avviato | `npm run tauri:build:mac-arm -- --bundles app,dmg` |
| macOS x86-64 | **sì**, non firmato, avviato con Rosetta | `npm run tauri:build:mac-intel -- --bundles app,dmg` |
| Windows x64 | **sì**, cross-compilato e non firmato; installer con runtime llama.cpp Windows + modello, installazione/disinstallazione provate sotto Wine 8, app non eseguita su Windows | `cargo-xwin` + LLVM + `makensis` su macOS (NSIS) |
| Linux x64 | no su questa macchina | workflow `.github/workflows/release.yml` su `ubuntu-22.04` |
| Android / ChromeOS | no | richiede SDK/NDK e `tauri android init` |

Artefatti macOS prodotti (in `dist-release/`, esclusa da Git):

```
CAA-Writer_0.13.0_macos-arm64.dmg      515,95 MiB
CAA-Writer_0.13.0_macos-arm64.app.zip  516,45 MiB
CAA-Writer_0.13.0_macos-x64.dmg        516,97 MiB
CAA-Writer_0.13.0_macos-x64.app.zip    517,54 MiB
CAA-Writer_0.13.0_windows-x64-setup.exe 490,58 MiB (NSIS)
CAA-Writer_0.13.0_cli-macos-arm64      6,5 MiB
CAA-Writer_0.13.0_cli-macos-x64        7,2 MiB
CAA-Writer_0.13.0_cli-windows-x64.exe  7,3 MiB
SHA256SUMS-macos-windows.txt
SHA256SUMS-cli.txt
AGENT.md
```

I due bundle macOS sono stati aperti: Apple Silicon nativamente, Intel tramite Rosetta 2.
L'installer Windows è stato controllato staticamente con 7-Zip e provato sotto Wine 8 (installazione
e disinstallazione): contiene `caa-writer.exe` (PE32+ x86-64), le risorse offline della libreria,
i 30 sistemi comunicativi, il runtime **llama.cpp Windows CPU** (`llama-cli.exe`,
`llama-server.exe`, DLL) e il modello `model.gguf` in `resources/ai`. Il runtime immagini Python
(torch/diffusers) non è incluso e si installa dalla scheda IA. Il bundle contiene il repertorio
offline `core`; i pittogrammi ARASAAC si scaricano on demand come PNG nella cartella dati
dell'utente e restano incorporati nei documenti.
Non è stato eseguito su Windows: la compilazione incrociata e i flag `CREATE_NO_WINDOW` sono
verificati solo staticamente. Un tentativo sotto Wine si ferma sul bootstrapper WebView2.
Non è prodotto il `.msi` perché WiX richiede un host Windows. La CLI è distribuita come binario
separato per macOS arm64 e Windows x64 (non è più inclusa nel bundle dell'app);
va avviata con `--resources` che punta alla cartella risorse dell'app installata. La dimensione
dipende dal modello GGUF in `resources/ai/`. Gli artefatti **non sono firmati né notarizzati**: al
primo avvio macOS può richiedere «Apri comunque» dal menu contestuale del Finder.

## Asset non versionati

Il repository non contiene `resources/ai/` (491 MB di modello, oltre i limiti GitHub) né
`resources/symbol-packs/arasaac/` (439 MB, solo asset di sviluppo). Prima di una build di release:

```bash
python3 scripts/fetch_release_assets.py
```

Lo script scarica il modello Qwen2.5-0.5B-Instruct, ne verifica lo sha256, scarica il runtime
llama.cpp per il sistema corrente. `--skip-arasaac` limita la build ai soli simboli originali;
`--llama-asset bin-macos-x64` serve a costruire l'app Intel da un host ARM. Il catalogo ARASAAC
completo non viene incluso nell'installer: resta come asset di sviluppo per la ricerca locale.

## Comandi per piattaforma

### macOS (eseguito)

```bash
python3 scripts/fetch_release_assets.py
npm run tauri:build:mac-arm -- --bundles app,dmg     # Apple Silicon
```

Per Intel da host ARM il runtime incluso deve essere x86-64, non arm64:

```bash
python3 scripts/fetch_release_assets.py --llama-asset bin-macos-x64 --force-llama
npm run tauri:build:mac-intel -- --bundles app,dmg
# poi ripristinare il runtime arm64 per le build successive
```

### Windows (non eseguito)

```bat
npm run tauri build -- --bundles nsis,msi
```

Richiede host Windows, WiX/NSIS e le icone `.ico` già presenti. Il workflow
`.github/workflows/build.yml` ha il job `windows-latest` già configurato, ma **non è mai stato
eseguito** su GitHub.

### Linux (non eseguito)

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libgtk-3-dev
npm run tauri build -- --bundles deb,appimage
```

Il job `ubuntu-22.04` è stato aggiunto al workflow, con le dipendenze di sistema; anche questo non è
stato eseguito.

### Android e ChromeOS (non eseguito)

ChromeOS non ha un formato installer dedicato: le app Tauri girano come APK Android (ARC) o come
applicazione Linux in Crostini. Non esiste un pacchetto «ChromeOS» separato e non va inventato.

```bash
npm run tauri android init
npm run tauri android build -- --apk
```

Prerequisiti: JDK 17, Android SDK, NDK, target Rust `aarch64-linux-android`. Il job `android` del
workflow è predisposto (solo `workflow_dispatch`) e non è mai stato eseguito. L'APK è di debug/non
firmato: per la distribuzione serve una chiave di firma.

### Release GitHub (`release.yml`)

Il workflow `.github/workflows/release.yml` parte da un tag `v*` e:

1. esegue test su macOS, Windows e Linux;
2. scarica modello, runtime e catalogo ARASAAC con
   `scripts/fetch_release_assets.py`;
3. compila bundle nativi con una matrice: macOS arm64, macOS x64, Windows x64,
   Linux x64;
4. calcola gli SHA-256 e raccoglie gli artefatti;
5. crea una GitHub Release **solo se il tag corrisponde alla versione** in
   `tauri.conf.json` e la release non esiste già.

Per macOS x64 il workflow forza `bin-macos-x64`, perché il runner `macos-latest`
è arm64. I pacchetti restano non firmati. **Il workflow non è stato eseguito in
questa sessione**: richiede un repository GitHub e il push del tag.

## Voce neurale nell'installer

Il runtime vocale neurale è **incluso nell’installer Windows**: voci `it_IT-serena-medium` e
`en_GB-cori-high` in `resources/voice/*.onnx`, motore `resources/voice/piper-win/piper.exe` con
DLL ed `espeak-ng-data`. `voice::resolve` cerca `piper-win/piper.exe`; il processo parte dalla
cartella del runtime per caricare le DLL. Su macOS il runtime Piper arm64 non è incluso e la
lettura usa la voce di sistema (in sviluppo è possibile usare `.voice-venv/bin/piper`). Se il
motore manca, `voice_status` lo dichiara con `backend: "system"`.

Per avere la voce neurale in una installazione confezionata serve impacchettare Piper con le sue
dylib (il pacchetto macOS ufficiale è incompleto) oppure distribuire un runtime Python dedicato.
Questo è il limite 35 di `Contesto.md`: la voce locale è operativa in sviluppo tramite
`scripts/setup_voice.sh`, non nell'installer.

La licenza della voce inglese (`en_GB-cori-high`, dataset LibriVox di pubblico dominio) e quella
della voce italiana (`it_IT-serena-medium`, CC BY 4.0 con attribuzione) vanno riportate nel
registro delle licenze di ogni distribuzione.
