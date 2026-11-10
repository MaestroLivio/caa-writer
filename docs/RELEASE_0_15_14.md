# CAA Writer 0.15.14 — note di rilascio

## Novità 0.15.14 — pittogrammi grandi al posto giusto e pacchetti

### Editor CAA

- **Pittogrammi di nuovo piccoli e ancorati alla parola**: su macOS il WebView (WebKit)
  disegnava l'immagine del pittogramma alla sua dimensione intrinseca (fino a 500 px),
  ignorando la misura dell'unità. Ora il simbolo è uno sfondo dimensionato con
  `background-size: contain`, quindi resta sempre sopra la parola, alla dimensione scelta,
  in modalità «sopra» e «sotto». Verificato direttamente in WKWebView, oltre che in Chromium.
- **Zoom in basso a destra**: lo slider del documento è fisso nell'angolo dello schermo anche
  nell'editor universale, non più in fondo alla sola area della pagina.

### Pacchetti e libreria

- **Menu «Pacchetti» nella libreria dei materiali**: ogni categoria è un pacchetto. Si può
  rimuovere dalla libreria e ripristinare senza cancellare i file inclusi nell'app, che
  continuano a funzionare offline.
- **ARASAAC in locale**: il pacchetto scarica dai server ARASAAC le immagini delle parole del
  lessico e le conserva nella cartella dati dell'utente. Se il catalogo viene rimosso, il
  comando lo ricostruisce dai metadati ufficiali (13.800+ pittogrammi) senza Python e senza
  catalogo nel repository. Ogni simbolo conserva autore, licenza e fonte.
- **Global Symbols in locale**: scarica un vocabolario di base in inglese, traduce le parole
  italiane con il glossario o il modello locale e salva le copie con licenza e attribuzione.
- **Rimozione dei pacchetti**: ARASAAC e Global Symbols si possono cancellare dalla libreria
  locale; i documenti già salvati conservano i pittogrammi incorporati. I pacchetti `core` e
  le collezioni personali sono protetti.
- **Aggiornamenti dal repository pubblico**: il programma confronta la propria versione e la
  data della libreria con `MaestroLivio/caa-writer`, mostra la pagina della release e dichiara
  quando il controllo non è disponibile per assenza di rete.

## Verifiche eseguite

```text
npm run check                            0 errori, 27 warning preesistenti
npm run test:frontend                    58/58 passati
cargo test --lib                         230/230 passati (2 nuovi test sui pacchetti)
cargo build --bin caa-cli                riuscito
caa-cli packages.catalog                 cataloghi e conteggi reali
caa-cli packages.updates                 release 0.15.13 e libreria confrontate
caa-cli packages.download ARASAAC        catalogo ricostruito (13.800) e 3 immagini salvate
caa-cli packages.download Global Symbols 1 simbolo salvato in locale
caa-cli packages.remove                  pacchetto rimosso dal database
```

Il dimensionamento del pittogramma è stato misurato in WKWebView (lo stesso motore dell'app
macOS): `::before` 56×56 px, testo sulla stessa linea di base, immagine non più a 500 px.

## Limiti dichiarati

- Lo scaricamento ARASAAC copre le parole del lessico e richiede la rete; non scarica tutte le
  14.000 immagini in un colpo solo. La rimozione del pacchetto è reversibile con lo stesso
  comando, che ricostruisce i metadati.
- Global Symbols non offre un archivio unico: il pacchetto scarica un vocabolario di base,
  non l'intero catalogo federato.
- Windows x64 e macOS Intel non sono stati eseguiti su hardware reale; macOS non è
  firmato/notarizzato.
- La selezione e la stampa non sono state riprovate manualmente nell'app Tauri in questa
  sessione; il dimensionamento è stato verificato nel motore WebKit.

## Licenza e attribuzione

CAA Writer è distribuito con **CC BY-NC-SA 4.0**. I pittogrammi ARASAAC restano soggetti alla
licenza ARASAAC (CC BY-NC-SA 4.0) con attribuzione ad ARASAAC e a Sergio Palao; quelli di
Global Symbols conservano la licenza del rispettivo symbolset. Nessun pittogramma di terzi è
incluso nel repository o negli installer.
