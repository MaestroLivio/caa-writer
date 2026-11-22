# CAA Writer 0.15.23 — motori installati all'avvio, libreria curata

Data: 22 novembre 2026

## Installazione automatica dei motori

Al primo avvio e dopo un aggiornamento il programma scarica e verifica i
pacchetti necessari prima di aprire l'editor: cataloghi di pittogrammi,
**modello semantico Horizon** e **voce neurale italiana**. Non esiste più il
rinvio «Più tardi»: senza pesi il motore userebbe solo le regole e la voce
cadrebbe su quella di sistema. La voce neurale e i suoni dell'app sono attivi
per impostazione predefinita.

Le build di release includono la feature `onnx`, quindi Horizon funziona
realmente dopo l'installazione e non solo nelle build di sviluppo.

## Meno componenti, nessun servizio esterno

- **Generazione immagini rimossa.** Il motore di diffusione, il runtime Python,
  i modelli e i provider online non esistono più. Le immagini si **importano**
  dai file e conservano autore, licenza, fonte e vincoli.
- **Provider IA online rimossi.** Non ci sono endpoint, chiavi API o voci nel
  portachiavi: la chat usa solo un modello GGUF locale se l'utente lo importa,
  altrimenti dichiara l'indisponibilità senza simulare risposte.
- CLI, server MCP, Skill e documentazione per agenti restano invariati.

## Correzioni

- **Errore di codifica su Windows.** Documenti, manifest, specifiche e file
  importati vengono letti riconoscendo BOM, UTF-16 e byte non UTF-8: non
  compare più «stream did not contain valid UTF-8» all'apertura.
- **Nessuna finestra console su Windows.** Download e installazione dei
  pacchetti passano dal processo con `CREATE_NO_WINDOW`.
- **Miniature delle pagine** nell'editor universale: sono ricavate dagli
  elementi locali della pagina, quindi non restano più riquadri vuoti.
- **TUTTO MAIUSCOLE** agisce anche sul testo dell'editor universale, non solo
  sul foglio dell'editor di testo.
- **Pannello laterale sempre ancorato**: la modalità sovrapposta, che su Windows
  copriva il documento, è stata rimossa e le preferenze salvate vengono migrate
  in «ancorato».
- **Reranking Horizon** non dipende più dal modello GGUF facoltativo: usa il
  modello semantico installato all'avvio.

## Libreria dei materiali curata

Da 535 a **350 materiali**, senza duplicati di titolo e con una sola voce per
ogni attività:

- 70 schede didattiche, 45 griglie e 30 quaderni di comunicazione (più il
  sistema «Comunicazione sociale adulta»);
- 30 materiali TEACCH, 25 agende visive e 22 sequenze;
- 25 storie sociali, 20 materiali ABA e 20 di autoregolazione;
- 16 stampabili, 14 materiali di autonomia, 12 giochi, 10 storie e 10 attività.

I 185 materiali esclusi sono conservati in `templates/archive/`, non nel bundle,
e possono essere ripristinati. Gli identificativi di categoria sono allineati
alla tassonomia della UI: non compaiono più categorie legacy come «didattica» o
«emozioni» fuori posto. La validazione della libreria riporta **0 errori**; gli
avvisi residui riguardano pittogrammi ARASAAC non presenti nel repository e
vengono risolti a runtime dal catalogo installato dall'utente.

Le anteprime non sono incluse nel bundle: l'editor ricava la miniatura dal
documento, così l'installer resta più piccolo senza perdere le immagini.

## Verifiche eseguite

- `npm run build`; `npm run check`: 0 errori, 59 avvisi.
- `npm run test:frontend`: 75/75 test superati.
- `cargo test --lib`: 313/313; `cargo test --lib --features onnx`: 318/318.
- `agent-skill/tests/check_skill.py`: coerente su 106/106 operazioni;
  `caa-cli methods` dal bundle non elenca più `image.*`.
- `caa-cli call library.validate` con i binari e le risorse del bundle:
  350 documenti, **0 errori**, 1600 avvisi di pittogrammi ARASAAC non presenti
  nel repository, 301 documenti validi.
- Build macOS arm64 firmata ad-hoc e verificata dal DMG montato; installer
  Windows x64 cross-compilato (`PE32+` GUI) e controllato staticamente.

Limite dichiarato: `cargo test --test semantic_corpus --features onnx` **non
passa** su questa macchina — 95,2 % di prima scelta e 15 approssimazioni
fuorvianti contro la soglia di 5. Il corpus richiede il catalogo ARASAAC
locale, che non è nel repository: senza di esso alcune parole finiscono su
concetti più generici. Il limite è noto dalle release precedenti e resta
aperto.
