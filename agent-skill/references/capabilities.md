# CAA Writer — capacità (API 1.0, versione 0.15.24)

## `app.capabilities` — Catalogo delle capacità

Restituisce il catalogo generato dal registro reale: operazioni, parametri, schemi, versione API, modalità di accesso e limitazioni. Usalo prima di pianificare un workflow.

- Categoria: applicazione
- Tool MCP: `app_get_capabilities`
- Accesso: lettura
- Parametri: `format`, `category`
- Esempio: `caa-cli capabilities`

## `app.status` — Stato dell'applicazione

Stato reale di motore, percorsi, database, modello neurale, libreria e bridge della GUI. Non avvia processi pesanti.

- Categoria: applicazione
- Tool MCP: `app_get_status`
- Accesso: lettura
- Esempio: `caa-cli call app.status '{}'`

## `app.diagnostics` — Diagnostica

CPU/RAM del processo, stato del modello, metriche della coda neurale e della simbolizzazione.

- Categoria: applicazione
- Tool MCP: `app_get_diagnostics`
- Accesso: lettura

## `info` — Informazioni sul motore

Versione, percorsi, statistiche del database, conteggi del lessico e avvisi di avvio.

- Categoria: applicazione
- Tool MCP: `engine_get_info`
- Accesso: lettura

## `methods` — Elenco metodi

Alias CLI di `app.capabilities`: elenco dei nomi dei metodi disponibili.

- Categoria: applicazione
- Accesso: lettura

## `settings.list` — Elenca impostazioni

Elenca le chiavi e i valori delle impostazioni persistite nel database. Le chiavi sconosciute sono incluse: non vengono inventate categorie.

- Categoria: impostazioni
- Tool MCP: `settings_list`
- Accesso: lettura
- Parametri: `prefix`

## `settings.get` — Leggi impostazione

Legge una singola impostazione persistita.

- Categoria: impostazioni
- Tool MCP: `settings_get`
- Accesso: lettura
- Parametri: `key` (obbligatorio)

## `settings.set` — Scrivi impostazione

Scrive una singola impostazione. Con dry_run=true non modifica nulla. Le preferenze manuali dell'utente non vengono toccate.

- Categoria: impostazioni
- Tool MCP: `settings_update`
- Accesso: scrittura
- Parametri: `key` (obbligatorio), `value` (obbligatorio)

## `project.list` — Elenca progetti

Elenca i file .caadoc nella cartella progetti (predefinita: <data-dir>/projects). Non attraversa collegamenti simbolici e non esce dalla radice.

- Categoria: progetti
- Tool MCP: `project_list`
- Accesso: lettura
- Parametri: `root`, `limit`

## `project.create` — Crea progetto

Crea un documento universale vuoto e lo salva nella radice progetti. `path` è relativo alla radice; non sono ammessi percorsi assoluti o `..`.

- Categoria: progetti
- Tool MCP: `project_create`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `title` (obbligatorio), `kind`, `layout`
- Esempio: `caa-cli project create --path demo.caadoc --title Demo`

## `project.open` — Apri progetto

Legge un progetto .caadoc dalla radice progetti e ne restituisce il documento completo.

- Categoria: progetti
- Tool MCP: `project_open`
- Accesso: lettura
- Parametri: `path` (obbligatorio)

## `project.validate` — Valida progetto

Valida schema, pagine, elementi, azioni, collegamenti e risorse del progetto.

- Categoria: progetti
- Tool MCP: `project_validate`
- Accesso: lettura
- Parametri: `path` (obbligatorio)

## `project.save` — Salva progetto

Salva un documento nella radice progetti. Sovrascrive solo se `overwrite=true`; altrimenti rifiuta il conflitto. Non idempotente se il titolo cambia (il nome file può cambiare).

- Categoria: progetti
- Tool MCP: `project_save`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `document` (obbligatorio), `overwrite`

## `project.duplicate` — Duplica progetto

Copia un progetto in un nuovo percorso della radice progetti; non sovrascrive mai un file esistente.

- Categoria: progetti
- Tool MCP: `project_duplicate`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `target` (obbligatorio)

## `project.rename` — Rinomina progetto

Sposta/rinomina un progetto. Rifiuta se la destinazione esiste.

- Categoria: progetti
- Tool MCP: `project_rename`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `target` (obbligatorio)

## `project.delete` — Elimina progetto

Elimina definitivamente un progetto. Richiede confirm=true; senza conferma non tocca nulla.

- Categoria: progetti
- Tool MCP: `project_delete`
- Accesso: scrittura, distruttiva (confirm=true, admin)
- Parametri: `path` (obbligatorio), `confirm`

## `project.export` — Esporta progetto

Esporta un progetto in pdf, png, html, svg, docx o pptx. La destinazione deve restare nella radice progetti.

- Categoria: progetti
- Tool MCP: `project_export`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `format`, `out`

## `document.create` — Crea documento

Crea un UniversalDocument vuoto; con `path` lo salva. Accetta una specifica completa con `spec`.

- Categoria: documenti
- Tool MCP: `document_create`
- Accesso: scrittura
- Parametri: `title`, `kind`, `layout`, `spec`, `path`

## `document.get` — Leggi documento

Legge un .caadoc e restituisce il documento universale; con `summary_only=true` restituisce solo il riepilogo.

- Categoria: documenti
- Tool MCP: `document_get`
- Accesso: lettura
- Parametri: `path` (obbligatorio), `summary_only`

## `document.update` — Modifica documento

Applica una transazione di DocumentCommand a un .caadoc e salva su `out` (default: stesso file).

- Categoria: documenti
- Tool MCP: `document_update`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `commands` (obbligatorio), `out`

## `document.export` — Esporta documento

Esporta un documento universale nei formati supportati dal renderer condiviso.

- Categoria: documenti
- Tool MCP: `document_export`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `format`, `out`

## `element.add` — Aggiungi elemento

Aggiunge un elemento a una pagina con un ID stabile.

- Categoria: contenuti
- Tool MCP: `element_create`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `page` (obbligatorio), `element` (obbligatorio), `index`, `out`

## `element.update` — Modifica elemento

Applica una patch JSON a un elemento identificato da pagina e ID.

- Categoria: contenuti
- Tool MCP: `element_update`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `page` (obbligatorio), `id` (obbligatorio), `patch` (obbligatorio), `out`

## `element.delete` — Elimina elemento

Rimuove un elemento da una pagina.

- Categoria: contenuti
- Tool MCP: `element_delete`
- Accesso: scrittura, distruttiva (confirm=true, admin)
- Parametri: `path` (obbligatorio), `page` (obbligatorio), `id` (obbligatorio), `confirm`, `out`

## `element.move` — Sposta elemento

Sposta un elemento in coordinate millimetriche sulla pagina.

- Categoria: contenuti
- Tool MCP: `element_move`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `page` (obbligatorio), `id` (obbligatorio), `x_mm` (obbligatorio), `y_mm` (obbligatorio), `out`

## `element.duplicate` — Duplica elemento

Duplica un elemento con un nuovo ID e un piccolo scostamento.

- Categoria: contenuti
- Tool MCP: `element_duplicate`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `page` (obbligatorio), `id` (obbligatorio), `new_id`, `out`

## `content.generate` — Genera contenuto

Genera un materiale dal prompt, una storia illustrata, un piano di layout o un'immagine. Ogni tipo usa il motore esistente e dichiara i limiti.

- Categoria: contenuti
- Tool MCP: `content_generate`
- Accesso: scrittura
- Parametri: `kind` (obbligatorio), `prompt`, `spec`, `out`

## `content.transform` — Trasforma contenuto

Interpreta una richiesta di modifica e restituisce operazioni validate; con `document_path` applica i comandi universali indicati in `commands`.

- Categoria: contenuti
- Tool MCP: `content_transform`
- Accesso: scrittura
- Parametri: `prompt` (obbligatorio), `document_path`, `commands`, `out`

## `content.validate` — Valida contenuto

Controlla struttura, riferimenti, simboli mancanti e avvisi di rendering di un testo o di un documento.

- Categoria: contenuti
- Tool MCP: `content_validate`
- Accesso: lettura
- Parametri: `text`, `path`

## `job.start` — Avvia job

Avvia un metodo in background e restituisce subito un ID. Funziona nel processo persistente (`caa-cli rpc` o server MCP); con una singola invocazione CLI il job non sopravvive alla chiusura del processo.

- Categoria: job
- Tool MCP: `job_start`
- Accesso: scrittura
- Parametri: `method` (obbligatorio), `params`

## `job.status` — Stato job

Stato, avanzamento dichiarato e tempi di un job.

- Categoria: job
- Tool MCP: `job_get_status`
- Accesso: lettura
- Parametri: `id` (obbligatorio)

## `job.result` — Risultato job

Restituisce il risultato di un job concluso o l'errore.

- Categoria: job
- Tool MCP: `job_get_result`
- Accesso: lettura
- Parametri: `id` (obbligatorio)

## `job.cancel` — Annulla job

Chiede l'annullamento cooperativo; le operazioni che non espongono cancellazione lo dichiarano.

- Categoria: job
- Tool MCP: `job_cancel`
- Accesso: scrittura
- Parametri: `id` (obbligatorio)

## `job.list` — Elenca job

Elenca i job della sessione corrente.

- Categoria: job
- Tool MCP: `job_list`
- Accesso: lettura

## `symbolize` — Simbolizza testo

Analizza un paragrafo e restituisce i token CAA. Non modifica documenti. include_svg=false alleggerisce la risposta.

- Categoria: simboli
- Tool MCP: `symbolize_text`
- Accesso: lettura
- Parametri: `text` (obbligatorio), `options`, `include_svg`
- Esempio: `caa-cli call symbolize '{"text":"Il gatto dorme."}'`

## `analyze.text` — Analizza testo

Simbolizza più paragrafi separati da riga vuota e riporta ogni risultato.

- Categoria: simboli
- Tool MCP: `analyze_text`
- Accesso: lettura
- Parametri: `text` (obbligatorio), `document_key`, `include_svg`

## `analyze.caadoc` — Analizza documento

Legge il testo semplice di un .caadoc e lo simbolizza.

- Categoria: simboli
- Tool MCP: `analyze_document`
- Accesso: lettura
- Parametri: `path` (obbligatorio), `include_svg`

## `search` — Cerca simboli

Ricerca FTS5 nel repertorio locale con priorità ai simboli personali e ARASAAC.

- Categoria: simboli
- Tool MCP: `search_symbols`
- Accesso: lettura
- Parametri: `query` (obbligatorio), `limit`, `offset`, `pack_id`, `collection_id`, `commercial_only`

## `affinities` — Affinità della parola

Restituisce gruppi esatto/lemma/varianti/sinonimi/generale/affine con punteggio, motivo e approssimazione. Non chiama il modello.

- Categoria: simboli
- Tool MCP: `word_affinities`
- Accesso: lettura
- Parametri: `word` (obbligatorio), `pack_id`, `commercial_only`

## `symbol.get` — Leggi simbolo

Restituisce etichetta, repertorio e dimensione dei media di un simbolo.

- Categoria: simboli
- Tool MCP: `symbol_get`
- Accesso: lettura
- Parametri: `id` (obbligatorio)

## `symbol.hydrate` — Idrata simboli

Scarica i PNG ARASAAC mancanti e aggiorna gli asset locali. Richiede rete; offline dichiara l'errore.

- Categoria: simboli
- Tool MCP: `symbol_hydrate`
- Accesso: scrittura
- Parametri: `ids` (obbligatorio)

## `symbol.resolve` — Risolvi simboli mancanti

Pipeline locale → ARASAAC → sinonimi italiani → Global Symbols → sinonimi inglesi. Può richiedere rete e importare copie personali.

- Categoria: simboli
- Tool MCP: `symbol_resolve`
- Accesso: scrittura
- Parametri: `words` (obbligatorio)

## `templates.list` — Elenca modelli

Elenca i modelli dello Studio con filtro per categoria o testo.

- Categoria: materiali
- Tool MCP: `templates_list`
- Accesso: lettura
- Parametri: `category`, `query`

## `templates.get` — Leggi modello

Restituisce un modello completo per ID.

- Categoria: materiali
- Tool MCP: `templates_get`
- Accesso: lettura
- Parametri: `id` (obbligatorio)

## `templates.plan` — Pianifica materiale

Interpreta un prompt o un modello e risolve i pittogrammi degli elementi.

- Categoria: materiali
- Tool MCP: `templates_plan`
- Accesso: lettura
- Parametri: `template_id`, `prompt`, `pack_id`

## `layout.build` — Costruisci layout

Valida una LayoutSpecification e calcola le pagine in millimetri con i simboli risolti.

- Categoria: materiali
- Tool MCP: `layout_build`
- Accesso: lettura
- Parametri: `spec` (obbligatorio)

## `layout.edit` — Modifica layout

Applica operazioni strutturate o interpreta un prompt italiano; gli elementi bloccati non vengono mossi.

- Categoria: materiali
- Tool MCP: `layout_edit`
- Accesso: lettura
- Parametri: `spec` (obbligatorio), `prompt`, `operations`

## `library.catalog` — Catalogo biblioteca

Categorie, sottocategorie e conteggi reali della biblioteca.

- Categoria: materiali
- Tool MCP: `library_catalog`
- Accesso: lettura

## `library.search` — Cerca in biblioteca

Ricerca per parola, categoria, pubblico e tipo di interazione.

- Categoria: materiali
- Tool MCP: `library_search`
- Accesso: lettura
- Parametri: `query`, `category`, `limit`, `offset`, `sort`

## `library.validate` — Valida biblioteca

Controlli strutturali ed editoriali su tutti i documenti. Operazione lunga: preferisci un job.

- Categoria: materiali
- Tool MCP: `library_validate`
- Accesso: lettura
- Parametri: `only_pilots`, `max_issues`

## `dev.library.packages` — Pacchetti libreria (dev)

Elenca i pacchetti e i documenti della libreria. Richiede la modalità sviluppatore attiva (CAA_WRITER_DEV_LIBRARY=1 o --dev-library, solo build debug).

- Categoria: sviluppo
- Tool MCP: `dev_library_packages`
- Accesso: lettura

## `dev.library.open` — Apri documento libreria (dev)

Apre un documento .caadoc della libreria per percorso relativo. Richiede la modalità sviluppatore.

- Categoria: sviluppo
- Tool MCP: `dev_library_open`
- Accesso: lettura
- Parametri: `path` (obbligatorio)

## `dev.library.preview` — Anteprima documento libreria (dev)

Restituisce l'anteprima SVG della prima pagina di un documento della libreria. Richiede la modalità sviluppatore.

- Categoria: sviluppo
- Tool MCP: `dev_library_preview`
- Accesso: lettura
- Parametri: `path` (obbligatorio)

## `dev.library.save` — Salva documento libreria (dev)

Salva un UniversalDocument in un percorso della libreria e aggiorna catalogo e anteprima. Distruttivo: richiede conferma.

- Categoria: sviluppo
- Tool MCP: `dev_library_save`
- Accesso: scrittura, distruttiva (confirm=true, admin)
- Parametri: `document` (obbligatorio), `path` (obbligatorio), `confirm`

## `dev.library.replace` — Sostituisci documento libreria (dev)

Sostituisce un documento della libreria con un altro .caadoc scelto dal disco, mantenendo l'ID della voce. Distruttivo: richiede conferma.

- Categoria: sviluppo
- Tool MCP: `dev_library_replace`
- Accesso: scrittura, distruttiva (confirm=true, admin)
- Parametri: `path` (obbligatorio), `source` (obbligatorio), `confirm`

## `dev.library.add` — Aggiungi documento libreria (dev)

Copia un .caadoc nella libreria, genera un ID univoco e lo aggiunge al catalogo. Richiede la modalità sviluppatore.

- Categoria: sviluppo
- Tool MCP: `dev_library_add`
- Accesso: scrittura
- Parametri: `source` (obbligatorio), `folder` (obbligatorio), `category_id` (obbligatorio), `subcategory_id` (obbligatorio), `category_name`, `subcategory_name`

## `dev.library.delete` — Elimina documento libreria (dev)

Elimina un documento dalla libreria, dalla sua anteprima e dal catalogo. Distruttivo: richiede conferma.

- Categoria: sviluppo
- Tool MCP: `dev_library_delete`
- Accesso: scrittura, distruttiva (confirm=true, admin)
- Parametri: `path` (obbligatorio), `confirm`

## `prompt.interpret` — Interpreta prompt

Converte una richiesta italiana in operazioni documentali validate; il modello interviene solo se il parser deterministico non capisce.

- Categoria: materiali
- Tool MCP: `prompt_interpret`
- Accesso: lettura
- Parametri: `prompt` (obbligatorio), `use_neural`

## `import.scan` — Analizza importazione

Visita file o cartelle, riconosce i formati dai byte, calcola checksum e duplicati. Non importa nulla.

- Categoria: repertori
- Tool MCP: `import_scan`
- Accesso: lettura
- Parametri: `paths` (obbligatorio)

## `import.run` — Importa pittogrammi

Importa gli elementi selezionati in una collezione personale con licenza e metadati.

- Categoria: repertori
- Tool MCP: `import_run`
- Accesso: scrittura
- Parametri: `entries` (obbligatorio), `collection`, `license`

## `collections.list` — Elenca collezioni

Elenca le collezioni personali con conteggi.

- Categoria: repertori
- Tool MCP: `collections_list`
- Accesso: lettura

## `collections.rename` — Rinomina collezione

Rinomina una collezione personale.

- Categoria: repertori
- Tool MCP: `collections_rename`
- Accesso: scrittura
- Parametri: `id` (obbligatorio), `name` (obbligatorio)

## `collections.delete` — Elimina collezione

Elimina una collezione e i suoi file. Richiede confirm=true.

- Categoria: repertori
- Tool MCP: `collections_delete`
- Accesso: scrittura, distruttiva (confirm=true, admin)
- Parametri: `id` (obbligatorio), `confirm`

## `collections.delete_symbols` — Elimina simboli

Elimina simboli personali specifici. Richiede confirm=true.

- Categoria: repertori
- Tool MCP: `collections_delete_symbols`
- Accesso: scrittura, distruttiva (confirm=true, admin)
- Parametri: `ids` (obbligatorio), `confirm`

## `collections.move` — Sposta simboli

Sposta simboli personali in un'altra collezione.

- Categoria: repertori
- Tool MCP: `collections_move`
- Accesso: scrittura
- Parametri: `ids` (obbligatorio), `collection_id` (obbligatorio)

## `profiles.list` — Elenca profili

Elenca i profili locali, senza dati sanitari.

- Categoria: profili
- Tool MCP: `profiles_list`
- Accesso: lettura

## `boards.list` — Elenca sistemi

Elenca i sistemi comunicativi disponibili.

- Categoria: profili
- Tool MCP: `boards_list`
- Accesso: lettura

## `boards.build` — Costruisci sistema

Risolve i pittogrammi e restituisce il grafo navigabile del sistema.

- Categoria: profili
- Tool MCP: `boards_build`
- Accesso: lettura
- Parametri: `id` (obbligatorio)

## `online.providers` — Provider online

Elenca i provider e il loro stato reale (abilitato/disabilitato).

- Categoria: rete
- Tool MCP: `online_providers`
- Accesso: lettura

## `online.search` — Ricerca online

Ricerca federata ARASAAC/Global Symbols/Openverse con cache 24h. Richiede rete; invia solo lemmi filtrati dal lessico.

- Categoria: rete
- Tool MCP: `online_search`
- Accesso: lettura
- Parametri: `terms` (obbligatorio), `force_refresh`

## `online.ensure` — Importa copie online

Cerca e salva copie offline ARASAAC per le parole indicate.

- Categoria: rete
- Tool MCP: `online_ensure`
- Accesso: scrittura
- Parametri: `words` (obbligatorio)

## `online.autofill` — Autofill online

Importa il primo risultato ARASAAC quando manca un'etichetta esatta.

- Categoria: rete
- Tool MCP: `online_autofill`
- Accesso: scrittura
- Parametri: `words` (obbligatorio)

## `online.global` — Global Symbols

Cerca solo su Global Symbols e restituisce i risultati senza importarli.

- Categoria: rete
- Tool MCP: `online_global`
- Accesso: lettura
- Parametri: `term` (obbligatorio)

## `online.english` — Alternativa inglese

Traduce, cerca su Global Symbols e importa fino a `limit` copie personali.

- Categoria: rete
- Tool MCP: `online_english`
- Accesso: scrittura
- Parametri: `word` (obbligatorio), `limit`

## `online.save` — Salva risultato online

Salva un risultato online scelto come copia offline con licenza e attribuzione.

- Categoria: rete
- Tool MCP: `online_save`
- Accesso: scrittura
- Parametri: `result` (obbligatorio), `lemma` (obbligatorio), `collection`

## `english.alternatives` — Alternative inglesi

Traduce in inglese, cerca ARASAAC inglese e importa le copie offline.

- Categoria: rete
- Tool MCP: `english_alternatives`
- Accesso: scrittura
- Parametri: `word` (obbligatorio), `limit`

## `translate.search` — Traduci per la ricerca

Traduce un termine per la ricerca italiana usando glossario o modello locale.

- Categoria: rete
- Tool MCP: `translate_search`
- Accesso: lettura
- Parametri: `text` (obbligatorio)

## `translate.english` — Traduci in inglese

Traduce un termine italiano in inglese con glossario o modello locale.

- Categoria: rete
- Tool MCP: `translate_english`
- Accesso: lettura
- Parametri: `text` (obbligatorio)

## `ai.status` — Stato IA

Stato del modello locale, backend e metriche. Nessun processo viene avviato solo per leggere lo stato.

- Categoria: ia
- Tool MCP: `ai_get_status`
- Accesso: lettura

## `ai.models` — Modelli IA

Catalogo dei modelli con stato installato e compatibilità hardware.

- Categoria: ia
- Tool MCP: `ai_list_models`
- Accesso: lettura

## `ai.chat` — Chat IA

Invia messaggi al modello locale attivo. Senza modello dichiara l'indisponibilità; può essere lento.

- Categoria: ia
- Tool MCP: `ai_chat`
- Accesso: lettura
- Parametri: `messages` (obbligatorio)

## `ai.tool` — Strumento IA

Esegue uno strumento IA tipizzato con validazione e fallback deterministico.

- Categoria: ia
- Tool MCP: `ai_run_tool`
- Accesso: lettura
- Parametri: `tool` (obbligatorio), `input`

## `ai.affinity` — Sinonimi IA

Chiede al modello parole affini e restituisce i simboli trovati; se il modello non c'è non inventa nulla.

- Categoria: ia
- Tool MCP: `ai_affinity`
- Accesso: lettura
- Parametri: `word` (obbligatorio)

## `ai.words` — Parole IA

Parole grezze proposte dal modello (diagnostica).

- Categoria: ia
- Tool MCP: `ai_words`
- Accesso: lettura
- Parametri: `word` (obbligatorio)

## `voice.status` — Stato voce

Motore vocale (neurale o di sistema), modello, licenza e cache.

- Categoria: voce
- Tool MCP: `voice_get_status`
- Accesso: lettura
- Parametri: `language`

## `voice.speak` — Pronuncia testo

Sintetizza e riproduce il testo. Può usare la voce di sistema; non modifica il documento.

- Categoria: voce
- Tool MCP: `voice_speak`
- Accesso: lettura
- Parametri: `text` (obbligatorio), `language`

## `speak` — Pronuncia (alias)

Alias CLI di `voice.speak`.

- Categoria: voce
- Accesso: lettura
- Parametri: `text` (obbligatorio)

## `packages.catalog` — Catalogo pacchetti

Pacchetti di simboli e librerie disponibili con stato installato.

- Categoria: pacchetti
- Tool MCP: `packages_catalog`
- Accesso: lettura

## `packages.updates` — Aggiornamenti pacchetti

Verifica aggiornamenti dei pacchetti installati.

- Categoria: pacchetti
- Tool MCP: `packages_updates`
- Accesso: lettura

## `packages.download` — Scarica pacchetto

Scarica un pacchetto di simboli. Richiede rete e può essere lungo.

- Categoria: pacchetti
- Tool MCP: `packages_download`
- Accesso: scrittura
- Parametri: `id` (obbligatorio), `limit`

## `packages.set_library` — Attiva libreria

Abilita o disabilita una libreria.

- Categoria: pacchetti
- Tool MCP: `packages_set_library`
- Accesso: scrittura
- Parametri: `id` (obbligatorio), `enabled` (obbligatorio)

## `packages.remove` — Rimuovi pacchetto

Rimuove un pacchetto di simboli. Richiede confirm=true.

- Categoria: pacchetti
- Tool MCP: `packages_remove`
- Accesso: scrittura, distruttiva (confirm=true, admin)
- Parametri: `id` (obbligatorio), `confirm`

## `document.open` — Apri .caadoc

Carica un documento .caadoc legacy o universale senza salvarlo.

- Categoria: documenti
- Tool MCP: `document_open`
- Accesso: lettura
- Parametri: `path` (obbligatorio)

## `document.save` — Salva .caadoc

Salva un DocumentPayload nel formato .caadoc con scrittura atomica.

- Categoria: documenti
- Tool MCP: `document_save`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `payload` (obbligatorio)

## `document.pdf` — PDF documento

Genera il PDF di un UniversalDocument.

- Categoria: documenti
- Tool MCP: `document_pdf`
- Accesso: scrittura
- Parametri: `document` (obbligatorio), `out` (obbligatorio)

## `export.html` — Esporta HTML legacy

Esporta un ExportDoc in HTML con gli asset del motore.

- Categoria: documenti
- Tool MCP: `export_html`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `document` (obbligatorio)

## `export.docx` — Esporta DOCX legacy

Esporta un ExportDoc in DOCX con gli asset del motore.

- Categoria: documenti
- Tool MCP: `export_docx`
- Accesso: scrittura
- Parametri: `path` (obbligatorio), `document` (obbligatorio)

## `decision.status` — Stato del motore decisionale

Stato reale del DecisionEngine, del modello Horizon installato e della cache. Non avvia inferenze.

- Categoria: decisione
- Tool MCP: `decision_status`
- Accesso: lettura
- Esempio: `caa-cli call decision.status '{}'`

## `decision.classify` — Classificazione semantica

Classifica un testo fra alternative predefinite usando il DecisionEngine condiviso. Il modello non esegue comandi e non modifica documenti.

- Categoria: decisione
- Tool MCP: `decision_classify`
- Accesso: lettura
- Parametri: `task` (obbligatorio), `text` (obbligatorio), `target`, `candidates`, `mode`, `multi_label`, `timeout_ms`, `priority`, `taxonomy_version`, `apply_threshold`, `suggest_threshold`
- Esempio: `caa-cli call decision.classify '{"task":"communication_intent","text":"Vorrei bere dell'acqua."}'`

## `decision.audit` — Diagnostica decisioni

Ultimi eventi del motore decisionale, senza testo utente né dati personali.

- Categoria: decisione
- Tool MCP: `decision_audit`
- Accesso: lettura

## `decision.validate` — Valida un dataset

Esegue un insieme di casi italiani sul DecisionEngine reale e riporta accuratezza, astensioni, copertura, errori e latenza media. Non modifica nulla.

- Categoria: decisione
- Tool MCP: `decision_validate`
- Accesso: lettura
- Parametri: `cases` (obbligatorio), `split`, `suggest_threshold`, `apply_threshold`
- Esempio: `caa-cli call decision.validate '{"cases":[...]}'`

## `decision.calibrate` — Calibra le soglie

Esegue i casi su Horizon e propone soglie di suggerimento/applicazione con copertura e precisione misurate. Non modifica la configurazione.

- Categoria: decisione
- Tool MCP: `decision_calibrate`
- Accesso: lettura
- Parametri: `cases` (obbligatorio), `split`
- Esempio: `caa-cli call decision.calibrate '{"cases":[...]}'`

## `decision.model.status` — Stato modello Horizon

Verifica presenza e integrità attesa dei file del modello Horizon nella cartella modelli.

- Categoria: decisione
- Tool MCP: `decision_model_status`
- Accesso: lettura

## `decision.model.verify` — Verifica modello Horizon

Calcola gli SHA-256 dei file del modello installato. Può richiedere qualche secondo.

- Categoria: decisione
- Tool MCP: `decision_model_verify`
- Accesso: lettura

## `decision.model.install` — Installa modello Horizon

Scarica i file ufficiali del modello Horizon e li installa in modo atomico. Richiede rete; dopo l'installazione funziona offline.

- Categoria: decisione
- Tool MCP: `decision_model_install`
- Accesso: scrittura
- Parametri: `force`

## `decision.model.remove` — Rimuovi modello Horizon

Rimuove i pesi locali del modello Horizon. Richiede confirm=true. Le funzioni deterministiche restano attive.

- Categoria: decisione
- Tool MCP: `decision_model_remove`
- Accesso: scrittura, distruttiva (confirm=true, admin)
- Parametri: `confirm`


