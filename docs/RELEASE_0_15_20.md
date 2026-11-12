# CAA Writer 0.15.20 — avvio corretto su ogni Mac

Data: 12 novembre 2026

## Correzione critica macOS

La 0.15.19 (e le versioni precedenti con lo stesso supporto AVIF) collega la
libreria `libdav1d` di Homebrew con un percorso assoluto:

```text
/opt/homebrew/opt/dav1d/lib/libdav1d.7.dylib
```

Su un Mac senza Homebrew o con dav1d non installato, macOS interrompe l'avvio
con `Library missing` prima ancora di mostrare la finestra.

Da questa versione la libreria è **dentro l'app**:

- `libdav1d.7.dylib` (BSD-2-Clause, dipende solo dalle librerie di sistema) è
  copiata in `CAA Writer.app/Contents/Frameworks/`;
- il binario la cerca con `@rpath/libdav1d.7.dylib` e il percorso
  `@executable_path/../Frameworks`;
- `scripts/sign_macos.sh` esegue la copia e la riscrittura prima della firma e
  **fallisce la build** se il binario conserva un percorso `/opt/homebrew`.

Verifiche realmente eseguite:

```text
otool -L .../caa-writer      →  @rpath/libdav1d.7.dylib
codesign --verify --deep --strict dal DMG montato → valido
avvio del binario dal DMG con DYLD_PRINT_LIBRARIES → libreria caricata da
  .../CAA Writer.app/Contents/Frameworks/libdav1d.7.dylib
```

L'app è rimasta in esecuzione per 6 secondi senza errori di dyld e il binario
non contiene più alcun percorso `/opt/homebrew`: non dipende più da file
esterni al bundle.

## Editor Utilizzatore

Sono incluse le modifiche della 0.15.19:

- pannello destro «Suggerimenti» ridimensionabile e minimizzabile;
- pannello «Simboli» in basso ridimensionabile e minimizzabile;
- niente più HomeScreen del Creator sotto i simboli;
- «Apri» per i documenti scritti con l'editor Utilizzatore.

## Come aggiornare

- Chi ha una versione che parte (0.15.18 o precedenti) riceve la notifica e usa
  «Aggiorna ora».
- **Chi ha la 0.15.19 su un Mac senza Homebrew non riesce ad avviarla**: deve
  scaricare manualmente `CAA-Writer_0.15.20_macos-arm64.dmg` da questa release,
  trascinare l'app in Applicazioni e, se richiesto, scegliere «Apri comunque».

## Verifiche eseguite

```text
npm run check                        0 errori, 57 warning
npm run test:frontend                69/69
npm run build                        riuscita
cargo test --lib                     239/239
scripts/check_context.py             21/21 sezioni, file non tracciato
```

## Artefatti

| Piattaforma | File | Byte |
|---|---|---|
| macOS arm64 | `CAA-Writer_0.15.20_macos-arm64.dmg` | 192.029.092 |
| macOS arm64 | `CAA-Writer_0.15.20_macos-arm64.app.zip` | 186.803.135 |
| Windows x64 | `CAA-Writer_0.15.20_windows-x64-setup.exe` | 145.803.698 |
| macOS arm64 | `CAA-Writer_0.15.20_cli-macos-arm64` | 7.225.728 |

Il bundle macOS è firmato ad-hoc sull'intero bundle, compresa la libreria
dav1d, e verificato dal DMG con `codesign --verify --deep --strict`; non è
notarizzato. L'installer Windows è stato cross-compilato e controllato
staticamente con 7-Zip; non è stato eseguito su hardware Windows.

## Limiti dichiarati

- Windows x64 e macOS Intel non sono stati eseguiti su hardware reale.
- Il test di avvio è stato fatto su questo Mac: `otool -L` non mostra più
  percorsi Homebrew e `DYLD_PRINT_LIBRARIES` conferma il caricamento da
  `Contents/Frameworks`. Non è stato provato su un Mac vergine senza Homebrew.
