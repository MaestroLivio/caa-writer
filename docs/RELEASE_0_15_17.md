# CAA Writer 0.15.17 — editor come word processor, aggiornamenti e pacchetti

Data: 12 novembre 2026

## Editor universale e modalità Utilizzatore

- Il pannello di destra della modalità Utilizzatore si può minimizzare e
  ridimensionare; larghezza e stato restano memorizzati per ogni profilo.
- La sintesi vocale durante la digitazione usa le stesse regole dell'editor
  universale: parola dopo lo spazio, frase dopo la punteggiatura o Invio.
- Nel pannello è disponibile «Inserisci immagine»: il file scelto entra nel
  documento nel punto del cursore e viene salvato con il documento.
- Le miniature delle pagine dell'editor universale usano lo stesso renderer
  SVG della stampa e dell'esportazione: non sono più una approssimazione DOM.

## Ribbon, menu e impostazioni

- Le etichette dei comandi del Ribbon si possono nascondere. Il valore
  predefinito è «solo icone»: la descrizione resta nel suggerimento e
  nell'etichetta per gli screen reader. L'interruttore è in Visualizza.
- Rimossi «Solo pittogrammi» e «Pittogrammi e testo» dalla scheda Pittogrammi.
- Rimossa la scelta «Modalità» da Impostazioni e dal pannello laterale: il
  testo libero usa «simboli sopra le parole». I componenti della libreria
  possono mantenere il proprio layout.
- Nella schermata iniziale del Creator ci sono i pulsanti per la Gestione
  utenti e per tornare all'area Utilizzatore.
- La scheda Utenti del Ribbon include «Assegna ad un profilo» per il documento
  universale aperto dalla libreria.
- La Gestione utenti elenca solo i materiali realmente assegnati al profilo,
  con rimozione immediata; l'aggiunta avviene da un elenco ricercabile.

## Download e aggiornamenti

- All'avvio viene controllata la release pubblica; se c'è una versione nuova
  compare una notifica con «Aggiorna ora» e «Più tardi», senza bloccare l'uso.
- «Aggiorna ora» sceglie l'asset adatto al computer (`macos-arm64.dmg` oppure
  `windows-x64-setup.exe`), lo scarica con avanzamento e apre l'installatore.
  Su macOS si apre il DMG; su Windows parte il setup NSIS.
- Il Gestore pacchetti e le Impostazioni mostrano lo stesso aggiornamento.
- La barra globale dei download è la media degli avanzamenti dei singoli job,
  non la somma dei byte.
- Nuova sezione «Modello di diffusione per le immagini» nel Gestore pacchetti,
  nelle Impostazioni e nella scelta del primo avvio. Il modello consigliato
  dipende dalla memoria fisica e dalla piattaforma; il download installa il
  runtime Python e il modello nella cartella dati dell'utente.

## Correzioni

- Condivisione del motore CAA e della sintesi vocale fra editor universale e
  modalità Utilizzatore.
- Il pannello laterale non mostra più un selettore di modalità in conflitto
  con le impostazioni globali.

## Verifiche eseguite

```text
npm run check                        0 errori, 56 warning
npm run test:frontend                69/69
npm run build                        riuscita
cargo test --lib                     239/239
cargo test --test universal_documents 11/11
scripts/check_context.py             21/21 sezioni, file non tracciato
```

## Artefatti

| Piattaforma | File | Byte |
|---|---|---|
| macOS arm64 | `CAA-Writer_0.15.17_macos-arm64.dmg` | 191.722.725 |
| macOS arm64 | `CAA-Writer_0.15.17_macos-arm64.app.zip` | 186.455.612 |
| Windows x64 | `CAA-Writer_0.15.17_windows-x64-setup.exe` | 145.806.097 |
| macOS arm64 | `CAA-Writer_0.15.17_cli-macos-arm64` | 7.225.728 |

Il bundle macOS è firmato ad-hoc sull'intero bundle e verificato dal DMG con
`codesign --verify --deep --strict`; non è notarizzato, quindi al primo avvio
può comparire «Apri comunque». L'installer Windows è stato cross-compilato e
controllato staticamente con 7-Zip: contiene `caa-writer.exe` PE32+ x86-64,
la libreria materiali, il repertorio `core` e il runtime Windows llama.cpp.
Non è stato eseguito su hardware Windows.

## Limiti dichiarati

- Windows x64 e macOS Intel non sono stati eseguiti su hardware reale.
- Il download completo ARASAAC/Global Symbols e l'installazione della voce
  Piper non sono stati eseguiti end-to-end in questa sessione.
- Il corpus semantico non è stato rieseguito: richiede il catalogo ARASAAC
  locale.
- Il download automatico della libreria materiali resta sulla pagina della
  release.
