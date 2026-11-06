//! Visual Resource Studio: catalogo locale di modelli e generazione
//! deterministica delle specifiche di una risorsa.
//!
//! Decisione di progetto: il piccolo LLM non produce HTML/CSS/SVG. Il prompt
//! viene interpretato in una specifica JSON validata; il motore di layout del
//! frontend costruisce il documento modificabile. Senza LLM il parser
//! deterministico copre i casi comuni, così la funzione resta utilizzabile
//! anche su un computer da 4 GB.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridSpec {
    #[serde(default = "default_columns")]
    pub columns: u32,
    #[serde(default = "default_rows")]
    pub rows: u32,
    #[serde(default = "default_true")]
    pub caption: bool,
    #[serde(default = "default_true")]
    pub symbol: bool,
    #[serde(default)]
    pub border: bool,
    #[serde(default)]
    pub color_coding: String,
}

fn default_columns() -> u32 {
    3
}
fn default_rows() -> u32 {
    3
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateItem {
    pub label: String,
    #[serde(default)]
    pub concept: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateField {
    pub key: String,
    pub label: String,
    pub kind: String,
    #[serde(default)]
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    /// `grid` | `schedule` | `tokens` | `sequence` | `worksheet`
    pub kind: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub grid: Option<GridSpec>,
    #[serde(default)]
    pub fields: Vec<TemplateField>,
    #[serde(default)]
    pub items: Vec<TemplateItem>,
    /// Metadati della biblioteca estesa (facoltativi per i modelli storici).
    #[serde(default)]
    pub complexity: Option<String>,
    #[serde(default)]
    pub pages: Option<u32>,
    #[serde(default)]
    pub orientation: Option<String>,
    #[serde(default)]
    pub printable: Option<bool>,
    #[serde(default)]
    pub interactive: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlannedItem {
    pub label: String,
    pub concept_id: Option<String>,
    pub strategy: String,
    pub symbol_id: Option<String>,
    pub svg: Option<String>,
    pub approximate: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourcePlan {
    pub template_id: String,
    pub title: String,
    pub kind: String,
    pub grid: GridSpec,
    pub items: Vec<PlannedItem>,
    pub notes: Vec<String>,
    pub source: String,
}

/// Legge il catalogo dei modelli dalla cartella risorse.
fn collect_templates(dir: &Path, out: &mut Vec<TemplateInfo>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("catalogo non leggibile: {e}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // La biblioteca estesa vive in sottocartelle (es. library/).
            collect_templates(&path, out)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        match serde_json::from_str::<TemplateInfo>(&raw) {
            Ok(template) => out.push(template),
            Err(e) => return Err(format!("modello {} non valido: {e}", path.display())),
        }
    }
    Ok(())
}

pub fn list_templates(dir: &Path) -> Result<Vec<TemplateInfo>, String> {
    let mut out = Vec::new();
    collect_templates(dir, &mut out)?;
    out.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));
    Ok(out)
}

pub fn find_template(dir: &Path, id: &str) -> Result<Option<TemplateInfo>, String> {
    Ok(list_templates(dir)?.into_iter().find(|t| t.id == id))
}

/// Interpretazione deterministica del prompt: riconosce il tipo di risorsa e
/// i contenuti elencati. Non produce codice: produce una specifica.
pub fn interpret_prompt(prompt: &str, templates: &[TemplateInfo]) -> (Option<TemplateInfo>, Vec<String>, Vec<String>) {
    let lower = prompt.to_lowercase();
    let mut notes = Vec::new();

    let keyword_map: &[(&str, &[&str])] = &[
        ("visual_schedule", &["agenda", "orario", "giornata", "settimanale", "calendario"]),
        ("communication_grid", &["tabella", "griglia", "comunicazione", "cAA", "scelta"]),
        ("token_board", &["token", "rinforzo", "gettoni", "premi"]),
        ("first_then", &["prima", "poi", "dopo"]),
    ];

    let mut chosen_id: Option<&str> = None;
    for (id, keywords) in keyword_map {
        if keywords.iter().any(|k| lower.contains(&k.to_lowercase())) {
            chosen_id = Some(id);
            break;
        }
    }

    let template = if let Some(id) = chosen_id {
        templates.iter().find(|t| t.id == id).cloned()
    } else {
        notes.push(
            "Richiesta non riconosciuta con certezza: è stato proposto il modello più adatto tra quelli disponibili."
                .to_string(),
        );
        templates
            .iter()
            .find(|t| t.kind == "grid")
            .or_else(|| templates.first())
            .cloned()
    };

    // Contenuti: dopo i due punti, separati da virgola / " e " / punto e virgola.
    let mut items: Vec<String> = Vec::new();
    if let Some(pos) = prompt.find(':') {
        let tail = &prompt[pos + 1..];
        for chunk in tail.split([',', ';']) {
            for piece in chunk.split(" e ") {
                let cleaned = piece
                    .trim()
                    .trim_matches('.')
                    .trim_matches('"')
                    .trim();
                if cleaned.len() >= 2 && cleaned.chars().count() <= 60 {
                    items.push(cleaned.to_string());
                }
            }
        }
    }

    (template, items, notes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn extended_library_loads_300_valid_templates() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/templates");
        let templates = list_templates(&dir).expect("catalogo leggibile");
        let library: Vec<&TemplateInfo> = templates
            .iter()
            .filter(|template| template.id.starts_with("lib_"))
            .collect();
        assert_eq!(library.len(), 300, "la biblioteca estesa deve avere 300 modelli");

        let ids: HashSet<&str> = library.iter().map(|template| template.id.as_str()).collect();
        assert_eq!(ids.len(), 300, "identificativi duplicati");
        let names: HashSet<&str> = library.iter().map(|template| template.name.as_str()).collect();
        assert_eq!(names.len(), 300, "nomi duplicati");

        let mut categories = std::collections::HashMap::new();
        for template in &library {
            *categories.entry(template.category.as_str()).or_insert(0usize) += 1;
            let minimum = if template.category == "Tabelle comunicative" { 2 } else { 4 };
            assert!(
                template.items.len() >= minimum,
                "{} ha troppe poche celle",
                template.id
            );
            assert!(
                template.items.iter().all(|item| !item.label.trim().is_empty()),
                "{} ha una cella vuota",
                template.id
            );
            assert!(!template.description.trim().is_empty());
            if template.category == "Tabelle comunicative" {
                // Le versioni essenziali a 2 e 4 celle sono previste dal
                // capitolato; tutte le altre hanno almeno 6 voci.
                let cells = template
                    .grid
                    .as_ref()
                    .map(|grid| grid.columns * grid.rows)
                    .unwrap_or(template.items.len() as u32);
                if cells > 4 {
                    assert!(
                        template.items.len() >= 6,
                        "{}: una tabella comunicativa deve avere almeno 6 voci",
                        template.id
                    );
                }
            }
        }
        let expected = [
            ("Tabelle comunicative", 60),
            ("Agende e calendari visivi", 35),
            ("Sequenze e routine", 35),
            ("Storie sociali", 30),
            ("Materiali TEACCH", 30),
            ("Materiali ABA e supporti visivi", 20),
            ("Materiali didattici", 45),
            ("Emozioni e autoregolazione", 20),
            ("Autonomia personale", 15),
            ("Giochi e attività interattive", 10),
        ];
        for (category, count) in expected {
            assert_eq!(
                categories.get(category).copied().unwrap_or(0),
                count,
                "conteggio errato per {category}"
            );
        }
        assert!(library
            .iter()
            .any(|template| template.interactive == Some(true)));
        assert!(library
            .iter()
            .any(|template| template.printable == Some(true)));
    }
}
