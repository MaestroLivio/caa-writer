# Audit della scheda «Inserisci» — 0.15.15

Data: 11 novembre 2026.

Questo documento elenca ogni comando visibile nella scheda **Inserisci** e ciò che
fa realmente. Un comando che non può funzionare è disattivato con il motivo
scritto nel suggerimento, oppure non compare. Non ci sono pulsanti decorativi.

## Modalità testo (word processor)

| Comando | Azione reale | Stato |
|---|---|---|
| Interruzione di pagina | `CaaEditor.insertPageBreak()` inserisce un nodo di interruzione nel documento | disponibile |
| Data | Inserisce la data corrente formattata `it-IT` nel punto del cursore | disponibile |
| Ora | Inserisce l'ora corrente `HH:MM` nel punto del cursore | disponibile |
| Tabella | Inserisce una tabella 3×3 modificabile (righe, colonne, unione celle) | disponibile |
| Pittogramma | Apre il pannello Simbolo per la parola selezionata | disponibile |
| Pittogramma sulla parola | Come sopra: la parola selezionata riceve il pittogramma scelto | disponibile |
| Pittogramma sulla selezione | — | **dichiarato non disponibile**: la selezione multipla non è supportata; il suggerimento lo dice e invita a selezionare una parola |
| Sequenza pittografica | Genera un piano con il motore risorse e apre il documento universale | disponibile solo con il motore IA configurato; in assenza il comando riporta l'errore del motore, non simula un risultato |
| Tabella comunicativa | Genera una tabella con il template `communication_grid` | come sopra |
| Striscia visiva | Genera un'agenda visiva con il template `visual_schedule` | come sopra |
| Ricerca pittogrammi | Apre la scheda Simbolo del pannello laterale | disponibile |
| Libreria materiali | Apre il browser della libreria (modelli, tabelle, documenti personali) | disponibile |

## Modalità documento universale

La scheda sostituisce i comandi del word processor con quelli della pagina
universale: interruzione di pagina, data/ora e tabella di testo qui non hanno
senso e non vengono mostrati.

### Gruppo Elementi (galleria a riquadri)

| Comando | Azione reale | Stato |
|---|---|---|
| Testo | `add_element` con `kind: "text"` | disponibile |
| Casella | `add_element` con `kind: "text_box"` | disponibile |
| Pulsante | `add_element` con `kind: "button"` e azione predefinita «Leggi» | disponibile |
| Forma | `add_element` con `kind: "shape"` | disponibile |
| Fumetto | `add_element` con `kind: "speech"`, coda e celle CAA | disponibile |
| Immagine | Dialogo di scelta file, poi `add_element` con SVG/PNG incorporato | disponibile nell'app desktop |
| Segnaposto | `add_element` con `kind: "placeholder"` | disponibile |
| Griglia | Crea una griglia con celle reali indipendenti | disponibile |
| Pregrafismo | Apre il generatore di fogli a punti; il foglio entra come immagine modificabile | disponibile nell'app desktop |

### Gruppo Pagine

| Comando | Azione reale | Stato |
|---|---|---|
| Aggiungi pagina | `add_page` con ID stabile | disponibile |
| Pagina di testo | Aggiunge una pagina di flusso con lo stesso motore CAA del word processor | disponibile |
| Mostra/nascondi pagine | Cambia solo la vista delle miniature | disponibile |

### Gruppo Selezione

| Comando | Azione reale | Stato |
|---|---|---|
| Raggruppa | `group` sugli elementi selezionati (Shift+clic per la selezione multipla) | disponibile |
| Separa | `ungroup` sul gruppo selezionato | disponibile |

### Gruppo Risorse

| Comando | Azione reale | Stato |
|---|---|---|
| Ricerca pittogrammi | Apre la scheda Simbolo del pannello laterale | disponibile |
| Libreria materiali | Apre il browser della libreria | disponibile |

## Controlli collegati verificati

- **Griglia**: 4×5 produce 20 celle reali (test Rust
  `grid_4_by_5_creates_twenty_real_cells`); +/− riga e +/− colonna passano da
  `update_grid` e ricostruiscono le celle; unisci/dividi cella passano da
  `merge_grid_cell` / `split_grid_cell`; ogni cella ha testo, testo pronunciato,
  pittogramma, sfondo, bordo, angoli e spaziatura indipendenti.
- **Pulsanti**: il pannello Documento espone tipo di azione (aggiungi alla frase,
  leggi, seleziona per esercizio, vai alla pagina, indietro, home, svuota) e la
  pagina di destinazione. Le stesse azioni sono eseguite dal player
  (`playAction`).
- **Elementi**: duplicazione (celle, simboli, stile e azione inclusi), testo, posizione, dimensione, rotazione, livello, blocco,
  visibilità, trasparenza, stile e simbolo sono modificabili dal pannello; il
  testo CAA si modifica con il motore condiviso.
- **Nessun comando finto**: l'unico comando disattivato nella scheda è
  «Pittogramma sulla selezione», con il motivo esplicito.
