# CAA Writer — note per Claude Code e altri agenti

Leggi **`Contesto.md`** nella radice del progetto prima di modificare il codice: è la memoria tecnica
condivisa (stato reale, decisioni, problemi aperti, test eseguiti, prossime attività). Aggiornalo
prima di terminare la sessione. Il file è deliberatamente escluso da Git: non pubblicarlo e non
copiarne il contenuto qui.

## Contesto essenziale

- Applicazione desktop **Tauri 2**: backend Rust (`src-tauri/src/`) e frontend Svelte 5 +
  TypeScript + ProseMirror (`src/`).
- Obiettivo: simbolizzazione CAA in italiano affidabile, offline, su computer con 4 GB di RAM.
- Il motore semantico è deterministico e verificabile; viene prima dell'LLM locale (llama.cpp +
  GGUF), che è incluso ma resta una proposta, non una decisione.
- I pittogrammi originali del pacchetto `core` e l'intero catalogo ARASAAC sono inclusi nel
  pacchetto `arasaac` (CC BY-NC-SA 4.0, autore Sergio Palao). Gli altri repertori di terzi non sono
  nel repository e si importano solo localmente.

## Regole

1. Non dichiarare test superati o integrazioni riuscite senza averli eseguiti: riporta i comandi e
   gli esiti reali.
2. Non simulare funzioni nell'interfaccia: se manca il backend, dichiaralo.
3. Rispetta le licenze e i metadati di ogni simbolo; nessun contenuto di terzi senza verifica.
4. Le scelte manuali dell'utente hanno precedenza assoluta sulle scelte automatiche.
5. Mantieni l'analisi incrementale: mai il modello a ogni tasto, mai ricostruire tutto il documento.

## Verifiche rapide

```bash
npm run build
cd src-tauri && cargo test
python3 scripts/check_context.py
```

Per la mappa completa dei moduli, del database e delle decisioni architetturali fai riferimento a
`Contesto.md` e a `docs/DEVELOPER.md`.
