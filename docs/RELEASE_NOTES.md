# CAA Writer 0.13.0 — note di rilascio

DOCX/PPTX modificabili elemento per elemento e sintesi vocale silenziosa durante la scrittura.

## Novità

- **DOCX**: testo, pittogrammi e griglie diventano elementi Word nativi
  modificabili singolarmente; le griglie a flusso sono tabelle reali.
- **PPTX**: ogni elemento e ogni cella sono forme indipendenti, con colori,
  sfondi, bordi e rotazione; il tema è conforme alla matrice di stili.
- Coordinate DOCX corrette in twip; niente più offset in EMU che spostavano le
  caselle fuori pagina.
- La lettura automatica durante la digitazione non mostra più avvisi: i
  messaggi restano solo per i pulsanti espliciti.
- Restano le funzioni della 0.12: repertori via API, default prudenti, griglie
  complete, testo inline, provider IA con chiave nel portachiavi.

## Limiti

- Azioni e collegamenti del player non sono rappresentati in Word/PowerPoint;
  `.caadoc` resta il formato completo.
- La resa visiva in Word/PowerPoint non è stata verificata su questa macchina
  (applicazioni non installate); DOCX/PPTX sono stati aperti con python-docx,
  python-pptx, `textutil`, `unzip -t` e `file`.
- Stampa/doppio clic non verificati su hardware reale; Windows non eseguito;
  Linux non compilato.

## Requisiti

- Windows 10/11 a 64 bit con WebView2 (presente di norma su Windows 11 e
  installabile su Windows 10).
- macOS 10.15 o successivo, Apple Silicon o Intel.
- Linux x64 con WebKitGTK 4.1 e le librerie indicate nel pacchetto `.deb`.
- 4 GB di RAM per l'uso senza modello neurale; il modello incluso richiede
  circa 1,5 GB in più. Nessuna GPU dedicata è necessaria.

## Installazione

- **Windows**: usare `CAA-Writer_0.13.0_windows-x64-setup.exe` (NSIS). In questa
  release non è prodotto il `.msi`. Se SmartScreen avvisa, il pacchetto non è
  firmato: verificare il checksum SHA-256 e scegliere «Ulteriori informazioni»
  → «Esegui comunque».
- **macOS**: aprire il `.dmg`, trascinare `CAA Writer.app` in Applicazioni.
  Al primo avvio macOS può chiedere di confermare l'apertura (l'app non è
  notarizzata). In alternativa usare il file `.app.zip`.
- **Linux**: nessun pacchetto è stato prodotto in questa sessione; il workflow
  è pronto ma non eseguito.

## Verifica dei file

Ogni artefatto ha un file `SHA256SUMS-*.txt`. Esempio:

```bash
sha256sum -c SHA256SUMS-windows-x64.txt   # Linux
shasum -a 256 -c SHA256SUMS-macos-arm64.txt  # macOS
```

Su Windows è possibile usare `certutil -hashfile <file> SHA256` e confrontare
il valore.

## Limitazioni note

- I pacchetti **non sono firmati né notarizzati**: mancano certificati e
  credenziali. Non viene simulata una firma.
- Windows è compilato in modo incrociato e ispezionato staticamente, ma **non
  eseguito** su una macchina Windows in questa sessione.
- La misura della latenza di battitura p95 è strumentata nel pannello
  Diagnostica, non ancora misurata da un test automatico del WebView.
- La ricerca online è opzionale e disattivata per default; l'app funziona
  offline.
- Il runtime vocale Piper dipende dal sistema/venv; resta il fallback dichiarato
  alla voce di sistema.
- La copertura semantica dipende dal lessico incluso e dal catalogo ARASAAC.
- I test su Windows con 4 GB di RAM, su macOS Intel e con utenti CAA non sono
  stati eseguiti da questo ambiente.
