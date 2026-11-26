# CAA Writer 0.16.2 — Windows: niente più finestre «piper.exe — Errore di sistema»

Data: 26 novembre 2026

## Il problema

Su Windows, a ogni frase da pronunciare, compariva una finestra di sistema da
chiudere a mano:

```text
piper.exe - Errore di sistema
L'esecuzione del codice non può proseguire perché piper_phonemize.dll
non è stato trovato. Per risolvere il problema, prova a reinstallare il
programma.
```

In altre prove la DLL indicata era `espeak-ng.dll`. La finestra bloccava la
lettura: dopo l'«OK» l'app ricadeva sulla voce di sistema, ma il messaggio
tornava alla frase successiva.

## La causa

Non era il download. `piper_windows_amd64.zip` veniva scaricato ed estratto
correttamente in `<dati>/voice/piper/piper/`, con `piper.exe`,
`piper_phonemize.dll`, `espeak-ng.dll`, `onnxruntime.dll`, `espeak-ng-data` e
`libtashkeel_model.ort` nella stessa cartella.

Era il passaggio successivo, `link_engine`, a copiare **solo `piper.exe`** in
`<dati>/voice/bin/`, la posizione cercata per prima da `voice::resolve`. Il
loader di Windows cerca le DLL nella cartella dell'eseguibile: in `bin/` non
c'erano, quindi il processo non partiva e Windows apriva il dialogo.

## La correzione

- **Windows: il motore si esegue dove è stato estratto.** `link_engine` non
  copia più l'eseguibile da solo; usa `<dati>/voice/piper/piper/piper.exe`, con
  DLL ed `espeak-ng-data` accanto, e rimuove la copia incompleta lasciata dalle
  versioni precedenti.
- **Le installazioni già rotte si riparano da sole.**
  `voice::runtime_is_complete` considera utilizzabile un motore solo se accanto
  ha `piper_phonemize.dll`, `espeak-ng.dll`, `onnxruntime.dll` ed
  `espeak-ng-data`. Un `piper.exe` isolato viene scartato e `resolve` passa al
  runtime integro: nessun nuovo download necessario.
- **Niente finestre di sistema, in ogni caso.**
  `SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX)` è impostato
  all'avvio dell'app e prima di ogni processo figlio: se una DLL manca davvero,
  l'errore resta un errore di avvio, `speak` ricade sulla voce di sistema e
  l'utente non deve chiudere nulla.
- **Download onesto.** Dopo l'estrazione il pacchetto `voice` verifica il
  runtime e fallisce con un messaggio esplicito invece di dichiarare installato
  un motore che non può partire.

## Verifiche

- `cargo test --lib --features onnx`: 321/321.
- `cargo test --all-targets --features onnx`: esito riportato nel commit.
- `npm run build`, `npm run check` (0 errori), `npm run test:frontend` 75/75.
- Il comportamento su Windows è coperto da test dedicati
  (`runtime_is_complete`, `resolve` che scarta l'eseguibile isolato,
  `link_engine` che non lascia una copia sola, maschera di `SetErrorMode`);
  l'esecuzione reale avviene nella CI Windows del tag `v0.16.2`.

Limite dichiarato: non ho un computer Windows per riprodurre il caso a mano;
la verifica è nei test Windows della CI e nell'installer prodotto dal workflow.
