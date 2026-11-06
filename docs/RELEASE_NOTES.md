# CAA Writer 0.9.0 — note di rilascio

Un unico motore documentale e un unico editor per tutti i materiali.

## Novità principali

- **CAA Universal Document Engine**: documenti, presentazioni, tabelle,
  agende, storie, sequenze e sistemi comunicativi usano lo stesso modello,
  gli stessi comandi e lo stesso rendering.
- **Editor universale**: pagine flow/absolute/grid, miniature, trascinamento,
  ridimensionamento, rotazione, livelli, gruppi, proprietà, celle, azioni e
  collegamenti.
- **`.caadoc` v3**: documento universale incorporato, migrazione automatica dei
  file precedenti, risorse e licenze conservate.
- **CLI ufficiale**: `document`, `page`, `element`, `board`, `template`, con
  creazione, validazione, rendering, esportazione e `template build-all`.
- **Biblioteca rigenerata**: 300 modelli + 30 sistemi = 330 documenti nativi,
  644 pagine e 4.919 elementi, tutti validati e testati in modifica/salvataggio/
  riapertura; 0 overflow.

## Limiti

- DOCX, PPTX e PNG dal documento universale non sono ancora esportati; sono
  disponibili `.caadoc`, HTML interattivo, SVG e stampa/PDF di sistema.
- Intestazioni/piè di pagina, numerazione automatica e guide magnetiche non sono
  completi.
- Windows è compilato in modo incrociato ma non eseguito; Linux non compilato.
- L'editor universale modifica il testo nel pannello proprietà; la scrittura
  fluida continua a usare il Word Processor ProseMirror.

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
