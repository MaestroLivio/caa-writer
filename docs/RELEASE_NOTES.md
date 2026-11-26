# CAA Writer 0.16.4 — pittogrammi e sintesi vocale

Data: 26 novembre 2026

## Modifiche

- Se la sintesi vocale è disattivata nelle impostazioni, l'app non mostra più
  messaggi che invitano ad attivarla.
- Una scelta manuale del pittogramma per una parola resta bloccata durante la
  rianalisi e non viene sostituita dal reranking automatico.
- ARASAAC è il repertorio predefinito, seguito da Global Symbols.
- Il pacchetto CAA Writer Core non è più incluso nel bundle. All'avvio, i
  database aggiornati eliminano i simboli interni e trasferiscono le
  associazioni salvate a un simbolo ARASAAC o Global Symbols dello stesso
  concetto, se disponibile.
- Il lemma «cagnolino» è associato al concetto «cane» e include le sue forme
  flesse generate dal lessico.
- Il filtro dei contenuti espliciti copre anche termini inglesi e le relative
  forme flessive; il controllo a parola intera evita corrispondenze accidentali.

## Verifiche

I risultati effettivi dei test e della pubblicazione sono registrati in
`Contesto.md` (memoria locale non distribuita) e negli esiti della CI release.
