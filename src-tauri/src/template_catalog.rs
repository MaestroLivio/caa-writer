//! Tassonomia e ricerca della biblioteca dei materiali.
//!
//! La biblioteca nasce da specifiche versionate, ma un elenco piatto non è
//! utilizzabile da un docente. Questo modulo aggiunge una classificazione
//! stabile (categorie e sottocategorie), sinonimi di ricerca, filtri e
//! metadati editoriali. Non modifica i documenti: li organizza.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::universal::{LibraryEntry, LibraryManifest};

#[derive(Debug, Clone, Serialize)]
pub struct Subcategory {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct Category {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub subcategories: Vec<Subcategory>,
}

fn sub(id: &'static str, name: &'static str, description: &'static str) -> Subcategory {
    Subcategory { id, name, description }
}

/// Tassonomia ufficiale della libreria. Gli identificativi sono stabili: la
/// GUI e la CLI possono salvarli senza dipendere dalle etichette.
pub fn taxonomy() -> Vec<Category> {
    vec![
        Category {
            id: "aac",
            name: "Comunicazione aumentativa e alternativa",
            description: "Tabelle, quaderni, strisce e scelte per comunicare in ogni contesto.",
            subcategories: vec![
                sub("tabelle", "Tabelle di comunicazione", "Griglie con vocabolario organizzato per tema."),
                sub("vocabolario-essenziale", "Vocabolario essenziale", "Nucleo di parole ad alta frequenza."),
                sub("tematiche", "Tabelle tematiche", "Cibi, casa, scuola, giochi, luoghi e persone."),
                sub("quaderni", "Quaderni di comunicazione", "Raccolte multipagina con navigazione."),
                sub("passaporti", "Passaporti comunicativi", "Presentazione personale e bisogni essenziali."),
                sub("strisce", "Strisce-frase", "Sequenze orizzontali per comporre frasi."),
                sub("scelte", "Scelta tra alternative", "Due o più opzioni con risposta immediata."),
                sub("richieste", "Richieste e bisogni", "Aiuto, pausa, oggetti, attenzione."),
                sub("emozioni", "Comunicazione delle emozioni", "Stati interni, intensità e strategie."),
                sub("scuola", "Comunicazione a scuola", "Lezioni, materiali, compagni, difficoltà."),
                sub("casa", "Comunicazione a casa", "Famiglia, routine domestiche, preferenze."),
                sub("pubblici", "Ambienti pubblici", "Negozi, trasporti, sanitario, tempo libero."),
            ],
        },
        Category {
            id: "storie-caa",
            name: "Storie in CAA",
            description: "Racconti e libri illustrati con pittogrammi sopra o sotto il testo.",
            subcategories: vec![
                sub("racconti-brevi", "Racconti brevi", "Una o poche frasi per pagina."),
                sub("fiabe", "Fiabe", "Narrazioni classiche riscritte in CAA."),
                sub("sequenziali", "Racconti sequenziali", "Azioni ordinate e nessi causali."),
                sub("libri-illustrati", "Libri illustrati", "Immagine grande e testo breve."),
                sub("sopra-testo", "Pittogrammi sopra il testo", "Simbolo, parola scritta, frase."),
                sub("sotto-testo", "Pittogrammi sotto il testo", "Frase leggibile e simboli di appoggio."),
                sub("senza-testo", "Libri senza testo", "Solo immagini per narrare."),
                sub("interattivi", "Libri interattivi", "Navigazione, scelte e lettura ad alta voce."),
                sub("comprensione", "Domande di comprensione", "Verifica dopo la lettura."),
                sub("personalizzabili", "Racconti personalizzabili", "Personaggi e ambienti modificabili."),
                sub("personaggi", "Personaggi ricorrenti", "Coerenza visiva tra le pagine."),
            ],
        },
        Category {
            id: "storie-sociali",
            name: "Storie sociali e anticipazione",
            description: "Narrazioni rispettose per capire situazioni, routine e imprevisti.",
            subcategories: vec![
                sub("sociali", "Storie sociali", "Testi informativi secondo criteri dichiarati."),
                sub("descrittive", "Storie descrittive", "Che cosa accade e chi è coinvolto."),
                sub("anticipazione", "Anticipazione di eventi", "Prepararsi a ciò che verrà."),
                sub("routine", "Cambiamenti di routine", "Prevedere variazioni e transizioni."),
                sub("ambienti", "Nuovi ambienti", "Prima visita a luoghi e servizi."),
                sub("visite", "Visite e appuntamenti", "Medico, dentista, parco, biblioteca."),
                sub("scuola", "Esperienze scolastiche", "Ingresso, verifiche, gite, ricreazione."),
                sub("relazioni", "Relazioni e interazioni", "Amicizia, turni, conflitti, confini."),
                sub("imprevisti", "Gestione degli imprevisti", "Piani alternativi e flessibilità."),
                sub("viaggi", "Uscite e viaggi", "Preparazione e svolgimento."),
                sub("emozioni", "Emozioni", "Riconoscere e comunicare stati interni."),
            ],
        },
        Category {
            id: "agende",
            name: "Agende e organizzazione visiva",
            description: "Tempo, sequenze e transizioni rappresentati in modo leggibile.",
            subcategories: vec![
                sub("giornaliera", "Agenda giornaliera", "Mattina, pomeriggio, sera."),
                sub("settimanale", "Agenda settimanale", "Cinque o sette giorni con attività."),
                sub("mensile", "Calendario mensile", "Mese, settimane, eventi."),
                sub("routine", "Routine", "Sequenze stabili della giornata."),
                sub("sequenze", "Sequenze operative", "Passi numerati con fine riconoscibile."),
                sub("prima-dopo", "Prima/dopo", "Due momenti collegati."),
                sub("prima-poi", "Prima/poi", "Attività corrente e attività successiva."),
                sub("orari", "Sequenze con orari", "Tempo e durata espliciti."),
                sub("transizioni", "Transizioni", "Preparazione al cambio di attività."),
                sub("checklist", "Checklist visive", "Cosa è fatto e cosa manca."),
                sub("scuola", "Attività scolastiche", "Orario, materie, materiali."),
            ],
        },
        Category {
            id: "teacch",
            name: "Materiali TEACCH",
            description: "Struttura visibile, lavoro autonomo e compiti con inizio e fine.",
            subcategories: vec![
                sub("sequenze-lavoro", "Sequenze di lavoro", "Passi da seguire in ordine."),
                sub("sistemi", "Sistemi di lavoro autonomo", "Cosa fare, quanto, come finire, cosa dopo."),
                sub("abbinamenti", "Abbinamenti", "Uguali, associazioni, coppie."),
                sub("classificazioni", "Classificazioni", "Categorie e criteri."),
                sub("ordinamenti", "Ordinamenti", "Serie, grandezze, successioni."),
                sub("corrispondenze", "Corrispondenze", "Quantità, numero, simbolo."),
                sub("discriminazione", "Discriminazione visiva", "Dettagli e differenze."),
                sub("completamento", "Completamento", "Parti mancanti e chiusura del compito."),
                sub("ritagliare", "Da ritagliare e associare", "Cartellini e basi."),
                sub("organizzatori", "Organizzatori di attività", "Spazi e materiali definiti."),
                sub("inizio-fine", "Inizio e fine riconoscibili", "Indicatori di avanzamento."),
            ],
        },
        Category {
            id: "comportamentali",
            name: "Supporti comportamentali",
            description: "Scelte, pausa, autoregolazione e comunicazione dei bisogni.",
            subcategories: vec![
                sub("token", "Token board", "Gettoni configurabili verso un obiettivo concordato."),
                sub("scelte", "Scelte visive", "Due o più alternative rispettose."),
                sub("pausa", "Richiesta di pausa", "Fermarsi e riprendere."),
                sub("bisogni", "Comunicazione di bisogni", "Aiuto, spazio, acqua, bagno."),
                sub("autoregolazione", "Autoregolazione", "Strategie calmanti e tempi."),
                sub("scale", "Scale delle emozioni", "Intensità e segnali."),
                sub("autovalutazione", "Autovalutazione", "Come sto andando, cosa ho imparato."),
                sub("alternative", "Alternative comunicative", "Più modi per chiedere."),
                sub("rientro", "Recupero e rientro", "Tornare all'attività dopo una pausa."),
            ],
        },
        Category {
            id: "schede",
            name: "Schede didattiche",
            description: "Esercizi disciplinari con istruzioni, spazio di risposta e soluzioni.",
            subcategories: vec![
                sub("italiano", "Italiano", "Pregrafismo, lettura, scrittura, grammatica."),
                sub("matematica", "Matematica", "Numero, calcolo, problemi, geometria, misure."),
                sub("inglese", "Inglese", "Vocaboli, ascolto, frasi, CLIL."),
                sub("scienze", "Scienze", "Osservare, classificare, cicli, organismi."),
                sub("storia", "Storia", "Tempo, fonti, sequenze, cause."),
                sub("geografia", "Geografia", "Mappe, ambienti, paesaggi, orientamento."),
                sub("civica", "Educazione civica", "Regole, diritti, responsabilità, convivenza."),
                sub("arte", "Arte e immagine", "Colore, forma, composizione."),
                sub("musica", "Musica", "Ritmo, ascolto, figure musicali."),
                sub("tecnologia", "Tecnologia", "Strumenti, coding unplugged, procedure."),
                sub("motoria", "Educazione motoria", "Movimento, equilibrio, coordinazione."),
                sub("interdisciplinari", "Attività interdisciplinari", "Collegamenti tra discipline."),
                sub("prerequisiti", "Prerequisiti", "Competenze di ingresso alla primaria."),
                sub("potenziamento", "Potenziamento cognitivo", "Memoria, attenzione, logica."),
                sub("inclusive", "Attività inclusive", "Versioni facilitate e adattamenti."),
            ],
        },
        Category {
            id: "giochi",
            name: "Giochi e attività interattive",
            description: "Giochi con regole chiare, da stampare o eseguire a schermo.",
            subcategories: vec![
                sub("memory", "Memory", "Coppie da trovare."),
                sub("tombola", "Tombola", "Cartelle e chiamate."),
                sub("domino", "Domino", "Tessere da collegare."),
                sub("puzzle", "Puzzle", "Parti da ricomporre."),
                sub("associazioni", "Associazioni", "Collegare elementi correlati."),
                sub("sequenze", "Sequenze", "Ordinare passi o eventi."),
                sub("scelte-multiple", "Scelte multiple", "Una risposta tra più opzioni."),
                sub("ordinamenti", "Ordinamenti", "Serie e criteri."),
                sub("classificazioni", "Classificazioni", "Raggruppare per categoria."),
                sub("lettura", "Giochi di lettura", "Parole, sillabe, frasi."),
                sub("matematica", "Giochi matematici", "Numero e calcolo."),
                sub("ascolto", "Attività di ascolto", "Indizi e consegne."),
            ],
        },
        Category {
            id: "stampabili",
            name: "Materiali stampabili",
            description: "Carte, etichette, poster e materiali manipolativi pronti da tagliare.",
            subcategories: vec![
                sub("flashcard", "Flashcard", "Carte con immagine e parola."),
                sub("etichette", "Etichette", "Nomi per oggetti e spazi."),
                sub("cartellini", "Cartellini", "Carte di piccole dimensioni."),
                sub("cartelloni", "Cartelloni", "Poster da parete."),
                sub("segnaletica", "Segnaletica", "Indicazioni per ambienti."),
                sub("strisce", "Strisce visive", "Sequenze da appendere."),
                sub("schede-plastificabili", "Schede plastificabili", "Attività riutilizzabili."),
                sub("carte-ritagliare", "Carte da ritagliare", "Materiali manipolativi."),
                sub("manipolativi", "Materiali manipolativi", "Oggetti e supporti tattili."),
                sub("poster", "Poster educativi", "Sintesi visive per la classe."),
            ],
        },
        Category {
            id: "personali",
            name: "Documenti personali",
            description: "Ciò che hai creato, importato o salvato come preferito.",
            subcategories: vec![
                sub("recenti", "Documenti recenti", "Aperti di recente su questo dispositivo."),
                sub("preferiti", "Preferiti", "Materiali segnati con la stella."),
                sub("creati", "Modelli creati", "Documenti nati nel tuo spazio di lavoro."),
                sub("importati", "Modelli importati", "File importati da altri dispositivi."),
                sub("raccolte", "Raccolte personalizzate", "Gruppi scelti da te."),
            ],
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Classification {
    pub category_id: String,
    pub category: String,
    pub subcategory_id: String,
    pub subcategory: String,
    pub audience: String,
    pub objective: String,
    pub tags: Vec<String>,
    pub complexity: String,
    pub needs_symbols: bool,
    pub printable: bool,
    pub interactive: bool,
    pub formats: Vec<String>,
}

fn normalized(value: &str) -> String {
    value
        .to_lowercase()
        .nfd()
        .filter(|ch| !matches!(ch, '\u{0300}'..='\u{036f}'))
        .collect::<String>()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn first_sentence(text: &str, limit: usize) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let end = trimmed
        .find(|ch| matches!(ch, '.' | '!' | '?'))
        .map(|index| index + 1)
        .unwrap_or(trimmed.len());
    let sentence = trimmed[..end].trim();
    if sentence.chars().count() <= limit {
        sentence.to_string()
    } else {
        let mut out: String = sentence.chars().take(limit.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn infer_audience(haystack: &str) -> String {
    if contains_any(haystack, &["infanzia", "materna", "nido", "3 anni", "4 anni", "5 anni"]) {
        "infanzia".to_string()
    } else if contains_any(haystack, &["secondaria", "medie", "superiori", "adolescent"]) {
        "secondaria".to_string()
    } else if contains_any(haystack, &["adulti", "adulto", "genitori", "lavoro"]) {
        "adulti".to_string()
    } else {
        "primaria".to_string()
    }
}

fn infer_subcategory(category_id: &str, haystack: &str) -> &'static str {
    match category_id {
        "schede" => {
            if contains_any(haystack, &["addiz", "sottraz", "moltiplic", "division", "contar", "numero", "numeri", "calcolo", "problem", "frazion", "geometr", "misur", "monet", "orologio", "abaco", "linea dei numeri"]) {
                "matematica"
            } else if contains_any(haystack, &["vocal", "consonant", "sillab", "parol", "frase", "lettur", "comprension", "ortograf", "grammatic", "sinonim", "contrari", "pregraf", "alfabet", "digramm", "trigramm"]) {
                "italiano"
            } else if contains_any(haystack, &["inglese", "english", "colour", "colors", "clil"]) {
                "inglese"
            } else if contains_any(haystack, &["scienz", "pianta", "animal", "ciclo", "corpo", "esperiment", "classific", "materia", "energia"]) {
                "scienze"
            } else if contains_any(haystack, &["storia", "linea del tempo", "fonti", "prima e dopo", "causa"]) {
                "storia"
            } else if contains_any(haystack, &["geograf", "mappa", "paesagg", "ambient", "orientament", "cartina"]) {
                "geografia"
            } else if contains_any(haystack, &["civica", "regol", "diritt", "responsabil", "convivenz", "cittadin"]) {
                "civica"
            } else if contains_any(haystack, &["arte", "colore", "disegn", "immagine", "forma"]) {
                "arte"
            } else if contains_any(haystack, &["music", "ritmo", "strument", "body percussion"]) {
                "musica"
            } else if contains_any(haystack, &["tecnolog", "coding", "computer", "robot", "procedur"]) {
                "tecnologia"
            } else if contains_any(haystack, &["motor", "moviment", "equilibri", "coordinazion", "corpo in movimento"]) {
                "motoria"
            } else if contains_any(haystack, &["prerequisit", "ingresso", "pregrafismo", "infanzia"]) {
                "prerequisiti"
            } else if contains_any(haystack, &["memoria", "attenzion", "logic", "potenziament", "ragionament"]) {
                "potenziamento"
            } else if contains_any(haystack, &["inclusiv", "facilitat", "semplificat", "adattament"]) {
                "inclusive"
            } else {
                "interdisciplinari"
            }
        }
        "aac" => {
            if contains_any(haystack, &["quadern", "libro", "raccolta"]) {
                "quaderni"
            } else if contains_any(haystack, &["passaport"]) {
                "passaporti"
            } else if contains_any(haystack, &["striscia", "frase"]) {
                "strisce"
            } else if contains_any(haystack, &["scelta", "scegli", "opzioni"]) {
                "scelte"
            } else if contains_any(haystack, &["richiest", "bisogn", "aiuto", "pausa"]) {
                "richieste"
            } else if contains_any(haystack, &["emozion", "sentiment", "umore"]) {
                "emozioni"
            } else if contains_any(haystack, &["scuola", "classe", "compagn", "maestr"]) {
                "scuola"
            } else if contains_any(haystack, &["casa", "famiglia", "mamma", "papa"]) {
                "casa"
            } else if contains_any(haystack, &["negozio", "trasport", "medico", "ospedale", "pubblic", "bar", "ristorante"]) {
                "pubblici"
            } else if contains_any(haystack, &["essenzial", "nucleo", "core"]) {
                "vocabolario-essenziale"
            } else if contains_any(haystack, &["tema", "animali", "cibi", "giochi", "vestiti", "corpo"]) {
                "tematiche"
            } else {
                "tabelle"
            }
        }
        "storie-caa" => {
            if contains_any(haystack, &["fiaba", "fiabe", "favola"]) {
                "fiabe"
            } else if contains_any(haystack, &["senza testo", "solo immagini"]) {
                "senza-testo"
            } else if contains_any(haystack, &["interattiv", "scegli", "naviga"]) {
                "interattivi"
            } else if contains_any(haystack, &["comprension", "domande"]) {
                "comprensione"
            } else if contains_any(haystack, &["personagg", "protagonista"]) {
                "personaggi"
            } else if contains_any(haystack, &["sequenz", "passi"]) {
                "sequenziali"
            } else if contains_any(haystack, &["illustrat", "immagine grande"]) {
                "libri-illustrati"
            } else if contains_any(haystack, &["sotto il testo", "didascalia"]) {
                "sotto-testo"
            } else {
                "racconti-brevi"
            }
        }
        "storie-sociali" => {
            if contains_any(haystack, &["anticip", "domani", "prossim"]) {
                "anticipazione"
            } else if contains_any(haystack, &["routine", "cambiament", "transizion"]) {
                "routine"
            } else if contains_any(haystack, &["medico", "dentista", "visita", "appuntament"]) {
                "visite"
            } else if contains_any(haystack, &["scuola", "classe", "verifica", "gita"]) {
                "scuola"
            } else if contains_any(haystack, &["amic", "litig", "turno", "confine", "relazion"]) {
                "relazioni"
            } else if contains_any(haystack, &["imprevist", "cambia", "alternativ"]) {
                "imprevisti"
            } else if contains_any(haystack, &["viaggio", "uscita", "gita", "vacanza"]) {
                "viaggi"
            } else if contains_any(haystack, &["emozion", "arrabbi", "triste", "paura", "felice"]) {
                "emozioni"
            } else if contains_any(haystack, &["nuovo", "ambiente", "luogo"]) {
                "ambienti"
            } else {
                "sociali"
            }
        }
        "agende" => {
            if contains_any(haystack, &["mensile", "mese", "calendario"]) {
                "mensile"
            } else if contains_any(haystack, &["settiman", "settimana"]) {
                "settimanale"
            } else if contains_any(haystack, &["giornalier", "giornata", "oggi"]) {
                "giornaliera"
            } else if contains_any(haystack, &["prima", "poi", "dopo"]) {
                "prima-poi"
            } else if contains_any(haystack, &["checklist", "fatto", "completat"]) {
                "checklist"
            } else if contains_any(haystack, &["orario", "ora", "durata"]) {
                "orari"
            } else if contains_any(haystack, &["scuola", "materie", "lezione"]) {
                "scuola"
            } else {
                "routine"
            }
        }
        "teacch" => {
            if contains_any(haystack, &["abbin", "coppia", "uguale"]) {
                "abbinamenti"
            } else if contains_any(haystack, &["classific", "categori", "raggrupp"]) {
                "classificazioni"
            } else if contains_any(haystack, &["ordina", "serie", "grandezz"]) {
                "ordinamenti"
            } else if contains_any(haystack, &["corrispond", "quantita", "numero"]) {
                "corrispondenze"
            } else if contains_any(haystack, &["discrimin", "differenz", "dettagl"]) {
                "discriminazione"
            } else if contains_any(haystack, &["complet", "mancant"]) {
                "completamento"
            } else if contains_any(haystack, &["ritagli", "cartellin", "associare"]) {
                "ritagliare"
            } else if contains_any(haystack, &["sistema", "lavoro autonomo", "quanto lavoro"]) {
                "sistemi"
            } else if contains_any(haystack, &["inizio", "fine", "finito"]) {
                "inizio-fine"
            } else {
                "sequenze-lavoro"
            }
        }
        "comportamentali" => {
            if contains_any(haystack, &["token", "getton"]) {
                "token"
            } else if contains_any(haystack, &["rinforz", "scelta", "scegli", "opzioni"]) {
                "scelte"
            } else if contains_any(haystack, &["pausa", "fermarsi"]) {
                "pausa"
            } else if contains_any(haystack, &["bisogn", "aiuto", "acqua", "bagno"]) {
                "bisogni"
            } else if contains_any(haystack, &["scala", "intensita", "termometro"]) {
                "scale"
            } else if contains_any(haystack, &["autovalut", "come sto", "imparato"]) {
                "autovalutazione"
            } else if contains_any(haystack, &["rientro", "recuper", "riprendere"]) {
                "rientro"
            } else if contains_any(haystack, &["alternativ", "comunicare"]) {
                "alternative"
            } else {
                "autoregolazione"
            }
        }
        "giochi" => {
            if contains_any(haystack, &["memory", "coppie"]) {
                "memory"
            } else if contains_any(haystack, &["tombola", "cartella"]) {
                "tombola"
            } else if contains_any(haystack, &["domino", "tessera"]) {
                "domino"
            } else if contains_any(haystack, &["puzzle", "ricomporre"]) {
                "puzzle"
            } else if contains_any(haystack, &["associaz", "colleg"]) {
                "associazioni"
            } else if contains_any(haystack, &["sequenz", "ordina"]) {
                "sequenze"
            } else if contains_any(haystack, &["scelta multipla", "quiz"]) {
                "scelte-multiple"
            } else if contains_any(haystack, &["classific", "categori"]) {
                "classificazioni"
            } else if contains_any(haystack, &["lettura", "sillab", "parola"]) {
                "lettura"
            } else if contains_any(haystack, &["matematic", "numero", "calcolo"]) {
                "matematica"
            } else if contains_any(haystack, &["ascolto", "indizio"]) {
                "ascolto"
            } else {
                "associazioni"
            }
        }
        "stampabili" => {
            if contains_any(haystack, &["flashcard", "flash card"]) {
                "flashcard"
            } else if contains_any(haystack, &["etichett", "nome"]) {
                "etichette"
            } else if contains_any(haystack, &["cartellone", "poster"]) {
                "cartelloni"
            } else if contains_any(haystack, &["segnal", "indicazion"]) {
                "segnaletica"
            } else if contains_any(haystack, &["striscia"]) {
                "strisce"
            } else if contains_any(haystack, &["plastific", "riutilizz"]) {
                "schede-plastificabili"
            } else if contains_any(haystack, &["ritagli", "carte"]) {
                "carte-ritagliare"
            } else if contains_any(haystack, &["manipol", "tattile"]) {
                "manipolativi"
            } else {
                "cartellini"
            }
        }
        _ => "recenti",
    }
}

fn legacy_category(category: &str) -> (&'static str, &'static str) {
    match category {
        "Sistemi comunicativi" => ("aac", "quaderni"),
        "Tabelle comunicative" => ("aac", "tabelle"),
        "Storie sociali" => ("storie-sociali", "sociali"),
        "Agende e calendari visivi" => ("agende", "giornaliera"),
        "Sequenze e routine" => ("agende", "sequenze"),
        "Materiali TEACCH" => ("teacch", "sequenze-lavoro"),
        "Materiali ABA e supporti visivi" => ("comportamentali", "token"),
        "Materiali didattici" => ("schede", "interdisciplinari"),
        "Giochi e attività interattive" => ("giochi", "associazioni"),
        "Emozioni e autoregolazione" => ("comportamentali", "autoregolazione"),
        "Autonomia personale" => ("agende", "routine"),
        _ => ("schede", "interdisciplinari"),
    }
}

fn tag_words(text: &str, limit: usize) -> Vec<String> {
    const STOP: &[&str] = &[
        "della", "delle", "degli", "dello", "dalla", "dalle", "nella", "nelle", "con", "senza",
        "per", "una", "uno", "gli", "che", "del", "dei", "dai", "come", "sono", "questo",
        "questa", "attivita", "materiale", "scheda", "storia", "tabella", "modello",
    ];
    let mut seen: Vec<String> = Vec::new();
    for word in normalized(text).split(|ch: char| !ch.is_alphanumeric() && ch != '-') {
        if word.chars().count() < 4 || STOP.contains(&word) {
            continue;
        }
        if !seen.iter().any(|existing| existing == word) {
            seen.push(word.to_string());
        }
        if seen.len() >= limit {
            break;
        }
    }
    seen
}

/// Classifica una voce della biblioteca. I metadati espliciti della specifica
/// hanno la precedenza; per i materiali storici si ricava la categoria dal
/// nome e dalla descrizione, senza inventare contenuti.
pub fn classify(entry: &LibraryEntry) -> Classification {
    let haystack = normalized(&format!(
        "{} {} {}",
        entry.title, entry.category, entry.description
    ));
    let (mut category_id, mut subcategory_id) = if !entry.category_id.is_empty() {
        (entry.category_id.clone(), entry.subcategory_id.clone())
    } else {
        let (category, sub) = legacy_category(&entry.category);
        (category.to_string(), sub.to_string())
    };
    if category_id == "schede" || category_id == "giochi" || category_id == "stampabili" {
        // Le discipline si ricavano dal contenuto, non solo dalla vecchia
        // etichetta «Materiali didattici».
        if entry.subcategory_id.is_empty() {
            subcategory_id = infer_subcategory(&category_id, &haystack).to_string();
        }
    } else if subcategory_id.is_empty() {
        subcategory_id = infer_subcategory(&category_id, &haystack).to_string();
    }
    if category_id == "comportamentali"
        && subcategory_id == "token"
        && !contains_any(&haystack, &["token", "getton"])
    {
        // La vecchia etichetta «Materiali ABA» non rende tutte le risorse
        // token board: osservazione, scelta e pausa hanno strutture diverse.
        subcategory_id = infer_subcategory(&category_id, &haystack).to_string();
    }
    if subcategory_id.is_empty() {
        subcategory_id = infer_subcategory(&category_id, &haystack).to_string();
    }
    let categories = taxonomy();
    let category = categories
        .iter()
        .find(|item| item.id == category_id)
        .unwrap_or(&categories[0]);
    let subcategory = category
        .subcategories
        .iter()
        .find(|item| item.id == subcategory_id)
        .or_else(|| category.subcategories.first());
    if subcategory.is_none() {
        category_id = "schede".to_string();
        subcategory_id = "interdisciplinari".to_string();
    }
    let category = categories
        .iter()
        .find(|item| item.id == category_id)
        .unwrap_or(&categories[0]);
    let subcategory = category
        .subcategories
        .iter()
        .find(|item| item.id == subcategory_id)
        .unwrap_or(&category.subcategories[0]);
    let audience = if entry.audience.is_empty() {
        infer_audience(&haystack)
    } else {
        entry.audience.clone()
    };
    let objective = if entry.objective.is_empty() {
        first_sentence(&entry.description, 180)
    } else {
        entry.objective.clone()
    };
    let mut tags = entry.tags.clone();
    if tags.is_empty() {
        tags = tag_words(&format!("{} {}", entry.title, entry.description), 10);
    }
    let complexity = if entry.complexity.is_empty() {
        if entry.pages > 4 {
            "articolato".to_string()
        } else if entry.elements > 24 {
            "intermedio".to_string()
        } else {
            "base".to_string()
        }
    } else {
        entry.complexity.clone()
    };
    let interactive = entry.interactive;
    let mut formats = vec!["stampa".to_string()];
    if interactive {
        formats.push("interattivo".to_string());
    }
    if entry.kind.contains("communication") {
        formats.push("player".to_string());
    }
    Classification {
        category_id: category.id.to_string(),
        category: category.name.to_string(),
        subcategory_id: subcategory.id.to_string(),
        subcategory: subcategory.name.to_string(),
        audience,
        objective,
        tags,
        complexity,
        needs_symbols: entry.needs_symbols || entry.missing_symbols > 0,
        printable: entry.printable || !interactive,
        interactive,
        formats,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CatalogQuery {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub subcategory: Option<String>,
    #[serde(default)]
    pub audience: Option<String>,
    #[serde(default)]
    pub interactive: Option<bool>,
    #[serde(default)]
    pub printable: Option<bool>,
    #[serde(default)]
    pub needs_symbols: Option<bool>,
    #[serde(default)]
    pub source_type: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogItem {
    #[serde(flatten)]
    pub entry: LibraryEntry,
    pub classification: Classification,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_svg: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogPage {
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub items: Vec<CatalogItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CountedSubcategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CountedCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub count: usize,
    pub subcategories: Vec<CountedSubcategory>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogOverview {
    pub total: usize,
    pub categories: Vec<CountedCategory>,
    pub audiences: BTreeMap<String, usize>,
    pub interactive: usize,
    pub printable: usize,
    pub needs_symbols: usize,
}

fn synonym_expansion(term: &str) -> Vec<&'static str> {
    match term {
        "token" | "gettoni" | "rinforzo" => {
            vec!["token", "gettoni", "comportamentali", "supporti"]
        }
        "storia" | "storie" | "sociale" | "sociali" => {
            vec!["storie", "sociali", "sociale", "narrazione"]
        }
        "caa" | "comunicazione" | "comunicare" | "aac" => {
            vec!["comunicazione", "tabelle", "simboli", "aac"]
        }
        "matematica" | "calcolo" | "numeri" => {
            vec!["matematica", "calcolo", "numero", "numeri", "addizioni", "sottrazioni"]
        }
        "italiano" | "lettura" | "scrittura" | "sillabe" => {
            vec!["italiano", "lettura", "scrittura", "sillabe", "parole", "frasi"]
        }
        "agenda" | "orario" | "settimana" | "routine" => {
            vec!["agenda", "orario", "settimana", "routine", "sequenze"]
        }
        "teacch" | "autonomia" | "lavoro" => {
            vec!["teacch", "autonomia", "lavoro", "sequenze", "sistema"]
        }
        "emozioni" | "rabbia" | "paura" | "calma" => {
            vec!["emozioni", "rabbia", "paura", "calma", "autoregolazione"]
        }
        "gioco" | "giochi" | "memory" | "tombola" => {
            vec!["giochi", "memory", "tombola", "domino", "attivita"]
        }
        "schede" | "esercizi" | "compiti" => {
            vec!["schede", "esercizi", "didattiche", "compiti"]
        }
        "stampabili" | "flashcard" | "carte" | "etichette" => {
            vec!["stampabili", "flashcard", "carte", "etichette", "cartellini"]
        }
        _ => Vec::new(),
    }
}

fn score(entry: &LibraryEntry, classification: &Classification, query: &str) -> i32 {
    let normalized_query = normalized(query);
    if normalized_query.trim().is_empty() {
        return 1;
    }
    let title = normalized(&entry.title);
    let description = normalized(&entry.description);
    let category = normalized(&format!("{} {}", classification.category, classification.subcategory));
    let objective = normalized(&classification.objective);
    let tags = normalized(&classification.tags.join(" "));
    let mut total = 0;
    if title.contains(&normalized_query) {
        total += 24;
    }
    let mut terms: Vec<String> = normalized_query
        .split_whitespace()
        .filter(|term| term.chars().count() >= 2)
        .map(str::to_string)
        .collect();
    if terms.is_empty() {
        terms.push(normalized_query.clone());
    }
    let mut expanded: Vec<String> = terms.clone();
    for term in &terms {
        for synonym in synonym_expansion(term) {
            expanded.push(synonym.to_string());
        }
    }
    expanded.sort();
    expanded.dedup();
    for term in &expanded {
        if title.contains(term) {
            total += 12;
        }
        if category.contains(term) {
            total += 8;
        }
        if objective.contains(term) {
            total += 5;
        }
        if tags.contains(term) {
            total += 4;
        }
        if description.contains(term) {
            total += 2;
        }
        if classification.audience.contains(term) {
            total += 1;
        }
    }
    total
}

pub fn overview(manifest: &LibraryManifest) -> CatalogOverview {
    let categories = taxonomy();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut subcounts: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    let mut audiences: BTreeMap<String, usize> = BTreeMap::new();
    let mut interactive = 0;
    let mut printable = 0;
    let mut needs_symbols = 0;
    for entry in &manifest.entries {
        let class = classify(entry);
        *counts.entry(category_id_ref(&class.category_id)).or_default() += 1;
        *subcounts
            .entry((category_id_ref(&class.category_id), subcategory_id_ref(&class.subcategory_id)))
            .or_default() += 1;
        *audiences.entry(class.audience.clone()).or_default() += 1;
        if class.interactive {
            interactive += 1;
        }
        if class.printable {
            printable += 1;
        }
        if class.needs_symbols {
            needs_symbols += 1;
        }
    }
    CatalogOverview {
        total: manifest.entries.len(),
        categories: categories
            .into_iter()
            .map(|category| CountedCategory {
                id: category.id.to_string(),
                name: category.name.to_string(),
                description: category.description.to_string(),
                count: counts.get(category.id).copied().unwrap_or(0),
                subcategories: category
                    .subcategories
                    .into_iter()
                    .map(|sub| CountedSubcategory {
                        id: sub.id.to_string(),
                        name: sub.name.to_string(),
                        description: sub.description.to_string(),
                        count: subcounts
                            .get(&(category.id, sub.id))
                            .copied()
                            .unwrap_or(0),
                    })
                    .collect(),
            })
            .collect(),
        audiences,
        interactive,
        printable,
        needs_symbols,
    }
}

// Le chiavi della mappa devono vivere quanto la tassonomia: gli ID sono
// stringhe statiche nella tassonomia, quindi si possono riusare.
fn category_id_ref(id: &str) -> &'static str {
    for category in taxonomy().iter() {
        if category.id == id {
            return category.id;
        }
        for sub in &category.subcategories {
            if sub.id == id {
                return sub.id;
            }
        }
    }
    "schede"
}

fn subcategory_id_ref(id: &str) -> &'static str {
    for category in taxonomy().iter() {
        for sub in &category.subcategories {
            if sub.id == id {
                return sub.id;
            }
        }
    }
    "interdisciplinari"
}

pub fn search(manifest: &LibraryManifest, query: &CatalogQuery) -> CatalogPage {
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(24).clamp(1, 60);
    let mut items: Vec<(i32, CatalogItem)> = Vec::new();
    for entry in &manifest.entries {
        let classification = classify(entry);
        if let Some(category) = &query.category {
            if !category.is_empty()
                && category != "tutti"
                && &classification.category_id != category
                && classification.category != *category
            {
                continue;
            }
        }
        if let Some(subcategory) = &query.subcategory {
            if !subcategory.is_empty()
                && subcategory != "tutte"
                && &classification.subcategory_id != subcategory
                && classification.subcategory != *subcategory
            {
                continue;
            }
        }
        if let Some(audience) = &query.audience {
            if !audience.is_empty() && audience != "tutte" && &classification.audience != audience {
                continue;
            }
        }
        if let Some(interactive) = query.interactive {
            if classification.interactive != interactive {
                continue;
            }
        }
        if let Some(printable) = query.printable {
            if classification.printable != printable {
                continue;
            }
        }
        if let Some(needs_symbols) = query.needs_symbols {
            if classification.needs_symbols != needs_symbols {
                continue;
            }
        }
        if let Some(source_type) = &query.source_type {
            if !source_type.is_empty() && &entry.source_type != source_type {
                continue;
            }
        }
        let value = score(
            entry,
            &classification,
            query.query.as_deref().unwrap_or("").trim(),
        );
        if query.query.as_deref().unwrap_or("").trim().is_empty() || value > 0 {
            items.push((
                value,
                CatalogItem {
                    entry: entry.clone(),
                    classification,
                    preview_svg: None,
                },
            ));
        }
    }
    let sort = query.sort.as_deref().unwrap_or("rilevanza");
    match sort {
        "nome" | "name" => items.sort_by(|a, b| a.1.entry.title.to_lowercase().cmp(&b.1.entry.title.to_lowercase())),
        "pagine" | "pages" => items.sort_by(|a, b| b.1.entry.pages.cmp(&a.1.entry.pages)),
        "complessita" | "complexity" => items.sort_by(|a, b| {
            let rank = |value: &str| match value {
                "base" => 0,
                "intermedio" => 1,
                _ => 2,
            };
            rank(&a.1.classification.complexity).cmp(&rank(&b.1.classification.complexity))
        }),
        _ => items.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.entry.title.to_lowercase().cmp(&b.1.entry.title.to_lowercase()))
        }),
    }
    let total = items.len();
    let items: Vec<CatalogItem> = items
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(_, item)| item)
        .collect();
    CatalogPage {
        total,
        offset,
        limit,
        items,
    }
}

/// Legge l'anteprima già prodotta dalla pipeline. Se manca non inventa
/// un'immagine: restituisce `None`.
/// Restituisce le voci richieste nell'ordine degli ID, senza inventare
/// documenti assenti.
pub fn by_ids(manifest: &LibraryManifest, ids: &[String]) -> Vec<CatalogItem> {
    let mut items = Vec::new();
    for id in ids {
        if let Some(entry) = manifest.entries.iter().find(|entry| &entry.id == id) {
            items.push(CatalogItem {
                entry: entry.clone(),
                classification: classify(entry),
                preview_svg: None,
            });
        }
    }
    items
}

pub fn preview_svg(resources: &Path, entry: &LibraryEntry) -> Option<String> {
    let relative = if entry.preview.is_empty() {
        format!("library/previews/{}.svg", entry.id)
    } else {
        entry.preview.clone()
    };
    let path = resources.join(relative.strip_prefix("resources/").unwrap_or(&relative));
    std::fs::read_to_string(path).ok()
}

pub fn load_manifest(resources: &Path) -> LibraryManifest {
    crate::universal::load_library_manifest(resources)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, title: &str, category: &str, description: &str) -> LibraryEntry {
        LibraryEntry {
            id: id.to_string(),
            title: title.to_string(),
            category: category.to_string(),
            description: description.to_string(),
            source_type: "template".to_string(),
            kind: "worksheet".to_string(),
            pages: 1,
            elements: 10,
            interactive: false,
            missing_symbols: 0,
            path: String::new(),
            preview: String::new(),
            category_id: String::new(),
            subcategory_id: String::new(),
            subcategory: String::new(),
            audience: String::new(),
            objective: String::new(),
            tags: Vec::new(),
            complexity: String::new(),
            needs_symbols: true,
            printable: true,
            customizable: true,
            featured: false,
        }
    }

    #[test]
    fn worksheets_are_classified_by_subject() {
        let math = classify(&entry(
            "add",
            "Addizioni entro il 10",
            "Materiali didattici",
            "Esercizi di calcolo con numeri entro dieci.",
        ));
        assert_eq!(math.category_id, "schede");
        assert_eq!(math.subcategory_id, "matematica");
        let italian = classify(&entry(
            "sil",
            "Parole bisillabe",
            "Materiali didattici",
            "Lettura e scrittura di sillabe e parole.",
        ));
        assert_eq!(italian.subcategory_id, "italiano");
    }

    #[test]
    fn search_finds_synonyms_and_respects_filters() {
        let mut manifest = LibraryManifest::default();
        manifest.entries = vec![
            entry(
                "token",
                "Token board con cinque gettoni",
                "Materiali ABA e supporti visivi",
                "Gettoni per un obiettivo concordato.",
            ),
            entry(
                "story",
                "La gita scolastica",
                "Storie sociali",
                "Anticipazione di una gita.",
            ),
        ];
        let page = search(
            &manifest,
            &CatalogQuery {
                query: Some("rinforzo".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].entry.id, "token");
        let page = search(
            &manifest,
            &CatalogQuery {
                category: Some("storie-sociali".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].entry.id, "story");
    }

    #[test]
    fn taxonomy_has_all_required_sections() {
        let categories = taxonomy();
        let ids: Vec<&str> = categories.iter().map(|item| item.id).collect();
        for expected in [
            "aac",
            "storie-caa",
            "storie-sociali",
            "agende",
            "teacch",
            "comportamentali",
            "schede",
            "giochi",
            "stampabili",
        ] {
            assert!(ids.contains(&expected), "categoria mancante: {expected}");
        }
        for category in &categories {
            assert!(!category.subcategories.is_empty());
        }
    }
}
