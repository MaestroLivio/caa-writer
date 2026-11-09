# CAA Writer 0.15.12 — note di rilascio

## Novità 0.15.12 — frasi CAA dentro forme e fumetti, libreria rifatta

### Scrivere dentro forme e fumetti

- **Le parole non spariscono più**: quando in una forma, un fumetto o una casella si inserisce un
  pittogramma PNG, l'editor mostra **immagine e parola insieme**, come nel rigo CAA normale.
  Prima il ramo grafico delle immagini nascondeva la didascalia.
- **Frase CAA in una forma**: seleziona la forma o il fumetto, scrivi la frase nel campo
  **Frase CAA** del pannello Documento e premi **Genera CAA dalla frase**. Ogni parola diventa
  una cella con pittogramma sopra e parola sotto; la punteggiatura resta attaccata alla parola
  precedente e le parole senza simbolo restano come testo.
- **Frase modificabile**: le celle sono selezionabili una per una; dall'ispettore si cambiano
  testo, testo pronunciato e pittogramma. La frase non è un'immagine piatta.
- **Righe e colonne**: la griglia della frase e le griglie normali hanno i comandi
  **+ Riga / − Riga / + Colonna / − Colonna**. Anche la pagina ha il selettore di layout
  (libero, flusso, griglia) e i campi righe/colonne.
- **Stampa ed esportazioni**: le celle con pittogramma e parola vengono disegnate anche in
  PDF, PNG, DOCX, PPTX e HTML; il testo del contenitore non viene duplicato sopra le celle.

### Libreria di modelli

- **Ricognizione completa**: 505 modelli + 30 sistemi comunicativi rigenerati (535 documenti
  validi). Ogni modello ha ora intestazione con categoria, titolo e istruzione per l'adulto,
  piè di pagina, obiettivo e tag per la ricerca. I testi tecnici generati per errore
  («checklist», «verticale») sono stati rimossi.
- **Materiali ABA e TEACCH**: board PRIMA/POI, token economy con scheda dati, sistema di lavoro
  TEACCH, analisi del compito, scheda dati per le richieste, abbinamento di immagini uguali,
  categorizzazione e sequenza della routine del mattino.
- **Materiali con velcro**: striscia delle frasi, carte dei bisogni, categorie da ordinare,
  token board, prima/poi, agenda della giornata, striscia «Io voglio», sequenza lavarsi le mani,
  contare fino a 10, abbinare forme e colori. Ogni foglio indica di stampare, plastificare e
  applicare il velcro; le linee di taglio sono tratteggiate.
- **Fumetti CAA completi**: sei dialoghi a tre vignette (ciao/come stai, ho fame, andiamo al
  parco, mi aiuti, sono felice, è ora di dormire) con personaggi, fumetti e frasi simbolizzate
  dentro le celle.
- **Tutto modificabile**: nessun elemento bloccato; ogni modello si apre nell'editor universale,
  si può cambiare layout, aggiungere righe o colonne e sostituire i pittogrammi.

## Verifiche eseguite

```text
npm run check                0 errori
npm run test:frontend        46/46 passati
cargo test --lib             228/228 passati
cargo test --test universal_documents  9/9 passati (535 documenti, nessun avviso di overflow)
caa-cli template build-all   505 modelli + 30 sistemi = 535 validi, 866 simboli mancanti dichiarati
```

Anteprime verificate: il fumetto «ciao, come stai?» contiene le celle con i pittogrammi e non
duplica il testo del fumetto; le strisce velcro hanno i bordi tratteggiati e le carte con
pittogramma e parola.

## Limiti dichiarati

- I 866 simboli mancanti restano dichiarati nel manifesto: le parole senza pittogramma restano
  come testo, non vengono inventate associazioni.
- Windows x64 non è stato eseguito su hardware reale; macOS non è firmato/notarizzato.
- `cargo test --test semantic_corpus` non è stato eseguito in questa sessione.

## Licenza e attribuzione

CAA Writer è distribuito con **CC BY-NC-SA 4.0**. I pittogrammi ARASAAC usati dall'utente restano
soggetti alla licenza ARASAAC (CC BY-NC-SA 4.0) con attribuzione ad ARASAAC e a Sergio Palao; non
sono inclusi nel repository né negli installer.
