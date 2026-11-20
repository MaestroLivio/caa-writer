//! Registro dei task decisionali e delle tassonomie predefinite.
//!
//! Le etichette predefinite sono italiane e stabili. Un chiamante può sempre
//! fornire i propri candidati; il registro li valida soltanto.

use serde_json::json;

use crate::decision::types::{Candidate, DecisionTask};

#[derive(Debug, Clone, Copy)]
pub struct TaskSpec {
    pub task: DecisionTask,
    pub title: &'static str,
    pub description: &'static str,
    pub advisory_only: bool,
}

pub const TASKS: &[TaskSpec] = &[
    TaskSpec {
        task: DecisionTask::PictogramDisambiguation,
        title: "Disambiguazione dei pittogrammi",
        description:
            "Sceglie fra significati alternativi della stessa parola usando il contesto della frase.",
        advisory_only: false,
    },
    TaskSpec {
        task: DecisionTask::CommunicationIntent,
        title: "Intenzione comunicativa",
        description: "Classifica una frase in una o più intenzioni comunicative predefinite.",
        advisory_only: false,
    },
    TaskSpec {
        task: DecisionTask::SemanticCategory,
        title: "Categoria semantica",
        description: "Assegna un concetto a una o più categorie della tassonomia.",
        advisory_only: false,
    },
    TaskSpec {
        task: DecisionTask::BoardCategory,
        title: "Categoria della tabella comunicativa",
        description: "Suggerisce in quale tabella comunicativa usare una frase.",
        advisory_only: false,
    },
    TaskSpec {
        task: DecisionTask::SemanticSearchRanking,
        title: "Ricerca semantica",
        description: "Riordina i pittogrammi già recuperati rispetto a una descrizione.",
        advisory_only: false,
    },
    TaskSpec {
        task: DecisionTask::DidacticTemplateSelection,
        title: "Selezione del modello didattico",
        description: "Sceglie fra i modelli didattici realmente disponibili.",
        advisory_only: true,
    },
    TaskSpec {
        task: DecisionTask::LearningObjective,
        title: "Obiettivo didattico",
        description: "Classifica l'obiettivo di un'attività fra quelli dichiarati.",
        advisory_only: true,
    },
    TaskSpec {
        task: DecisionTask::ExerciseType,
        title: "Tipologia di esercizio",
        description: "Classifica la tipologia di un esercizio.",
        advisory_only: true,
    },
    TaskSpec {
        task: DecisionTask::SocialStoryTopic,
        title: "Tema della storia sociale",
        description: "Individua il tema di una storia sociale esistente.",
        advisory_only: true,
    },
    TaskSpec {
        task: DecisionTask::ExpressedEmotion,
        title: "Emozione espressa",
        description: "Classifica l'emozione espressa dal testo, non lo stato psicologico reale.",
        advisory_only: true,
    },
    TaskSpec {
        task: DecisionTask::InteractiveAction,
        title: "Azione del pulsante",
        description: "Propone l'identificativo di un'azione supportata dal player.",
        advisory_only: false,
    },
    TaskSpec {
        task: DecisionTask::LayoutTemplate,
        title: "Layout",
        description: "Suggerisce una tipologia di layout; la geometria resta al motore di impaginazione.",
        advisory_only: false,
    },
    TaskSpec {
        task: DecisionTask::CaaSemanticCoherence,
        title: "Coerenza semantica CAA",
        description: "Segnala una possibile incongruenza fra testo, parola e pittogramma.",
        advisory_only: true,
    },
    TaskSpec {
        task: DecisionTask::ContentCataloguing,
        title: "Catalogazione dei contenuti",
        description: "Propone metadati di catalogo per un materiale.",
        advisory_only: true,
    },
    TaskSpec {
        task: DecisionTask::CommandRouting,
        title: "Instradamento dei comandi",
        description: "Classifica una richiesta rispetto ai comandi supportati; non li esegue.",
        advisory_only: true,
    },
];

pub fn task_spec(task: DecisionTask) -> &'static TaskSpec {
    TASKS
        .iter()
        .find(|spec| spec.task == task)
        .unwrap_or(&TASKS[0])
}

fn candidate(id: &str, label: &str) -> Candidate {
    Candidate {
        id: id.to_string(),
        label: label.to_string(),
        hypothesis: None,
        locked: false,
        blocked: false,
        metadata: json!({}),
    }
}

/// Candidati predefiniti, quando il dominio ha una tassonomia stabile.
/// Per i task che dipendono dal documento (pittogrammi, ricerca, modelli) il
/// chiamante deve fornire i propri candidati.
pub fn default_candidates(task: DecisionTask) -> Vec<Candidate> {
    match task {
        DecisionTask::CommunicationIntent => vec![
            candidate("richiesta", "una richiesta"),
            candidate("rifiuto", "un rifiuto"),
            candidate("domanda", "una domanda"),
            candidate("saluto", "un saluto"),
            candidate("affermazione", "un'affermazione"),
            candidate("bisogno", "un bisogno"),
            candidate("preferenza", "una preferenza"),
            candidate("ringraziamento", "un ringraziamento"),
            candidate("scusa", "una scusa"),
            candidate("richiesta_aiuto", "una richiesta di aiuto"),
            candidate("commento", "un commento"),
            candidate("espressione_emotiva", "un'espressione emotiva"),
        ],
        DecisionTask::SemanticCategory => vec![
            candidate("persone", "persone"),
            candidate("animali", "animali"),
            candidate("alimenti", "alimenti"),
            candidate("bevande", "bevande"),
            candidate("oggetti", "oggetti"),
            candidate("luoghi", "luoghi"),
            candidate("azioni", "azioni"),
            candidate("emozioni", "emozioni"),
            candidate("bisogni", "bisogni"),
            candidate("abbigliamento", "abbigliamento"),
            candidate("materiale_scolastico", "materiale scolastico"),
            candidate("trasporti", "trasporti"),
            candidate("natura", "natura"),
            candidate("tempo", "tempo"),
            candidate("corpo", "corpo"),
            candidate("igiene", "igiene"),
            candidate("salute", "salute"),
            candidate("gioco", "gioco"),
            candidate("relazioni_sociali", "relazioni sociali"),
        ],
        DecisionTask::BoardCategory => vec![
            candidate("bisogni", "bisogni"),
            candidate("azioni", "azioni"),
            candidate("cibo", "cibo"),
            candidate("persone", "persone"),
            candidate("luoghi", "luoghi"),
            candidate("emozioni", "emozioni"),
            candidate("attivita", "attività"),
            candidate("routine", "routine"),
            candidate("corpo", "corpo"),
            candidate("scuola", "scuola"),
        ],
        DecisionTask::ExpressedEmotion => vec![
            candidate("gioia", "gioia"),
            candidate("tristezza", "tristezza"),
            candidate("rabbia", "rabbia"),
            candidate("paura", "paura"),
            candidate("sorpresa", "sorpresa"),
            candidate("preoccupazione", "preoccupazione"),
            candidate("frustrazione", "frustrazione"),
            candidate("soddisfazione", "soddisfazione"),
            candidate("incertezza", "incertezza"),
        ],
        DecisionTask::InteractiveAction => vec![
            candidate("append", "aggiungere la parola alla frase"),
            candidate("speak", "leggere la frase ad alta voce"),
            candidate("toggle", "selezionare la cella per un esercizio"),
            candidate("navigate", "andare a un'altra pagina"),
            candidate("back", "tornare alla pagina precedente"),
            candidate("home", "andare alla pagina iniziale"),
            candidate("clear", "svuotare la frase"),
        ],
        DecisionTask::LayoutTemplate => vec![
            candidate("tabella_comunicativa", "tabella comunicativa"),
            candidate("sequenza_temporale", "sequenza temporale"),
            candidate("agenda_visiva", "agenda visiva"),
            candidate("scheda_esercizi", "scheda di esercizi"),
            candidate("storia_caa", "storia CAA"),
            candidate("storia_sociale", "storia sociale"),
            candidate("tabella_scelta", "tabella di scelta"),
            candidate("documento_narrativo", "documento narrativo"),
        ],
        DecisionTask::LearningObjective => vec![
            candidate("comprensione", "comprensione"),
            candidate("denominazione", "denominazione"),
            candidate("associazione", "associazione"),
            candidate("categorizzazione", "categorizzazione"),
            candidate("sequenziamento", "sequenziamento"),
            candidate("consapevolezza_fonologica", "consapevolezza fonologica"),
            candidate("lettura", "lettura"),
            candidate("scrittura", "scrittura"),
            candidate("calcolo", "calcolo"),
            candidate("confronto", "confronto"),
            candidate("ordinamento", "ordinamento"),
            candidate("classificazione_logica", "classificazione logica"),
            candidate("comprensione_temporale", "comprensione temporale"),
            candidate("comprensione_spaziale", "comprensione spaziale"),
        ],
        DecisionTask::ExerciseType => vec![
            candidate("scelta_multipla", "scelta multipla"),
            candidate("abbinamento", "abbinamento"),
            candidate("ordinamento", "ordinamento"),
            candidate("completamento", "completamento"),
            candidate("classificazione", "classificazione"),
            candidate("sequenza_temporale", "sequenza temporale"),
            candidate(
                "associazione_parola_immagine",
                "associazione parola-immagine",
            ),
            candidate("associazione_suono_immagine", "associazione suono-immagine"),
            candidate("composizione_frase", "composizione di frase"),
        ],
        DecisionTask::SocialStoryTopic => vec![
            candidate("aspettare_il_turno", "aspettare il turno"),
            candidate("chiedere_aiuto", "chiedere aiuto"),
            candidate("affrontare_cambiamenti", "affrontare cambiamenti"),
            candidate("gestire_conflitti", "gestire conflitti"),
            candidate("prepararsi_a_una_visita", "prepararsi a una visita"),
            candidate("rispettare_regole_condivise", "rispettare regole condivise"),
            candidate("esprimere_bisogni", "esprimere bisogni"),
            candidate("comunicare_emozioni", "comunicare emozioni"),
            candidate(
                "partecipare_ad_attivita_collettive",
                "partecipare ad attività collettive",
            ),
        ],
        DecisionTask::CaaSemanticCoherence => vec![
            candidate("coerente", "coerente con il significato della frase"),
            candidate("incoerente", "non coerente con il significato della frase"),
            candidate("ambiguo", "ambiguo e da verificare"),
        ],
        DecisionTask::ContentCataloguing => vec![
            candidate("italiano", "italiano"),
            candidate("matematica", "matematica"),
            candidate("scienze", "scienze"),
            candidate("storia", "storia"),
            candidate("geografia", "geografia"),
            candidate("inglese", "inglese"),
            candidate("educazione_civica", "educazione civica"),
            candidate("arte", "arte e immagine"),
            candidate("musica", "musica"),
            candidate("educazione_motoria", "educazione motoria"),
        ],
        DecisionTask::CommandRouting => vec![
            candidate("CREATE_GRID", "creare una griglia"),
            candidate("CREATE_BOARD", "creare una tabella comunicativa"),
            candidate("OPEN_DOCUMENT", "aprire un documento"),
            candidate("EXPORT_PDF", "esportare in PDF"),
            candidate("PRINT", "stampare il documento"),
            candidate("SEARCH_SYMBOL", "cercare un pittogramma"),
            candidate("SET_LANGUAGE", "cambiare la lingua dell'interfaccia"),
            candidate("CHANGE_LAYOUT", "cambiare il layout della pagina"),
        ],
        DecisionTask::PictogramDisambiguation
        | DecisionTask::SemanticSearchRanking
        | DecisionTask::DidacticTemplateSelection => Vec::new(),
    }
}

/// Alcuni task richiedono candidati forniti dal documento.
pub fn requires_candidates(task: DecisionTask) -> bool {
    default_candidates(task).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_task_has_a_spec() {
        for task in DecisionTask::ALL {
            let spec = task_spec(task);
            assert_eq!(spec.task, task);
            assert!(!spec.title.is_empty());
            assert!(!spec.description.is_empty());
        }
    }

    #[test]
    fn stable_taxonomies_are_not_empty() {
        for task in DecisionTask::ALL {
            if !requires_candidates(task) {
                let candidates = default_candidates(task);
                assert!(candidates.len() >= 3, "{:?}", task);
                let mut ids: Vec<&str> = candidates
                    .iter()
                    .map(|candidate| candidate.id.as_str())
                    .collect();
                ids.sort_unstable();
                ids.dedup();
                assert_eq!(ids.len(), candidates.len(), "{:?}", task);
            }
        }
    }

    #[test]
    fn ontology_tasks_require_caller_candidates() {
        assert!(requires_candidates(DecisionTask::PictogramDisambiguation));
        assert!(requires_candidates(DecisionTask::SemanticSearchRanking));
        assert!(!requires_candidates(DecisionTask::ExpressedEmotion));
    }
}
