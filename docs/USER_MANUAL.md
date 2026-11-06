# CAA Writer — Manuale utente

## 1. Che cosa fa

CAA Writer è un videoscrittore in italiano che, mentre scrivi, propone **pittogrammi CAA** per le
parole e le espressioni del testo. Puoi sempre:

- vedere il significato scelto dal sistema;
- sostituire il pittogramma con un'alternativa;
- bloccare la scelta solo per quel punto del documento, per quel contesto o per sempre;
- eliminare il pittogramma quando nessuna immagine è adeguata;
- importare un SVG nuovo e associarlo a una parola.

L'applicazione funziona **senza Internet**. L'analisi semantica avviene sul computer; nessun testo
viene inviato a servizi esterni.

## 2. Primo avvio

1. Apri CAA Writer. Il documento nuovo è già pronto per la scrittura.
2. Scrivi una frase, per esempio: `Il bambino mangia una mela.`
3. Dopo una breve pausa compaiono i pittogrammi sopra le parole.
4. Fai clic su un pittogramma per aprire il pannello **Alternative**.

Al primo avvio l'applicazione crea il database locale e importa il repertorio incluso (88
pittogrammi originali). Se qualcosa non riesce, l'applicazione lo segnala e resta utilizzabile in
modalità solo testo.

## 3. Le tre aree dell'interfaccia

- **Barra superiore**: nuovo, apri, salva, importa; formattazione (grassetto, corsivo, sottolineato,
  titoli, allineamento, elenchi, tabelle, colore e dimensione); modalità CAA e intensità;
  esportazione PDF/DOCX/HTML; registro delle licenze; ascolto del testo.
- **Area centrale**: il documento. È l'unica area che viene stampata o esportata come contenuto.
- **Pannello laterale destro**: alternative, ricerca simboli, impostazioni, Visual Resource Studio.
  Si chiude con il pulsante `»` per avere più spazio per la scrittura.

## 4. Modalità di rappresentazione

| Modalità | Che cosa vedi |
|---|---|
| A · Testo | solo testo, nessun pittogramma |
| B · Simboli sopra | pittogramma sopra ogni unità comunicativa |
| C · Didascalia sotto | pittogramma con la parola sotto |
| D · Solo simboli | pittogrammi grandi, testo attenuato ma presente |
| E · CAA semplificata | solo i concetti fondamentali |

L'**intensità** decide quante parole ricevono un pittogramma:

- **Minima**: parole chiave (nomi, verbi pieni, aggettivi importanti).
- **Media**: tutte le unità semantiche utili.
- **Completa**: copertura massima, senza simboli fuorvianti.

Cambiare modalità o intensità non modifica il testo: puoi tornare indietro in qualsiasi momento.

## 5. Scegliere e bloccare un pittogramma

1. Fai clic sul pittogramma nel documento.
2. Nel pannello vedi il simbolo attuale, il significato, la strategia usata e le alternative.
3. Fai clic su un'alternativa: la sostituzione è immediata e vale **solo per quella unità**.

Per rendere la scelta stabile:

- **Usa sempre questo pittogramma per «parola»**: preferenza globale.
- **Usa in questo contesto**: preferenza legata a un contesto (scuola, casa, cibo…).
- **Ripristina scelta automatica**: rimuove il blocco dell'unità.

Le scelte manuali non vengono mai sovrascritte dalle scelte automatiche, nemmeno quando modifichi la
frase.

## 6. Quando manca un pittogramma

L'applicazione segue questa scala e **te lo dice**:

1. **Sinonimo equivalente** (automobile → macchina).
2. **Forma base** (camminavano → camminare).
3. **Concetto più generale** (bassotto → cane): la parola scritta resta «bassotto», il pittogramma è
   quello del cane ed è segnalato come approssimazione.
4. **Composizione** (monopattino elettrico → monopattino + elettricità).
5. **Scomposizione concettuale** per termini complessi.
6. **Concetto vicino**, sempre marcato come non esatto.
7. **Nessun simbolo**: la parola resta senza pittogramma.

Il pittogramma approssimativo ha il bordo tratteggiato e la didascalia con `≈`. Se nessuna
rappresentazione è corretta, è meglio nessun simbolo che un simbolo sbagliato.

## 7. Aggiungere un pittogramma

1. Seleziona l'unità nel documento.
2. Nel pannello, sezione **Importa un pittogramma SVG**, scegli il file.
3. L'SVG viene validato (niente script, niente riferimenti esterni, `viewBox` obbligatorio) e
   associato alla parola; diventa subito disponibile offline.

I simboli importati conservano l'indicazione di licenza e autore nel registro delle licenze.
Importa solo materiale che hai il diritto di usare.

## 8. Salvare, riaprire, esportare

- **Salva** produce un file `.caadoc`: contiene testo, formattazione, pittogrammi, scelte manuali,
  metadati e gli SVG usati. È un archivio ZIP con manifest versionato e scrittura atomica (un
  salvataggio interrotto non corrompe il file precedente).
- **Apri** ripristina il documento completo, anche senza connessione.
- **Importa** accetta TXT, DOCX, HTML e Markdown: il testo viene conservato, la formattazione
  complessa no. L'applicazione te lo segnala.
- **DOCX** esporta testo e pittogrammi rasterizzati in PNG.
- **HTML** esporta un file autonomo con gli SVG incorporati.
- **PDF** usa la stampa di sistema: scegli «Salva come PDF» nel dialogo. È il modo previsto per
  ottenere un PDF con proporzioni corrette.
- **Licenze** esporta il registro delle attribuzioni dei simboli usati.

Ogni 20 secondi, se ci sono modifiche non salvate, viene scritta una bozza di recupero. All'avvio,
se esiste, l'applicazione propone di recuperarla.

## 9. Visual Resource Studio

Nel pannello, scheda **Studio**:

1. Descrivi la risorsa in italiano, per esempio: «Crea un'agenda visiva settimanale per la scuola
   primaria con entrata, intervallo, pranzo, gioco».
2. Scegli un modello oppure lascia «Modello automatico».
3. Premi **Genera**: ottieni un'anteprima con pittogrammi ed etichette modificabili.
4. **Stampa / PDF** produce il foglio impaginato.

La generazione interpreta la richiesta con regole deterministiche e risolve i pittogrammi nel
repertorio locale. Non produce HTML o CSS tramite il modello linguistico. Se la richiesta è troppo
complessa, l'applicazione propone un modello e ti lascia completarlo.

I materiali ABA e TEACCH sono strumenti di supporto alla progettazione: non automatizzano valutazioni
cliniche e non devono essere usati per pratiche punitive o coercitive.

## 10. Ascoltare il testo

Il pulsante **Ascolta** legge il paragrafo selezionato o l'intero documento con la voce di sistema
(macOS `say`, Windows SAPI). La sintesi legge il **testo originale**, non la descrizione del
pittogramma.

## 11. Accessibilità

- Navigazione completa da tastiera; focus sempre visibile.
- Temi chiaro, scuro e alto contrasto (Impostazioni).
- **Modalità tocco**: bersagli più grandi, adatta a smartboard e tablet Windows.
- Dimensione del testo regolabile; rispetto di «riduci animazioni» del sistema.
- Etichette `aria-label` su pulsanti e pittogrammi; il pannello è annunciato come regione.

## 12. Computer con poca memoria

- L'analisi è incrementale: solo il paragrafo modificato viene rianalizzato.
- I pittogrammi sono caricati su richiesta e memorizzati in una cache limitata.
- Il modello linguistico (se configurato) resta spento finché non lo attivi.
- Se il modello non parte, l'applicazione continua a funzionare con la sola analisi lessicale.

## 13. Problemi frequenti

| Sintomo | Cosa fare |
|---|---|
| Nessun pittogramma | Controlla la modalità (non deve essere «Solo testo») e l'intensità; verifica che il database sia stato creato al primo avvio. |
| molta parola senza simbolo | Il repertorio incluso è volutamente essenziale: importa SVG o crea un pacchetto con `scripts/import_symbols.py`. |
| PDF diverso dall'anteprima | Il PDF usa il motore di stampa del sistema: imposta margini e formato nel dialogo di stampa. |
| L'applicazione non parte su Windows | Verifica WebView2; l'installer tenta di scaricarlo se assente. |
| L'IA non risponde | In Impostazioni indica il percorso di `llama-cli` e del modello GGUF; senza modello l'app funziona comunque. |

## Prestazioni e diagnostica (0.7.0)

In **Impostazioni → Prestazioni** puoi scegliere:

- **Risparmio risorse**: pensato per computer con 4 GB di RAM; meno thread e
  analisi meno frequente.
- **Bilanciato**: impostazione predefinita.
- **Prestazioni elevate**: più thread e contesto, se la macchina lo consente.
- **Personalizzato**: regoli tu le attese dell'analisi e della rifinitura IA.

La digitazione non viene mai bloccata: il testo compare subito e i pittogrammi
si aggiornano quando l'analisi è pronta. Se continui a scrivere, i risultati
vecchi vengono scartati.

In **Impostazioni → Diagnostica misurata** trovi valori reali, non stimati:
latenza di input, tempo di aggiornamento dell'editor, blocchi dell'interfaccia,
memoria e CPU dell'app e del modello, coda delle inferenze e tempi di
simbolizzazione. Su Windows la CPU del processo è riportata come secondi
cumulati, perché il sistema non espone la stessa percentuale istantanea di
macOS.

Quando la voce legge, compare un indicatore con **Ferma**. La lettura non
blocca l'editor e il motore non avvia un'altra inferenza pesante nello stesso
momento.

Per correggere un pittogramma puoi scegliere l'ambito: solo questa occorrenza,
tutte le occorrenze equivalenti nel documento, solo il profilo Utilizzatore
attivo, tutti i documenti futuri oppure il vocabolario condiviso del Creator.
Le associazioni bloccate non vengono mai sovrascritte.


## Comunicatore e foglio libero (0.8.0)

Nel comunicatore aperto da un materiale dello Studio premi **Modifica** per cambiare
l'etichetta, il testo pronunciato, il pittogramma e l'azione di ogni cella. Puoi aggiungere
celle e pagine, collegare una cella a un'altra pagina e salvare il comunicatore in
`.caadoc`; riaprendo il file torni direttamente al comunicatore.

Azioni disponibili: aggiungi alla frase, pronuncia la cella, vai alla pagina, indietro,
home, svuota la frase. Sono le uniche azioni ammesse: il file non può eseguire codice,
URL o comandi di sistema.

Nel foglio libero il pulsante **Salva .caadoc** conserva titolo, colonne, celle e
pittogrammi. Riaprendo il file ritrovi lo stesso foglio, modificabile e stampabile.

## Editor Utilizzatore (0.8.0)

La voce **Scrivo** della home Utilizzatore apre un editor diverso per ogni livello del
profilo:

- **Livello 1 — Solo simboli**: un grande vocabolario di base; tocchi i pittogrammi e
  costruisci la frase, poi la ascolti.
- **Livello 2 — Scrivo con aiuto**: editor CAA con tastiera visuale, parole frequenti e
  lettura della frase.
- **Livello 3 — Scrivo**: come il livello 2, con grassetto, corsivo, sottolineato e
  annulla.

L'editor Utilizzatore non mostra il Ribbon, lo Studio o i comandi del Creator. Il documento
si salva in `.caadoc` e rispetta le funzionalità del profilo (per esempio la voce).
