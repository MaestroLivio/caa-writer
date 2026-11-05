# Registro delle licenze — CAA Writer

Questo file separa esplicitamente le licenze delle diverse componenti.
Non tutto il contenuto del repository è CC BY-NC-SA 4.0: i componenti di terzi (modello, runtime) conservano la propria licenza.

## 1. Codice originale del software
- Licenza: **CC BY-NC-SA 4.0** (scelta del committente, vedi `LICENSE`).
- Nota: Creative Commons non è una licenza pensata per il software. La scelta
  è documentata e rispettata; l'eventuale passaggio a una licenza software
  (MIT/Apache-2.0) è una decisione del committente.
- Autori: CAA Writer contributors.

## 2. Documentazione
- `README.md`, `docs/**`, `LICENSES/**`: **CC BY-NC-SA 4.0**.

## 3. Template (Visual Resource Studio e documenti didattici)
- `resources/templates/**`: **CC BY-NC-SA 4.0**, autori CAA Writer contributors.

## 4. Pittogrammi originali (pacchetto `core`)
- `resources/symbol-packs/core/**`: **CC BY-NC-SA 4.0**.
- Autore: CAA Writer contributors. Disegnati ex novo, geometrie SVG originali.
- Nessuna imitazione di stili o opere tutelate.

## 5. Pittogrammi di terze parti
Il pacchetto **`arasaac`** è incluso nel repository (vedi sotto). Gli altri
repertori restano importabili localmente, mantenendo le licenze d'origine. Il
campo `commercial_ok` nella tabella `symbol_packs` governa il filtro
"escludi simboli non commerciali".

| Repertorio | Licenza tipica | Ridistribuibile nel repo | Note |
|---|---|---|---|
| ARASAAC | CC BY-NC-SA 4.0 | **Sì, incluso** | Attribuzione obbligatoria (ARASAAC, Sergio Palao), uso non commerciale, share-alike. Pacchetto `resources/symbol-packs/arasaac`; PNG ufficiali incorporati in SVG. |
| Mulberry Symbols | CC BY-SA 4.0 (varie) | Da verificare per singolo file | Verificare il singolo simbolo. |
| OpenSymbols | Aggregatore: la licenza è quella della fonte | No | Il metadata della fonte viene conservato per ogni simbolo. |
| Global Symbols | Dipende dalla fonte | No | Attribuzione per simbolo. |
| Sclera | CC BY-NC 4.0 (per set) | No | Solo uso non commerciale. |
| OpenMoji | CC BY-SA 4.0 | Sì, con attribuzione | Importabile come repertorio aggiuntivo. |
| Tawasol | CC BY-SA 4.0 | Da verificare | Attribuzione richiesta. |

Regola applicata dal software: se il documento viene destinato a uso
commerciale e l'utente attiva "solo simboli commercialmente compatibili",
i simboli con `commercial_ok = 0` vengono esclusi e il motore usa i fallback.

## 6. Modelli linguistici (GGUF)
- Non inclusi nel repository né negli installer.
- Modello consigliato: Qwen3-0.6B GGUF Q4_K_M (Apache-2.0 secondo il
  repository Qwen/Qwen3-0.6B-GGUF; verificare il file di licenza al download).
- L'utente scarica il modello separatamente; il software non lo ridistribuisce.

## 7. Dipendenze software
Vedere `LICENSES/dependencies.md` per l'elenco generato.
Codice Rust: per lo più MIT/Apache-2.0. Frontend: MIT (Svelte, ProseMirror, Vite).
SQLite (via rusqlite `bundled`): dominio pubblico.
Tauri: MIT/Apache-2.0.
