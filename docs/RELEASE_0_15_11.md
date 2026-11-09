# CAA Writer 0.15.11 — note di rilascio

## Novità 0.15.11 — Inserisci visivo, ispettore integrato, Stile CAA

- **Menu Inserisci visivo**: nella modalità documento la scheda **Inserisci** è una galleria di
  tessere CAA con icona in mattonella colorata ed etichetta. Contiene Testo, Casella, Pulsante,
  Forma, Fumetto, Immagine, Segnaposto e Griglia; restano i gruppi Pagine, Selezione e Risorse.
  I comandi del word processor non applicabili a una pagina impaginata (interruzione di pagina,
  data/ora, tabella di testo) non vengono più mostrati come pulsanti finti.
- **Proprietà nel pannello laterale**: selezionando un elemento si apre la scheda **Documento**
  del pannello, con testo, testo pronunciato, posizione, nudge da 1 mm, allineamento nel foglio,
  dimensioni, livello (dietro/in linea/davanti), sfondo, bordo, rotazione, trasparenza,
  visibilità, blocco, svincolo della cella, ricerca del pittogramma ed eliminazione. Per le celle
  di griglia si modificano testo e testo pronunciato della cella. La vecchia colonna di proprietà
  interna all'editor resta solo nell'editor universale autonomo.
- **Fumetti sulle pagine di testo**: la pagina di testo mostra ora anche gli elementi liberi
  (fumetti, immagini, forme) sopra il testo CAA. La sincronizzazione del testo non li cancella
  più: il fumetto resta visibile anche dopo la digitazione e la rianalisi.
- **Stile CAA**: il vecchio tema interno `araword` è diventato `caa`; l'interfaccia mostra
  «Stile CAA» nel selettore del Ribbon e nelle Impostazioni. Le installazioni precedenti vengono
  migrate automaticamente. Le tessere degli elementi usano colori distinti per tipo.
- **Miniatura del fumetto**: la miniatura della pagina disegna il fumetto con sfondo, bordo e
  testo, invece di mostrare solo un rettangolo generico.

## Verifiche eseguite

```text
npm run check                0 errori, 26 avvisi in 9 file
npm run build                0 errori (JS 536,20 kB, gzip 170,95 kB)
npm run test:frontend        40/40 passati (5 nuovi su flusso + elementi liberi)
cargo test --lib             227/227 passati
python3 scripts/check_context.py  conforme
```

Verifica visiva su macOS arm64 nella finestra Tauri debug con frontend Vite: scheda Inserisci
visiva, inserimento del fumetto sulla pagina di testo, fumetto visibile anche nella miniatura,
ispettore aperto nella scheda Documento e modifiche di testo, testo pronunciato e posizione
propagate all'elemento.

Il bundle macOS 0.15.11 è stato montato dal DMG e verificato in `Info.plist`
(`CFBundleShortVersionString = 0.15.11`). L'eseguibile Windows contiene il frontend aggiornato
(`index-CiJ6aCBl.js`, lo stesso hash generato da `npm run build`).

## Installer

- macOS arm64:
  - `CAA-Writer_0.15.11_macos-arm64.dmg` (814 MB)
  - `CAA-Writer_0.15.11_macos-arm64.app.zip` (814 MB)
  - `CAA-Writer_0.15.11_cli-macos-arm64` (6,8 MB)
- Windows x64:
  - `CAA-Writer_0.15.11_windows-x64-setup.exe` (816 MB, NSIS, 2.715 file, 1,14 GB decompressi)
  - `CAA-Writer_0.15.11_cli-windows-x64.exe` (7,7 MB, artefatto separato non incluso
    nell'installer)
  - Contiene il runtime llama.cpp Windows CPU, il modello `model.gguf`, il runtime Piper Windows
    con le voci Serena e Cori, il bootstrapper WebView2 e il runtime Visual C++.
- `SHA256SUMS.txt` con i checksum SHA-256 di tutti gli artefatti.

## Limiti dichiarati

- Windows x64 non è stato eseguito su hardware reale in questa sessione: la build, il contenuto
  dell'installer e l'eseguibile PE32+ sono verificati staticamente, ma non c'è una prova di
  avvio, stampa, WebView2 o voce su Windows.
- macOS non è firmato né notarizzato: il bundle è ad-hoc/linker-signed come nelle versioni
  precedenti.
- `cargo test --test semantic_corpus` non è stato eseguito in questa sessione; il catalogo
  ARASAAC locale è assente e il corpus non è una prova indipendente.
- La digitazione automatica nel WebView non è stata usata come verifica: gli eventi sintetici
  non hanno portato il focus a ProseMirror in modo deterministico. La conservazione del fumetto
  dopo la sincronizzazione è coperta dai test di `mergeTextElements`.

## Licenza e attribuzione

CAA Writer è distribuito con **CC BY-NC-SA 4.0**. I pittogrammi ARASAAC usati dall'utente restano
soggetti alla licenza ARASAAC (CC BY-NC-SA 4.0) con attribuzione ad ARASAAC e a Sergio Palao; non
sono inclusi nel repository né negli installer.
