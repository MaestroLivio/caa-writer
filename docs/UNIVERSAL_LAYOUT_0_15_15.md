# Editor universale: testo, elementi e frasi CAA nelle celle

Aggiornamento 0.15.15, 11 novembre 2026.

Questo documento descrive il modello e le regole introdotte per far convivere
testo libero ed elementi nella stessa pagina. Non sostituisce la specifica
generale: dichiara cosa è stato implementato e cosa resta escluso.

## Modello documentale

Ogni `UniversalElement` ha in più il campo:

```json
"disposition": "inline | block | wrap_left | wrap_right | front | behind | free"
```

- Campo vuoto o assente: comportamento storico della pagina, per compatibilità
  con i documenti già salvati.
- Il campo è serializzato in `.caadoc` e sopravvive a salvataggio e riapertura
  (test Rust `element_caa_metadata_survives_caadoc_save_and_reopen`).

Ogni `GridCell` ha in più:

```json
"metadata": { "caa": { "doc_json": …, "plain_text": "…", "tokens": [ … ] } }
```

La cella conserva testo, token compatti e struttura ProseMirror. Non è
un'immagine: il testo resta modificabile e i singoli pittogrammi restano
sostituibili.

## Regole di impaginazione

Il motore Rust (`render_document`) e l'anteprima TypeScript applicano le stesse
regole.

- **inline**: l'elemento entra in una linea di flusso; se non c'è spazio va a
  capo. Adatto a pittogrammi e piccole immagini.
- **block**: l'elemento occupa la larghezza disponibile e il contenuto
  successivo riprende sotto.
- **wrap_left / wrap_right**: l'elemento occupa una fascia laterale; i
  paragrafi di testo successivi si dispongono nella colonna libera finché non
  superano la sua altezza. La larghezza è al massimo il 62 % della colonna.
- **front / behind / free**: l'elemento usa le proprie coordinate X/Y, non
  consuma spazio nel flusso. `behind` ha `z-index` inferiore al testo,
  `front` superiore.
- La disposizione è scelta dal pannello Documento → **Disponi rispetto al
  testo**. Le modalità non compatibili con una pagina di testo sono disattivate
  con una spiegazione, non simulate.

### Pagine di testo

Nelle pagine di testo il flusso delle parole è il `CaaEditor` (ProseMirror).
Gli elementi della pagina restano sovrapposti con posizione libera, davanti o
dietro. In linea/sopra-sotto/testo intorno non sono disponibili in questa
pagina: il pannello lo dichiara. Per il testo che circonda un'immagine si usa
una pagina documento con paragrafi di testo.

## Scrittura in aree libere

Nelle pagine non testuali un **doppio clic** in un'area vuota crea un vero
elemento di testo, con ID stabile, nel punto scelto (posizione libera) oppure
in coda al flusso (pagina `flow`). Non è una casella invisibile sopra la
pagina: l'elemento è selezionabile, modificabile e salvato nel documento.

## Frasi CAA nelle celle

- Doppio clic su una cella, oppure pulsante **Modifica frase CAA e pittogrammi
  della cella**, apre lo stesso `CaaEditor` del word processor.
- La cella può contenere una parola, un'espressione o una frase completa.
- Il rendering mostra ogni parola con il proprio pittogramma sopra, con a capo
  automatico; il renderer SVG fa lo stesso per stampa, PDF, PNG, DOCX e PPTX.
- Le alternative usano `token.alternatives` prodotti dalla stessa analisi del
  testo libero: a parità di testo e impostazioni l'ordine è lo stesso. La
  ricerca manuale (`search_symbols`) resta un fallback esplicito.
- La sostituzione di un pittogramma aggiorna solo quel token; testo e altri
  simboli restano intatti.

## Pacchetti

`packages::merge_symbol_packages` costruisce l'elenco da un catalogo noto
(ARASAAC, Global Symbols, core) e usa il database solo per conteggi e versioni.
La rimozione cancella i file e i simboli, non la voce di catalogo. Dopo la
rimozione il pacchetto resta visibile con `installed = false` e può essere
reinstallato. Test: `uninstalled_packs_remain_in_the_catalog`.

## Verifiche automatiche aggiunte

```text
universal::tests::text_flows_beside_and_below_wrapped_elements
universal::tests::grid_cell_renders_a_complete_caa_sentence
universal::tests::element_caa_metadata_survives_update_and_roundtrip
universal_documents::element_caa_metadata_survives_caadoc_save_and_reopen
universal_documents::grid_cell_caa_sentence_survives_caadoc_roundtrip
packages::tests::uninstalled_packs_remain_in_the_catalog
tests/frontend/element-caa.test.mjs (cella di griglia)
```

## Limiti dichiarati

- Nelle pagine di testo il testo intorno a un'immagine non è supportato: si usa
  una pagina documento con paragrafi.
- Il testo intorno è un'impaginazione a fasce (colonna libera a sinistra o a
  destra), non un contorno sagomato per immagini con trasparenza.
- Il ritaglio delle immagini non è implementato; sono disponibili posizione,
  dimensione, rotazione, livelli, blocco e testo alternativo.
- Le tabelle usano le celle della griglia; la prima riga è stilizzata come
  intestazione, ma non esiste una semantica di `<thead>` separata.
- L'azione «Riproduci audio» non è offerta: il player non ha ancora un
  protocollo di riproduzione degli asset audio, quindi non compare un pulsante
  finto.
