# Tutorial video per docenti — pipeline di produzione

Copione, riprese, narrazione, clip e montaggio Concat del tutorial per docenti.

## Contenuto

| Percorso | Cosa contiene |
|---|---|
| `script.json` | copione dei 10 capitoli: titolo, sottotitolo, punti chiave, visuale e narrazione |
| `audio/chNN.wav` | narrazione di ogni capitolo (voce **Italian Explainer**, lingua italiana) |
| `takes/chNN.mp4` | ripresa Recordly del capitolo, 2880 px, 60 fps, **senza audio** |
| `clips/chNN.mp4` | clip 1920×1080, 30 fps, **muta**, alla durata della voce, con motion graphics |
| `captures/`, `assets/` | fermo immagine e materiali esportati dall'app, usati nelle prime versioni |
| `tmp/` | livelli PNG intermedi (etichette di capitolo, screenshot di controllo) |
| `takes/` esclusa da Git | contiene video pesanti, prodotti localmente |

## Comandi

```bash
# 1. narrazione (VoiceStudio su http://localhost:3900, voce Italian Explainer)
python3 tutorial/make_audio.py

# 2. riprese: una presa Recordly per capitolo (Recordly con porta CDP 9222)
python3 tutorial/take.py 03          # prepara lo stato, registra, archivia in takes/
python3 tutorial/take.py 03 --prepare  # solo preparazione, per controllare la scena

# 3. clip con motion graphics (Pillow: usare .image-venv)
.image-venv/bin/python tutorial/build_clips.py

# 4. montaggio: 10 riprese su una traccia video + 10 voci su una traccia audio
python3 tutorial/build_concat.py
```

## Come funziona la ripresa

`take.py` fa tre cose per ogni capitolo:

1. **prepara lo stato** con clic e tasti (`uia.py`): avvia l'app, apre l'Area
   Creator, la Libreria dei materiali, il materiale da mostrare, e chiude il
   banner della bozza recuperata;
2. **registra** con Recordly: riavvia Recordly con `--remote-debugging-port=9222`,
   pilota la HUD via CDP (`cdp.js`), preme Registra, esegue la sequenza del
   capitolo e ferma la presa;
3. **archivia** il file grezzo da `~/Library/Application Support/Recordly/recordings/`
   in `takes/chNN.mp4` e chiude l'editor.

La HUD e l'editor di Recordly sono contenuto web: non sono raggiungibili con
l'accessibilità di macOS, quindi si pilotano via CDP. Il mouse e la tastiera
verso l'app usano `CGEvent` (`uia.py`) e `System Events`.

## Scelte di montaggio

- **Video e audio separati per costruzione**: le riprese sono mute, la voce sta
  in WAV distinti; in Concat entrano su due tracce diverse, così ogni capitolo
  resta ritoccabile in modo indipendente.
- **Motion graphics generate, non simulate**: etichetta di capitolo, titolo e
  sottotitolo entrano in basso a sinistra con dissolvenza, filo colorato in
  alto e barra di avanzamento in basso; la ripresa è ritagliata in 16:9 e
  portata alla durata esatta della voce.
- **Durata**: se la ripresa è più corta della voce viene rallentata fino al 35%,
  oltre quella soglia resta ferma sull'ultimo fotogramma; se è più lunga viene
  tagliata.

## Vincoli verificati sul campo

- **Recordly va riavviato a ogni presa**: quando si apre l'editor la HUD
  sparisce, e un elenco finestre vecchio può associare il nome «CAA Writer» a
  un'altra finestra.
- **Finestra contro schermo**: la ripresa per finestra è immune ai cambi di
  primo piano ma dipende dall'elenco di Recordly; se l'elenco è ambiguo si
  registra lo schermo intero e si ritaglia in montaggio (`build_clips.py`
  riconosce le due altezze, 1686 e 1864).
- **I tasti vanno all'app in primo piano**: dopo l'avvio della registrazione
  `take.py` riporta davanti CAA Writer (o Terminale per il capitolo 10),
  altrimenti la digitazione finisce all'app sbagliata.
- **Dock**: i pulsanti in basso a sinistra (Controlli, zoom) stanno sotto il
  Dock quando la finestra non è in primo piano.

## Stato delle riprese

Tutte e dieci le prese esistono e sono state controllate a campione. Il
capitolo 7 mostra la vista Utilizzatore con i materiali di Luca ma non il
Comunicatore; il capitolo 10 riprende il Terminale dentro lo schermo intero.
