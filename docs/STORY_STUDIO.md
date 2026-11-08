# Story Studio — storie illustrate in CAA

Data: 7 novembre 2026.

## Cosa fa

`StoryStudio.svelte` (schermata iniziale → «Crea una storia illustrata in CAA») e il comando
`caa-cli story create` costruiscono un `UniversalDocument` multipagina:

- copertina con titolo, ambientazione e personaggi;
- 10–20 pagine con scena, testo e striscia CAA di parole chiave;
- pulsanti Indietro/Avanti/Home con collegamenti per ID stabile;
- pagina finale con traccia di comprensione;
- salvataggio `.caadoc`, apertura nello stesso `UniversalEditor`, player e PDF.

## Backend immagini

È attivo il **generatore vettoriale offline**: ogni personaggio, sfondo e oggetto è un elemento
`symbol` indipendente. Non è un'immagine unica fusa: si può sostituire, spostare, nascondere e
bloccare ogni elemento.

I backend diffusion locali/online **non sono dichiarati attivi**. Non sono stati scaricati modelli,
non è stato verificato un runtime su 4 GB e non esistono pulsanti di download o modelli dichiarati
compatibili senza prova. Il requisito «modello più leggero compatibile» resta aperto finché non
esiste una misura reale.

## Layout

- `image_top`: scena sopra, striscia CAA e testo sotto.
- `pictograms_top`: striscia e testo sopra, scena sotto.
- `image_left`: scena a sinistra, testo CAA a destra.
- `image_right`: testo CAA a sinistra, scena a destra.

Il cambio di layout si può fare modificando la specifica e rigenerando; l'editor permette comunque
di spostare i singoli elementi.

## Coerenza dei personaggi

Il generatore riusa gli stessi pittogrammi (ID/etichetta) in tutte le pagine. Non usa image-to-image
o LoRA: la coerenza è quindi di tipo grafico-compositivo, non fotografico. Le immagini approvate si
possono bloccare nell'editor per evitare sostituzioni automatiche.

## Limiti

- Nessuna generazione cartoon neurale.
- Nessuna API online di immagini.
- Nessun catalogo di modelli diffusion.
- Le scene usano i pittogrammi disponibili; se un simbolo manca, il testo resta e il validatore lo
  segnala.
- La revisione pedagogica resta manuale.
