# Runtime IA locale

Questa cartella contiene `llama-cli`, `llama-server` e le librerie native del
motore neurale. I binari **non sono versionati** (pesano decine di MB e
dipendono dal sistema operativo).

Per preparare una build:

```bash
python3 scripts/fetch_llama.py
```

Lo script scarica la build ufficiale di llama.cpp per la macchina corrente e la
installa qui. Il modello GGUF invece non è più incluso nell'installer: si
scarica dalla scheda Impostazioni → IA oppure dal Gestore pacchetti.

Il file `README.md` serve a mantenere la cartella nel repository, così il glob
`../resources/ai/runtime/**/*` di `tauri.conf.json` esiste anche su un clone
nuovo. Senza runtime l'app si avvia e funziona: le funzioni neurali dichiarano
di non essere disponibili e propongono l'installazione.
