# CAA Writer 0.16.5 — correzioni editor e miniature

Data: 27 novembre 2026

## Modifiche

- Le miniature desktop usano lo stesso rendering SVG di stampa/PDF e seguono
  l’impostazione sui contenuti espliciti.
- Il menu contestuale nell’editor utente permette di modificare il testo
  conservando il pittogramma, scegliere alternative e ascoltare la parola.
- Il pannello Suggerimenti e comandi può essere flottante e spostato; la
  posizione viene ricordata per profilo.
- Rimossa la tastiera flottante.
- I suggerimenti automatici di ripiego non diventano override persistenti:
  restano sostituibili e non ricompaiono su parole diverse.

## Verifiche

Build, test e verifica dei bundle sono eseguiti dal workflow GitHub Actions
`release` associato al tag `v0.16.5`.
