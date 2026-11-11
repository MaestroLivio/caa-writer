# CAA Writer 0.15.16 — correzioni e download guidati

## Novità 0.15.16

### Editor universale di nuovo utilizzabile

- Corretto l'errore che impediva l'apertura dell'editor: la classe `FlowLayout`
  veniva usata prima della sua inizializzazione. Ora l'editor si apre sia da
  «Word Processor CAA» sia da un modello della libreria.
- La libreria materiali include **manifest, 505 modelli e 30 sistemi
  comunicativi** nell'installer. Le anteprime mancanti vengono generate al
  volo dal renderer del documento: aprire un modello non richiede più la
  libreria esterna.

### Download guidati e barra globale

- Nuovo gestore download in background con avanzamento condiviso: più pacchetti
  possono essere scaricati insieme.
- **Primo avvio**: ARASAAC e Global Symbols sono richiesti e non
  deselezionabili; la finestra non si chiude finché i download non sono
  terminati. Voce neurale e modello linguistico restano opzionali.
- **Pacchetti**: gli stessi download sono disponibili in seguito, con la stessa
  barra globale e l'avviso di non chiudere la finestra.
- Se un download fallisce, il riquadro mostra l'errore, permette «Riprova» e
  consente di continuare offline con quanto già installato.

### Cataloghi completi

- **ARASAAC**: il download copre tutte le 14.000 voci del catalogo italiano,
  non solo le parole del lessico. Le immagini sono salvate nella cartella dati
  dell'utente e collegate ai metadati con autore, licenza e fonte.
- **Global Symbols**: vengono salvati tutti i risultati per ogni lemma del
  lessico, non più un campione di 80 parole.
- **Voce neurale**: installabile dalla GUI. Scarica il runtime Piper della
  piattaforma, la voce italiana Serena e la configurazione, nella cartella dati
  dell'utente. Il motore vocale la trova senza script manuali.
- **Modello linguistico**: resta installabile dal catalogo modelli, con il
  proprio avanzamento. Nessun peso è incluso nell'installer.

### Funzionamento offline

Dopo i download scelti il programma funziona interamente senza rete: pittogrammi,
libreria, voce e modello sono locali. I download non partono mai da soli.

## Verifiche eseguite

```text
npm run check                          0 errori
npm run test:frontend                  69/69 passati
npm run build                          riuscito
cargo test --lib                       237/237 passati
cargo test --test universal_documents  11/11 passati
```

Dimensioni reali: macOS arm64 `.app` 225,45 MiB, `.dmg` 190.918.403 byte
(182,1 MiB); installer Windows x64 NSIS 142.742.012 byte (136,1 MiB),
cross-compilato e non firmato.

## Limiti dichiarati

- Global Symbols non offre un archivio federato completo: vengono scaricati
  tutti i risultati del servizio per i lemmi del lessico, non l'intera
  federazione.
- Il download ARASAAC completo è grande e richiede tempo; la barra globale
  indica l'avanzamento e il numero di immagini.
- Le anteprime dei modelli non sono incluse nel bundle: vengono disegnate al
  momento dal documento.
- Windows x64 viene compilato e impacchettato, ma non eseguito su hardware
  Windows in questa sessione.


## Nota macOS — firma del bundle

La prima pubblicazione degli asset macOS era firmata solo sul binario: le
risorse aggiunte dopo invalidavano la firma e macOS mostrava «danneggiata».
Gli asset sono stati rifirmati ad-hoc sull'intero bundle con
`scripts/sign_macos.sh` e verificati con `codesign --verify --deep --strict`
sia dal DMG sia dallo ZIP. L'app non è notarizzata: al primo avvio macOS può
chiedere «Apri comunque». Se un download precedente mostra ancora «danneggiata»,
scarica di nuovo il DMG/ZIP aggiornato.
