//! Modifiche strutturate al layout.
//!
//! Le stesse operazioni servono ai comandi manuali e alla richiesta in
//! linguaggio naturale: non esistono due sistemi di modifica. Ogni operazione è
//! validata e gli elementi bloccati non possono essere spostati, nascosti o
//! ridimensionati.

use serde::{Deserialize, Serialize};

use crate::layout::{LayoutSpecification, PageLayout};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayoutOperation {
    SetPageOrientation { value: String },
    SetPageSize { value: String },
    SetMargin { value_mm: f32 },
    SetGridColumns { value: u32 },
    SetGridRows { value: u32 },
    SetGap { value_mm: f32 },
    SetLargeSymbols { value: bool },
    SetTitle { value: String },
    MoveElement { id: String, row: u32, column: u32 },
    MoveElementToPage { id: String, page: u32 },
    HideElement { id: String, hidden: bool },
    LockElement { id: String, locked: bool },
    /// Ridistribuisce gli elementi su un numero di pagine, mantenendo l'ordine.
    DistributePages { pages: u32 },
}

#[derive(Debug, Clone, Serialize)]
pub struct LayoutEditResult {
    pub spec: LayoutSpecification,
    pub pages: Vec<PageLayout>,
    pub notes: Vec<String>,
    /// Descrizione leggibile delle operazioni realmente applicate.
    pub operations: Vec<String>,
}

fn element_index(spec: &LayoutSpecification, id: &str) -> Result<usize, String> {
    spec.elements
        .iter()
        .position(|element| element.id == id)
        .ok_or_else(|| format!("elemento non trovato: {id}"))
}

fn protect(spec: &LayoutSpecification, index: usize, action: &str) -> Result<(), String> {
    let element = &spec.elements[index];
    if element.locked {
        return Err(format!(
            "«{}» è bloccato: la scelta manuale resta e non può essere {action}",
            element.label
        ));
    }
    Ok(())
}

/// Applica le operazioni in ordine, fermandosi al primo errore. La specifica
/// ricevuta non viene modificata dal chiamante se la funzione fallisce.
pub fn apply(
    spec: &mut LayoutSpecification,
    operations: &[LayoutOperation],
) -> Result<Vec<String>, String> {
    let mut applied = Vec::new();
    for operation in operations {
        match operation {
            LayoutOperation::SetPageOrientation { value } => {
                spec.page.orientation = value.clone();
                applied.push(format!("orientamento: {value}"));
            }
            LayoutOperation::SetPageSize { value } => {
                spec.page.size = value.clone();
                applied.push(format!("formato: {value}"));
            }
            LayoutOperation::SetMargin { value_mm } => {
                spec.page.margin_mm = *value_mm;
                applied.push(format!("margine: {value_mm} mm"));
            }
            LayoutOperation::SetGridColumns { value } => {
                spec.layout.columns = *value;
                applied.push(format!("colonne: {value}"));
            }
            LayoutOperation::SetGridRows { value } => {
                spec.layout.rows = *value;
                applied.push(format!("righe: {value}"));
            }
            LayoutOperation::SetGap { value_mm } => {
                spec.layout.gap_mm = *value_mm;
                applied.push(format!("spaziatura: {value_mm} mm"));
            }
            LayoutOperation::SetLargeSymbols { value } => {
                spec.accessibility.large_symbols = *value;
                applied.push(if *value {
                    "pittogrammi grandi".to_string()
                } else {
                    "pittogrammi compatti".to_string()
                });
            }
            LayoutOperation::SetTitle { value } => {
                spec.title = value.clone();
                applied.push(format!("titolo: {value}"));
            }
            LayoutOperation::MoveElement { id, row, column } => {
                let index = element_index(spec, id)?;
                protect(spec, index, "spostata")?;
                spec.elements[index].row = *row;
                spec.elements[index].column = *column;
                applied.push(format!("spostato «{}»", spec.elements[index].label));
            }
            LayoutOperation::MoveElementToPage { id, page } => {
                let index = element_index(spec, id)?;
                protect(spec, index, "spostata di pagina")?;
                spec.elements[index].page = *page;
                applied.push(format!(
                    "«{}» → pagina {}",
                    spec.elements[index].label,
                    page + 1
                ));
            }
            LayoutOperation::HideElement { id, hidden } => {
                let index = element_index(spec, id)?;
                protect(spec, index, if *hidden { "nascosta" } else { "mostrata" })?;
                spec.elements[index].hidden = *hidden;
                applied.push(format!(
                    "«{}» {}",
                    spec.elements[index].label,
                    if *hidden { "nascosto" } else { "mostrato" }
                ));
            }
            LayoutOperation::LockElement { id, locked } => {
                let index = element_index(spec, id)?;
                spec.elements[index].locked = *locked;
                applied.push(format!(
                    "«{}» {}",
                    spec.elements[index].label,
                    if *locked { "bloccato" } else { "sbloccato" }
                ));
            }
            LayoutOperation::DistributePages { pages } => {
                if *pages == 0 || *pages > 40 {
                    return Err("numero di pagine fuori intervallo (1-40)".to_string());
                }
                if let Some(element) = spec.elements.iter().find(|element| element.locked) {
                    return Err(format!(
                        "«{}» è bloccato: la divisione in pagine cambierebbe la sua posizione",
                        element.label
                    ));
                }
                let total = spec.elements.len().max(1);
                let per_page = (total as f32 / *pages as f32).ceil() as usize;
                for (index, element) in spec.elements.iter_mut().enumerate() {
                    element.page = (index / per_page) as u32;
                    element.row = 0;
                    element.column = 0;
                }
                // Senza posizioni esplicite la griglia riempirebbe una sola cella
                // per pagina: ricostruiamo riga e colonna in ordine di lettura.
                let columns = spec.layout.columns.max(1) as usize;
                let rows = spec.layout.rows.max(1) as usize;
                for (index, element) in spec.elements.iter_mut().enumerate() {
                    let slot = (index % per_page) % (columns * rows);
                    element.row = (slot / columns) as u32;
                    element.column = (slot % columns) as u32;
                }
                applied.push(format!("diviso in {pages} pagine"));
            }
        }
    }
    crate::layout::validate(spec)?;
    Ok(applied)
}

/// Riconosce le richieste italiane più comuni senza modello neurale. Ciò che
/// non viene riconosciuto è dichiarato in `notes`: nessuna modifica inventata.
pub fn operations_from_prompt(
    prompt: &str,
    spec: &LayoutSpecification,
) -> (Vec<LayoutOperation>, Vec<String>) {
    let text = prompt.to_lowercase();
    let mut operations = Vec::new();
    let mut notes = Vec::new();

    if text.contains("orizzontale") || text.contains("landscape") {
        operations.push(LayoutOperation::SetPageOrientation {
            value: "landscape".to_string(),
        });
    } else if text.contains("verticale") || text.contains("portrait") {
        operations.push(LayoutOperation::SetPageOrientation {
            value: "portrait".to_string(),
        });
    }

    for size in ["a3", "a4", "a5"] {
        if text.contains(size) {
            operations.push(LayoutOperation::SetPageSize {
                value: size.to_uppercase(),
            });
            break;
        }
    }

    if let Some(value) = value_near(&text, "colonne") {
        operations.push(LayoutOperation::SetGridColumns {
            value: value.round() as u32,
        });
    }
    if let Some(value) = value_near(&text, "righe") {
        operations.push(LayoutOperation::SetGridRows {
            value: value.round() as u32,
        });
    }
    if let Some(value) = value_near(&text, "margine").or_else(|| value_near(&text, "margini")) {
        operations.push(LayoutOperation::SetMargin { value_mm: value });
    }
    if let Some(value) = value_near(&text, "spaziatura") {
        operations.push(LayoutOperation::SetGap { value_mm: value });
    }

    if text.contains("pittogrammi grandi")
        || text.contains("simboli grandi")
        || text.contains("ingrandisci")
        || (text.contains("aumenta") && (text.contains("pittogrammi") || text.contains("simboli")))
    {
        operations.push(LayoutOperation::SetLargeSymbols { value: true });
    }
    if text.contains("pittogrammi piccoli") || text.contains("simboli piccoli") {
        operations.push(LayoutOperation::SetLargeSymbols { value: false });
    }

    if let Some(pages) = pages_requested(&text) {
        if pages > 1 {
            operations.push(LayoutOperation::DistributePages { pages });
        }
    }

    let hide = text.contains("nascondi");
    let lock = text.contains("blocca");
    if hide || lock {
        match find_element(&text, spec) {
            Some(id) => {
                operations.push(LayoutOperation::HideElement {
                    id: id.clone(),
                    hidden: hide,
                });
                if lock {
                    operations.push(LayoutOperation::LockElement { id, locked: true });
                }
            }
            None => notes.push(
                "Non ho riconosciuto l'elemento da nascondere o bloccare: indicalo con la sua etichetta."
                    .to_string(),
            ),
        }
    }

    if operations.is_empty() {
        notes.push(
            "Richiesta non riconosciuta: il layout non è stato modificato. Prova con «metti tutto in orizzontale», «aumenta i pittogrammi», «dividi in due pagine»."
                .to_string(),
        );
    }
    (operations, notes)
}

fn value_near(text: &str, keyword: &str) -> Option<f32> {
    if let Some(index) = text.find(keyword) {
        // Numero dopo la parola: "colonne 4", "colonne: 4".
        if let Some(value) = text[index + keyword.len()..]
            .split(|c: char| !c.is_ascii_digit() && c != '.' && c != ',')
            .find(|part| !part.is_empty())
            .and_then(|part| part.replace(',', ".").parse::<f32>().ok())
        {
            return Some(value);
        }
    }
    // Numero prima della parola: "4 colonne".
    let words: Vec<&str> = text.split_whitespace().collect();
    for (index, word) in words.iter().enumerate() {
        if word.trim_matches(|c: char| !c.is_alphanumeric()) == keyword {
            if index == 0 {
                return None;
            }
            let candidate = words[index - 1].replace(',', ".");
            if let Ok(value) = candidate.parse::<f32>() {
                return Some(value);
            }
        }
    }
    None
}

fn pages_requested(text: &str) -> Option<u32> {
    let words: Vec<&str> = text.split_whitespace().collect();
    for (index, word) in words.iter().enumerate() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
        if clean == "pagine" || clean == "pagine." {
            if index > 0 {
                if let Ok(value) = words[index - 1].parse::<u32>() {
                    return Some(value);
                }
            }
        }
    }
    if text.contains("due pagine") {
        return Some(2);
    }
    if text.contains("tre pagine") {
        return Some(3);
    }
    if text.contains("due") && text.contains("pagin") {
        return Some(2);
    }
    None
}

fn find_element(text: &str, spec: &LayoutSpecification) -> Option<String> {
    spec.elements
        .iter()
        .find(|element| {
            let label = element.label.to_lowercase();
            label.len() >= 3 && text.contains(&label)
        })
        .map(|element| element.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{ElementSpec, LayoutGridSpec};

    fn spec() -> LayoutSpecification {
        let mut spec = LayoutSpecification {
            schema_version: 1,
            document_type: "communication_grid".to_string(),
            title: "Prova".to_string(),
            layout: LayoutGridSpec {
                columns: 2,
                rows: 2,
                gap_mm: 2.0,
                ..Default::default()
            },
            ..Default::default()
        };
        spec.elements.push(ElementSpec {
            id: "a".to_string(),
            label: "mela".to_string(),
            ..Default::default()
        });
        spec.elements.push(ElementSpec {
            id: "b".to_string(),
            label: "cane".to_string(),
            column: 1,
            ..Default::default()
        });
        spec.elements.push(ElementSpec {
            id: "c".to_string(),
            label: "casa".to_string(),
            row: 1,
            ..Default::default()
        });
        spec.elements.push(ElementSpec {
            id: "d".to_string(),
            label: "scuola".to_string(),
            row: 1,
            column: 1,
            ..Default::default()
        });
        spec
    }

    #[test]
    fn prompt_recognises_orientation_and_columns() {
        let (operations, notes) = operations_from_prompt(
            "Metti tutto in orizzontale e usa 4 colonne con pittogrammi grandi",
            &spec(),
        );
        assert!(notes.is_empty());
        assert!(operations.iter().any(|operation| matches!(
            operation,
            LayoutOperation::SetPageOrientation { value } if value == "landscape"
        )));
        assert!(operations.iter().any(|operation| matches!(
            operation,
            LayoutOperation::SetGridColumns { value } if *value == 4
        )));
        assert!(operations.iter().any(|operation| matches!(
            operation,
            LayoutOperation::SetLargeSymbols { value: true }
        )));
    }

    #[test]
    fn applied_operations_change_real_geometry() {
        let mut spec = spec();
        let (operations, _) = operations_from_prompt("orizzontale e 4 colonne", &spec);
        let applied = apply(&mut spec, &operations).unwrap();
        assert_eq!(applied.len(), 2);
        let pages = crate::layout::paginate(&spec).unwrap();
        assert_eq!(pages[0].width_mm, 297.0);
        assert_eq!(spec.layout.columns, 4);
    }

    #[test]
    fn locked_element_cannot_be_moved_or_hidden() {
        let mut spec = spec();
        spec.elements[0].locked = true;
        let operations = vec![LayoutOperation::HideElement {
            id: "a".to_string(),
            hidden: true,
        }];
        let error = apply(&mut spec, &operations).unwrap_err();
        assert!(error.contains("bloccato"), "{error}");
        assert!(!spec.elements[0].hidden);
    }

    #[test]
    fn columns_that_push_elements_out_are_rejected() {
        let mut spec = spec();
        let operations = vec![LayoutOperation::SetGridColumns { value: 1 }];
        let error = apply(&mut spec, &operations).unwrap_err();
        assert!(error.contains("colonne"), "{error}");
        assert_eq!(spec.layout.columns, 1);
    }

    #[test]
    fn distribute_pages_keeps_order_and_positions() {
        let mut spec = spec();
        let applied = apply(&mut spec, &[LayoutOperation::DistributePages { pages: 2 }]).unwrap();
        assert_eq!(applied, vec!["diviso in 2 pagine".to_string()]);
        assert_eq!(spec.elements[0].page, 0);
        assert_eq!(spec.elements[1].page, 0);
        assert_eq!(spec.elements[2].page, 1);
        assert_eq!(spec.elements[3].page, 1);
        crate::layout::validate(&spec).unwrap();
    }

    #[test]
    fn unknown_prompt_reports_without_changing_anything() {
        let (operations, notes) = operations_from_prompt("rendi tutto più bello", &spec());
        assert!(operations.is_empty());
        assert!(notes[0].contains("non riconosciuta"));
    }

    #[test]
    fn hide_targets_the_named_element() {
        let (operations, _) = operations_from_prompt("nascondi mela", &spec());
        assert!(matches!(
            &operations[0],
            LayoutOperation::HideElement { id, hidden: true } if id == "a"
        ));
    }
}
