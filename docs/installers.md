# Release e installer

## Stato reale 0.16.3 (26 novembre 2026)

Il bundle macOS 0.16.2 aveva una dipendenza esterna: i tre eseguibili
(`caa-writer`, `caa-cli`, `caa-mcp`) caricavano `libdav1d.7.dylib` da
`/opt/homebrew/opt/dav1d/lib/`. La libreria è ora inclusa in
`Contents/Frameworks` e i binari la caricano con `@rpath`. Il packaging usa
`scripts/sign_macos.sh`, che controlla che non restino dipendenze verso
Homebrew, `/Volumes` o directory utente. Il test sulla copia scaricata del
bundle 0.16.2 ha verificato la firma, il riferimento `@rpath` in tutti e tre i
binari e la presenza della dylib nel bundle. Il bundle 0.16.3 scaricato dalla
release è stato montato e verificato: firma valida, tutti i Mach-O senza
dipendenze da Homebrew/volumi/cartelle utente, CLI avviata con risorse dal
bundle.

La release `v0.16.3` è pubblicata come Latest. Asset macOS arm64:
`CAA-Writer_0.16.3_macos-arm64.dmg` (210.525.053 byte) e
`CAA-Writer_0.16.3_macos-arm64.app.zip` (204.291.611 byte).
SHA-256 verificati sui file scaricati: DMG
`5f821a2ccbd41c5f6006607303e461362e810a14a47259364a7ed9ad26cc78b8`,
ZIP `df9d20d9da33241b10752f3813ea9348922cb52cb9e9f04cf7789007fd0c4fe3`.

## Stato reale 0.16.2 (26 novembre 2026)

Correzione della voce neurale su Windows. `link_engine` copiava soltanto
`piper.exe` in `<dati>/voice/bin/`, lasciando DLL ed `espeak-ng-data` in
`<dati>/voice/piper/piper/`: il loader di Windows non li trovava e a ogni
lettura compariva la finestra «piper.exe — L'esecuzione del codice non può
proseguire perché piper_phonemize.dll non è stato trovato» (o `espeak-ng.dll`).

- Il motore si esegue dal runtime estratto, con DLL ed `espeak-ng-data` accanto;
  la copia incompleta delle versioni precedenti viene rimossa.
- `voice::runtime_is_complete` scarta un `piper.exe` isolato: le installazioni
  già rotte passano al runtime integro senza riscaricare nulla.
- `SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX)` spegne le
  finestre di errore del loader; se una DLL manca davvero, la lettura ricade
  sulla voce di sistema.

Verifiche reali (macOS arm64):

```text
cargo test --all-targets --features onnx    326 lib + integrazione, 0 falliti
npm run build / npm run check                 ok, 0 errori
npm run test:frontend                         75/75
cargo xwin check --all-targets --target x86_64-pc-windows-msvc   compila
```

Il comportamento Windows è coperto dai test `runtime_is_complete`,
`resolve` che scarta l'eseguibile isolato, `link_engine` senza copia sola e
maschera di `SetErrorMode`; l'esecuzione reale è nella CI del tag `v0.16.2`.

## Stato reale 0.15.24 (22 novembre 2026)

Patch di manutenzione sulla 0.15.23, con tre correzioni emerse dai test su
Windows nella CI:

- i percorsi degli asset riparati usano il separatore nativo (niente più
  `assets\arasaac/picto.png` misti) e `..` viene ignorato;
- il fixture della voce crea `piper.exe` su Windows;
- il test del runtime llama.cpp si salta, dichiarandolo, quando
  `resources/ai/runtime` non è presente (la cartella non è in Git).
- macOS arm64: `.dmg` 214.329.266 byte (204,4 MiB), `.app.zip` 208.208.183 byte,
  `.app` 278,25 MiB, firmato ad-hoc e verificato dal DMG montato; CLI macOS
  arm64 22.040.352 byte.
- Windows x64: installer NSIS cross-compilato 145.057.925 byte (138,3 MiB),
  `caa-writer.exe` PE32+ GUI; non eseguito su hardware Windows.

Verifiche reali (macOS arm64):

```text
cargo test --lib                                     313/313
cargo test --lib --features onnx                     318/318
npm run build / npm run check                          ok, 0 errori
npm run test:frontend                                 75/75
cargo test --test semantic_corpus --features onnx     saltato: catalogo ARASAAC assente
```

## Stato reale 0.15.23 (22 novembre 2026)

- Modello semantico Horizon e voce neurale italiana installati e verificati al
  primo avvio, senza rinvio; voce e suoni attivi per impostazione predefinita.
  Le build di release usano `--features onnx`, quindi Horizon funziona davvero
  fuori dalle build di sviluppo.
- Motore di generazione immagini (diffusione, runtime Python, provider online) e
  provider IA online rimossi: il registro espone **106 operazioni**, nessuna
  `image.*`; la chat usa solo un GGUF locale opzionale e non simula risposte.
- Lettura tollerante alla codifica (BOM, UTF-16, byte non validi) per documenti,
  manifest, specifiche e file importati.
- Libreria curata: **350 materiali** su 9 categorie canoniche, 0 errori di
  validazione, 1600 avvisi di pittogrammi ARASAAC assenti dal repository
  (risolti a runtime dal catalogo installato). Le anteprime non sono nel bundle.
- `Cargo.toml` dichiara i binari con `autobins = false`: su un volume exFAT
  macOS creava `src/bin/._caa-cli.rs`, che Cargo trattava come un binario e il
  bundler Tauri cercava come `._caa-cli`, facendo fallire il bundle.
- macOS arm64: `.app` 292.427.627 byte (278,88 MiB), `.dmg` 214.440.765 byte
  (204,5 MiB), `.app.zip` 208.207.987 byte, firmato ad-hoc sull'intero bundle e
  verificato dal DMG montato.
- Windows x64: installer NSIS cross-compilato con `cargo-xwin` e `makensis`
  (145.061.121 byte, 138,3 MiB); `caa-writer.exe` è PE32+ **GUI** (nessuna
  finestra console), non eseguito su hardware Windows.
- CLI macOS arm64: 22.040.336 byte.

Verifiche reali:

```text
codesign --verify --deep --strict (app dal DMG)      valido
caa-cli info (binario e risorse del bundle)          lessico 127 concetti / 192 lemmi
caa-cli call library.validate (dal bundle)           350 documenti, 0 errori, 1600 avvisi, 301 validi
caa-cli methods (dal bundle)                         106 operazioni, nessuna image.*
agent-skill/tests/check_skill.py                     coerente 106/106
python3 scripts/check_context.py                     conforme, 21/21 sezioni
npm run build                                        ok
npm run check                                        0 errori, 59 avvisi
npm run test:frontend                                75/75
cargo test --lib --features onnx                     318/318
cargo test --lib (feature predefinite)               313/313
cargo test --test semantic_corpus --features onnx    FALLITO: 95,2% prima scelta,
                                                     15 approssimazioni fuorvianti
                                                     (soglia 5), senza catalogo ARASAAC
```

## Stato reale 0.15.22 (22 novembre 2026)

- Pittogrammi legati alla parola: la migrazione 6 aggiunge `surface` a
  `document_overrides`; una scelta fatta per «casa» non viene applicata a
  «famiglia». I suggerimenti automatici Horizon non creano più override
  bloccati.
- Runtime immagini Windows: `python_path` e `install_runtime` cercano anche
  `py -3`, `python` e `python3`, non solo `python3`.
- Modalità touch predefinita e pulsante **Controlli** sempre visibile, con
  dimensione testo (10–32 px) e TUTTO MAIUSCOLE. Rimosse le due opzioni dalle
  impostazioni.
- Rimosso Qwen2.5-0.5B-Instruct dal pacchetto e dal catalogo; resta il runtime
  llama.cpp e i modelli 1.5B, 3B e SmolLM2 1.7B scaricabili.
- macOS arm64: `.dmg` 263.755.492 byte, `.app.zip` 257.104.599 byte, firmato
  ad-hoc sull'intero bundle e verificato dal DMG.
- Windows x64: installer NSIS cross-compilato (169.889.985 byte), controllato
  staticamente con 7-Zip; non eseguito su hardware Windows.
- CLI macOS arm64: 22.219.984 byte.

Verifiche reali:

```text
codesign --verify --deep --strict dal DMG montato   valido
otool -L caa-writer                                  solo @rpath/libdav1d.7.dylib
caa-cli call decision.status                         runtime_feature: true
avvio dell'app dal bundle                            processo attivo, chiuso correttamente
npm run build                                        ok
npm run test:frontend                                75/75
cargo test --lib                                     315/315
cargo test --lib --features onnx                     320/320
```

## Stato reale 0.15.21 (20 novembre 2026)

- Correzione dei pittogrammi espliciti: scrivendo «pap» non compare più «pap
  test» con i contenuti espliciti disattivati. Import ARASAAC vincolato al
  titolo, migrazione 5 per nascondere gli import sbagliati, filtro nei ripieghi
  automatici e nei token salvati dell'editor universale.
- Build di release con `--features onnx`: ONNX Runtime è collegato
  staticamente, quindi non serve una libreria esterna. I pesi del modello
  restano separati e si installano dal Gestore pacchetti.
- Aggiunto `mainBinaryName` in `tauri.conf.json`: il bundle macOS usa
  `caa-writer` come eseguibile principale. Rimossi i file AppleDouble `._*`
  dalla cartella `src/bin`, che facevano credere a Tauri che esistesse un
  binario chiamato `._caa-cli`.
- macOS arm64: `.dmg` 263.829.218 byte (251,6 MiB), `.app.zip`
  257.104.118 byte, firmato ad-hoc sull'intero bundle e verificato dal DMG.
- Windows x64: installer NSIS cross-compilato (169.757.756 byte), controllato
  staticamente con 7-Zip; non eseguito su hardware Windows.
- CLI macOS arm64: 22.219.984 byte.

Verifiche reali:

```text
codesign --verify --deep --strict dal DMG montato   valido
otool -L caa-writer                                  solo @rpath/libdav1d.7.dylib
caa-cli info (dal bundle)                            versione 0.15.21
caa-cli call decision.status                         runtime_feature: true
```

## Stato reale 0.15.20 (12 novembre 2026)

- Correzione critica macOS: `libdav1d.7.dylib` è inclusa in
  `Contents/Frameworks`, il binario la cerca con
  `@rpath/libdav1d.7.dylib` e `scripts/sign_macos.sh` fallisce se resta un
  percorso Homebrew. La 0.15.19 non partiva su Mac senza Homebrew.
- Verifica reale: `otool -L` senza percorsi `/opt/homebrew`, app avviata dal
  DMG montato con `DYLD_PRINT_LIBRARIES` e libreria caricata dal bundle.
- Include le modifiche dell'editor Utilizzatore della 0.15.19.
- macOS arm64: `.dmg` 192.029.092 byte, `.app.zip` 186.803.135 byte, firmato
  ad-hoc sull'intero bundle.
- Windows x64: installer NSIS cross-compilato (145.803.698 byte), controllato
  staticamente con 7-Zip; non eseguito su hardware Windows.
- CLI macOS arm64: 7.225.728 byte.

## Stato reale 0.15.19 (12 novembre 2026)

- Editor Utilizzatore: pannello destro e pannello Simboli ridimensionabili e
  minimizzabili, preferenze per profilo; l'editor occupa la finestra e non
  mostra più la schermata Creator sotto i simboli.
- Aggiunto «Apri» per i documenti scritti con l'editor Utilizzatore; i
  materiali della libreria restano nell'Area Creator.
- macOS arm64: `.dmg` 192.019.637 byte, `.app.zip` 186.456.553 byte, firmato
  ad-hoc sull'intero bundle e verificato dal DMG.
- Windows x64: installer NSIS cross-compilato (145.814.213 byte), controllato
  staticamente con 7-Zip; non eseguito su hardware Windows.
- CLI macOS arm64: 7.225.728 byte.

## Stato reale 0.15.18 (12 novembre 2026)

- «Aggiorna ora» scarica il pacchetto della nuova versione per il computer
  corrente e apre l'installatore, sia dalla notifica di avvio sia dal Gestore
  pacchetti; non si apre più la pagina della release.
- La riga «Libreria materiali» spiega che la libreria è inclusa nell'app e si
  aggiorna con essa.
- macOS arm64: `.dmg` 191.979.880 byte, `.app.zip` 186.455.620 byte, firmato
  ad-hoc sull'intero bundle e verificato dal DMG.
- Windows x64: installer NSIS cross-compilato (145.810.034 byte), controllato
  staticamente con 7-Zip; non eseguito su hardware Windows.
- CLI macOS arm64: 7.225.728 byte.

## Stato reale 0.15.17 (12 novembre 2026)

- Editor universale e modalità Utilizzatore più vicini a un word processor:
  pannello destro minimizzabile e ridimensionabile, sintesi durante la
  digitazione, inserimento immagini.
- Miniature delle pagine generate con lo stesso renderer SVG di stampa ed
  esportazione.
- Ribbon con etichette nascondibili (predefinito: solo icone), rimosse le
  varianti di modalità; Gestione utenti con soli materiali assegnati.
- Aggiornamento automatico all'avvio: scelta dell'asset per piattaforma,
  download con avanzamento e apertura dell'installatore.
- Modello semantico Horizon e voce neurale italiana installati e verificati al
  primo avvio, senza rinvio; le immagini si importano dai file.
- macOS arm64: `.app` 225,44 MiB, `.dmg` 191.722.725 byte, firmato ad-hoc
  sull'intero bundle e verificato dal DMG.
- Windows x64: installer NSIS cross-compilato (145.806.097 byte), controllato
  staticamente con 7-Zip; non eseguito su hardware Windows.
- CLI macOS arm64: 7.225.728 byte.

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
`llama-server.exe`, DLL) e il modello `model.gguf` in `resources/ai`. Il motore di diffusione non esiste più:
le immagini si importano dai file e nessun runtime Python/torch è richiesto. Il bundle contiene il repertorio
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

Il repository non contiene alcun modello linguistico GGUF né
`resources/symbol-packs/arasaac/` (439 MB, solo asset di sviluppo). Prima di una build di release:

```bash
python3 scripts/fetch_release_assets.py
```

Lo script scarica il runtime llama.cpp per il sistema corrente e non scarica più alcun modello
linguistico: i modelli si scelgono dal catalogo dell'app. `--skip-arasaac` limita la build ai soli
simboli originali; `--llama-asset bin-macos-x64` serve a costruire l'app Intel da un host ARM. Il
catalogo ARASAAC completo non viene incluso nell'installer: resta come asset di sviluppo per la
ricerca locale.

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

## Voce neurale: download al primo avvio, non nell'installer

Il runtime vocale **non è dentro gli installer**: `resources/voice` pesa circa
250 MB e resterebbe inutilizzato su chi non usa la sintesi. Al primo avvio il
pacchetto `voice` scarica da solo:

- `piper_windows_amd64.zip` (Windows), `piper_macos_aarch64.tar.gz` (macOS
  arm64) o `piper_linux_x86_64.tar.gz` dai rilasci `rhasspy/piper`
  `2023.11.14-2`, estratto in `<dati>/voice/piper/`;
- `it_IT-serena-medium.onnx` e il suo `.json` da
  `rhasspy/piper-voices` (Hugging Face).

`voice::resolve` cerca il motore nelle risorse (`voice/piper-win`,
`voice/piper`) e poi in `<dati>/voice/{bin,piper/piper,piper}`. Il processo
parte dalla cartella dell'eseguibile, dove stanno DLL ed `espeak-ng-data`.

**Correzione 0.16.2.** Fino alla 0.16.1 su Windows `link_engine` copiava
soltanto `piper.exe` in `<dati>/voice/bin/`, lasciando DLL e `espeak-ng-data` in
`<dati>/voice/piper/piper/`. Il loader di Windows non le trovava e a ogni
lettura compariva la finestra di sistema «piper.exe — L'esecuzione del codice
non può proseguire perché piper_phonemize.dll non è stato trovato» (oppure
`espeak-ng.dll`). Ora:

- su Windows non si copia nulla: si usa il runtime estratto, con DLL ed
  `espeak-ng-data` accanto al motore; la copia incompleta delle versioni
  precedenti viene rimossa;
- `voice::runtime_is_complete` scarta un `piper.exe` isolato, quindi anche le
  installazioni già rotte passano al runtime integro senza riscaricare nulla;
- `SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX)` spegne le
  finestre di errore del loader per l'app e i suoi processi figli: se una DLL
  manca davvero, la lettura ricade sulla voce di sistema senza dialoghi.

Su macOS il runtime Piper arm64 incluso nel pacchetto ufficiale è incompleto e
la voce neurale si installa con lo stesso download; in sviluppo resta
`.voice-venv/bin/piper`. Se il motore manca o è incompleto, `voice_status` lo
dichiara con `backend: "system"` e il pulsante «Scarica e installa la voce» in
Impostazioni → Pacchetti e aggiornamenti → Voci.

La licenza della voce inglese (`en_GB-cori-high`, dataset LibriVox di pubblico dominio) e quella
della voce italiana (`it_IT-serena-medium`, CC BY 4.0 con attribuzione) vanno riportate nel
registro delle licenze di ogni distribuzione.
