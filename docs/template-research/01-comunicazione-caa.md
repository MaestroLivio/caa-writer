# Comunicazione CAA

Fonti consultate: ARASAAC Aula abierta (tavole comunicative, quaderni), CAST UDL Guidelines,
Global Symbols API spec.

## Caratteristiche osservate

- Tavole con **vocabolario essenziale** (core words) e tavole tematiche (negozi, casa, scuola).
- Celle grandi, simbolo sopra e parola sotto; etichette leggibili anche a distanza.
- Quaderni multipagina con una **home** e pagine di categoria; la navigazione deve essere visibile.
- Le raccolte ARASAAC usano materiali diversi (foto, pittogrammi, lettere) e lo dichiarano.
- UDL: rappresentazione multipla, azione/espressione multipla, coinvolgimento; non un unico formato.

## Requisiti ricavati

1. Griglia configurabile, celle quadrate o leggermente rettangolari, non alte quanto la pagina.
2. Simbolo grande con etichetta sotto, bordo leggero, spazio interno sufficiente.
3. Barra della frase modificabile; nelle tabelle interattive azioni `append`, `speak`, `navigate`,
   `back`, `home`.
4. Home con categorie e pagine con ritorno sempre raggiungibile.
5. Vocabolario essenziale separato dal vocabolario tematico.
6. Possibilità di sostituire un simbolo senza cambiare il testo.

## Componenti editor necessari

- Pagina `grid` con area contenuto, origini, pesi di righe/colonne.
- Elemento `symbol` con didascalia; `grid` annidata per le strisce-frase.
- Azioni tipizzate e collegamenti per ID stabile.
- Player/anteprima con tocco, tastiera e sintesi vocale.

## Decisioni realizzate

- `pilota_tabella_caa_4x4`: 16 celle, barra della frase, paesaggio A4.
- `pilota_quaderno_comunicazione`: 10 pagine, home + 9 categorie, Home/Indietro.
- Celle 4×4: 67 mm circa per lato in A4 orizzontale, simbolo ~40 mm.

## Licenze

ARASAAC: CC BY-NC-SA 4.0, autore Sergio Palao, titolare Governo di Aragona. Global Symbols
restituisce la licenza del symbolset; Openverse restituisce licenza e autore dell'immagine.

## Test previsti

- Conteggio celle = righe × colonne.
- Link di navigazione esistenti e nessuna pagina orfana.
- Simbolo presente o assenza dichiarata.
- Rendering A4 paesaggio e stampa senza tagli.
