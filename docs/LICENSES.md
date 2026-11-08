# Licenze e attribuzioni

## 1. Scelta del committente

Il progetto è distribuito secondo la scelta del committente: **Creative Commons Attribution -
NonCommercial - ShareAlike 4.0 International (CC BY-NC-SA 4.0)** per le componenti originali.

- Testo della licenza: https://creativecommons.org/licenses/by-nc-sa/4.0/
- File `LICENSE` nella radice.

### Avvertenza sulle licenze software

Creative Commons **sconsiglia** esplicitamente le proprie licenze per il software, perché non
trattano garanzie, brevetti e compatibilità con altre licenze software. Il committente può mantenere
CC BY-NC-SA 4.0 per documentazione, template e pittogrammi originali e adottare per il **codice** una
licenza software riconosciuta (per esempio MIT o Apache-2.0). La decisione spetta al committente:
questo documento non la modifica, la segnala.

## 2. Separazione delle componenti

| Componente | Licenza | Dove |
|---|---|---|
| Codice sorgente originale | CC BY-NC-SA 4.0 (come richiesto) — vedi avvertenza sopra | `src/`, `src-tauri/src/` |
| Documentazione | CC BY-NC-SA 4.0 | `README.md`, `docs/`, `Contesto.md` (locale) |
| Template di risorse | CC BY-NC-SA 4.0 | `resources/templates/*.json` |
| Pittogrammi originali (88 SVG) | CC BY-NC-SA 4.0, autori «CAA Writer contributors» | `resources/symbol-packs/core/svg/` |
| Pittogrammi ARASAAC (intero catalogo italiano: 13.906) | CC BY-NC-SA 4.0, Sergio Palao / ARASAAC | `resources/symbol-packs/arasaac/svg/` |
| Icone dell'applicazione | CC BY-NC-SA 4.0, generate da `scripts/generate_icons.py` | `src-tauri/icons/` |
| Dizionario italiano originale | CC BY-NC-SA 4.0 | `resources/dictionaries/it-core.json` |
| Modello linguistico incluso (Qwen2.5-0.5B-Instruct GGUF) | **Apache-2.0** (licenza del modello, invariata) | `resources/ai/model.gguf` |
| Modelli facoltativi scaricabili (Qwen2.5-1.5B, Qwen2.5-3B, SmolLM2-1.7B) | **Apache-2.0**; non sono inclusi nel repository | cartella `models/` dei dati utente |
| Runtime llama.cpp | MIT (software di terzi) | `resources/ai/` |
| Dipendenze software | licenze dei rispettivi progetti (MIT/Apache-2.0) | vedi `LICENSES/dependencies.md` |

## 3. Pittogrammi di terzi

**ARASAAC è incluso** nel pacchetto `resources/symbol-packs/arasaac/`, perché l'intero progetto è
distribuito con **CC BY-NC-SA 4.0**, la stessa licenza dei pittogrammi. Ogni simbolo porta autore
(Sergio Palao), titolare dei diritti (ARASAAC — Governo di Aragona), licenza, URL della licenza e
della fonte; i PNG ufficiali a 500 px sono incorporati in un contenitore SVG e la modifica è
dichiarata nel manifest.

Gli altri repertori con clausole diverse o non verificabili (Sclera, Mulberry, OpenSymbols, Global
Symbols, Tawasol) **non sono inclusi** e si importano solo localmente, mantenendo la licenza della
fonte.

Per importare un repertorio di terzi:

1. verifica la licenza e conserva autore, titolare dei diritti, URL della licenza e della fonte;
2. usa `scripts/import_symbols.py` indicando licenza, URL, autore e `--commercial-ok` solo se
   realmente consentito;
3. mantieni lo stato `da verificare` finché un responsabile non approva il simbolo;
4. non modificare la licenza del materiale importato.

L'applicazione supporta il filtro «solo simboli compatibili con uso commerciale», che esclude i
pacchetti marcati come non commerciali.

## 4. Attribuzione

Ogni simbolo conserva nel database: identificativo, repertorio, autore, titolare dei diritti,
licenza, URL della licenza, URL della fonte, vincoli, note sulle modifiche. Il comando
`export_attributions` genera un documento HTML con il registro dei simboli usati in un documento;
il pulsante **Licenze** nella barra superiore lo esporta.

Attribuzione minima per i contenuti originali:

> Pittogrammi e template: CAA Writer contributors, CC BY-NC-SA 4.0.
> Pittogrammi ARASAAC: Sergio Palao / ARASAAC — Governo di Aragona, CC BY-NC-SA 4.0.
> Codice: CAA Writer contributors, CC BY-NC-SA 4.0 (vedi avvertenza sull'uso di CC per il software).

## 5. Dipendenze principali

| Dipendenza | Licenza | Uso |
|---|---|---|
| Tauri 2 | MIT / Apache-2.0 | contenitore desktop |
| rusqlite (SQLite) | MIT | database e FTS5 (SQLite è di pubblico dominio) |
| serde / serde_json | MIT / Apache-2.0 | serializzazione |
| zip | MIT | contenitore `.caadoc` e DOCX |
| sha2 | MIT / Apache-2.0 | impronte dei pacchetti |
| roxmltree | MIT / Apache-2.0 | parsing XML sicuro |
| base64 | MIT / Apache-2.0 | asset nel DOCX |
| time | MIT / Apache-2.0 | timestamp |
| thiserror | MIT / Apache-2.0 | errori |
| Svelte 5 | MIT | interfaccia |
| ProseMirror (model, view, state, commands, history, keymap, schema-list, tables) | MIT | editor |
| Vite | MIT | build frontend |
| llama.cpp (non incluso) | MIT | inferenza locale, se configurato dall'utente |

Registro completo in `LICENSES/dependencies.md`.

## 6. Modelli linguistici

Il pacchetto include **Qwen2.5-0.5B-Instruct GGUF Q4_K_M** (`resources/ai/model.gguf`) e il runtime
llama.cpp, entrambi con licenza del progetto di origine (Apache-2.0 il modello, MIT il runtime).

Il catalogo dell'applicazione propone altri modelli **Apache-2.0** (Qwen2.5-1.5B-Instruct,
Qwen2.5-3B-Instruct, SmolLM2-1.7B-Instruct) che l'utente può scaricare esplicitamente. CAA Writer
**non li ridistribuisce**: il download avviene dal repository ufficiale indicato nel catalogo e il
file viene verificato con l'impronta sha256 pubblicata. L'utente può anche importare un GGUF già
presente sul proprio computer: ogni modello conserva la propria licenza, che non viene modificata.

Senza modello l'applicazione funziona con l'analisi lessicale deterministica; nessun modello viene
scaricato o attivato senza un'azione dell'utente.

## 7. Modelli per le immagini

Il motore immagini è **opzionale**. CAA Writer non ridistribuisce i pesi dei modelli diffusion:
l'utente li scarica dal repository ufficiale del modello scelto nella cache dell'app
(`models/image`), con la licenza dichiarata dal repository stesso.

- `segmind/tiny-sd` — CreativeML OpenRAIL-M (uso consentito con le restrizioni della licenza).
- `stabilityai/sd-turbo` — Stability AI Non-Commercial Research Community License: solo uso non
  commerciale, coerente con la licenza CC BY-NC-SA 4.0 del progetto; i pesi non sono inclusi.
- Qualsiasi altro modello diffusers inserito dall'utente conserva la propria licenza; l'app mostra
  il modello e lascia all'utente la verifica dei termini.

Il provider online è scelto e configurato dall'utente: si applicano i termini del servizio, e la
chiave API resta nel portachiavi di sistema.
