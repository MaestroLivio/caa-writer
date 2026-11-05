# Changelog

Tutte le date sono in formato ISO (AAAA-MM-GG). Il progetto segue SemVer:
`MAJOR` per cambi incompatibili del formato o delle preferenze, `MINOR` per
funzionalità compatibili, `PATCH` per correzioni compatibili.

## 0.2.0 — 2026-11-05

### Editor e selezione

- **Qualsiasi parola è selezionabile**: clic o selezione nel testo aprono il
  pannello anche quando la parola non ha un pittogramma. Non serve più un chip
  preesistente.
- **Aggiunta di un pittogramma a una parola senza simbolo**: ricerca manuale,
  gruppi di affinità o alternative del repertorio; il simbolo viene applicato
  subito, il testo originale non cambia.
- **Selezione stabile**: prima di applicare una scelta il motore verifica che
  la parola all'indice selezionato sia ancora quella scelta; dopo una
  rianalisi non si applica più il simbolo alla parola sbagliata.
- **Nessuna terza riga**: la didascalia sotto il pittogramma è spenta per
  default. Restano il pittogramma e il testo digitato; l'etichetta originale
  del repertorio vive nei metadati (pannello, tooltip, licenze). La didascalia
  si può riattivare dalle impostazioni.
- **Stili di bordo persistenti**: nessun bordo, solo immagine, immagine e
  testo, bordi separati, bordo con divisore.

### Ricerca semantica

- **Affinità della parola** (`word_affinities`): gruppi esatto, lemma,
  varianti, sinonimi, concetto generale, affini; punteggio, motivo leggibile e
  indicazione di approssimazione; deduplica per simbolo.
- **Parole Fuori lessico**: «principesse» → «principessa» tramite regole non
  ambigue verificate sul repertorio; nessun concetto inventato.
- **Condizionale**: «assaggerei» viene ricondotto ad «assaggiare».
- **Sinonimi e concetti nuovi**: «capire» → «comprendere», «gelati» →
  «gelato», «non so» come espressione di incertezza, «da sole» come
  espressione (non il Sole).
- **Unità multiword**: le espressioni non ereditano più la funzione
  grammaticale della prima parola; «non so» non propaga la negazione al verbo
  successivo.

### Correzioni

- **Bundle macOS**: Tauri 2 colloca `bundle.resources` in
  `Contents/Resources/_up_/resources`; l'app confezionata si chiudeva
  all'avvio perché non trovava il lessico. La risoluzione delle risorse copre
  ora sviluppo, `_up_` e bundle; 3 test dedicati.
- **Seed idempotente**: concetti, lemmi, significati, flessioni e sinonimi non
  vengono duplicati quando un pacchetto viene reimportato.
- **Parser delle risposte neurali**: accetta elenchi su una riga separati da
  punti, punti esclamativi/interrogativi e barre.
- **Processo `llama-server`** fermato all'uscita dell'app.
- **Cache neurale** legata al modello attivo: cambiare GGUF non restituisce
  più risposte del modello precedente.

### Test

- 151 test di libreria Rust, corpus semantico, prestazioni, diagnostica ed esportazioni.
- 7 test Node della mappatura selezione → token (`npm run test:frontend`).
- Verifica manuale nella finestra Tauri: «capire» selezionato dal testo,
  pittogramma «comprendere» applicato, testo invariato, didascalie spente.

### Limiti noti

- Windows, Linux e macOS Intel non sono compilati su questa macchina; le build
  native sono affidate ai runner GitHub Actions.
- Nessuna firma o notarizzazione: mancano certificati.
- La ricerca online di immagini non è inclusa in questa versione.
- La copertura di sinonimi e relazioni dipende dal lessico incluso.
