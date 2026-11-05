## Requisiti

- Windows 10/11 a 64 bit con WebView2 (presente di norma su Windows 11 e
  installabile su Windows 10).
- macOS 10.15 o successivo, Apple Silicon o Intel.
- Linux x64 con WebKitGTK 4.1 e le librerie indicate nel pacchetto `.deb`.
- 4 GB di RAM per l'uso senza modello neurale; il modello incluso richiede
  circa 1,5 GB in più. Nessuna GPU dedicata è necessaria.

## Installazione

- **Windows**: usare `CAA-Writer_*_x64-setup.exe` (NSIS) oppure il file `.msi`.
  Se SmartScreen avvisa, il pacchetto non è firmato: verificare il checksum
  SHA-256 pubblicato nella release e scegliere «Ulteriori informazioni» →
  «Esegui comunque».
- **macOS**: aprire il `.dmg`, trascinare `CAA Writer.app` in Applicazioni.
  Al primo avvio macOS può chiedere di confermare l'apertura (l'app non è
  notarizzata). In alternativa usare il file `.app.zip`.
- **Linux**: installare il `.deb` con `sudo apt install ./CAA-Writer_*.deb`
  oppure usare l'AppImage (rendere eseguibile e avviare). Il pacchetto `.rpm`
  è per le distribuzioni che usano RPM.

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
- La ricerca online di immagini non è inclusa; l'app funziona offline.
- Il runtime vocale Piper non è incluso nel bundle: si usa la voce di sistema.
- La copertura semantica dipende dal lessico incluso e dal catalogo ARASAAC.
- I test su Windows con 4 GB di RAM, su macOS Intel e con utenti CAA non sono
  stati eseguiti da questo ambiente.
