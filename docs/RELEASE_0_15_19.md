# CAA Writer 0.15.19 — editor Utilizzatore ripulito e pannelli gestibili

Data: 12 novembre 2026

## Editor della modalità Utilizzatore

- Il pannello di destra «Suggerimenti» è un vero pannello: larghezza
  trascinabile dal bordo e stato aperto/chiuso ricordato per profilo.
- Il pannello «Simboli» in basso è ora ridimensionabile in altezza dal bordo
  superiore e minimizzabile con il pulsante ×; anche altezza e stato sono
  ricordati per profilo. Quando è chiuso resta una barra «Simboli» per
  riaprirlo.
- L'editor occupa la finestra: l'area di scrittura e i pannelli scorrono al
  proprio interno, senza far scendere la pagina.
- Eliminata la schermata del Creator che compariva sotto il pannello Simboli:
  in modalità Utilizzatore l'editor è l'unica cosa visibile. Non ci sono più
  accessi a libreria, pacchetti o altri documenti da questa sezione.
- Aggiunto «Apri» nella barra dell'editor: apre i documenti `.caadoc` scritti
  con l'editor Utilizzatore e li ricarica nel punto giusto. I materiali della
  libreria e i fogli liberi non vengono aperti qui: un messaggio spiega che
  appartengono all'Area Creator.
- I documenti di livello 1 conservano le unità con i loro pittogrammi, così
  alla riapertura la frase ricompare com'era.

## Area Creator

- I documenti creati con l'editor Utilizzatore restano normali `.caadoc`: si
  aprono ed editano dal Creator con «Apri documento» e il word processor
  completo.

## Verifiche eseguite

```text
npm run check                        0 errori, 57 warning
npm run test:frontend                69/69
npm run build                        riuscita
cargo test --lib                     239/239
scripts/check_context.py             21/21 sezioni, file non tracciato
```

La disposizione dei pannelli è stata verificata con il controllo dei tipi e
del markup; non è stato eseguito un test visivo automatico dell'interfaccia
desktop.

## Artefatti

| Piattaforma | File | Byte |
|---|---|---|
| macOS arm64 | `CAA-Writer_0.15.19_macos-arm64.dmg` | 192.019.637 |
| macOS arm64 | `CAA-Writer_0.15.19_macos-arm64.app.zip` | 186.456.553 |
| Windows x64 | `CAA-Writer_0.15.19_windows-x64-setup.exe` | 145.814.213 |
| macOS arm64 | `CAA-Writer_0.15.19_cli-macos-arm64` | 7.225.728 |

Il bundle macOS è firmato ad-hoc sull'intero bundle e verificato dal DMG con
`codesign --verify --deep --strict`; non è notarizzato, quindi al primo avvio
può comparire «Apri comunque». L'installer Windows è stato cross-compilato e
controllato staticamente con 7-Zip: 1024 file, `caa-writer.exe` PE32+ x86-64,
libreria materiali, repertorio `core` e runtime Windows llama.cpp. Non è stato
eseguito su hardware Windows.

## Limiti dichiarati

- Windows x64 e macOS Intel non sono stati eseguiti su hardware reale.
- Non è stato eseguito un test automatico dell'aspetto a schermo: i pannelli
  sono verificati nel codice e nei controlli di compilazione.
