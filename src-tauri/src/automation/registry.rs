//! Registro unico delle operazioni applicative.
//!
//! Da questo registro derivano: elenco dei metodi CLI, catalogo delle
//! capacità, strumenti MCP, schemi JSON e test di conformità. Una nuova
//! operazione si aggiunge qui e nel `dispatch`: il test di parità fallisce se
//! una delle due parti resta indietro.

use serde_json::{json, Value};

pub const API_VERSION: &str = "1.0";

#[derive(Debug, Clone, Copy)]
pub struct Param {
    pub name: &'static str,
    /// JSON Schema type: string, boolean, integer, number, array, object.
    pub kind: &'static str,
    pub required: bool,
    pub description: &'static str,
    pub default: Option<&'static str>,
    pub enum_values: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct Operation {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    /// Nome dello strumento MCP. `None` = alias CLI, non esposto come tool.
    pub mcp: Option<&'static str>,
    pub params: &'static [Param],
    pub output: &'static str,
    pub mutating: bool,
    pub destructive: bool,
    pub idempotent: bool,
    pub example: Option<&'static str>,
    /// Parametri legacy documentati in `docs/AGENT.md` e accettati come
    /// oggetto JSON libero. Non nasconde nulla: il catalogo lo dichiara.
    pub free_form_params: bool,
}

const fn p(
    name: &'static str,
    kind: &'static str,
    required: bool,
    description: &'static str,
) -> Param {
    Param {
        name,
        kind,
        required,
        description,
        default: None,
        enum_values: &[],
    }
}

macro_rules! op {
    (
        $name:expr, $title:expr, $category:expr, $mcp:expr,
        $desc:expr, $out:expr,
        mutating: $mutating:expr, destructive: $destructive:expr, idempotent: $idempotent:expr,
        params: [$($param:expr),* $(,)?]
        $(, example: $example:expr)?
    ) => {
        Operation {
            name: $name,
            title: $title,
            description: $desc,
            category: $category,
            mcp: $mcp,
            params: &[$($param),*],
            output: $out,
            mutating: $mutating,
            destructive: $destructive,
            idempotent: $idempotent,
            #[allow(unused_assignments, unused_mut)]
            example: {
                let mut value: Option<&'static str> = None;
                $(value = Some($example);)?
                value
            },
            free_form_params: false,
        }
    };
}

const PATH: Param = p("path", "string", true, "Percorso del file .caadoc");
const OUT: Param = p("out", "string", false, "Percorso del file o della cartella di destinazione");
const OUT_REQUIRED: Param = p("out", "string", true, "Percorso del file di destinazione");
const FORMAT: Param = p("format", "string", false, "Formato di esportazione: pdf, png, html, svg, docx, pptx, json");
const DOCUMENT: Param = p("document", "object", true, "UniversalDocument completo");
const CONFIRM: Param = p("confirm", "boolean", false, "Conferma esplicita per un'operazione irreversibile");

pub const OPERATIONS: &[Operation] = &[
    // ------------------------------------------------------------------
    // Applicazione e scoperta
    // ------------------------------------------------------------------
    op!(
        "app.capabilities", "Catalogo delle capacità", "applicazione", Some("app_get_capabilities"),
        "Restituisce il catalogo generato dal registro reale: operazioni, parametri, schemi, versione API, modalità di accesso e limitazioni. Usalo prima di pianificare un workflow.",
        "Oggetto con versione, operazioni, schemi JSON e note sui limiti.",
        mutating: false, destructive: false, idempotent: true,
        params: [
            p("format", "string", false, "json (predefinito) o markdown"),
            p("category", "string", false, "Filtra per categoria"),
        ],
        example: "caa-cli capabilities"
    ),
    op!(
        "app.status", "Stato dell'applicazione", "applicazione", Some("app_get_status"),
        "Stato reale di motore, percorsi, database, modello neurale, libreria e bridge della GUI. Non avvia processi pesanti.",
        "Stato applicativo strutturato.",
        mutating: false, destructive: false, idempotent: true,
        params: [],
        example: "caa-cli call app.status '{}'"
    ),
    op!(
        "app.diagnostics", "Diagnostica", "applicazione", Some("app_get_diagnostics"),
        "CPU/RAM del processo, stato del modello, metriche della coda neurale e della simbolizzazione.",
        "RuntimeDiagnostics del motore.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "info", "Informazioni sul motore", "applicazione", Some("engine_get_info"),
        "Versione, percorsi, statistiche del database, conteggi del lessico e avvisi di avvio.",
        "Informazioni del motore.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "methods", "Elenco metodi", "applicazione", None,
        "Alias CLI di `app.capabilities`: elenco dei nomi dei metodi disponibili.",
        "Array di nomi.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),

    // ------------------------------------------------------------------
    // Impostazioni
    // ------------------------------------------------------------------
    op!(
        "settings.list", "Elenca impostazioni", "impostazioni", Some("settings_list"),
        "Elenca le chiavi e i valori delle impostazioni persistite nel database. Le chiavi sconosciute sono incluse: non vengono inventate categorie.",
        "Array di {key,value}.",
        mutating: false, destructive: false, idempotent: true,
        params: [
            p("prefix", "string", false, "Filtro per prefisso, per esempio ai. o speech."),
        ]
    ),
    op!(
        "settings.get", "Leggi impostazione", "impostazioni", Some("settings_get"),
        "Legge una singola impostazione persistita.",
        "Oggetto {key,value}; value è null se assente.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("key", "string", true, "Chiave dell'impostazione")]
    ),
    op!(
        "settings.set", "Scrivi impostazione", "impostazioni", Some("settings_update"),
        "Scrive una singola impostazione. Con dry_run=true non modifica nulla. Le preferenze manuali dell'utente non vengono toccate.",
        "Oggetto {ok,key,value,changed}.",
        mutating: true, destructive: false, idempotent: true,
        params: [
            p("key", "string", true, "Chiave dell'impostazione"),
            p("value", "string", true, "Nuovo valore serializzato come stringa"),
        ]
    ),

    // ------------------------------------------------------------------
    // Progetti
    // ------------------------------------------------------------------
    op!(
        "project.list", "Elenca progetti", "progetti", Some("project_list"),
        "Elenca i file .caadoc nella cartella progetti (predefinita: <data-dir>/projects). Non attraversa collegamenti simbolici e non esce dalla radice.",
        "Array di {name,path,bytes,modified}.",
        mutating: false, destructive: false, idempotent: true,
        params: [
            p("root", "string", false, "Sottocartella relativa alla radice progetti"),
            p("limit", "integer", false, "Numero massimo di risultati (default 200)"),
        ]
    ),
    op!(
        "project.create", "Crea progetto", "progetti", Some("project_create"),
        "Crea un documento universale vuoto e lo salva nella radice progetti. `path` è relativo alla radice; non sono ammessi percorsi assoluti o `..`.",
        "Riepilogo del documento salvato.",
        mutating: true, destructive: false, idempotent: false,
        params: [
            p("path", "string", true, "Percorso relativo, per esempio esercizi/cane.caadoc"),
            p("title", "string", true, "Titolo del documento"),
            p("kind", "string", false, "document, communication_board, presentation (default document)"),
            p("layout", "string", false, "flow, absolute, grid (default flow)"),
        ],
        example: "caa-cli project create --path demo.caadoc --title Demo"
    ),
    op!(
        "project.open", "Apri progetto", "progetti", Some("project_open"),
        "Legge un progetto .caadoc dalla radice progetti e ne restituisce il documento completo.",
        "Documento universale e riepilogo.",
        mutating: false, destructive: false, idempotent: true,
        params: [PATH]
    ),
    op!(
        "project.validate", "Valida progetto", "progetti", Some("project_validate"),
        "Valida schema, pagine, elementi, azioni, collegamenti e risorse del progetto.",
        "Report di validazione con eventuali warning di rendering.",
        mutating: false, destructive: false, idempotent: true,
        params: [PATH]
    ),
    op!(
        "project.save", "Salva progetto", "progetti", Some("project_save"),
        "Salva un documento nella radice progetti. Sovrascrive solo se `overwrite=true`; altrimenti rifiuta il conflitto. Non idempotente se il titolo cambia (il nome file può cambiare).",
        "Riepilogo del file salvato.",
        mutating: true, destructive: false, idempotent: false,
        params: [
            PATH,
            p("document", "object", true, "Documento universale completo"),
            p("overwrite", "boolean", false, "Consenti la sovrascrittura (default false)"),
        ]
    ),
    op!(
        "project.duplicate", "Duplica progetto", "progetti", Some("project_duplicate"),
        "Copia un progetto in un nuovo percorso della radice progetti; non sovrascrive mai un file esistente.",
        "Riepilogo della copia.",
        mutating: true, destructive: false, idempotent: false,
        params: [PATH, p("target", "string", true, "Nuovo percorso relativo")]
    ),
    op!(
        "project.rename", "Rinomina progetto", "progetti", Some("project_rename"),
        "Sposta/rinomina un progetto. Rifiuta se la destinazione esiste.",
        "Riepilogo del nuovo file.",
        mutating: true, destructive: false, idempotent: false,
        params: [PATH, p("target", "string", true, "Nuovo percorso relativo")]
    ),
    op!(
        "project.delete", "Elimina progetto", "progetti", Some("project_delete"),
        "Elimina definitivamente un progetto. Richiede confirm=true; senza conferma non tocca nulla.",
        "Oggetto {ok,removed}.",
        mutating: true, destructive: true, idempotent: false,
        params: [PATH, CONFIRM]
    ),
    op!(
        "project.export", "Esporta progetto", "progetti", Some("project_export"),
        "Esporta un progetto in pdf, png, html, svg, docx o pptx. La destinazione deve restare nella radice progetti.",
        "Percorso e formato generati.",
        mutating: true, destructive: false, idempotent: true,
        params: [PATH, FORMAT, OUT]
    ),

    // ------------------------------------------------------------------
    // Documenti universali e contenuti
    // ------------------------------------------------------------------
    op!(
        "document.create", "Crea documento", "documenti", Some("document_create"),
        "Crea un UniversalDocument vuoto; con `path` lo salva. Accetta una specifica completa con `spec`.",
        "Documento creato e riepilogo.",
        mutating: true, destructive: false, idempotent: false,
        params: [
            p("title", "string", false, "Titolo"),
            p("kind", "string", false, "Tipo di documento"),
            p("layout", "string", false, "flow, absolute, grid"),
            p("spec", "object", false, "Specifica completa alternativa"),
            p("path", "string", false, "Percorso di salvataggio"),
        ]
    ),
    op!(
        "document.get", "Leggi documento", "documenti", Some("document_get"),
        "Legge un .caadoc e restituisce il documento universale; con `summary_only=true` restituisce solo il riepilogo.",
        "Documento universale o riepilogo.",
        mutating: false, destructive: false, idempotent: true,
        params: [PATH, p("summary_only", "boolean", false, "Solo riepilogo")]
    ),
    op!(
        "document.update", "Modifica documento", "documenti", Some("document_update"),
        "Applica una transazione di DocumentCommand a un .caadoc e salva su `out` (default: stesso file).",
        "Riepilogo del documento aggiornato.",
        mutating: true, destructive: false, idempotent: false,
        params: [
            PATH,
            p("commands", "array", true, "Elenco di DocumentCommand tipizzati"),
            OUT,
        ]
    ),
    op!(
        "document.export", "Esporta documento", "documenti", Some("document_export"),
        "Esporta un documento universale nei formati supportati dal renderer condiviso.",
        "Percorso generato e warning.",
        mutating: true, destructive: false, idempotent: true,
        params: [PATH, FORMAT, OUT]
    ),
    op!(
        "element.add", "Aggiungi elemento", "contenuti", Some("element_create"),
        "Aggiunge un elemento a una pagina con un ID stabile.",
        "Riepilogo del documento.",
        mutating: true, destructive: false, idempotent: false,
        params: [
            PATH, p("page", "string", true, "ID pagina"),
            p("element", "object", true, "Elemento universale"),
            p("index", "integer", false, "Posizione nell'elenco"),
            OUT,
        ]
    ),
    op!(
        "element.update", "Modifica elemento", "contenuti", Some("element_update"),
        "Applica una patch JSON a un elemento identificato da pagina e ID.",
        "Riepilogo del documento.",
        mutating: true, destructive: false, idempotent: true,
        params: [
            PATH, p("page", "string", true, "ID pagina"),
            p("id", "string", true, "ID elemento"),
            p("patch", "object", true, "Campi da aggiornare"),
            OUT,
        ]
    ),
    op!(
        "element.delete", "Elimina elemento", "contenuti", Some("element_delete"),
        "Rimuove un elemento da una pagina.",
        "Riepilogo del documento.",
        mutating: true, destructive: true, idempotent: false,
        params: [PATH, p("page", "string", true, "ID pagina"), p("id", "string", true, "ID elemento"), CONFIRM, OUT]
    ),
    op!(
        "element.move", "Sposta elemento", "contenuti", Some("element_move"),
        "Sposta un elemento in coordinate millimetriche sulla pagina.",
        "Riepilogo del documento.",
        mutating: true, destructive: false, idempotent: true,
        params: [
            PATH, p("page", "string", true, "ID pagina"), p("id", "string", true, "ID elemento"),
            p("x_mm", "number", true, "Coordinata X"), p("y_mm", "number", true, "Coordinata Y"), OUT,
        ]
    ),
    op!(
        "element.duplicate", "Duplica elemento", "contenuti", Some("element_duplicate"),
        "Duplica un elemento con un nuovo ID e un piccolo scostamento.",
        "Riepilogo con il nuovo ID.",
        mutating: true, destructive: false, idempotent: false,
        params: [
            PATH, p("page", "string", true, "ID pagina"), p("id", "string", true, "ID elemento"),
            p("new_id", "string", false, "ID del duplicato"), OUT,
        ]
    ),
    op!(
        "content.generate", "Genera contenuto", "contenuti", Some("content_generate"),
        "Genera un materiale dal prompt, una storia illustrata, un piano di layout o un'immagine. Ogni tipo usa il motore esistente e dichiara i limiti.",
        "Piano, documento o immagine con warning.",
        mutating: true, destructive: false, idempotent: false,
        params: [
            p("kind", "string", true, "material, story, image o layout"),
            p("prompt", "string", false, "Richiesta in linguaggio naturale"),
            p("spec", "object", false, "Specifica strutturata"),
            OUT,
        ]
    ),
    op!(
        "content.transform", "Trasforma contenuto", "contenuti", Some("content_transform"),
        "Interpreta una richiesta di modifica e restituisce operazioni validate; con `document_path` applica i comandi universali indicati in `commands`.",
        "Piano di operazioni, descrizioni e warning.",
        mutating: true, destructive: false, idempotent: false,
        params: [
            p("prompt", "string", true, "Richiesta di modifica"),
            p("document_path", "string", false, "Documento universale da aggiornare"),
            p("commands", "array", false, "Comandi da applicare"),
            OUT,
        ]
    ),
    op!(
        "content.validate", "Valida contenuto", "contenuti", Some("content_validate"),
        "Controlla struttura, riferimenti, simboli mancanti e avvisi di rendering di un testo o di un documento.",
        "Report con errori, warning e conteggi.",
        mutating: false, destructive: false, idempotent: true,
        params: [
            p("text", "string", false, "Testo da analizzare"),
            p("path", "string", false, "Documento .caadoc"),
        ]
    ),

    // ------------------------------------------------------------------
    // Operazioni asincrone
    // ------------------------------------------------------------------
    op!(
        "job.start", "Avvia job", "job", Some("job_start"),
        "Avvia un metodo in background e restituisce subito un ID. Funziona nel processo persistente (`caa-cli rpc` o server MCP); con una singola invocazione CLI il job non sopravvive alla chiusura del processo.",
        "Oggetto {id,state}.",
        mutating: true, destructive: false, idempotent: false,
        params: [
            p("method", "string", true, "Metodo da eseguire"),
            p("params", "object", false, "Parametri del metodo"),
        ]
    ),
    op!(
        "job.status", "Stato job", "job", Some("job_get_status"),
        "Stato, avanzamento dichiarato e tempi di un job.",
        "Oggetto job.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("id", "string", true, "ID job")]
    ),
    op!(
        "job.result", "Risultato job", "job", Some("job_get_result"),
        "Restituisce il risultato di un job concluso o l'errore.",
        "Oggetto job con result o error.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("id", "string", true, "ID job")]
    ),
    op!(
        "job.cancel", "Annulla job", "job", Some("job_cancel"),
        "Chiede l'annullamento cooperativo; le operazioni che non espongono cancellazione lo dichiarano.",
        "Oggetto job aggiornato.",
        mutating: true, destructive: false, idempotent: true,
        params: [p("id", "string", true, "ID job")]
    ),
    op!(
        "job.list", "Elenca job", "job", Some("job_list"),
        "Elenca i job della sessione corrente.",
        "Array di job.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),

    // ------------------------------------------------------------------
    // Simbolizzazione, ricerca, analisi
    // ------------------------------------------------------------------
    op!(
        "symbolize", "Simbolizza testo", "simboli", Some("symbolize_text"),
        "Analizza un paragrafo e restituisce i token CAA. Non modifica documenti. include_svg=false alleggerisce la risposta.",
        "ParagraphResult con token, alternative e metriche.",
        mutating: false, destructive: false, idempotent: true,
        params: [
            p("text", "string", true, "Testo da simbolizzare"),
            p("options", "object", false, "SymbolizeOptions"),
            p("include_svg", "boolean", false, "Includi SVG/immagini (default false)"),
        ],
        example: "caa-cli call symbolize '{\"text\":\"Il gatto dorme.\"}'"
    ),
    op!(
        "analyze.text", "Analizza testo", "simboli", Some("analyze_text"),
        "Simbolizza più paragrafi separati da riga vuota e riporta ogni risultato.",
        "Elenco di paragrafi simbolizzati.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("text", "string", true, "Testo"), p("document_key", "string", false, "Chiave documento"), p("include_svg", "boolean", false, "Includi immagini")]
    ),
    op!(
        "analyze.caadoc", "Analizza documento", "simboli", Some("analyze_document"),
        "Legge il testo semplice di un .caadoc e lo simbolizza.",
        "Paragrafi simbolizzati e warning.",
        mutating: false, destructive: false, idempotent: true,
        params: [PATH, p("include_svg", "boolean", false, "Includi immagini")]
    ),
    op!(
        "search", "Cerca simboli", "simboli", Some("search_symbols"),
        "Ricerca FTS5 nel repertorio locale con priorità ai simboli personali e ARASAAC.",
        "SearchResultPage.",
        mutating: false, destructive: false, idempotent: true,
        params: [
            p("query", "string", true, "Parola o etichetta"),
            p("limit", "integer", false, "Default 40"), p("offset", "integer", false, "Default 0"),
            p("pack_id", "string", false, "Filtro repertorio"), p("collection_id", "string", false, "Filtro collezione"),
            p("commercial_only", "boolean", false, "Solo uso commerciale"),
        ]
    ),
    op!(
        "affinities", "Affinità della parola", "simboli", Some("word_affinities"),
        "Restituisce gruppi esatto/lemma/varianti/sinonimi/generale/affine con punteggio, motivo e approssimazione. Non chiama il modello.",
        "WordAffinity.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("word", "string", true, "Parola"), p("pack_id", "string", false, "Repertorio"), p("commercial_only", "boolean", false, "Solo uso commerciale")]
    ),
    op!(
        "symbol.get", "Leggi simbolo", "simboli", Some("symbol_get"),
        "Restituisce etichetta, repertorio e dimensione dei media di un simbolo.",
        "Metadati del simbolo.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("id", "string", true, "ID simbolo")]
    ),
    op!(
        "symbol.hydrate", "Idrata simboli", "simboli", Some("symbol_hydrate"),
        "Scarica i PNG ARASAAC mancanti e aggiorna gli asset locali. Richiede rete; offline dichiara l'errore.",
        "ID idratati e conteggio.",
        mutating: true, destructive: false, idempotent: true,
        params: [p("ids", "array", true, "ID simbolo")]
    ),
    op!(
        "symbol.resolve", "Risolvi simboli mancanti", "simboli", Some("symbol_resolve"),
        "Pipeline locale → ARASAAC → sinonimi italiani → Global Symbols → sinonimi inglesi. Può richiedere rete e importare copie personali.",
        "Alternative trovate per parola.",
        mutating: true, destructive: false, idempotent: true,
        params: [p("words", "array", true, "Parole")]
    ),

    // ------------------------------------------------------------------
    // Template, layout, biblioteca
    // ------------------------------------------------------------------
    op!(
        "templates.list", "Elenca modelli", "materiali", Some("templates_list"),
        "Elenca i modelli dello Studio con filtro per categoria o testo.",
        "Array di modelli.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("category", "string", false, "Categoria"), p("query", "string", false, "Testo")]
    ),
    op!(
        "templates.get", "Leggi modello", "materiali", Some("templates_get"),
        "Restituisce un modello completo per ID.",
        "Template.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("id", "string", true, "ID modello")]
    ),
    op!(
        "templates.plan", "Pianifica materiale", "materiali", Some("templates_plan"),
        "Interpreta un prompt o un modello e risolve i pittogrammi degli elementi.",
        "Piano con elementi risolti.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("template_id", "string", false, "ID modello"), p("prompt", "string", false, "Richiesta"), p("pack_id", "string", false, "Repertorio")]
    ),
    op!(
        "layout.build", "Costruisci layout", "materiali", Some("layout_build"),
        "Valida una LayoutSpecification e calcola le pagine in millimetri con i simboli risolti.",
        "LayoutDocument.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("spec", "object", true, "LayoutSpecification")]
    ),
    op!(
        "layout.edit", "Modifica layout", "materiali", Some("layout_edit"),
        "Applica operazioni strutturate o interpreta un prompt italiano; gli elementi bloccati non vengono mossi.",
        "Specifica aggiornata, pagine e operazioni.",
        mutating: false, destructive: false, idempotent: false,
        params: [p("spec", "object", true, "LayoutSpecification"), p("prompt", "string", false, "Richiesta"), p("operations", "array", false, "Operazioni tipizzate")]
    ),
    op!(
        "library.catalog", "Catalogo biblioteca", "materiali", Some("library_catalog"),
        "Categorie, sottocategorie e conteggi reali della biblioteca.",
        "Catalogo.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "library.search", "Cerca in biblioteca", "materiali", Some("library_search"),
        "Ricerca per parola, categoria, pubblico e tipo di interazione.",
        "Pagina di risultati con anteprime.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("query", "string", false, "Testo"), p("category", "string", false, "Categoria"), p("limit", "integer", false, "Default 24"), p("offset", "integer", false, "Default 0"), p("sort", "string", false, "Ordine")]
    ),
    op!(
        "library.validate", "Valida biblioteca", "materiali", Some("library_validate"),
        "Controlli strutturali ed editoriali su tutti i documenti. Operazione lunga: preferisci un job.",
        "Report con errori, avvisi e conteggi.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("only_pilots", "boolean", false, "Solo i piloti"), p("max_issues", "integer", false, "Default 80")]
    ),
    op!(
        "dev.library.packages", "Pacchetti libreria (dev)", "sviluppo", Some("dev_library_packages"),
        "Elenca i pacchetti e i documenti della libreria. Richiede la modalità sviluppatore attiva (CAA_WRITER_DEV_LIBRARY=1 o --dev-library, solo build debug).",
        "Pacchetti con documenti, percorsi e conteggi.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "dev.library.open", "Apri documento libreria (dev)", "sviluppo", Some("dev_library_open"),
        "Apre un documento .caadoc della libreria per percorso relativo. Richiede la modalità sviluppatore.",
        "Documento universale e percorso.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("path", "string", true, "Percorso relativo alla libreria, es. library/templates/foo.caadoc")]
    ),
    op!(
        "dev.library.preview", "Anteprima documento libreria (dev)", "sviluppo", Some("dev_library_preview"),
        "Restituisce l'anteprima SVG della prima pagina di un documento della libreria. Richiede la modalità sviluppatore.",
        "SVG.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("path", "string", true, "Percorso relativo alla libreria")]
    ),
    op!(
        "dev.library.save", "Salva documento libreria (dev)", "sviluppo", Some("dev_library_save"),
        "Salva un UniversalDocument in un percorso della libreria e aggiorna catalogo e anteprima. Distruttivo: richiede conferma.",
        "Voce di catalogo aggiornata.",
        mutating: true, destructive: true, idempotent: false,
        params: [p("document", "object", true, "UniversalDocument completo"), p("path", "string", true, "Percorso relativo alla libreria"), CONFIRM]
    ),
    op!(
        "dev.library.replace", "Sostituisci documento libreria (dev)", "sviluppo", Some("dev_library_replace"),
        "Sostituisce un documento della libreria con un altro .caadoc scelto dal disco, mantenendo l'ID della voce. Distruttivo: richiede conferma.",
        "Voce di catalogo aggiornata.",
        mutating: true, destructive: true, idempotent: false,
        params: [p("path", "string", true, "Percorso relativo alla libreria"), p("source", "string", true, "File .caadoc di origine"), CONFIRM]
    ),
    op!(
        "dev.library.add", "Aggiungi documento libreria (dev)", "sviluppo", Some("dev_library_add"),
        "Copia un .caadoc nella libreria, genera un ID univoco e lo aggiunge al catalogo. Richiede la modalità sviluppatore.",
        "Voce di catalogo creata.",
        mutating: true, destructive: false, idempotent: false,
        params: [
            p("source", "string", true, "File .caadoc di origine"),
            p("folder", "string", true, "boards oppure templates"),
            p("category_id", "string", true, "ID categoria"),
            p("subcategory_id", "string", true, "ID sottocategoria"),
            p("category_name", "string", false, "Nome categoria"),
            p("subcategory_name", "string", false, "Nome sottocategoria")
        ]
    ),
    op!(
        "dev.library.delete", "Elimina documento libreria (dev)", "sviluppo", Some("dev_library_delete"),
        "Elimina un documento dalla libreria, dalla sua anteprima e dal catalogo. Distruttivo: richiede conferma.",
        "Esito dell'eliminazione.",
        mutating: true, destructive: true, idempotent: false,
        params: [p("path", "string", true, "Percorso relativo alla libreria"), CONFIRM]
    ),
    op!(
        "prompt.interpret", "Interpreta prompt", "materiali", Some("prompt_interpret"),
        "Converte una richiesta italiana in operazioni documentali validate; il modello interviene solo se il parser deterministico non capisce.",
        "Piano con operazioni, descrizioni e warning.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("prompt", "string", true, "Richiesta"), p("use_neural", "boolean", false, "Default true")]
    ),

    // ------------------------------------------------------------------
    // Import, repertori e profili
    // ------------------------------------------------------------------
    op!(
        "import.scan", "Analizza importazione", "repertori", Some("import_scan"),
        "Visita file o cartelle, riconosce i formati dai byte, calcola checksum e duplicati. Non importa nulla.",
        "Anteprima con elementi e proposta di collezione.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("paths", "array", true, "File o cartelle")]
    ),
    op!(
        "import.run", "Importa pittogrammi", "repertori", Some("import_run"),
        "Importa gli elementi selezionati in una collezione personale con licenza e metadati.",
        "Report imported/skipped/failed.",
        mutating: true, destructive: false, idempotent: false,
        params: [p("entries", "array", true, "Elementi approvati"), p("collection", "string", false, "Collezione"), p("license", "object", false, "Licenza")]
    ),
    op!(
        "collections.list", "Elenca collezioni", "repertori", Some("collections_list"),
        "Elenca le collezioni personali con conteggi.",
        "Array di collezioni.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "collections.rename", "Rinomina collezione", "repertori", Some("collections_rename"),
        "Rinomina una collezione personale.",
        "Oggetto {ok}.",
        mutating: true, destructive: false, idempotent: true,
        params: [p("id", "string", true, "ID collezione"), p("name", "string", true, "Nuovo nome")]
    ),
    op!(
        "collections.delete", "Elimina collezione", "repertori", Some("collections_delete"),
        "Elimina una collezione e i suoi file. Richiede confirm=true.",
        "Conteggio elementi rimossi.",
        mutating: true, destructive: true, idempotent: false,
        params: [p("id", "string", true, "ID collezione"), CONFIRM]
    ),
    op!(
        "collections.delete_symbols", "Elimina simboli", "repertori", Some("collections_delete_symbols"),
        "Elimina simboli personali specifici. Richiede confirm=true.",
        "Conteggio rimosso.",
        mutating: true, destructive: true, idempotent: false,
        params: [p("ids", "array", true, "ID simbolo"), CONFIRM]
    ),
    op!(
        "collections.move", "Sposta simboli", "repertori", Some("collections_move"),
        "Sposta simboli personali in un'altra collezione.",
        "Conteggio spostato.",
        mutating: true, destructive: false, idempotent: true,
        params: [p("ids", "array", true, "ID simbolo"), p("collection_id", "string", true, "Destinazione")]
    ),
    op!(
        "profiles.list", "Elenca profili", "profili", Some("profiles_list"),
        "Elenca i profili locali, senza dati sanitari.",
        "Array di profili.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "boards.list", "Elenca sistemi", "profili", Some("boards_list"),
        "Elenca i sistemi comunicativi disponibili.",
        "Array di sistemi.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "boards.build", "Costruisci sistema", "profili", Some("boards_build"),
        "Risolve i pittogrammi e restituisce il grafo navigabile del sistema.",
        "Board.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("id", "string", true, "ID sistema")]
    ),

    // ------------------------------------------------------------------
    // Rete, IA, voce, immagini
    // ------------------------------------------------------------------
    op!(
        "online.providers", "Provider online", "rete", Some("online_providers"),
        "Elenca i provider e il loro stato reale (abilitato/disabilitato).",
        "Array di provider.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "online.search", "Ricerca online", "rete", Some("online_search"),
        "Ricerca federata ARASAAC/Global Symbols/Openverse con cache 24h. Richiede rete; invia solo lemmi filtrati dal lessico.",
        "Risultati con fonte, licenza e warning.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("terms", "array", true, "Lemmi, mai frasi"), p("force_refresh", "boolean", false, "Ignora cache")]
    ),
    op!(
        "online.ensure", "Importa copie online", "rete", Some("online_ensure"),
        "Cerca e salva copie offline ARASAAC per le parole indicate.",
        "Simboli salvati.",
        mutating: true, destructive: false, idempotent: true,
        params: [p("words", "array", true, "Parole")]
    ),
    op!(
        "online.autofill", "Autofill online", "rete", Some("online_autofill"),
        "Importa il primo risultato ARASAAC quando manca un'etichetta esatta.",
        "Simboli salvati.",
        mutating: true, destructive: false, idempotent: true,
        params: [p("words", "array", true, "Parole")]
    ),
    op!(
        "online.global", "Global Symbols", "rete", Some("online_global"),
        "Cerca solo su Global Symbols e restituisce i risultati senza importarli.",
        "Risultati e warning.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("term", "string", true, "Termine")]
    ),
    op!(
        "online.english", "Alternativa inglese", "rete", Some("online_english"),
        "Traduce, cerca su Global Symbols e importa fino a `limit` copie personali.",
        "Termine e alternative.",
        mutating: true, destructive: false, idempotent: true,
        params: [p("word", "string", true, "Parola italiana"), p("limit", "integer", false, "Default 3")]
    ),
    op!(
        "online.save", "Salva risultato online", "rete", Some("online_save"),
        "Salva un risultato online scelto come copia offline con licenza e attribuzione.",
        "Alternative pronta per l'applicazione.",
        mutating: true, destructive: false, idempotent: false,
        params: [p("result", "object", true, "Risultato online"), p("lemma", "string", true, "Lemma"), p("collection", "string", false, "Collezione")]
    ),
    op!(
        "english.alternatives", "Alternative inglesi", "rete", Some("english_alternatives"),
        "Traduce in inglese, cerca ARASAAC inglese e importa le copie offline.",
        "Termine e alternative.",
        mutating: true, destructive: false, idempotent: true,
        params: [p("word", "string", true, "Parola"), p("limit", "integer", false, "Default 4")]
    ),
    op!(
        "translate.search", "Traduci per la ricerca", "rete", Some("translate_search"),
        "Traduce un termine per la ricerca italiana usando glossario o modello locale.",
        "Testo tradotto.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("text", "string", true, "Testo")]
    ),
    op!(
        "translate.english", "Traduci in inglese", "rete", Some("translate_english"),
        "Traduce un termine italiano in inglese con glossario o modello locale.",
        "Testo tradotto.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("text", "string", true, "Testo")]
    ),
    op!(
        "ai.status", "Stato IA", "ia", Some("ai_get_status"),
        "Stato del modello locale, backend e metriche. Nessun processo viene avviato solo per leggere lo stato.",
        "AiStatus.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "ai.models", "Modelli IA", "ia", Some("ai_list_models"),
        "Catalogo dei modelli con stato installato e compatibilità hardware.",
        "Array di modelli.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "ai.chat", "Chat IA", "ia", Some("ai_chat"),
        "Invia messaggi al modello locale o al provider online configurato. Può essere lento.",
        "Risposta, backend e tempo.",
        mutating: false, destructive: false, idempotent: false,
        params: [p("messages", "array", true, "Messaggi {role,content}")]
    ),
    op!(
        "ai.tool", "Strumento IA", "ia", Some("ai_run_tool"),
        "Esegue uno strumento IA tipizzato con validazione e fallback deterministico.",
        "Risultato dello strumento.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("tool", "string", true, "Nome strumento"), p("input", "object", false, "Input")]
    ),
    op!(
        "ai.affinity", "Sinonimi IA", "ia", Some("ai_affinity"),
        "Chiede al modello parole affini e restituisce i simboli trovati; se il modello non c'è non inventa nulla.",
        "Alternative.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("word", "string", true, "Parola")]
    ),
    op!(
        "ai.words", "Parole IA", "ia", Some("ai_words"),
        "Parole grezze proposte dal modello (diagnostica).",
        "Array di parole.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("word", "string", true, "Parola")]
    ),
                    op!(
        "voice.status", "Stato voce", "voce", Some("voice_get_status"),
        "Motore vocale (neurale o di sistema), modello, licenza e cache.",
        "Stato voce.",
        mutating: false, destructive: false, idempotent: true,
        params: [p("language", "string", false, "it o en")]
    ),
    op!(
        "voice.speak", "Pronuncia testo", "voce", Some("voice_speak"),
        "Sintetizza e riproduce il testo. Può usare la voce di sistema; non modifica il documento.",
        "Motore usato e percorso WAV quando neurale.",
        mutating: false, destructive: false, idempotent: false,
        params: [p("text", "string", true, "Testo"), p("language", "string", false, "Lingua")]
    ),
    op!(
        "speak", "Pronuncia (alias)", "voce", None,
        "Alias CLI di `voice.speak`.",
        "Motore usato.",
        mutating: false, destructive: false, idempotent: false,
        params: [p("text", "string", true, "Testo")]
    ),

    // ------------------------------------------------------------------
    // Pacchetti ed esportazioni legacy
    // ------------------------------------------------------------------
    op!(
        "packages.catalog", "Catalogo pacchetti", "pacchetti", Some("packages_catalog"),
        "Pacchetti di simboli e librerie disponibili con stato installato.",
        "Catalogo pacchetti.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "packages.updates", "Aggiornamenti pacchetti", "pacchetti", Some("packages_updates"),
        "Verifica aggiornamenti dei pacchetti installati.",
        "Stato aggiornamenti.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "packages.download", "Scarica pacchetto", "pacchetti", Some("packages_download"),
        "Scarica un pacchetto di simboli. Richiede rete e può essere lungo.",
        "Report download.",
        mutating: true, destructive: false, idempotent: true,
        params: [p("id", "string", true, "ID pacchetto"), p("limit", "integer", false, "Limite elementi")]
    ),
    op!(
        "packages.set_library", "Attiva libreria", "pacchetti", Some("packages_set_library"),
        "Abilita o disabilita una libreria.",
        "Stato.",
        mutating: true, destructive: false, idempotent: true,
        params: [p("id", "string", true, "ID"), p("enabled", "boolean", true, "Stato")]
    ),
    op!(
        "packages.remove", "Rimuovi pacchetto", "pacchetti", Some("packages_remove"),
        "Rimuove un pacchetto di simboli. Richiede confirm=true.",
        "Pacchetto rimosso.",
        mutating: true, destructive: true, idempotent: false,
        params: [p("id", "string", true, "ID"), CONFIRM]
    ),
    op!(
        "document.open", "Apri .caadoc", "documenti", Some("document_open"),
        "Carica un documento .caadoc legacy o universale senza salvarlo.",
        "Documento caricato.",
        mutating: false, destructive: false, idempotent: true,
        params: [PATH]
    ),
    op!(
        "document.save", "Salva .caadoc", "documenti", Some("document_save"),
        "Salva un DocumentPayload nel formato .caadoc con scrittura atomica.",
        "Risultato esportazione.",
        mutating: true, destructive: false, idempotent: true,
        params: [PATH, p("payload", "object", true, "DocumentPayload")]
    ),
    op!(
        "document.pdf", "PDF documento", "documenti", Some("document_pdf"),
        "Genera il PDF di un UniversalDocument.",
        "Percorso e byte.",
        mutating: true, destructive: false, idempotent: true,
        params: [DOCUMENT, OUT_REQUIRED]
    ),
    op!(
        "export.html", "Esporta HTML legacy", "documenti", Some("export_html"),
        "Esporta un ExportDoc in HTML con gli asset del motore.",
        "Percorso generato.",
        mutating: true, destructive: false, idempotent: true,
        params: [PATH, DOCUMENT]
    ),
    op!(
        "export.docx", "Esporta DOCX legacy", "documenti", Some("export_docx"),
        "Esporta un ExportDoc in DOCX con gli asset del motore.",
        "Percorso generato.",
        mutating: true, destructive: false, idempotent: true,
        params: [PATH, DOCUMENT]
    ),

    // ------------------------------------------------------------------
    // Motore decisionale semantico (Horizon multilingual-zeroshot-small)
    // ------------------------------------------------------------------
    op!(
        "decision.status", "Stato del motore decisionale", "decisione", Some("decision_status"),
        "Stato reale del DecisionEngine, del modello Horizon installato e della cache. Non avvia inferenze.",
        "Stato del motore e del modello.",
        mutating: false, destructive: false, idempotent: true,
        params: [],
        example: "caa-cli call decision.status '{}'"
    ),
    op!(
        "decision.classify", "Classificazione semantica", "decisione", Some("decision_classify"),
        "Classifica un testo fra alternative predefinite usando il DecisionEngine condiviso. Il modello non esegue comandi e non modifica documenti.",
        "Risposta decisionale con ranking, azione e metadati.",
        mutating: false, destructive: false, idempotent: true,
        params: [
            p("task", "string", true, "Tipo di decisione: pictogram_disambiguation, communication_intent, semantic_category, ..."),
            p("text", "string", true, "Testo da classificare"),
            p("target", "object", false, "Intervallo {text,start,end} in caratteri Unicode"),
            p("candidates", "array", false, "Candidati {id,label,hypothesis?,locked?,blocked?}; se assente usa la tassonomia predefinita"),
            p("mode", "string", false, "suggest (predefinito) o automatic"),
            p("multi_label", "boolean", false, "Classificazione a più etichette"),
            p("timeout_ms", "integer", false, "Timeout dell'inferenza"),
            p("priority", "string", false, "high, medium o low"),
            p("taxonomy_version", "string", false, "Versione della tassonomia usata nella cache"),
            p("apply_threshold", "number", false, "Soglia di applicazione automatica"),
            p("suggest_threshold", "number", false, "Soglia di suggerimento"),
        ],
        example: "caa-cli call decision.classify '{\"task\":\"communication_intent\",\"text\":\"Vorrei bere dell'acqua.\"}'"
    ),
    op!(
        "decision.audit", "Diagnostica decisioni", "decisione", Some("decision_audit"),
        "Ultimi eventi del motore decisionale, senza testo utente né dati personali.",
        "Elenco di eventi tecnici.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "decision.validate", "Valida un dataset", "decisione", Some("decision_validate"),
        "Esegue un insieme di casi italiani sul DecisionEngine reale e riporta accuratezza, astensioni, copertura, errori e latenza media. Non modifica nulla.",
        "Metriche reali del dataset.",
        mutating: false, destructive: false, idempotent: true,
        params: [
            p("cases", "array", true, "Casi con task, text, target, candidates ed expected"),
            p("split", "string", false, "all (predefinito), calibration o validation"),
            p("suggest_threshold", "number", false, "Soglia di suggerimento da usare nella valutazione"),
            p("apply_threshold", "number", false, "Soglia di applicazione da usare nella valutazione"),
        ],
        example: "caa-cli call decision.validate '{\"cases\":[...]}'"
    ),
    op!(
        "decision.calibrate", "Calibra le soglie", "decisione", Some("decision_calibrate"),
        "Esegue i casi su Horizon e propone soglie di suggerimento/applicazione con copertura e precisione misurate. Non modifica la configurazione.",
        "Soglie proposte e metriche per task.",
        mutating: false, destructive: false, idempotent: true,
        params: [
            p("cases", "array", true, "Casi con task, text, target, candidates ed expected"),
            p("split", "string", false, "calibration (predefinito), validation o all"),
        ],
        example: "caa-cli call decision.calibrate '{\"cases\":[...]}'"
    ),
    op!(
        "decision.model.status", "Stato modello Horizon", "decisione", Some("decision_model_status"),
        "Verifica presenza e integrità attesa dei file del modello Horizon nella cartella modelli.",
        "Stato del modello.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "decision.model.verify", "Verifica modello Horizon", "decisione", Some("decision_model_verify"),
        "Calcola gli SHA-256 dei file del modello installato. Può richiedere qualche secondo.",
        "Report di verifica per file.",
        mutating: false, destructive: false, idempotent: true,
        params: []
    ),
    op!(
        "decision.model.install", "Installa modello Horizon", "decisione", Some("decision_model_install"),
        "Scarica i file ufficiali del modello Horizon e li installa in modo atomico. Richiede rete; dopo l'installazione funziona offline.",
        "Report di installazione.",
        mutating: true, destructive: false, idempotent: true,
        params: [p("force", "boolean", false, "Reinstalla anche se già presente")]
    ),
    op!(
        "decision.model.remove", "Rimuovi modello Horizon", "decisione", Some("decision_model_remove"),
        "Rimuove i pesi locali del modello Horizon. Richiede confirm=true. Le funzioni deterministiche restano attive.",
        "Esito della rimozione.",
        mutating: true, destructive: true, idempotent: false,
        params: [CONFIRM]
    ),
];

pub fn find(name: &str) -> Option<&'static Operation> {
    OPERATIONS.iter().find(|operation| operation.name == name)
}

pub fn methods() -> Vec<&'static str> {
    OPERATIONS.iter().map(|operation| operation.name).collect()
}

pub fn operations_by_category(category: Option<&str>) -> Vec<&'static Operation> {
    OPERATIONS
        .iter()
        .filter(|operation| match category { Some(value) => operation.category == value, None => true })
        .collect()
}

pub fn categories() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for operation in OPERATIONS {
        if !out.contains(&operation.category) {
            out.push(operation.category);
        }
    }
    out
}

fn param_schema(param: &Param) -> Value {
    let mut schema = json!({
        "type": param.kind,
        "description": param.description,
    });
    if param.required {
        schema["x-required"] = json!(true);
    }
    if let Some(default) = param.default {
        schema["default"] = json!(default);
    }
    if !param.enum_values.is_empty() {
        schema["enum"] = json!(param.enum_values);
    }
    if param.kind == "array" {
        schema["items"] = json!({"type": "string"});
    }
    schema
}

pub fn input_schema(operation: &Operation) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for param in operation.params {
        properties.insert(param.name.to_string(), param_schema(param));
        if param.required {
            required.push(json!(param.name));
        }
    }
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": operation.free_form_params,
        "x-caa-method": operation.name,
    })
}

pub fn output_schema(operation: &Operation) -> Value {
    json!({
        "type": "object",
        "description": operation.output,
    })
}

pub fn operation_json(operation: &Operation) -> Value {
    json!({
        "name": operation.name,
        "tool": operation.mcp,
        "title": operation.title,
        "category": operation.category,
        "description": operation.description,
        "input_schema": input_schema(operation),
        "output": operation.output,
        "output_schema": output_schema(operation),
        "mutating": operation.mutating,
        "destructive": operation.destructive,
        "idempotent": operation.idempotent,
        "requires_confirmation": operation.destructive,
        "free_form_params": operation.free_form_params,
        "example": operation.example,
    })
}

pub fn catalog(version: &str, category: Option<&str>) -> Value {
    let operations: Vec<Value> = operations_by_category(category)
        .into_iter()
        .map(operation_json)
        .collect();
    json!({
        "application": "CAA Writer",
        "version": version,
        "api_version": API_VERSION,
        "schema_dialect": "JSON Schema (subset 2020-12)",
        "categories": categories(),
        "operations": operations,
        "access_modes": ["read", "write", "admin"],
        "notes": [
            "Le operazioni distruttive richiedono confirm=true e la modalità admin.",
            "Le operazioni mutating vengono rifiutate in modalità read.",
            "I parametri legacy free_form_params sono documentati in docs/AGENT.md.",
            "Il server MCP espone solo le operazioni con `tool` valorizzato; le altre restano alias CLI."
        ],
        "limitations": [
            "Le modifiche al documento ProseMirror aperto nella GUI richiedono un evento di ricaricamento: il bridge notifica, non simula la digitazione.",
            "Le operazioni di rete dichiarano l'errore quando offline; non simulano risultati.",
            "I job esistono solo nel processo che li avvia: usa `caa-cli rpc` o il server MCP, non una singola invocazione CLI.",
            "L'annullamento dei job è cooperativo."
        ]
    })
}

pub fn mcp_tools() -> Vec<Value> {
    OPERATIONS
        .iter()
        .filter_map(|operation| {
            let name = operation.mcp?;
            let mut schema = input_schema(operation);
            // `dry_run` è un controllo del trasporto MCP, non un parametro del
            // metodo: viene rimosso prima del dispatch.
            if let Some(properties) = schema.get_mut("properties").and_then(|value| value.as_object_mut()) {
                properties.insert(
                    "dry_run".to_string(),
                    json!({
                        "type": "boolean",
                        "description": "Mostra cosa farebbe l'operazione senza eseguirla (solo metodi che scrivono)."
                    }),
                );
            }
            let mut tool = json!({
                "name": name,
                "description": format!(
                    "{}. Categoria: {}. Metodo CLI: {}.{}",
                    operation.description,
                    operation.category,
                    operation.name,
                    if operation.destructive {
                        " OPERAZIONE DISTRUTTIVA: richiede confirm=true e modalità admin."
                    } else if operation.mutating {
                        " Modifica dati persistenti."
                    } else {
                        " Sola lettura."
                    }
                ),
                "inputSchema": schema,
                "annotations": {
                    "title": operation.title,
                    "readOnlyHint": !operation.mutating,
                    "destructiveHint": operation.destructive,
                    "idempotentHint": operation.idempotent,
                    "openWorldHint": matches!(operation.category, "rete" | "ia"),
                },
                "x-caa-method": operation.name,
            });
            if let Some(example) = operation.example {
                tool["x-caa-example"] = json!(example);
            }
            Some(tool)
        })
        .collect()
}

pub fn mcp_method_for_tool(tool: &str) -> Option<&'static str> {
    OPERATIONS
        .iter()
        .find(|operation| operation.mcp == Some(tool))
        .map(|operation| operation.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn registry_names_are_unique() {
        let mut seen = HashSet::new();
        for operation in OPERATIONS {
            assert!(seen.insert(operation.name), "duplicato: {}", operation.name);
        }
        let mut tools = HashSet::new();
        for operation in OPERATIONS {
            if let Some(tool) = operation.mcp {
                assert!(tools.insert(tool), "tool duplicato: {tool}");
            }
        }
    }

    #[test]
    fn mcp_names_are_stable_and_prefixed() {
        for operation in OPERATIONS {
            if let Some(tool) = operation.mcp {
                assert!(
                    tool.chars().all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()),
                    "nome tool non valido: {tool}"
                );
            }
        }
    }

    #[test]
    fn catalog_is_derived_from_registry() {
        let catalog = catalog("test", None);
        assert_eq!(
            catalog["operations"].as_array().unwrap().len(),
            OPERATIONS.len()
        );
        assert_eq!(catalog["api_version"], API_VERSION);
    }

    #[test]
    fn destructive_operations_require_confirmation() {
        for operation in OPERATIONS.iter().filter(|operation| operation.destructive) {
            assert!(
                operation.params.iter().any(|param| param.name == "confirm"),
                "operazione distruttiva senza confirm: {}",
                operation.name
            );
        }
    }

    #[test]
    fn mcp_tools_expose_dry_run_without_changing_methods() {
        for tool in mcp_tools() {
            assert_eq!(
                tool["inputSchema"]["properties"]["dry_run"]["type"], "boolean",
                "tool senza dry_run: {}",
                tool["name"]
            );
        }
    }

    #[test]
    fn every_mcp_tool_maps_back_to_one_method() {
        for operation in OPERATIONS {
            if let Some(tool) = operation.mcp {
                assert_eq!(mcp_method_for_tool(tool), Some(operation.name));
            }
        }
    }
}
