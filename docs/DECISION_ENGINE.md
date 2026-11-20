# Motore decisionale semantico — Horizon multilingual-zeroshot-small

Documento tecnico della milestone di integrazione. Descrive ciò che è stato
realmente implementato e verificato, e ciò che resta da fare. Non sostituisce
`Contesto.md`.

## 1. Audit del repository

Mappa reale dei punti di integrazione (nomi verificati nel codice):

| Funzione | File |
|---|---|
| Simbolizzazione per paragrafo | `src-tauri/src/engine.rs` (`symbolize`, `symbolize_internal`) |
| Classificazione dei concetti candidati | `src-tauri/src/symbol_search/mod.rs` (`rank_concepts`, `score_concept`) |
| Alternative del pittogramma | `src-tauri/src/engine.rs` (`build_alternatives`, `library_alternatives`) |
| Lessico, cue di contesto, morfologia | `src-tauri/src/linguistics/` |
| Preferenze e scelte manuali | database SQLite + `src/lib/user-symbols.ts` |
| Documento universale `.caadoc` | `src-tauri/src/universal.rs`, `universal_build.rs` |
| Dispatch condiviso CLI/MCP/GUI | `src-tauri/src/automation/mod.rs` |
| Registro operazioni e schemi | `src-tauri/src/automation/registry.rs` |
| Politica accesso/dry-run/conferme | `src-tauri/src/automation/policy.rs` |
| CLI | `src-tauri/src/bin/caa-cli.rs` |
| Server MCP | `src-tauri/src/bin/caa-mcp.rs` |
| Comandi Tauri | `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` |
| Impostazioni | `src/lib/components/SettingsDialog.svelte` |

Il motore CAA esistente non è stato riscritto: il DecisionEngine è un servizio
separato che riceve candidati già trovati dal motore di ricerca.

## 2. Artefatto verificato

| Campo | Valore verificato |
|---|---|
| Identificativo | `Horizon-Labs/multilingual-zeroshot-small` |
| Revisione | `17af5f8a728d101ce33a9bffad227b0cd005e95e` |
| Licenza | Apache-2.0 |
| Parametri | 141.000.000 |
| Architettura | ModernBERT (mmBERT-small), 22 layer, hidden 384, vocabolario 256.000, contesto 8.192 (fine-tuning fino a 1.024) |
| Runtime | ONNX Runtime 1.20 (`ort` 2.0.0-rc.9) |
| Tokenizer | `tokenizers` 0.21, `tokenizer.json` ufficiale |
| Pesi usati | `onnx/model_quantized.onnx`, 268.409.234 byte |
| Input ONNX | `input_ids`, `attention_mask` (int64, `[1, seq]`) |
| Output ONNX | `logits` `[1, 2]`: 0 = not_entailment, 1 = entailment |
| Totale installazione | 302.835.263 byte |

Hash ufficiali verificati con `SHA256SUMS` del repository:
`0ac394942a086fb1c02beb0fde0357bf7f3bd739ca07af7fa6c9fa11c9c05512`
(pesi quantizzati), più tokenizer, configurazione, licenza e notice.
Manifest versionato: `resources/decision/horizon-multilingual-zeroshot-small.manifest.json`.

La model card dichiara che storicamente etichette e template erano in inglese;
la versione v1.3 aggiunge etichette native. Il progetto usa ipotesi italiane e
lo dichiara nei metadati (`hypothesis_version = it-1`).

## 3. Architettura implementata

```
automation::dispatch / comandi Tauri / CLI / MCP
                    │
                    ▼
        decision::engine::DecisionEngine
                    │
     ┌──────────────┼───────────────────┐
     ▼              ▼                   ▼
 DecisionCache   Scheduler          DecisionAudit
     │              │                   │
     └──────────────┼───────────────────┘
                    ▼
              Core::evaluate
     ┌──────────────┼───────────────────┐
     ▼              ▼                   ▼
 RuleEngine     HypothesisBuilder   ConfidencePolicy
 (omonimi)      (template it-1)     (soglie prudenti)
     │              │                   │
     └──────────────┼───────────────────┘
                    ▼
              HorizonScorer
        ┌───────────┴────────────┐
        ▼                        ▼
 OnnxHorizon (feature onnx)   RuleScorer
```

File:

| File | Responsabilità |
|---|---|
| `decision/types.rs` | Contratti, 15 task, stati, azioni, priorità, validazione |
| `decision/registry.rs` | Task registry e tassonomie predefinite italiane |
| `decision/hypothesis.rs` | Template italiani versionati (`it-1`) |
| `decision/rules.rs` | Vincoli deterministici per omonimi italiani |
| `decision/scores.rs` | Softmax stabile, ranking, margine |
| `decision/policy.rs` | Soglie, astensione, task consultivi |
| `decision/cache.rs` | Cache LRU in memoria con chiavi versionate |
| `decision/audit.rs` | Diagnostica senza testo utente |
| `decision/engine.rs` | Orchestrazione, scheduler a priorità, annullamento, facade |
| `decision/model.rs` | Manifest, verifica SHA-256, installazione atomica, rimozione |
| `decision/onnx.rs` | Sessione ONNX e tokenizer (solo feature `onnx`) |

## 4. Contratto API

Richiesta (`decision.classify` / `decision_classify`):

```json
{
  "task": "pictogram_disambiguation",
  "language": "it",
  "input": {
    "text": "Ho mangiato una pesca.",
    "target": { "text": "pesca", "start": 16, "end": 21 }
  },
  "candidates": [
    { "id": "frutto", "label": "frutto (alimento)" },
    { "id": "attivita", "label": "attività di pesca (catturare pesci)" }
  ],
  "options": { "mode": "suggest", "multi_label": false }
}
```

Gli offset di `target` sono indici di **caratteri Unicode**, non byte.

Risposta: `status`, `selected_candidate_id`, `ranked_candidates` con punteggio e
`score_kind` (`entailment`, `hybrid`, `rule`, `locked`), `decision.action`
(`apply` / `suggest` / `abstain`) e metadati (`method`, `model_revision`,
`cached`, `latency_ms`, `hypothesis_version`, `taxonomy_version`).

Stati: `queued`, `running`, `completed`, `cancelled`, `timed_out`, `invalid`,
`unavailable`, `failed`.

Task: `pictogram_disambiguation`, `communication_intent`, `semantic_category`,
`board_category`, `semantic_search_ranking`, `didactic_template_selection`,
`learning_objective`, `exercise_type`, `social_story_topic`,
`expressed_emotion`, `interactive_action`, `layout_template`,
`caa_semantic_coherence`, `content_cataloguing`, `command_routing`.

## 5. Regole e confidenza

- Una scelta manuale (`locked`) prevale sempre e non viene sovrascritta.
- I candidati `blocked` sono esclusi prima dell'inferenza.
- Per gli omonimi noti (`pesca`, `amo`, `riso`, `porta`, `piano`, `penna`) le
  regole deterministiche entrano nel punteggio con peso 0,7; Horizon fornisce
  il restante 0,3. Senza indizi la regola non scatta e decide il modello.
- I task didattici, emotivi, di catalogazione, coerenza e instradamento comandi
  restano sempre `suggest`: nessuna decisione pedagogica vincolante.
- Le soglie predefinite sono prudenti (`suggest 0,60`, `apply 0,88`,
  margine minimo `0,15`) e **non sono calibrate**: vanno validate sui dati
  italiani prima di abbassarle.

## 6. Comandi

CLI (con feature ONNX):

```bash
cd src-tauri
cargo build --features onnx --bin caa-cli

caa-cli call decision.status '{}'
caa-cli call decision.model.status '{}'
caa-cli call decision.model.install '{}'
caa-cli call decision.model.verify '{}'
caa-cli call decision.model.remove '{"confirm":true}'
caa-cli call decision.classify '{"task":"communication_intent","text":"Vorrei bere dell'\''acqua."}'
caa-cli call decision.validate - < parametri-casi.json
```

`decision.validate` accetta `{"cases":[...]}` e restituisce `accuracy`,
`abstained`, `mean_latency_ms` ed `errors`. Il corpus curato è
`tests/decision_corpus.json` (23 casi): prodotto da
`python3 scripts/build_decision_corpus.py`.
Il corpus principale è `tests/decision_corpus_500.json`: **649 casi** generati
da `python3 scripts/build_decision_corpus_500.py`, con split deterministico
453 calibrazione / 196 validazione. Il report delle soglie è
`resources/decision/calibration.json`; `decision.calibrate` calcola anche i
margini per singolo task.

MCP: gli stessi metodi sono esposti come `decision_status`,
`decision_classify`, `decision_audit`, `decision_model_status`,
`decision_model_verify`, `decision_model_install`, `decision_model_remove`.
Il server MCP usa lo stesso `dispatch`, quindi la logica non è duplicata.

Tauri: `decision_status`, `decision_classify`, `decision_model_install`,
`decision_model_remove`, `decision_model_verify`.

## 7. Verifiche reali eseguite

Ambiente: macOS arm64, Rust 1.99.1, modello quantizzato scaricato da Hugging
Face con revisione bloccata.

| Verifica | Esito |
|---|---|
| Caricamento sessione ONNX (lazy, alla prima inferenza) | ~2,5 s |
| Prima inferenza CLI (include caricamento lazy) | ~2,5 s, `method = neural+rules`, `status = completed` |
| Inferenze successive nello stesso processo (MCP/GUI) | ~33 ms |
| Casi obbligatori di disambiguazione | 12/12 con Horizon + regole |
| Frasi nuove (held-out, stessi indizi lessicali) | 11/11 |
| Solo Horizon, senza regole, sui 12 casi | 8–9/12 (limite reale del modello) |
| Verifica SHA-256 dei file installati | `ok = true` su tutti i file |
| Installazione da zero | 302.835.263 byte in ~36 s |
| Rimozione | `removed = true` |
| Cache | hit/miss riportati in `decision.status` |
| `decision.validate` sul corpus curato da 23 casi | 23/23, latenza media 27 ms |
| Corpus di calibrazione | 649 casi, split 453/196 |
| Calibrazione | soglie globali 0,37/0,55 più margini per task |
| Holdout (196 casi) | copertura 26,0%, precisione sui suggeriti 92,2%, accuratezza complessiva 24,0% |
| Soglie per task | disambiguazione 0,01; intenzione 0,40; categoria 0,51; board 0,80; emozione 0,06; azione 0,01; layout 0,01; obiettivo 0,40 |
| Holdout: disambiguazione | 5/5 |
| Holdout: altre aree | molte astensioni; il modello piccolo non è affidabile per emozioni, esercizi, storie e layout |
| Memoria (RSS, build debug macOS arm64) | 822 MB con sessione ONNX attiva |

I punteggi zero-shot reali sono bassi e non calibrati (es. «pesca/frutto»
0,17 contro «attività» 0,14). La combinazione con le regole produce il
ranking corretto, ma la confidenza assoluta non va presentata come percentuale
di accuratezza.

## 8. Integrazione negli editor

Il DecisionEngine è collegato in modo asincrono, con debounce, `cancel_key` e
scarto delle risposte obsolete:

| Editor | File | Task | Comportamento |
|---|---|---|---|
| Digitazione | `src/App.svelte` | `pictogram_disambiguation` | dopo 1,2 s di pausa rilavora fino a 6 token incerti; applica solo con azione `apply` e non tocca scelte manuali (confidence 1.0) né token bloccati |
| Alternative del pittogramma | `SidePanel.svelte` | `pictogram_disambiguation` | riordina le alternative, mostra «suggerito»; non modifica il simbolo |
| Ricerca nel pannello | `SidePanel.svelte` | `semantic_search_ranking` | riordina i risultati già recuperati |
| Categoria semantica | `SidePanel.svelte` | `semantic_category` | mostra la categoria suggerita della parola |
| Coerenza CAA | `SidePanel.svelte` | `caa_semantic_coherence` | verifica dedicata su richiesta, con esito o astensione |
| Comunicatore | `Communicator.svelte` | `communication_intent`, `board_category`, `expressed_emotion` | suggerisce intenzione, tabella ed emozione |
| Story Studio | `StoryStudio.svelte` | `social_story_topic`, `layout_template` | suggerisce tema e tipo di storia |
| Resource Studio | `SidePanel.svelte` | `didactic_template_selection` | suggerisce un modello didattico reale |
| Ispettore elemento | `SidePanel.svelte` | `interactive_action` | propone un tipo di azione del player |
| Browser libreria | `LibraryBrowser.svelte` | `content_cataloguing`, `learning_objective`, `exercise_type` | propone categoria, obiettivo ed esercizio |
| Console CLI | `CliConsole.svelte` | `command_routing` | propone un metodo, senza eseguirlo |

Con le soglie per task molte aree si astengono: è voluto. La UI mostra un
suggerimento solo quando il margine supera la soglia del task e non visualizza
percentuali non calibrate. Per esempio, una frase di `board_category` richiede
margine 0,80: sotto quella soglia la UI non propone nulla e lascia la scelta
manuale.

Le funzioni pure di riordino sono in `src/lib/decision/ranking.ts` e hanno test
Node (`tests/frontend/decision-ranking.test.mjs`). Il client asincrono è in
`src/lib/decision/client.ts`. Le impostazioni invalidano lo stato del motore
dopo installazione/rimozione tramite l'evento `caa-decision-changed`.

## 9. Limiti dichiarati

1. **Copertura moderata.** Sul corpus il sistema suggerisce il 26,0% dei casi
   holdout, con precisione 92,2%. È una scelta deliberata: le aree deboli
   (board category, parte degli obiettivi) si astengono invece di sbagliare.
2. **Corpus sintetico.** I 649 casi sono scritti a mano ma non annotati da
   clinici o docenti; non sostituiscono un benchmark validato.
3. **Task collegati ma non tutti applicativi.** Tutti i 15 task hanno un hook
   UI; solo alcuni applicano automaticamente (disambiguazione, azione,
   layout). Gli altri restano suggerimenti con conferma manuale.
5. **Memoria su 4 GB.** La RSS misurata è 822 MB su build debug macOS arm64.
   Non è stata misurata su una macchina da 4 GB né in build release.
6. **Windows e Linux.** Build e runtime non verificati su hardware reale.
7. **Runtime ONNX incluso, pesi separati.** La build di release abilita la
   feature `onnx` e collega staticamente ONNX Runtime: non serve una libreria
   esterna. I pesi del modello (268 MB) non sono nel bundle e si installano
   dal Gestore pacchetti. Windows e Linux sono compilati dal workflow ma non
   eseguiti su hardware reale.

## 10. Prossimi passi

1. Usare le regole del lessico esistente (`rank_concepts`) come prior invece
   della tabella locale degli omonimi.
2. Far annotare il corpus da docenti/logopedisti e ricalibrare le soglie.
3. Verificare su hardware reale Windows e Linux il runtime ONNX statico
   (macOS arm64 verificato; Windows/Linux solo compilati in CI).
4. Misurare la RAM su 4 GB e in build release.
5. Valutare un secondo passaggio di reranking sui paragrafi lunghi, sempre
   asincrono e con limite di token.
