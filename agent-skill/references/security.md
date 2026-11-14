# Sicurezza, accesso e licenze

## Modalità di accesso

La politica è applicata nel dispatch condiviso (`automation::policy`), quindi
vale per CLI, RPC e MCP.

| Modalità | Letture | Scritture non distruttive | Distruttive |
|---|---|---|---|
| `read` | sì | no | no |
| `write` | sì | sì | no |
| `admin` | sì | sì | sì, con `confirm: true` |

- Le operazioni distruttive sono dichiarate nel catalogo con
  `destructive: true` e `requires_confirmation: true`.
- La conferma è un parametro esplicito `confirm: true`. Se manca, il dispatch
  restituisce un errore e **non tocca nulla**.
- `--dry-run` (CLI) o `dry_run: true` (argomento MCP) restituisce
  `{"dry_run": true, "would_execute": true, ...}` per le operazioni mutating,
  senza eseguirle.
- Non esiste alcun tool o metodo per eseguire comandi shell arbitrari.
- Il server MCP parte in modalità `write` per impostazione predefinita. Per un
  agente che deve solo leggere, avvialo con `--access read`.

## Percorsi

- `project.*` lavora **solo** dentro `<data-dir>/projects`.
- Sono rifiutati: percorsi assoluti, componenti `..`, prefissi di unità,
  percorsi vuoti.
- I collegamenti simbolici vengono risolti e devono restare dentro la radice.
- `project.list` non segue link simbolici e non esce dalla radice.
- `project.create`/`save` non sovrascrivono per caso: serve `overwrite: true`.
- `document.*` ed `element.*` accettano un percorso esplicito perché sono
  operazioni documentali avanzate: usali con percorsi controllati dall'agente.

Per isolare test e automazioni usa sempre `--data-dir` e `--resources` dedicati:

```bash
TMP=$(mktemp -d)
caa-cli --data-dir "$TMP/data" --resources /percorso/risorse info
```

## Dati e privacy

- Il database, gli asset importati e i progetti stanno nella cartella dati
  dell'utente; `Contesto.md` e i segreti non entrano nel repository.
- Le operazioni di rete inviano solo lemmi/termini necessari, non il documento.
- Le chiavi dei provider online restano locali (keyring o impostazioni locali).
- Non inserire dati personali o sanitari nei prompt destinati a servizi remoti.

## Licenze dei simboli

- Simboli originali CAA Writer: CC BY-NC-SA 4.0.
- ARASAAC: CC BY-NC-SA 4.0, autore **Sergio Palao**; il catalogo non è nel
  repository. Le copie scaricate restano locali con attribuzione.
- Altri repertori importati: conserva autore, licenza, URL e vincoli nei
  metadati del simbolo.

Non eliminare le informazioni di licenza durante trasformazioni o esportazioni.

## Cosa non fare

- Non aggirare `confirm: true` con un default dell'agente.
- Non dichiarare riuscita un'operazione di rete quando la rete manca.
- Non affermare che la GUI è sincronizzata: in questa versione non esiste il
  bridge e le modifiche ai file non ricaricano l'editor aperto.
- Non usare la modalità `admin` in un servizio esposto a input non fidato.
- Non introdurre un tool di esecuzione comandi: l'API è tipizzata per scelta.
