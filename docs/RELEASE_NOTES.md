# CAA Writer 0.10.0 — note di rilascio

Griglie reali, PDF multipagina, player con associazione file e audit tecnico.

## Novità principali

- **Griglia 4×5 = 20 celle reali**: dialogo righe/colonne, aggiunta/rimozione
  righe e colonne, celle modificabili, svuotamento e riordino.
- **PDF multipagina** generato dal renderer canonico, con pittogrammi, testi,
  bordi, colori e formato pagina; **PNG** della prima pagina; **Stampa** con
  tutte le pagine tramite dialogo di sistema.
- **Player** per `.caadoc` e `.caaplay`, apertura da doppio clic/associazione,
  menu Apri/Importa/Modifica/Stampa/Schermo intero/Home/Esci.
- **Importazione** come copia modificabile nella biblioteca dell'utente.
- **Audit** in `docs/AUDIT_0_10.md`; rimosso il simbolo personale di prova dal
  database locale, senza toccare ARASAAC/core.

## Limiti

- Unione/divisione celle e dimensioni indipendenti per cella non complete.
- DOCX/PPTX non esportati; PNG solo prima pagina.
- Testo del documento universale modificato dal pannello proprietà.
- Stampa nativa e doppio clic non verificati su hardware reale in questa sessione.
- Windows compilato in modo incrociato ma non eseguito; Linux non compilato.

## Requisiti

- Windows 10/11 a 64 bit con WebView2 (presente di norma su Windows 11 e
  installabile su Windows 10).
- macOS 10.15 o successivo, Apple Silicon o Intel.
- Linux x64 con WebKitGTK 4.1 e le librerie indicate nel pacchetto `.deb`.
- 4 GB di RAM per l'uso senza modello neurale; il modello incluso richiede
  circa 1,5 GB in più. Nessuna GPU dedicata è necessaria.

## Installazione

- **Windows**: usare `CAA-Writer_0.8.0_windows-x64-setup.exe` (NSIS). In questa
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
