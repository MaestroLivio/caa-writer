# CAA Writer 0.15.24 — correzioni Windows dai test in CI

Data: 22 novembre 2026

Versione di manutenzione: la 0.15.23 introduce i motori installati al primo
avvio, la rimozione di diffusione e provider IA online e la libreria curata a
350 materiali. Questa patch corregge tre problemi emersi eseguendo i test su
Windows nella CI, che su macOS non si vedono.

## Correzioni

- **Percorsi degli asset riparati con il separatore sbagliato.** Su Windows i
  percorsi ricostruiti da un database importato da un altro sistema operativo
  restavano misti (`assets\arasaac/picto.png`). Ora ogni componente viene
  aggiunto con `Path::join` e il percorso salvato è nativo; `..` viene ignorato
  per non uscire dalla cartella di base.
- **Voce neurale e test del motore.** Il test della voce cerca `piper.exe` su
  Windows: il fixture ora crea il nome corretto per la piattaforma. Il test del
  runtime llama.cpp si salta, dichiarandolo, quando `resources/ai/runtime` non
  è presente (la cartella è esclusa da Git e la scarica CI): prima falliva su
  ogni runner Windows.

## Verifiche eseguite

- `cargo test --lib`: 313/313; `cargo test --lib --features onnx`: 318/318.
- `npm run build`; `npm run check`: 0 errori.
- `npm run test:frontend`: 75/75.
- `codesign --verify --deep --strict` sull'app del DMG: valida.
- `caa-cli app.capabilities` dal bundle: versione **0.15.24**;
  `caa-cli call library.validate` dal bundle: 350 documenti, 0 errori,
  1600 avvisi, 301 validi.
- Installer Windows x64 cross-compilato (`PE32+` GUI, nessuna finestra
  console), non eseguito su hardware Windows.

## Artefatti

| File | Byte | MiB |
|---|---|---|
| `CAA-Writer_0.15.24_macos-arm64.dmg` | 214.329.266 | 204,4 |
| `CAA-Writer_0.15.24_macos-arm64.app.zip` | 208.208.183 | 198,6 |
| `CAA-Writer_0.15.24_cli-macos-arm64` | 22.040.352 | 21,0 |
| `CAA-Writer_0.15.24_windows-x64-setup.exe` | 145.057.925 | 138,3 |

Limite dichiarato: `cargo test --test semantic_corpus --features onnx` resta
rosso senza il catalogo ARASAAC locale (95,2 % di prima scelta, 15
approssimazioni fuorvianti contro la soglia di 5), come nella 0.15.23.
