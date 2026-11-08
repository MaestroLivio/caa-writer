# Audit della libreria e dell'editor — novembre 2026

Questo documento registra i problemi **verificati nel codice e nelle rappresentazioni**, non
impressioni. Per ogni voce: causa, gravità, componenti, correzione, test e stato.

## Sintesi

| # | Problema | Gravità | Stato |
|---|---|---|---|
| 1 | Le griglie di pagina occupavano tutta l'altezza: una riga di cinque gettoni diventava alta 186 mm | alta | corretto |
| 2 | Il testo degli elementi SVG non andava a capo e si sovrapponeva al pittogramma | alta | corretto |
| 3 | Le token board erano cinque (o tre/dieci) caselle di testo, non gettoni | alta | corretto |
| 4 | `token_board` non esisteva nella biblioteca universale | media | corretto con alias e fallback silenzioso |
| 5 | La libreria non aveva un catalogo navigabile: solo un elenco piatto di quattro modelli storici | alta | corretto con `LibraryBrowser` |
| 6 | I pittogrammi mancanti erano conteggiati ma non visibili sulla scheda | media | corretto: metadato, badge e report |
| 7 | Le schede didattiche storiche erano elenchi di parole, senza esercizio né soluzioni | alta | corretto: 45 schede rigenerate con istruzioni, risposte e soluzioni |
| 8 | Titoli duplicati nelle schede storiche | media | corretto: 0 avvisi `titolo-ripetuto` |
| 9 | La ricerca `2 + 1 = ___` cercava un pittogramma per l'intera stringa | media | evitato nei nuovi generatori matematici |
| 10 | Anteprima, editor, stampa e PDF potevano divergere | alta | anteprime generate dallo stesso renderer; test A4/paesaggio |

## Dettaglio

### 1. Griglie troppo alte

- **Causa**: `render_document` calcolava `grid_h = altezza_utile / righe` e applicava la stessa
  altezza a ogni cella; non esisteva un'area della griglia distinta dalla pagina.
- **Correzione**: `GridSettings` ha `content_width_mm`, `content_height_mm`, `origin_x_mm`,
  `origin_y_mm`, pesi di colonna/riga e allineamento. Le celle usano quei limiti.
- **Test**: `grid_content_box_prevents_page_height_cells`, `absolute_element_on_grid_page_keeps_its_coordinates`.
- **Verifica visiva**: la tabella CAA 4×4 e la token board pilota non hanno più celle alte quanto la pagina.

### 2. Testo non mandato a capo

- **Causa**: `render_element_svg` emetteva un solo `<text>`; il pittogramma occupava tutta la casella
  e la didascalia veniva disegnata al 90% dell'altezza.
- **Correzione**: `wrap_text`, più righe `<text>`, spazio riservato alla didascalia, allineamento
  verticale, grassetto/corsivo/sottolineato nel rendering.
- **Test**: `long_text_is_wrapped_in_multiple_lines`.

### 3. Token board non vere

- **Causa**: le specifiche `lib_aba_token_*` usavano le parole del modello come celle di una griglia.
- **Correzione**: il generatore produce `token_slot`, azione `toggle`, obiettivo, rinforzo e attività;
  il modello pilota `pilota_token_board` ha cinque gettoni selezionabili.
- **Controllo**: `library validate` segnala `token-board-non-vera` se mancano gettoni o slot.

### 4. `token_board` non trovato

- **Causa**: il vecchio identificativo dello Studio veniva passato alla libreria universale, che non
  lo contiene; l'errore veniva mostrato prima del ripiego.
- **Correzione**: alias verso i modelli nuovi e fallback senza messaggio fuorviante.
- **Stato**: il modello si apre; il vecchio percorso Studio resta disponibile solo se la libreria
  universale non è installata.

### 5. Catalogo piatto

- **Causa**: la schermata iniziale mostrava i quattro modelli storici; il manifest della biblioteca
  era usato solo per assegnare materiali ai profili.
- **Correzione**: `LibraryBrowser.svelte` con categorie, sottocategorie, ricerca per sinonimi,
  filtri, preferiti, recenti, documenti personali, anteprime reali, elenco compatto e paginazione.

### 6. Pittogrammi mancanti

- **Correzione**: ogni elemento/cella ha `missing_symbol`; il manifest espone `missing_symbols`;
  il catalogo mostra il badge. `library validate` distingue avviso ed errore.
- **Limite**: la biblioteca storica ha ancora 1.463 note di simboli mancanti; non sono nascoste.

### 7. Schede didattiche

- **Correzione**: il generatore delle 45 schede storiche produce ora una pagina esercizio, una
  pagina soluzioni, istruzioni operative e righe/caselle di risposta. Per la matematica le
  operazioni sono deterministiche e le risposte sono calcolate.
- **Piloti corretti**: addizioni entro il 10, sillabe, comprensione, classificazione scientifica.
- **Regola**: un modello non è pubblicabile se il validatore segnala `esercizio-assente` o
  `soluzioni-assenti`.
- **Limite**: alcune schede storiche sono esercizi di copiatura/lettura, non attività complesse;
  richiedono una revisione didattica, ma non sono più elenchi senza risposta.

### 8. Titoli duplicati

- **Correzione**: il generatore rimuove il titolo ripetuto nella stessa pagina.
- **Verifica**: zero avvisi `titolo-ripetuto` dopo la rigenerazione.

### 9. Espressioni matematiche

- **Decisione**: le operazioni non vengono inviate alla simbolizzazione. I generatori deterministici
  producono testo, caselle e soluzioni; i pittogrammi servono solo per immagini pertinenti.

### 10. Renderer

- **Stato**: editor, anteprima, SVG, HTML e PDF usano `UniversalDocument` e `render_document`.
- **Test**: PDF reale e PNG decodificabile (`universal_pdf`); confronto visivo dei piloti in
  `PNG` e `PDF` generati dalla CLI.
- **Limite**: la stampa di sistema non è stata riprovata su Windows/Linux in questa sessione.

## Test eseguiti realmente

```text
cargo test --lib                         217 passati
npm run test:frontend                    29 passati
npm run check                            0 errori
npm run build                            build Vite riuscita
caa-cli template build-all               481 template + 30 sistemi, 511 validi, 0 falliti
caa-cli library validate --only-pilots  12 documenti, 12 validi, 0 errori, 0 avvisi
caa-cli library validate                 511 documenti, 0 errori, 2.154 avvisi storici
```

## Problemi aperti

- Ridurre i 1.098 pittogrammi mancanti usando il repertorio in modo più preciso.
- Verificare la stampa reale da finestra Tauri, non solo il PDF della CLI.
- Il catalogo nuovo ha superato i 180 previsti (181 modelli nuovi).
- Rivedere didatticamente le schede storiche rigenerate: hanno ora esercizio e soluzioni, ma
  alcune sono esercizi di copiatura.
