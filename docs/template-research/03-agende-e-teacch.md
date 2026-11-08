# Agende, TEACCH e sistemi di lavoro

Fonti consultate: TEACCH Tip #1 Visual Schedule, Autism Classroom Resources — First-Then,
ARASAAC Aula abierta, CAST UDL Guidelines.

## Caratteristiche osservate

- La sequenza visiva rende visibili **che cosa accade, in che ordine e che cosa viene dopo**.
- Le attività completate si rimuovono, si spuntano o si cancellano.
- Le attività preferite sono visibili nella giornata.
- Il formato si adatta al livello: oggetti, foto, simboli, parole.
- Il sistema di lavoro TEACCH risponde a quattro domande: che cosa fare, quanto lavoro, come
  riconoscere la fine, che cosa succede dopo.
- Prima/dopo è una struttura minima, utile nelle transizioni.

## Requisiti ricavati

1. La griglia settimanale non deve diventare cinque colonne altissime con etichette minuscole.
2. Le righe devono adattarsi al numero di attività e all'orientamento.
3. Intestazione dei giorni leggibile, attività con simbolo e testo, stato completato.
4. Sequenza TEACCH con passi numerati, casella “fatto”, indicatore di fine.
5. Materiali da ritagliare separati dalla scheda da usare.

## Componenti editor

- Pagina `grid` con `row_heights`/`content_height_mm`.
- Elementi `checkbox`, `token_slot`, `badge`, `answer_line`.
- `first/then` come due contenitori affiancati, non come due parole.
- Stato “completato” stampabile e interattivo.

## Decisioni realizzate

- `pilota_agenda_settimanale`: 5 giorni × 4 attività, pesi di riga, cartellini da ritagliare.
- `pilota_teacch_sequenza`: quattro domande, cinque passi, caselle “Fatto”, badge “FINITO → pausa”,
  pagina di cartellini.

## Licenze

Nessun asset TEACCH proprietario è stato copiato. Struttura e domande sono principi funzionali;
simboli ARASAAC con attribuzione.

## Test previsti

- Numero di giorni e attività coerente con i parametri.
- Nessun testo tagliato; caselle sufficientemente grandi.
- Ordine dei passi e fine riconoscibile.
- Stampa fronte/retro e ritaglio.
