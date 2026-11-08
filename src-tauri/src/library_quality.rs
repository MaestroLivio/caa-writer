//! Controlli editoriali e strutturali sulla biblioteca.
//!
//! I controlli non sostituiscono la revisione umana: rendono espliciti i
//! problemi che un modello non deve nascondere (elementi fuori pagina,
//! istruzioni mancanti, celle vuote, pittogrammi assenti, testi duplicati).

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

use crate::template_catalog::{self, Classification};
use crate::universal::{self, LibraryEntry, LibraryManifest, RenderedDocument, UniversalDocument};

#[derive(Debug, Clone, Serialize)]
pub struct QualityIssue {
    pub document_id: String,
    pub severity: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryQualityReport {
    pub documents: usize,
    pub valid: usize,
    pub errors: usize,
    pub warnings: usize,
    pub by_category: Vec<CategoryQuality>,
    pub issues: Vec<QualityIssue>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryQuality {
    pub category: String,
    pub documents: usize,
    pub errors: usize,
    pub warnings: usize,
}

pub fn check_document(
    entry: &LibraryEntry,
    document: &UniversalDocument,
    rendered: &RenderedDocument,
) -> Vec<QualityIssue> {
    let class = template_catalog::classify(entry);
    let mut issues = Vec::new();
    let mut push = |severity: &str, code: &str, message: String| {
        issues.push(QualityIssue {
            document_id: entry.id.clone(),
            severity: severity.to_string(),
            code: code.to_string(),
            message,
        });
    };
    if document.pages.is_empty() {
        push("errore", "pagine-assenti", "il documento non ha pagine".to_string());
        return issues;
    }
    if document.title.trim().is_empty() {
        push("errore", "titolo-assente", "il documento non ha titolo".to_string());
    }
    if rendered.pages.len() != document.pages.len() {
        push(
            "errore",
            "rendering-incompleto",
            format!(
                "pagine dichiarate {} ma renderizzate {}",
                document.pages.len(),
                rendered.pages.len()
            ),
        );
    }
    let mut heading_seen: HashSet<String> = HashSet::new();
    let mut instruction_found = false;
    let mut exercise_parts = 0usize;
    let mut solution_found = false;
    let mut token_slots = 0usize;
    let mut toggle_actions = 0usize;
    let mut social_pages = 0usize;

    for (page, rendered_page) in document.pages.iter().zip(rendered.pages.iter()) {
        let mut text_seen: HashSet<String> = HashSet::new();
        for element in &page.elements {
            let normalized_text = element.text.trim().to_lowercase();
            if !normalized_text.is_empty() {
                if element.kind == "title" && !text_seen.insert(normalized_text.clone()) {
                    push(
                        "avviso",
                        "testo-duplicato",
                        format!("pagina {}: titolo ripetuto «{}»", page.id, element.text),
                    );
                }
                if normalized_text == document.title.trim().to_lowercase() {
                    if !heading_seen.insert(page.id.clone()) {
                        push(
                            "avviso",
                            "titolo-ripetuto",
                            format!("pagina {}: il titolo è ripetuto più volte", page.id),
                        );
                    }
                }
                if [
                    "istruzion",
                    "completa",
                    "osserva",
                    "scrivi",
                    "collega",
                    "scegli",
                    "leggi",
                    "cerchia",
                    "ritaglia",
                    "numera",
                ]
                .iter()
                .any(|needle| normalized_text.contains(needle))
                {
                    instruction_found = true;
                }
                if normalized_text.contains("soluzion")
                    || normalized_text.contains("risposte corrette")
                    || normalized_text.contains("per il docente")
                {
                    solution_found = true;
                }
            }
            if matches!(
                element.kind.as_str(),
                "question" | "answer_line" | "handwriting_line" | "checkbox" | "matching"
                    | "ten_frame" | "number_line" | "timeline"
            ) {
                exercise_parts += 1;
            }
            if element.kind == "token_slot" {
                token_slots += 1;
            }
            if element
                .action
                .as_ref()
                .map(|action| action.action_type == "toggle")
                .unwrap_or(false)
            {
                toggle_actions += 1;
            }
            if page.elements.len() >= 2 {
                social_pages += 1;
            }
            if let Some(grid) = &element.grid {
                let capacity = (grid.columns * grid.rows) as usize;
                if grid.cells.is_empty() {
                    push(
                        "errore",
                        "griglia-vuota",
                        format!("{}: griglia senza celle", element.id),
                    );
                } else if grid.cells.len() < capacity {
                    push(
                        "avviso",
                        "griglia-incompleta",
                        format!(
                            "{}: {} celle su {} posti disponibili",
                            element.id,
                            grid.cells.len(),
                            capacity
                        ),
                    );
                }
                let mut cell_ids = HashSet::new();
                for cell in &grid.cells {
                    if !cell_ids.insert(cell.id.as_str()) {
                        push(
                            "errore",
                            "cella-duplicata",
                            format!("{}: cella duplicata {}", element.id, cell.id),
                        );
                    }
                    if cell.missing_symbol {
                        push(
                            "avviso",
                            "pittogramma-assente",
                            format!("{}: «{}» non ha un pittogramma", cell.id, cell.text),
                        );
                    }
                }
                let mut filled = grid.cells.iter().filter(|cell| !cell.hidden).count();
                if grid.cells.len() == capacity {
                    filled = filled.max(1);
                }
                if filled == 0 {
                    push(
                        "errore",
                        "griglia-senza-contenuto",
                        format!("{}: nessuna cella visibile", element.id),
                    );
                }
            }
            if element
                .metadata
                .get("missing_symbol")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                push(
                    "avviso",
                    "pittogramma-assente",
                    format!("{}: «{}» resta testo, senza simbolo", element.id, element.text),
                );
            }
            if element.kind == "text" && element.style.font_size > 0.0 && element.style.font_size < 2.4 {
                push(
                    "avviso",
                    "testo-piccolo",
                    format!(
                        "{}: testo di {:.1} mm, sotto la soglia leggibile",
                        element.id, element.style.font_size
                    ),
                );
            }
            if element.visible
                && !element.text.trim().is_empty()
                && element.kind == "symbol"
                && element.symbol_id.is_none()
            {
                push(
                    "avviso",
                    "simbolo-non-risolto",
                    format!("{}: simbolo non risolto per «{}»", element.id, element.text),
                );
            }
        }
        for element in &rendered_page.elements {
            if !element.visible {
                continue;
            }
            let tolerance = 0.6;
            if element.x_mm < -tolerance
                || element.y_mm < -tolerance
                || element.x_mm + element.w_mm > rendered_page.width_mm + tolerance
                || element.y_mm + element.h_mm > rendered_page.height_mm + tolerance
            {
                if element.kind != "symbol" {
                    push(
                        "errore",
                        "fuori-pagina",
                        format!(
                            "{}: elemento fuori pagina ({:.1},{:.1} {:.1}×{:.1})",
                            element.id, element.x_mm, element.y_mm, element.w_mm, element.h_mm
                        ),
                    );
                }
            }
            if element.w_mm < 1.0 || element.h_mm < 1.0 {
                push(
                    "avviso",
                    "elemento-minuscolo",
                    format!("{}: dimensione {}×{} mm", element.id, element.w_mm, element.h_mm),
                );
            }
        }
        if page.elements.is_empty() {
            push(
                "errore",
                "pagina-vuota",
                format!("pagina {}: nessun elemento", page.id),
            );
        }
    }

    if class.category_id == "schede" {
        if !instruction_found {
            push(
                "avviso",
                "istruzioni-assenti",
                "scheda senza un'istruzione operativa riconoscibile".to_string(),
            );
        }
        if exercise_parts < 2 {
            push(
                "avviso",
                "esercizio-assente",
                "scheda senza componenti di risposta (righe, caselle, domande)".to_string(),
            );
        }
        if !solution_found {
            push(
                "avviso",
                "soluzioni-assenti",
                "manca una pagina o una sezione con le soluzioni".to_string(),
            );
        }
    }
    if class.category_id == "comportamentali" && class.subcategory_id == "token" {
        if token_slots < 3 && toggle_actions < 3 {
            push(
                "errore",
                "token-board-non-vera",
                "una token board deve avere almeno tre gettoni o slot interattivi".to_string(),
            );
        }
    }
    if class.category_id == "storie-sociali" && document.pages.len() < 3 {
        push(
            "avviso",
            "storia-troppo-breve",
            "una storia sociale multipagina dovrebbe avere almeno tre pagine".to_string(),
        );
    }
    let _ = social_pages;
    issues
}

pub fn validate_manifest(resources: &Path, manifest: &LibraryManifest) -> LibraryQualityReport {
    let mut issues = Vec::new();
    let mut valid = 0usize;
    let mut by_category: std::collections::BTreeMap<String, (usize, usize, usize)> =
        std::collections::BTreeMap::new();
    for entry in &manifest.entries {
        let class: Classification = template_catalog::classify(entry);
        let bucket = by_category
            .entry(class.category.clone())
            .or_insert((0, 0, 0));
        bucket.0 += 1;
        let path = match crate::universal::resolve_library_path(
            manifest,
            &entry.id,
            Some(&entry.source_type),
            resources,
        ) {
            Ok(path) => path,
            Err(error) => {
                issues.push(QualityIssue {
                    document_id: entry.id.clone(),
                    severity: "errore".to_string(),
                    code: "documento-non-raggiungibile".to_string(),
                    message: error,
                });
                bucket.1 += 1;
                continue;
            }
        };
        let document = match universal::load_from_caadoc(&path) {
            Ok(document) => document,
            Err(error) => {
                issues.push(QualityIssue {
                    document_id: entry.id.clone(),
                    severity: "errore".to_string(),
                    code: "documento-non-valido".to_string(),
                    message: error,
                });
                bucket.1 += 1;
                continue;
            }
        };
        let rendered = match document.render() {
            Ok(rendered) => rendered,
            Err(error) => {
                issues.push(QualityIssue {
                    document_id: entry.id.clone(),
                    severity: "errore".to_string(),
                    code: "rendering-fallito".to_string(),
                    message: error,
                });
                bucket.1 += 1;
                continue;
            }
        };
        let document_issues = check_document(entry, &document, &rendered);
        for issue in &document_issues {
            if issue.severity == "errore" {
                bucket.1 += 1;
            } else {
                bucket.2 += 1;
            }
        }
        if document_issues.is_empty() {
            valid += 1;
        }
        issues.extend(document_issues);
    }
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == "errore")
        .count();
    let warnings = issues
        .iter()
        .filter(|issue| issue.severity == "avviso")
        .count();
    LibraryQualityReport {
        documents: manifest.entries.len(),
        valid,
        errors,
        warnings,
        by_category: by_category
            .into_iter()
            .map(|(category, (documents, errors, warnings))| CategoryQuality {
                category,
                documents,
                errors,
                warnings,
            })
            .collect(),
        issues,
        generated_at: crate::util::now_iso(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_page_is_an_error() {
        let mut document = UniversalDocument::empty("doc", "Prova", "document");
        document.pages[0].elements.clear();
        let rendered = document.render().unwrap();
        let entry = LibraryEntry {
            id: "doc".to_string(),
            title: "Prova".to_string(),
            category: "Materiali didattici".to_string(),
            source_type: "template".to_string(),
            ..Default::default()
        };
        let issues = check_document(&entry, &document, &rendered);
        assert!(issues.iter().any(|issue| issue.code == "pagina-vuota"));
    }
}
