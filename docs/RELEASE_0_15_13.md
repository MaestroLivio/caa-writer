# CAA Writer 0.15.13 — note di rilascio

## Novità 0.15.13 — il pittogramma sta sulla parola

### Editor CAA

- **Pittogramma ancorato alla parola**: ogni pittogramma è una decorazione inline sul range
  esatto della parola o dell'espressione. Non esiste più una striscia separata sopra il
  paragrafo né un livello posizionato per misura: il simbolo segue la parola anche quando il
  testo va a capo.
- **Selezione del testo affidabile**: clic, trascinamento con mouse o touch, doppio clic,
  Ctrl/Cmd+A, Shift+frecce, copia, taglia e incolla agiscono sul documento intero. Il cursore
  resta dentro le parole senza salti; il clic sul pittogramma apre il pannello senza rubare la
  selezione al testo.
- **Espressioni multiword**: «non vedo l'ora» e le altre espressioni riconosciute restano una
  sola unità con un solo pittogramma centrato. Le parole senza pittogramma restano nel flusso,
  con spaziatura e linea di base corrette.
- **Modalità «sotto»**: il pittogramma può stare sotto la parola senza sollevarla: l'unità usa
  una linea di base coerente con il resto del testo, verificata con misure nel browser.
- **Stampa**: in stampa restano i pittogrammi e spariscono il bordo del blocco e l'alone di
  selezione; le unità non si spezzano tra due pagine.

### Recupero e interfaccia

- **Nessun documento fantasma**: la bozza automatica viene eliminata al salvataggio,
  all'apertura di un documento, alla creazione di un nuovo documento e con «Ignora». Non
  vengono più create bozze vuote, quindi l'app non propone di recuperare un lavoro mai fatto.
- **Niente doppio dischetto**: il comando Salva non compare due volte nella stessa barra.
- **Pannello sovrapposto**: il pulsante è stato rimosso dall'intestazione del pannello; il
  comando è in **Ribbon → Visualizza → Pannelli → Pannello sovrapposto**.
- **Meno notifiche**: «Elemento aggiornato», «Stile aggiornato» e «Azione aggiornata» non
  interrompono più ogni modifica; gli errori restano visibili.
- **Zoom persistente**: lo slider in basso a destra scala il documento, non l'interfaccia;
  Ctrl/Cmd+rotella e pinch funzionano sul foglio e il valore viene ricordato.

## Verifiche eseguite

```text
npm run check                            0 errori, 26 warning preesistenti
npm run test:frontend                    58/58 passati (13 nuovi test sulle unità CAA)
npm run build                            completato
cargo test --lib                         228/228 passati
cargo test --test universal_documents    9/9 passati
cargo build --bin caa-cli                riuscito; caa-cli info risponde
python3 scripts/check_context.py         21/21 sezioni, Contesto.md fuori da Git
```

Verifica visiva in Chromium su una pagina temporanea (poi rimossa): modalità «sopra» e
«sotto», espressione multiword, parole senza simbolo e allineamento della linea di base
misurato con `getBoundingClientRect`.

## Limiti dichiarati

- Selezione, cursore, menu contestuale e stampa **non sono stati provati nell'app Tauri** in
  questa sessione: la verifica è nel browser e nei test unitari.
- Windows x64 e macOS Intel non sono stati eseguiti su hardware reale; macOS non è
  firmato/notarizzato.
- `cargo test --test semantic_corpus` non è stato eseguito: il catalogo ARASAAC locale non è
  presente.

## Licenza e attribuzione

CAA Writer è distribuito con **CC BY-NC-SA 4.0**. I pittogrammi ARASAAC usati dall'utente
restano soggetti alla licenza ARASAAC (CC BY-NC-SA 4.0) con attribuzione ad ARASAAC e a Sergio
Palao; non sono inclusi nel repository né negli installer.
