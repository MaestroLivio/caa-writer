# Giochi, materiali stampabili e libri

Fonti consultate: CAST UDL Guidelines, ARASAAC Aula abierta, Book Creator, Boardmaker Online.

## Caratteristiche osservate

- I materiali stampabili usano **bersagli ampi**, immagini riconoscibili, etichette leggibili e
  spazio per ritagliare/plastificare.
- I giochi hanno regole brevi, un obiettivo chiaro e una fine riconoscibile.
- I libri digitali alternano immagine grande e testo breve; la navigazione è esplicita.
- Le attività interattive non devono punire l'errore: il feedback serve a riprovare.

## Requisiti ricavati

1. Flashcard, etichette, cartellini, poster con margini di taglio e dimensioni coerenti.
2. Memory/tombola/domino con carte della stessa dimensione e coppie non ambigue.
3. Giochi interattivi con selezione, feedback, riprova e nessuna penalizzazione.
4. Libri con copertina, pagine, numerazione, riordino e modalità lettura.
5. Striscia CAA sopra il testo conservando il testo originale.

## Componenti editor

- `grid` con celle quadrate, `symbol`, `token_slot`, azioni `toggle`, `navigate`, `speak`.
- Pagine `absolute` per copertine e libri.
- Feedback di risposta nel player e nell'HTML esportato.
- Esportazione PDF/PNG e `.caadoc`.

## Decisioni realizzate

- `pilota_abbinamento`: scheda da collegare + verifica interattiva con feedback corretto/riprova.
- `pilota_libro_caa` («Il gatto e la farfalla»): 6 pagine, illustrazione, striscia CAA, testo,
  domande di comprensione.
- Token board pilota con gettoni selezionabili e scelta del rinforzo.

## Licenze

Nessuna carta o illustrazione proprietaria è stata copiata. I simboli ARASAAC sono attribuiti;
le eventuali fotografie importate dall'utente conservano la licenza dichiarata.

## Test previsti

- Celle della stessa dimensione e immagini non minuscole.
- Feedback non punitivo, possibilità di riprovare.
- Testo del libro integro e nessuna didascalia duplicata.
- Numerazione e link di navigazione corretti.
