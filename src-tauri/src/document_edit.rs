//! Interpretazione di richieste in linguaggio naturale in operazioni
//! documentali tipizzate.
//!
//! Il modello non tocca mai il DOM e non esegue codice: produce un elenco di
//! operazioni ammesse, validate qui e applicate dal frontend con i comandi
//! dell'editor. In assenza di modello il parser deterministico copre le
//! richieste più comuni.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentOperation {
    SetUppercase { value: bool },
    ScaleFontSize { factor: f32 },
    SetFontSize { value: f32 },
    ScaleSymbolSize { factor: f32 },
    SetSymbolGap { value: u32 },
    SetChipBorderColor { value: String },
    SetAlignment { value: String },
    InsertTable { rows: u32, columns: u32 },
    AddTableRow,
    AddTableColumn,
    GenerateBoard { prompt: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentEditPlan {
    pub operations: Vec<DocumentOperation>,
    pub descriptions: Vec<String>,
    pub warnings: Vec<String>,
    /// `deterministico`, `neurale` oppure `nessuno`.
    pub source: String,
}

fn color_from_prompt(lower: &str) -> Option<&'static str> {
    const COLORS: &[(&str, &str)] = &[
        ("blu", "#2563eb"),
        ("rosso", "#dc2626"),
        ("rossa", "#dc2626"),
        ("verde", "#16a34a"),
        ("nero", "#111827"),
        ("nera", "#111827"),
        ("grigio", "#6b7280"),
        ("giallo", "#eab308"),
        ("arancione", "#ea580c"),
        ("viola", "#7c3aed"),
        ("rosa", "#db2777"),
    ];
    COLORS
        .iter()
        .find(|(name, _)| lower.contains(name))
        .map(|(_, value)| *value)
}

fn number_after(text: &str, keyword: &str) -> Option<f32> {
    let lower = text.to_lowercase();
    let position = lower.find(keyword)?;
    let tail = &lower[position + keyword.len()..];
    let number: String = tail
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    number.replace(',', ".").parse().ok()
}

/// Numero che precede una parola chiave («20%», «4 colonne», «2 righe»).
fn number_before(text: &str, keyword: &str) -> Option<f32> {
    let position = text.find(keyword)?;
    let prefix = &text[..position];
    let mut digits = String::new();
    for c in prefix.chars().rev() {
        if c.is_ascii_digit() || c == '.' || c == ',' {
            digits.push(c);
        } else if c.is_whitespace() && digits.is_empty() {
            continue;
        } else {
            break;
        }
    }
    let number: String = digits.chars().rev().collect();
    number.replace(',', ".").parse().ok()
}

/// Forma «4x3» / «4×3» usata spesso nelle richieste di tabelle.
fn grid_from_pattern(text: &str) -> Option<(u32, u32)> {
    let chars: Vec<char> = text.chars().collect();
    for index in 0..chars.len() {
        if chars[index] != 'x' && chars[index] != '×' {
            continue;
        }
        let mut left = String::new();
        for c in chars[..index].iter().rev() {
            if c.is_ascii_digit() {
                left.push(*c);
            } else {
                break;
            }
        }
        let mut right = String::new();
        for c in chars[index + 1..].iter() {
            if c.is_ascii_digit() {
                right.push(*c);
            } else {
                break;
            }
        }
        let left: String = left.chars().rev().collect();
        if let (Ok(columns), Ok(rows)) = (left.parse::<u32>(), right.parse::<u32>()) {
            if columns > 0 && rows > 0 {
                return Some((rows, columns));
            }
        }
    }
    None
}

/// Descrizione leggibile di un'operazione, mostrata nell'anteprima.
pub fn describe(operation: &DocumentOperation) -> String {
    match operation {
        DocumentOperation::SetUppercase { value: true } => "Testo in maiuscolo".to_string(),
        DocumentOperation::SetUppercase { value: false } => "Testo in minuscolo".to_string(),
        DocumentOperation::ScaleFontSize { factor } => {
            format!("Dimensione del testo ×{factor:.2}")
        }
        DocumentOperation::SetFontSize { value } => format!("Dimensione del testo {value:.0} pt"),
        DocumentOperation::ScaleSymbolSize { factor } => {
            format!("Dimensione dei pittogrammi ×{factor:.2}")
        }
        DocumentOperation::SetSymbolGap { value } => format!("Spazio immagine-testo {value} px"),
        DocumentOperation::SetChipBorderColor { value } => format!("Bordo dei pittogrammi {value}"),
        DocumentOperation::SetAlignment { value } => format!("Allineamento: {value}"),
        DocumentOperation::InsertTable { rows, columns } => {
            format!("Inserisci tabella {rows}×{columns}")
        }
        DocumentOperation::AddTableRow => "Aggiungi una riga alla tabella".to_string(),
        DocumentOperation::AddTableColumn => "Aggiungi una colonna alla tabella".to_string(),
        DocumentOperation::GenerateBoard { prompt } => {
            format!("Genera una tabella comunicativa: {prompt}")
        }
    }
}

/// Valida un'operazione prima che il frontend la applichi.
pub fn validate(operation: &DocumentOperation) -> Result<(), String> {
    match operation {
        DocumentOperation::ScaleFontSize { factor } => {
            if !(0.5..=2.0).contains(factor) {
                return Err("la scala del testo deve essere tra 0,5 e 2".to_string());
            }
        }
        DocumentOperation::SetFontSize { value } => {
            if !(8.0..=72.0).contains(value) {
                return Err("la dimensione del testo deve essere tra 8 e 72 pt".to_string());
            }
        }
        DocumentOperation::ScaleSymbolSize { factor } => {
            if !(0.5..=2.0).contains(factor) {
                return Err("la scala dei pittogrammi deve essere tra 0,5 e 2".to_string());
            }
        }
        DocumentOperation::SetSymbolGap { value } => {
            if *value > 64 {
                return Err("lo spazio immagine-testo non può superare 64 px".to_string());
            }
        }
        DocumentOperation::SetChipBorderColor { value } => {
            let valid = value.len() == 7
                && value.starts_with('#')
                && value[1..].chars().all(|c| c.is_ascii_hexdigit());
            if !valid {
                return Err("colore non valido".to_string());
            }
        }
        DocumentOperation::SetAlignment { value } => {
            if !matches!(value.as_str(), "left" | "center" | "right" | "justify") {
                return Err("allineamento non supportato".to_string());
            }
        }
        DocumentOperation::InsertTable { rows, columns } => {
            if !(1..=12).contains(rows) || !(1..=12).contains(columns) {
                return Err("la tabella deve avere tra 1 e 12 righe e colonne".to_string());
            }
        }
        DocumentOperation::GenerateBoard { prompt } => {
            if prompt.trim().len() < 4 {
                return Err("la richiesta per la tabella è troppo breve".to_string());
            }
        }
        _ => {}
    }
    Ok(())
}

/// Parser deterministico delle richieste italiane frequenti. Non inventa
/// operazioni: ciò che non riconosce resta un avviso esplicito.
pub fn interpret(prompt: &str) -> DocumentEditPlan {
    let lower = prompt.to_lowercase();
    let mut operations: Vec<DocumentOperation> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut consumed = false;

    if lower.contains("maiuscol") {
        operations.push(DocumentOperation::SetUppercase { value: true });
        consumed = true;
    } else if lower.contains("minuscol") {
        operations.push(DocumentOperation::SetUppercase { value: false });
        consumed = true;
    }

    // Le richieste possono contenere più frasi: «ingrandisci i pittogrammi
    // del 20% e metti il testo in maiuscolo» non deve ingrandire anche il
    // testo solo perché contiene la parola «testo».
    let clauses: Vec<String> = lower
        .split(['.', ';', ','])
        .flat_map(|chunk| chunk.split(" e "))
        .map(|chunk| chunk.trim().to_string())
        .filter(|chunk| !chunk.is_empty())
        .collect();
    for clause in &clauses {
        let mentions_symbols = clause.contains("pittogramm")
            || clause.contains("simbol")
            || clause.contains("immagin");
        let mentions_text = clause.contains("testo") || clause.contains("caratter");
        if mentions_symbols {
            if let Some(percent) = number_before(clause, "%") {
                let factor = 1.0 + percent / 100.0;
                if (0.5..=2.0).contains(&factor) {
                    let value = if clause.contains("riduci")
                        || clause.contains("rimpicciolisci")
                        || clause.contains("più piccol")
                    {
                        1.0 - percent / 100.0
                    } else {
                        factor
                    };
                    operations.push(DocumentOperation::ScaleSymbolSize { factor: value });
                    consumed = true;
                }
            } else if clause.contains("più grand")
                || clause.contains("ingrandisci")
                || clause.contains("aumenta")
            {
                operations.push(DocumentOperation::ScaleSymbolSize { factor: 1.15 });
                consumed = true;
            } else if clause.contains("più piccol")
                || clause.contains("riduci")
                || clause.contains("rimpicciolisci")
            {
                operations.push(DocumentOperation::ScaleSymbolSize { factor: 0.85 });
                consumed = true;
            }
        }
        if mentions_text {
            if let Some(size) = number_after(clause, "dimensione") {
                operations.push(DocumentOperation::SetFontSize { value: size });
                consumed = true;
            } else if clause.contains("più grand")
                || clause.contains("ingrandisci")
                || clause.contains("aumenta")
            {
                operations.push(DocumentOperation::ScaleFontSize { factor: 1.15 });
                consumed = true;
            } else if clause.contains("più piccol")
                || clause.contains("riduci")
                || clause.contains("rimpicciolisci")
            {
                operations.push(DocumentOperation::ScaleFontSize { factor: 0.85 });
                consumed = true;
            }
        }
    }

    if lower.contains("bord") {
        if let Some(color) = color_from_prompt(&lower) {
            operations.push(DocumentOperation::SetChipBorderColor {
                value: color.to_string(),
            });
            consumed = true;
        }
    }

    if lower.contains("allinea") || lower.contains("centra") || lower.contains("giustifica") {
        let value = if lower.contains("centr") {
            "center"
        } else if lower.contains("destr") {
            "right"
        } else if lower.contains("giustific") {
            "justify"
        } else {
            "left"
        };
        operations.push(DocumentOperation::SetAlignment {
            value: value.to_string(),
        });
        consumed = true;
    }

    if lower.contains("riga") || lower.contains("colonn") {
        if lower.contains("aggiungi") || lower.contains("inserisci") {
            if lower.contains("colonn") {
                operations.push(DocumentOperation::AddTableColumn);
            } else {
                operations.push(DocumentOperation::AddTableRow);
            }
            consumed = true;
        }
    }

    if lower.contains("tabella")
        && (lower.contains("crea") || lower.contains("inserisci"))
        && !lower.contains("comunicativa")
    {
        let from_pattern = grid_from_pattern(&lower);
        let columns = number_before(&lower, "colonn")
            .or_else(|| number_after(&lower, "colonn"))
            .map(|value| value as u32)
            .or(from_pattern.map(|(_, columns)| columns))
            .unwrap_or(3);
        let rows = number_before(&lower, "rig")
            .or_else(|| number_after(&lower, "rig"))
            .map(|value| value as u32)
            .or(from_pattern.map(|(rows, _)| rows))
            .unwrap_or(3);
        operations.push(DocumentOperation::InsertTable {
            rows: rows.clamp(1, 12),
            columns: columns.clamp(1, 12),
        });
        consumed = true;
    }

    if lower.contains("tabella comunicativa") || lower.contains("tabella per la mensa") {
        operations.push(DocumentOperation::GenerateBoard {
            prompt: prompt.trim().to_string(),
        });
        consumed = true;
    }

    // Richieste note ma non applicabili in questa versione: meglio dirlo che
    // simulare una modifica.
    for (needle, message) in [
        (
            "sostituisci i pittogrammi",
            "La sostituzione automatica dei pittogrammi generici richiede una scelta manuale nel pannello Alternative.",
        ),
        (
            "lettura vocale",
            "L'aggiunta della lettura vocale alle celle del documento non è ancora supportata.",
        ),
        (
            "sei celle",
            "Il ridimensionamento di una tabella esistente non è ancora supportato.",
        ),
        (
            "più semplice",
            "La semplificazione del testo è disponibile con lo strumento «Semplifica una frase», non come modifica automatica dell'intero documento.",
        ),
    ] {
        if lower.contains(needle) {
            warnings.push(message.to_string());
        }
    }

    if !consumed && warnings.is_empty() {
        warnings.push(
            "Richiesta non riconosciuta: nessuna modifica è stata proposta. Prova con «testo maiuscolo», «pittogrammi più grandi del 20%», «bordi blu», «allinea a sinistra», «inserisci una tabella 3x4», «aggiungi una riga»."
                .to_string(),
        );
    }

    let mut unique: Vec<DocumentOperation> = Vec::new();
    for operation in operations {
        if !unique.contains(&operation) {
            unique.push(operation);
        }
    }
    let mut operations = unique;
    operations.retain(|operation| validate(operation).is_ok());
    DocumentEditPlan {
        descriptions: operations.iter().map(describe).collect(),
        operations,
        warnings,
        source: "deterministico".to_string(),
    }
}

/// Estrae il primo oggetto JSON da una risposta del modello e valida ogni
/// operazione. Le operazioni sconosciute vengono scartate, non interpretate.
pub fn parse_model_plan(raw: &str) -> DocumentEditPlan {
    let mut warnings = Vec::new();
    let start = raw.find('{');
    let end = raw.rfind('}');
    let value = start
        .zip(end)
        .filter(|(start, end)| end > start)
        .and_then(|(start, end)| serde_json::from_str::<serde_json::Value>(&raw[start..=end]).ok());
    let Some(value) = value else {
        return DocumentEditPlan {
            operations: Vec::new(),
            descriptions: Vec::new(),
            warnings: vec!["Il modello non ha restituito un piano JSON valido.".to_string()],
            source: "neurale".to_string(),
        };
    };
    let Some(items) = value.get("operations").and_then(|value| value.as_array()) else {
        return DocumentEditPlan {
            operations: Vec::new(),
            descriptions: Vec::new(),
            warnings: vec!["Il piano neurale non contiene operazioni.".to_string()],
            source: "neurale".to_string(),
        };
    };
    let mut operations = Vec::new();
    for item in items {
        match serde_json::from_value::<DocumentOperation>(item.clone()) {
            Ok(operation) => match validate(&operation) {
                Ok(()) => operations.push(operation),
                Err(reason) => warnings.push(format!("Operazione rifiutata: {reason}")),
            },
            Err(_) => warnings.push("Operazione sconosciuta rifiutata.".to_string()),
        }
    }
    if operations.len() > 20 {
        operations.truncate(20);
        warnings.push("Piano ridotto a 20 operazioni.".to_string());
    }
    DocumentEditPlan {
        descriptions: operations.iter().map(describe).collect(),
        operations,
        warnings,
        source: "neurale".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_parser_understands_common_requests() {
        let plan = interpret("Ingrandisci tutti i pittogrammi del 20% e metti il testo in maiuscolo.");
        assert!(plan
            .operations
            .contains(&DocumentOperation::ScaleSymbolSize { factor: 1.2 }));
        assert!(plan
            .operations
            .contains(&DocumentOperation::SetUppercase { value: true }));
        assert!(plan
            .descriptions
            .iter()
            .any(|line| line.contains("pittogrammi")));
    }

    #[test]
    fn deterministic_parser_handles_tables_and_colors() {
        let plan = interpret("Inserisci una tabella 4x3 con bordi blu e allinea a sinistra.");
        assert!(plan.operations.contains(&DocumentOperation::InsertTable {
            rows: 3,
            columns: 4
        }));
        assert!(plan.operations.contains(&DocumentOperation::SetChipBorderColor {
            value: "#2563eb".to_string()
        }));
        assert!(plan
            .operations
            .contains(&DocumentOperation::SetAlignment {
                value: "left".to_string()
            }));
        // «4x3» significa quattro colonne e tre righe.
    }

    #[test]
    fn columns_and_rows_are_read_from_words() {
        let plan = interpret("Crea una tabella con 4 colonne e 2 righe.");
        assert!(plan.operations.contains(&DocumentOperation::InsertTable {
            rows: 2,
            columns: 4
        }));
    }

    #[test]
    fn unsupported_requests_are_declared_not_faked() {
        let plan = interpret("Aggiungi la lettura vocale a ogni cella.");
        assert!(plan.operations.is_empty());
        assert!(plan.warnings.iter().any(|w| w.contains("lettura vocale")));
    }

    #[test]
    fn model_plan_rejects_unknown_and_out_of_range_operations() {
        let raw = r#"{"operations":[
            {"type":"set_font_size","value":99},
            {"type":"set_uppercase","value":true},
            {"type":"run_javascript","code":"alert(1)"}
        ]}"#;
        let plan = parse_model_plan(raw);
        assert_eq!(plan.operations.len(), 1);
        assert!(plan
            .operations
            .contains(&DocumentOperation::SetUppercase { value: true }));
        assert!(plan.warnings.len() >= 2);
    }

    #[test]
    fn invalid_colors_are_not_applied() {
        assert!(validate(&DocumentOperation::SetChipBorderColor {
            value: "javascript:alert(1)".to_string()
        })
        .is_err());
    }
}
