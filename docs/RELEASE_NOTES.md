# CAA Writer 0.16.1 — aggiornamento dall'app e bundle macOS corretto

Data: 25 novembre 2026

## Il problema

Due difetti della 0.16.0, entrambi su macOS:

1. il programma vedeva la versione nuova nel repository ma, premendo «Aggiorna
   ora», il download si fermava su «Non esiste un pacchetto di aggiornamento per
   macos-aarch64». Gli asset della release erano stati rinominati con lo schema
   del bundler (`CAA.Writer_0.16.0_aarch64.dmg`), mentre il controllo
   aggiornamenti cerca i nomi stabili `CAA-Writer_<versione>_macos-arm64.dmg`;
2. scaricando il DMG a mano e copiando l'app in Applicazioni, macOS diceva
   **«CAA Writer è danneggiata e non può essere aperta»**: il bundle aveva solo
   la firma del linker, senza firma coerente dell'intero pacchetto.

## La soluzione

- La release pubblica di nuovo i nomi stabili degli asset (`macos-arm64.dmg`,
  `macos-x64.dmg`, `windows-x64-setup.exe`). Il controllo aggiornamenti della
  0.16.0 installata riconosce questi nomi: dalla 0.16.1 la fascia «Aggiorna
  ora» scarica davvero il pacchetto giusto e apre l'installatore.
- Il bundle macOS è firmato ad-hoc per intero durante la build. Senza
  certificato Developer ID la notarizzazione resta impossibile, ma la firma
  coerente elimina l'errore «danneggiata»: al primo avvio macOS chiede di
  confermare con «Apri comunque» (Impostazioni di Sistema → Privacy e
  sicurezza). Gli aggiornamenti scaricati dall'app non hanno l'attributo di
  quarantena e si aprono senza avvisi.
- La CI verifica la firma e fallisce se Gatekeeper risponde «code has no
  resources but signature indicates they must be present».
- Impostazioni → Informazioni: «Controlla aggiornamenti» con esito esplicito e
  il pulsante per scaricare la versione disponibile.

## Nota per chi ha la 0.16.0

Questa correzione va installata una volta a mano (DMG della 0.16.1): la 0.16.0
non contiene il codice corretto. Dalla 0.16.1 in poi l'aggiornamento si avvia
dall'app. Su Apple Silicon, al primo avvio della 0.16.1, confermare con «Apri
comunque».

---

# CAA Writer 0.16.0 — aggiornamenti della libreria per singolo pacchetto

Data: 23 novembre 2026

## Il problema

Il programma segnalava che nel repository era disponibile una versione più
recente della libreria, ma non esisteva alcun comando per applicarla: l'unica
strada era reinstallare l'app o copiare i file a mano. Inoltre il confronto
avveniva sulla libreria intera, quindi non si poteva sapere quale categoria
fosse cambiata.

## La soluzione

Ogni categoria della libreria è ora un **pacchetto con una versione propria**
(`aac`, `schede`, `agende`, `comportamentali`, `teacch`, `storie-sociali`,
`giochi`, `stampabili`, `storie-caa`). Per ciascuno l'app mostra tre versioni:

- **installata** nel tuo profilo;
- **inclusa** nell'app installata;
- **pubblicata** nel repository.

Quando una delle due sorgenti è più recente, il pacchetto compare come
«aggiornabile» con il pulsante per aggiornarlo. L'aggiornamento copia solo i
documenti di quel pacchetto nella cartella dati dell'utente, che ha la
precedenza sulla copia inclusa: gli altri materiali restano come sono e non si
duplicano i 150 MB dell'intera libreria.

Funziona anche **senza rete**: la sorgente «inclusa» legge i file dell'app. Con
la rete, «repository» scarica dal branch `main`; «auto» prova il repository e
ripiega sulla copia inclusa dichiarandolo.

Sostituire una versione già installata e diversa richiede una conferma esplicita:
non ci sono downgrade o sovrascritture silenziose.

## Interfaccia

- **Gestore pacchetti**: versione inclusa, installata e del repository accanto a
  ogni categoria, avviso sui pacchetti da aggiornare, «Aggiorna pacchetto» per
  ciascuno e «Aggiorna tutti». Lo stato viene annunciato ai lettori di schermo.
- **Scheda didattica** di ogni pacchetto: obiettivi dichiarati, pubblico,
  sottocategorie e conteggi reali, senza aprire i documenti.
- **Libreria**: il pulsante «Pacchetti» mostra quanti aggiornamenti ci sono e una
  fascia in cima permette di aggiornare subito o di scegliere il singolo
  pacchetto.
- **Accessibilità**: indicatori di focus visibili (3 px), bersagli da 44 px con
  puntatore grossolano, contrasto del testo secondario ≥ 4.5:1, rispetto di
  `prefers-contrast` e `prefers-reduced-motion`, barre di download con
  `role="progressbar"` e percentuale annunciata.

## CLI, MCP e Agent Skill

```bash
# stato per pacchetto (con rete: check_remote true)
caa-cli call library.packages.status '{"check_remote":false}'

# aggiorna un solo pacchetto dalla copia inclusa
caa-cli call library.update_package '{"id":"schede","source":"inclusa","confirm":true}'

# aggiorna tutti quelli indietro
caa-cli call library.packages.update_all '{"source":"auto"}'

# scheda didattica del pacchetto
caa-cli call library.package.outline '{"id":"teacch"}'
```

Da MCP gli stessi metodi sono `library_packages_status`, `library_update_package`,
`library_packages_update_all` e `library_package_outline`. La skill è aggiornata
con la nuova sezione, la ricetta in `references/workflows.md` e la regola di
conferma in `references/security.md`.

## Verifiche

- `cargo test --lib`: 318 test superati, 0 falliti.
- `npm run build`: controllo tipi e build superati.
- Prova reale su data-dir isolata: stato iniziale con 0 pacchetti da aggiornare,
  aggiornamento di `storie-caa` (10 documenti, 3,5 MB), versione installata
  abbassata a mano a `2026.09` → `aggiornabile`; rifiuto senza `confirm`,
  successo con `confirm: true`; ricerca e scheda didattica rileggono il
  pacchetto aggiornato.
- Non verificato: download dal repository con rete reale e resa grafica delle
  nuove schermate in un browser.
