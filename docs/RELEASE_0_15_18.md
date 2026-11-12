# CAA Writer 0.15.18 — aggiornamento in un clic

Data: 12 novembre 2026

## Aggiornamenti

- «Aggiorna ora» è l'unica azione di aggiornamento, sia nella notifica di
  avvio sia nel Gestore pacchetti: scarica il pacchetto della nuova versione
  adatto a questo computer (`macos-arm64.dmg` oppure `windows-x64-setup.exe`)
  con la barra di avanzamento e apre l'installatore. Non si apre più la pagina
  della release per aggiornare.
- La riga «Libreria materiali» non rimanda più al sito: la libreria viaggia
  dentro l'app e si aggiorna con essa. Se il repository pubblico è più
  recente dell'ultima release, la finestra lo dice e non chiede alcun download
  manuale.
- Il download parte dal Gestore pacchetti e la finestra di avanzamento non
  resta più dietro alla finestra dei pacchetti.

## Verifiche eseguite

```text
npm run check                        0 errori, 56 warning
npm run test:frontend                69/69
npm run build                        riuscita
cargo test --lib                     239/239
cargo check --lib                    riuscita
scripts/check_context.py             21/21 sezioni, file non tracciato
```

## Artefatti

| Piattaforma | File | Byte |
|---|---|---|
| macOS arm64 | `CAA-Writer_0.15.18_macos-arm64.dmg` | 191.979.880 |
| macOS arm64 | `CAA-Writer_0.15.18_macos-arm64.app.zip` | 186.455.620 |
| Windows x64 | `CAA-Writer_0.15.18_windows-x64-setup.exe` | 145.810.034 |
| macOS arm64 | `CAA-Writer_0.15.18_cli-macos-arm64` | 7.225.728 |

Il bundle macOS è firmato ad-hoc sull'intero bundle e verificato dal DMG con
`codesign --verify --deep --strict`; non è notarizzato, quindi al primo avvio
può comparire «Apri comunque». L'installer Windows è stato cross-compilato e
controllato staticamente con 7-Zip: 1024 file, `caa-writer.exe` PE32+ x86-64,
libreria materiali, repertorio `core` e runtime Windows llama.cpp. Non è stato
eseguito su hardware Windows.

## Limiti dichiarati

- Windows x64 e macOS Intel non sono stati eseguiti su hardware reale.
- L'aggiornamento automatico è stato verificato come codice e come asset
  pubblicati (nomi e URL), ma il download completo seguito dall'apertura
  dell'installatore non è stato eseguito su un'installazione precedente in
  questa sessione.
- Download completi ARASAAC/Global Symbols, voce Piper e runtime di diffusione
  restano da provare end-to-end.
