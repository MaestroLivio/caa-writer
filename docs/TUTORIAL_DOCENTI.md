# CAA Writer — Tutorial per docenti

Guida pratica all'uso del programma in classe e nello studio di progettazione.
Ogni sezione indica **cosa fa la funzione**, **come si usa** e **un caso reale**.
Le funzioni elencate esistono nell'applicazione installata: nulla è simulato.
Il video del tutorial (dieci capitoli, voce italiana, grafiche animate) segue lo
stesso ordine di questa guida.

Versione di riferimento: **0.15.22**. Piattaforme testate: macOS (Apple
Silicon) e Windows x64 (installer compilato e controllato staticamente).

---

## 1. Che cos'è CAA Writer

Videoscrittore italiano per la **Comunicazione Aumentativa e Alternativa**.
Mentre scrivi, il programma propone pittogrammi per le parole e costruisce
materiali stampabili: tabelle comunicative, agende, token board, storie, schede.

- **Offline per primo**: apertura, modifica, salvataggio e stampa non richiedono
  rete né GPU. La ricerca online di simboli è un'opzione esplicita.
- **Il docente decide**: le scelte manuali non vengono mai sovrascritte
  dall'analisi automatica.
- **Nessun pulsante finto**: se il motore o un repertorio non sono disponibili,
  l'applicazione lo dichiara e resta utilizzabile in sola modalità testo.
- **Licenza**: progetto con CC BY-NC-SA 4.0. I pittogrammi ARASAAC usati sono
  CC BY-NC-SA 4.0, autore Sergio Palao, titolare ARASAAC — Governo di Aragona:
  l'attribuzione è registrata nel simbolo e nel registro delle licenze.

**Caso d'uso** — Un docente di sostegno prepara la giornata di un alunno con
disturbo dello spettro autistico: agenda visiva, tabella per chiedere aiuto,
token board per il rinforzo. Tutto viene creato e stampato sul portatile della
scuola, senza connessione.

---

## 2. Primo avvio: profilo dell'alunno o Area Creator

All'apertura compare una schermata con due ingressi.

| Ingresso | A chi serve | Cosa permette |
|---|---|---|
| **Profilo dell'alunno** | allievo, famiglia, operatore in sessione | solo i materiali assegnati, il livello di scrittura e la voce previsti dal profilo |
| **Area Creator** | insegnanti, logopedisti, educatori | documenti, libreria, Studio, profili, impostazioni |

Nel primo avvio l'applicazione crea il database locale e importa il repertorio
incluso. Se qualcosa non riesce lo segnala: il documento resta scrivibile in
modalità solo testo.

**Caso d'uso** — Il docente apre l'Area Creator e crea il profilo «Luca,
primaria»; assegna l'agenda settimanale e la tabella «chiedo aiuto», imposta il
livello 2 e attiva la voce. In classe apre il profilo: Luca vede solo quelle due
schermate, grandi e toccabili.

---

## 3. Scrivere e simbolizzare

1. Apri un documento nuovo e scrivi una frase: `Il bambino mangia una mela.`
2. Dopo una breve pausa (debounce) i pittogrammi compaiono **sopra le parole**.
3. L'analisi è **incrementale**: viene rianalizzato solo il paragrafo modificato,
   quindi la digitazione non viene mai bloccata. I risultati vecchi vengono
   scartati se continui a scrivere.

### Modalità di rappresentazione

| Modalità | Cosa vede l'alunno | Quando usarla |
|---|---|---|
| A · Testo | solo testo | dettato, verifica, testo da leggere |
| B · Simboli sopra | pittogramma sopra ogni unità | lettura con supporto visivo |
| C · Didascalia sotto | pittogramma con la parola sotto | primo apprendimento del lessico |
| D · Solo simboli | pittogrammi grandi, testo attenuato | comunicazione rapida |
| E · CAA semplificata | solo i concetti fondamentali | frasi lunghe da ridurre |

### Intensità

- **Minima** — parole chiave (nomi, verbi pieni, aggettivi importanti).
- **Media** — tutte le unità semantiche utili.
- **Completa** — copertura massima, senza simboli fuorvianti.

Cambiare modalità o intensità **non modifica il testo**: si torna indietro in
qualsiasi momento.

### Unità multiparola

Espressioni come «sintesi vocale», «computer portatile», «carta di identità»
diventano **una sola unità** con un solo pittogramma. Si possono unire e
separare anche manualmente: l'unione è persistente e annullabile.

**Caso d'uso** — Lezione di scienze in classe quarta: il docente scrive il
paragrafo alla lavagna digitale in modalità B e intensità media; gli alunni
leggono il testo e collegano ogni parola al pittogramma. La stessa pagina,
stampata, diventa la scheda di studio.

---

## 4. Correggere e bloccare un pittogramma

1. Fai clic sul pittogramma nel documento.
2. Nel pannello **Alternative** vedi: simbolo attuale, significato attribuito,
   strategia usata (esatto, lemma, sinonimo, concetto generale…) e alternative.
3. Scegli un'alternativa: la sostituzione è **immediata** e vale **solo per
   quell'occorrenza**.
4. Per renderla stabile scegli l'ambito:
   - **solo questa occorrenza**;
   - **tutte le occorrenze equivalenti nel documento**;
   - **solo il profilo Utilizzatore attivo**;
   - **tutti i documenti futuri** (preferenza globale);
   - **vocabolario condiviso del Creator**.

Le associazioni bloccate non vengono mai sovrascritte. Dalla 0.15.22 la scelta è
legata **alla parola**: se scrivi «famiglia» al posto di «casa», il pittogramma
di «casa» non resta. I suggerimenti automatici non creano blocchi.

**Caso d'uso** — In una tabella sulla famiglia il pittogramma di «casa» non è
adatto: la docente lo sostituisce con «famiglia» e sceglie «usa sempre per questa
parola». Da quel momento ogni documento userà quel pittogramma solo per
«famiglia».

---

## 5. Quando un pittogramma manca: la scala di fallback

L'applicazione non inventa. Segue questa scala e **lo dichiara**:

1. **Sinonimo equivalente** (automobile → macchina).
2. **Forma base** (camminavano → camminare).
3. **Concetto più generale** (bassotto → cane): la parola scritta resta
   «bassotto», il pittogramma è quello del cane.
4. **Composizione** (monopattino elettrico → monopattino + elettricità).
5. **Scomposizione** dei termini complessi.
6. **Concetto vicino**, sempre marcato come non esatto.
7. **Nessun simbolo**: meglio nessun pittogramma che un pittogramma sbagliato.

Il pittogramma approssimativo ha **bordo tratteggiato** e didascalia con `≈`.

### Aggiungere un simbolo

- **Ricerca simboli**: per parola o per categoria, con il repertorio locale e i
  simboli personali prima dei repertori installati.
- **Importa SVG**: il file viene validato (niente script, niente riferimenti
  esterni, `viewBox` obbligatorio) e associato alla parola; resta disponibile
  offline.
- **Importa immagini o cartelle**: PNG, JPEG, GIF, WebP, BMP, TIFF, ICO, AVIF,
  SVG; le cartelle diventano **collezioni personali**. HEIC non è supportato e
  viene segnalato.
- **Ricerca online (opzionale)**: ARASAAC come prima scelta, Wikimedia come
  ripiego. Vengono inviati al massimo quattro **termini generici** filtrati dal
  lessico locale; la frase completa non lascia il dispositivo. Ogni copia
  scaricata conserva autore, licenza, URL della fonte e vincoli d'uso.

**Caso d'uso** — Un'alunna parla di «bassotto»: la maestra accetta il cane come
approssimazione (bordo tratteggiato, `≈`), poi importa una foto del cane di
famiglia come simbolo personale nella collezione «Animali di casa».

---

## 6. Materiali: Studio, libreria, editor universale

### Studio (Visual Resource Studio)

1. Descrivi la risorsa in italiano: «Crea un'agenda visiva settimanale per la
   scuola primaria con entrata, intervallo, pranzo, gioco».
2. Scegli un modello o lascia **Modello automatico**.
3. Premi **Genera**: ottieni un'anteprima con pittogrammi ed etichette
   modificabili.
4. **Stampa / PDF** produce il foglio impaginato.

L'interpretazione è deterministica: nessun HTML o CSS viene generato da un
modello linguistico. Se la richiesta è troppo complessa l'applicazione propone
un modello da completare.

### Libreria

Oltre **500 materiali nativi** (30 board comunicative, 4 quaderni, 77 griglie di
comunicazione, 15 agende, token board ABA, storie, schede), organizzati per
categoria e sottocategoria. Ogni board e ogni agenda include la pagina
**Vocabolario essenziale** con 24 parole core: un pattern stabile, identico in
tutti i materiali, che i docenti possono usare come punto di partenza.

### Editor universale

I materiali sono **documenti `.caadoc`**, non immagini:

- sposta, ridimensiona, ruota, duplica, raggruppa, blocca, nasconde elementi;
- modifica testi, voci, pittogrammi, colori, bordi, celle, azioni, collegamenti;
- aggiunge e riordina pagine: **documento** (flusso), **presentazione**
  (posizione libera), **griglia** (tabella comunicativa);
- il pulsante **Griglia** crea tutte le celle richieste (per esempio 4×5 = 20);
- **Anteprima** e **Utilizzatore** mostrano lo stesso documento con navigazione,
  composizione della frase e voce.

**Caso d'uso** — La classe prepara il «giro della scuola»: dal materiale
«Comunicazione libera avanzata» il docente duplica una pagina, sostituisce le
celle con i luoghi reali (palestra, mensa, biblioteca) e collega ogni cella alla
pagina corrispondente. Il file salvato si apre direttamente nel comunicatore.

---

## 7. Comunicatore e modalità Utilizzatore

- **Tocca per parlare** — un tocco, una parola pronunciata.
- **Componi frase** — le parole si accumulano; **Parla frase** le legge;
  **Svuota** ricomincia.
- **Modifica** — etichetta, testo pronunciato, pittogramma e azione di ogni
  cella; si possono aggiungere celle e pagine e salvare in `.caadoc`.

Azioni ammesse: **aggiungi alla frase, pronuncia, vai alla pagina, indietro,
home, svuota**. Sono le uniche: un file non può eseguire codice, aprire URL o
lanciare comandi.

### Tre livelli dell'editor Utilizzatore

| Livello | Cosa vede | Per chi |
|---|---|---|
| 1 · Solo simboli | grande vocabolario di base | primi passi, accesso a scansione |
| 2 · Scrivo con aiuto | tastiera visuale e parole frequenti | avvio della scrittura |
| 3 · Scrivo | scrittura completa con formattazione | autonomia |

L'editor Utilizzatore non mostra Ribbon, Studio o comandi del Creator.

**Caso d'uso** — In mensa, l'alunno usa il livello 1: tocca pane, acqua, poi
«Parla frase». In classe, con lo stesso profilo, passa al livello 2 per
scrivere il titolo del quaderno.

---

## 8. Voce e accessibilità

- **Ascolta** legge il paragrafo selezionato o tutto il documento con la voce di
  sistema (macOS `say`, Windows SAPI); legge il **testo originale**, non la
  descrizione del pittogramma.
- Durante la lettura compare un indicatore con **Ferma**; la lettura non blocca
  l'editor.
- **Controlli** (sempre in basso a sinistra): tastiera a schermo, spazio,
  cancellazione, invio, annulla/ripeti, ascolto, dimensione del testo **10–32
  px**, interruttore **TUTTO MAIUSCOLE**. Il pannello non compare in stampa.
- Temi **chiaro, scuro, alto contrasto**; etichette `aria-label`; rispetto di
  «riduci animazioni» del sistema; interfaccia ottimizzata per touchscreen.
- La sintesi vocale è **disattivata per default**: si attiva in Impostazioni →
  Accessibilità.

**Caso d'uso** — Un alunno ipovedente usa tema alto contrasto, testo 28 px e
tutto maiuscole; ascolta la consegna della scheda con «Ascolta» senza perdere il
punto in cui si trova.

---

## 9. Salvare, esportare, stampare

- **Salva** produce un file **`.caadoc`**: archivio ZIP con manifest versionato
  che contiene testo, formattazione, pittogrammi, scelte manuali, metadati e i
  simboli usati. La scrittura è **atomica**: un salvataggio interrotto non
  corrompe il file precedente.
- **Bozza di recupero**: ogni 20 secondi, se ci sono modifiche non salvate;
  all'avvio l'applicazione propone di recuperarla.
- **Importa**: TXT, DOCX, HTML, Markdown — il testo viene conservato, la
  formattazione complessa no (l'applicazione lo segnala).
- **Esporta**:
  - **PDF** multipagina;
  - **PNG** della prima pagina;
  - **HTML** autonomo con SVG incorporati (versione interattiva con navigazione
    e voce del browser per i materiali dell'editor universale);
  - **DOCX**: testo, pittogrammi e griglie come elementi Word nativi, caselle di
    testo e immagini selezionabili singolarmente;
  - **PPTX**: ogni elemento e ogni cella come forma indipendente, con colori,
    sfondi, bordi e rotazione.
- **Stampa**: dialogo nativo della finestra (da lì anche «Salva come PDF»).
- **Licenze**: esporta il registro delle attribuzioni dei simboli usati.

**Caso d'uso** — Il docente esporta la token board in PPTX per ritoccare i
colori, l'agenda in PDF per la stampa a colori e il quaderno in HTML per
condividerlo sulla LIM.

---

## 10. Automazione per docenti

Il motore è utilizzabile **senza interfaccia grafica**: la CLI `caa-cli` usa lo
stesso dispatch dell'app.

```bash
cd src-tauri
cargo run --bin caa-cli -- info
cargo run --bin caa-cli -- methods
cargo run --bin caa-cli -- call symbolize '{"text":"Il gatto dorme."}'
cargo run --bin caa-cli -- rpc     # richieste JSONL da stdin
```

- `--data-dir` e `--resources` isolano i test; `--access read|write|admin`
  limita le operazioni; `--dry-run` mostra cosa farebbe senza eseguire.
- Le operazioni distruttive richiedono `confirm: true`.
- I percorsi dei progetti restano confinati in `<data-dir>/projects`: percorsi
  assoluti, `..` e link simbolici che escono dalla radice vengono rifiutati.
- Il server MCP `caa-mcp` espone gli stessi metodi (stdio) a un assistente
  virtuale.

**Casi d'uso reali**

1. **Preparare dieci schede in blocco** — un ciclo `rpc` che chiama `symbolize`
   per dieci frasi e salva un documento per ciascuna.
2. **Controllo di coerenza** — `document.export` in PNG di tutti i materiali di
   una cartella per la revisione prima della stampa.
3. **Verifica della libreria** — `caa-cli library validate`: 535 documenti,
   0 errori (1717 avvisi di pittogrammi mancanti già presenti nei materiali
   originali).

Limiti dichiarati: la CLI **non** pilota la GUI aperta, non apre dialoghi di file
e non firma né pubblica nulla.

---

## 11. Scenari didattici completi

### Primaria — «La giornata di Luca»
Agenda settimanale dallo Studio → modifica celle nell'editor universale →
profilo «Luca, primaria» livello 2 → stampa in PDF per la famiglia.
Tempo di preparazione: circa 15 minuti.

### Secondaria — «Riassunto di storia»
Testo nel documento, modalità B, intensità media → correzione dei pittogrammi
ambigui con «usa sempre per questa parola» → esportazione DOCX per il compagno
con DSA.

### ABA / TEACCH — «Prima e poi» e token economy
Materiali `aba_prima_poi_board` e `aba_token_economy` → inserimento delle
attività reali della giornata → esportazione PPTX per la personalizzazione.
Sono strumenti di supporto alla progettazione: non automatizzano valutazioni
cliniche e non vanno usati in pratiche punitive o coercitive.

### Logopedia — «Allenamento lessicale»
Collezione personale di immagini del bambino → simboli personali associati alle
parole → ascolto della frase con la voce di sistema → registrazione dei progressi
in una scheda dati.

---

## 12. Limiti e trasparenza

- Windows e Linux sono compilati dal workflow di release ma **non eseguiti su
  hardware reale** in questa sessione; il pannello Controlli non è stato provato
  su un touchscreen reale.
- Il modello neurale opzionale (268 MB) non è incluso: si installa dal Gestore
  pacchetti. Senza modello l'applicazione usa solo l'analisi deterministica e lo
  dichiara.
- La stampa passa dal dialogo di sistema: margini e formato si impostano lì.
- I pittogrammi ARASAAC si cercano online e diventano copie locali dell'utente;
  i repertori di terzi non sono nel repository.

---

## Capitoli del video

| # | Capitolo | Contenuto |
|---|---|---|
| 01 | Che cos'è CAA Writer | accesso, offline, ambito d'uso |
| 02 | Aree di lavoro e profili | Creator e Utilizzatore, assegnazione materiali |
| 03 | Scrivere e simbolizzare | modalità, intensità, unità multiparola |
| 04 | Correggere e bloccare | alternative, ambiti, legame con la parola |
| 05 | Quando manca un simbolo | scala di fallback, ricerca, ARASAAC |
| 06 | Materiali | Studio, libreria, editor universale |
| 07 | Comunicatore e Utilizzatore | tre livelli, tocca per parlare |
| 08 | Voce e accessibilità | ascolto, pannello Controlli |
| 09 | Salvare, esportare, stampare | `.caadoc`, PDF, DOCX, PPTX, licenze |
| 10 | Automazione | `caa-cli`, casi d'uso in blocco |
