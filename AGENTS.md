# Istruzioni per gli agenti di coding

## Prima di tutto: leggi `Contesto.md`

`Contesto.md` (nella radice, **non versionato**) è la memoria tecnica condivisa del progetto:
obiettivi, architettura, decisioni, problemi aperti, test realmente eseguiti, stato delle
funzionalità e prossime attività. Leggilo prima di qualsiasi modifica significativa e aggiornalo
prima di terminare la sessione. Non copiarlo qui: questo file deve restare breve.

Se `Contesto.md` manca, ricostruiscilo dalle 21 sezioni elencate nella specifica del committente e
verifica lo stato reale del repository prima di dichiarare qualcosa come completo.

## Regole non negoziabili

1. **Nessun risultato inventato.** Non dichiarare test superati, integrazioni attive, simboli
   scaricati o server MCP usati se non è realmente accaduto. Se un test non è eseguibile
   nell'ambiente, scrivilo come limite.
2. **Offline per prime.** Il software deve funzionare senza rete e senza GPU. Nessun servizio cloud
   obbligatorio per aprire, modificare, salvare o stampare documenti.
3. **Licenze.** Il progetto è distribuito con **CC BY-NC-SA 4.0**. I pittogrammi ARASAAC sono
   inclusi nel pacchetto `arasaac` con la loro licenza CC BY-NC-SA 4.0 e l'attribuzione ad ARASAAC
   e a Sergio Palao; gli altri repertori di terzi restano fuori dal repository e si importano solo
   localmente. Ogni simbolo conserva autore, licenza, URL della fonte e vincoli.
4. **Controllo dell'utente.** Le preferenze manuali non vengono mai sovrascritte da scelte
   automatiche; l'IA propone, non decide.
5. **Niente pulsanti finti.** Una funzione assente deve essere dichiarata tale nell'interfaccia, non
   simulata.
6. **`Contesto.md` resta locale:** `.gitignore` contiene `/Contesto.md`. Non pubblicarlo, non
   includerlo negli installer, non inserirvi segreti o dati personali.

## Comandi di verifica

```bash
npm run build                     # controllo tipi Svelte + build Vite
cd src-tauri && cargo test        # unit test, test d'integrazione, corpus semantico
python3 scripts/check_context.py  # Contesto.md: presenza, 21 sezioni, esclusione da Git
```

Il corpus semantico (`tests/semantic_corpus.json`, 205 frasi annotate) produce
`tests/last_report.json`: riporta i numeri reali, senza arrotondarli per eccesso.

## Convenzioni tecniche

- Rust: moduli separati per database, linguistica, ricerca simbolica, IA, asset, sintesi vocale,
  esportazione; errori espliciti (`Result<_, String>` verso il frontend); niente `unwrap()` nei
  percorsi utente.
- Frontend: Svelte 5 con rune, TypeScript rigoroso, ProseMirror con `pid` stabile per paragrafo,
  analisi incrementale con debounce (mai l'IA a ogni tasto).
- Comandi Tauri: argomenti in camelCase dal JavaScript, snake_case lato Rust.
- Database: migrazioni versionate, FTS5 per la ricerca, nessun dato utente nella cartella dell'app.
- Formato nativo `.caadoc`: ZIP con manifest versionato, asset incorporati, scrittura atomica.

## Prima di concludere una sessione

Aggiorna in `Contesto.md`: stato delle funzionalità, decisioni prese, problemi nuovi, test eseguiti
con i risultati e le prossime attività in ordine di priorità. Poi esegui i comandi di verifica qui
sopra e riporta gli esiti reali.
