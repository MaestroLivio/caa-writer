# Audit tecnico — CAA Writer 0.10

Data: 7 novembre 2026. Ambito: motore documentale, editor universale, griglie,
player, stampa/PDF, biblioteca, importazione pittogrammi.
Le cause indicate come **verificate** sono state riprodotte nel codice o con un
test; le ipotesi residue sono marcate come tali.

| # | Problema | File / funzione | Causa verificata | Gravità | Soluzione | Test | Stato |
|---|---|---|---|---|---|---|---|
| 1 | Il comando «Griglia» creava una griglia vuota | `UniversalEditor.svelte` `addGrid`; `universal.rs` `CreateGrid` | Il comando creava `cells: Vec::new()` | Bloccante | Creazione automatica di `righe × colonne` celle con ID stabili; dialogo righe/colonne | `grid_4_by_5_creates_twenty_real_cells` | Corretto |
| 2 | Ridimensionare una griglia perdeva/duplicava celle | `universal.rs` `UpdateGrid` | Il vettore celle non veniva adeguato alla nuova capacità | Alta | Aggiunta/rimozione deterministica, ID unici, troncamento esplicito | test griglia + round-trip | Corretto |
| 3 | PDF non veniva generato | `commands.rs` `document_export` | Mancava un esportatore PDF; c'era solo la stampa browser | Bloccante | `export/universal_pdf.rs`: rasterizzazione pagine con resvg e PDF multipagina valido | `library_documents_export_real_pdf_and_png` | Corretto |
| 4 | Testi assenti nel PDF/PNG | `export/universal_pdf.rs` `page_to_png` | `usvg::Options` aveva un font database vuoto | Bloccante | Caricamento dei font di sistema prima del rendering | test PDF/PNG + ispezione immagine | Corretto |
| 5 | Pittogrammi con attributi SVG duplicati | `universal.rs` `embed_symbol` | `width`/`height` già presenti nell'SVG ARASAAC e aggiunti di nuovo | Alta | Rimozione degli attributi geometrici prima dell'innesto | test PDF su documento di biblioteca | Corretto |
| 6 | Testo delle celle non compariva nell'anteprima SVG/PDF | `universal.rs` `render_element_svg` | Il testo era disegnato solo per elementi non-symbol | Alta | Didascalia sotto il pittogramma per tutti gli elementi visibili | ispezione PNG + test round-trip | Corretto |
| 7 | Percorsi della biblioteca potevano uscire da `resources/library` | `universal.rs` `resolve_library_path` | Il controllo era lessicale, non canonicalizzato | Sicurezza | `canonicalize` + prefisso della biblioteca | `library_paths_are_resolved_inside_the_library` | Corretto |
| 8 | Modelli legacy e modelli universali duplicati | `App.svelte` `openTemplate`; `studio.rs` | La Home apriva il vecchio piano dello Studio | Alta | La Home apre il `.caadoc` universale; fallback dichiarato solo se manca | `universal_documents` | Corretto |
| 9 | File `.caadoc` non associati al player | `tauri.conf.json` `bundle.fileAssociations`; `lib.rs` `RunEvent::Opened` | Nessuna associazione né gestione dell'evento | Alta | Associazione `.caadoc`/`.caaplay`, `PendingOpen`, evento `open-document`, player | compilazione; non eseguito su Windows | Configurato |
| 10 | Simbolo personale di prova negli installer | database locale, non `resources/` | Import di prova `custom.cane.cdefa9e7` nella cartella dati utente | Media | Backup del database e rimozione del solo simbolo di prova; nessuna modifica ai repertori | query DB: 0 simboli personal | Corretto |
| 11 | DOCX/PPTX/PNG completi non disponibili | `export/` | Non esisteva un esportatore Office/PNG multipagina | Media | PNG prima pagina e PDF multipagina implementati; DOCX/PPTX dichiarati mancanti | test PDF/PNG | Parziale |
| 12 | Unione/divisione celle e ridimensionamento per cella non completo | `universal.rs` `GridData` | Il modello usa celle uniformi; mancano span e larghezze per riga/colonna | Media | Modello pronto per l'estensione; non dichiarato completo | — | Aperto |
| 13 | Testo inline nell'editor universale | `UniversalEditor.svelte` | Modifica nel pannello proprietà, non contenteditable | Media | Word Processor ProseMirror per il flusso; proprietà per il layout | — | Limite dichiarato |
| 14 | Stampa non verificata end-to-end | `UniversalEditor.svelte` `printDocument` | Iframe di stampa dipende dal WebView | Alta | HTML server-side con tutte le pagine, poi `window.print` | test HTML; non automate le finestre native | Da verificare in Tauri |
| 15 | Chiavi API online | non implementate | Nessun provider online/portachiavi | Media | Non dichiarato attivo; resta il motore locale | — | Aperto |

## Note di compatibilità

- `.caadoc` v1/v2 restano leggibili: `layout` viene migrato in `UniversalDocument`.
- `.caaplay` usa lo stesso contenitore del formato canonico; cambia l'estensione e
  l'apertura diretta nel player.
- I 330 documenti della biblioteca sono generati dalla CLI e verificati con lo
  stesso motore della GUI.
