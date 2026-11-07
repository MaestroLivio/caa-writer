# CAA Writer 0.14.0 — note di rilascio

Profili con materiali assegnati, stampa nativa, comunicatore tocco e layout visuale.

## Novità

- **Profili e utenti**: ogni profilo vede solo i materiali assegnati; gestione
  utenti con ricerca tra i 330 documenti, PIN Creator e cambio profilo.
- **Ritorno alla modalità Utilizzatore** dall’Area Creator; il documento aperto
  viene chiuso e si torna alla schermata del profilo.
- **Stampa nativa** (`WebviewWindow::print`) per documento, generatore e foglio
  libero. Il vecchio `window.print()` non è più il percorso principale.
- **Comunicatore**: «Componi frase» e «Tocca per parlare», con pronuncia
  immediata in modalità tocco.
- **Didascalie automatiche** quando il bordo include il testo; rimossa
  l’opzione separata «Didascalie visibili».
- **Alto contrasto reversibile** e selettore tema in Visualizza; **schermo
  intero** con i permessi Tauri necessari.
- **Editor visuale**: miniature richiudibili, righello con margini
  trascinabili, guide magnetiche, celle svincolabili, ancoraggio
  davanti/dietro/in linea e ripaginazione automatica dei documenti a flusso.
- **Righello** anche nel word processor da Layout → Pagina.

## Limiti

- La stampa è collegata al dialogo nativo; l’apertura reale del dialogo non è
  stata automatizzata in questa sessione.
- I file `.caadoc` arbitrari fuori dalla biblioteca non hanno ancora un elenco
  dedicato per l’assegnazione ai profili.
- Windows è compilato e ispezionato staticamente ma non eseguito; Linux non è
  compilato.
- La resa visiva in Word/PowerPoint non è stata verificata su questa macchina.

## Requisiti

- Windows 10/11 a 64 bit con WebView2 (presente di norma su Windows 11 e
  installabile su Windows 10).
- macOS 10.15 o successivo, Apple Silicon o Intel.
- Linux x64 con WebKitGTK 4.1 e le librerie indicate nel pacchetto `.deb`.
- 4 GB di RAM per l'uso senza modello neurale; il modello incluso richiede
  circa 1,5 GB in più. Nessuna GPU dedicata è necessaria.

## Installazione

- **Windows**: usare `CAA-Writer_0.14.0_windows-x64-setup.exe` (NSIS). In questa
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
