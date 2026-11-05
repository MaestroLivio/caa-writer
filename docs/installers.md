# Release e installer

## Stato reale (versione 0.3.0, 5 novembre 2026)

| Piattaforma | Costruito | Come |
|---|---|---|
| macOS arm64 | **sì**, non firmato, avviato | `npm run tauri:build:mac-arm -- --bundles app,dmg` |
| macOS x86-64 | **sì**, non firmato, avviato con Rosetta | `npm run tauri:build:mac-intel -- --bundles app,dmg` |
| Windows x64 | no su questa macchina | workflow `.github/workflows/release.yml` su `windows-latest` |
| Linux x64 | no su questa macchina | workflow `.github/workflows/release.yml` su `ubuntu-22.04` |
| Android / ChromeOS | no | richiede SDK/NDK e `tauri android init` |

Artefatti macOS prodotti (in `dist-release/`, esclusa da Git):

```
CAA-Writer_0.3.0_macos-arm64.dmg       786,80 MiB
CAA-Writer_0.3.0_macos-arm64.app.zip   795,6 MiB
CAA-Writer_0.3.0_macos-x64.dmg         787,15 MiB
CAA-Writer_0.3.0_macos-x64.app.zip     795,6 MiB
SHA256SUMS-macos.txt
```

I due bundle sono stati aperti: Apple Silicon nativamente, Intel tramite Rosetta 2. La dimensione
dipende dal modello GGUF in `resources/ai/`. Gli artefatti **non sono firmati né notarizzati**: al
primo avvio macOS può richiedere «Apri comunque» dal menu contestuale del Finder.

## Asset non versionati

Il repository non contiene `resources/ai/` (491 MB di modello, oltre i limiti GitHub) né
`resources/symbol-packs/arasaac/` (439 MB). Prima di una build di release:

```bash
python3 scripts/fetch_release_assets.py
```

Lo script scarica il modello Qwen2.5-0.5B-Instruct, ne verifica lo sha256, scarica il runtime
llama.cpp per il sistema corrente e il catalogo ARASAAC. `--skip-arasaac` limita la build ai soli
simboli originali; `--llama-asset bin-macos-x64` serve a costruire l'app Intel da un host ARM.

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

Il runtime vocale **non è incluso nel bundle**: `voice::resolve` cerca `resources/voice/bin/piper`
oppure `.voice-venv/bin/piper`, e nella build di release nessuno dei due esiste. Senza runtime, la
lettura usa la voce di sistema e `voice_status` lo dichiara con `backend: "system"`.

Per avere la voce neurale in una installazione confezionata serve impacchettare Piper con le sue
dylib (il pacchetto macOS ufficiale è incompleto) oppure distribuire un runtime Python dedicato.
Questo è il limite 35 di `Contesto.md`: la voce locale è operativa in sviluppo tramite
`scripts/setup_voice.sh`, non nell'installer.

La licenza della voce inglese (`en_GB-cori-high`, dataset LibriVox di pubblico dominio) e quella
della voce italiana (`it_IT-serena-medium`, CC BY 4.0 con attribuzione) vanno riportate nel
registro delle licenze di ogni distribuzione.
