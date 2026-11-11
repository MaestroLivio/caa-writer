//! Costruzione dei documenti universali da specifiche versionate.
//!
//! La stessa funzione è usata da `caa-cli template build-all`, dai test e dal
//! generatore neurale: nessun modello viene creato con un percorso diverso.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::engine::Engine;
use crate::export::caadoc;
use crate::model::{
    CaaMode, DocumentAsset, DocumentPayload, ExportResult, Intensity, SymbolizeOptions,
};
use crate::universal::{
    self, GridCell, LibraryEntry, LibraryManifest, UniversalAction, UniversalDocument,
    UniversalElement, UniversalPage,
};

#[derive(Debug, Clone)]
pub struct BuildOutcome {
    pub document: UniversalDocument,
    pub assets: Vec<DocumentAsset>,
    pub notes: Vec<String>,
}

fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|item| item.as_str()).map(str::to_string)
}

fn number(value: &Value, key: &str) -> Option<f32> {
    value.get(key).and_then(|item| item.as_f64()).map(|v| v as f32)
}

fn page_id(value: &Value, fallback: &str) -> String {
    string(value, "id").filter(|id| !id.trim().is_empty()).unwrap_or_else(|| fallback.to_string())
}

fn normalize_symbol_label(label: &str) -> String {
    label.replace('_', " ").trim().to_string()
}

fn resolve_symbol(
    engine: &Engine,
    label: &str,
    explicit_id: Option<&str>,
    concept_hint: Option<&str>,
) -> Result<Option<(String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)>, String> {
    if let Some(symbol_id) = explicit_id {
        let db = engine.db()?;
        if let Some(record) = db.symbol_by_id(symbol_id).map_err(|e| e.to_string())? {
            return Ok(Some((
                record.id,
                record.concept_id,
                record.svg,
                record.image,
                Some(record.label),
                None,
            )));
        }
    }
    let cleaned = normalize_symbol_label(label);
    if cleaned.is_empty() {
        return Ok(None);
    }
    // Preferenza esplicita: ARASAAC prima di CAA Writer Core. Si accetta solo
    // un'etichetta esatta; se ARASAAC non ce l'ha, il motore e il fallback
    // possono usare il simbolo Core o un'altra fonte.
    let arasaac = crate::database::SymbolFilters {
        pack_id: Some("arasaac".to_string()),
        ..Default::default()
    };
    if let Some(record) = engine
        .db()?
        .symbol_by_label(&cleaned, &arasaac)
        .map_err(|e| e.to_string())?
    {
        return Ok(Some((
            record.id,
            record.concept_id.or_else(|| concept_hint.map(str::to_string)),
            record.svg,
            record.image,
            Some(record.label),
            None,
        )));
    }
    let options = SymbolizeOptions {
        paragraph_id: format!("build-{}", crate::pictogram_import::slugify(&cleaned)),
        intensity: Intensity::Minima,
        mode: CaaMode::Sopra,
        ..Default::default()
    };
    let result = engine.symbolize(&cleaned, &options)?;
    let token = result
        .tokens
        .iter()
        .find(|token| token.symbol_id.is_some() || !token.composition.is_empty());
    if let Some(token) = token {
        if let Some(symbol_id) = &token.symbol_id {
            return Ok(Some((
                symbol_id.clone(),
                token.concept_id.clone().or_else(|| concept_hint.map(str::to_string)),
                token.svg.clone(),
                token.image.clone(),
                token.symbol_label.clone(),
                token.lemma.clone(),
            )));
        }
        if let Some(part) = token.composition.first() {
            return Ok(Some((
                part.symbol_id.clone().unwrap_or_default(),
                if part.concept_id.trim().is_empty() {
                    concept_hint.map(str::to_string)
                } else {
                    Some(part.concept_id.clone())
                },
                part.svg.clone(),
                part.image.clone(),
                Some(part.label.clone()),
                token.lemma.clone(),
            )));
        }
    }
    // Le parole fuori lessico non hanno un concetto: la ricerca per etichetta
    // esatta usa comunque il repertorio reale, senza inventare associazioni.
    let db = engine.db()?;
    let filters = crate::database::SymbolFilters::default();
    if let Some(record) = db
        .symbol_by_label(&cleaned, &filters)
        .map_err(|e| e.to_string())?
    {
        return Ok(Some((
            record.id,
            record.concept_id.or_else(|| concept_hint.map(str::to_string)),
            record.svg,
            record.image,
            Some(record.label),
            None,
        )));
    }
    // I titoli di sezione sono spesso al plurale («Persone», «Farmaci»,
    // «Documenti»). Si provano forme singolari *esatte* nel repertorio: niente
    // ricerca fuzzy, quindi un simbolo sbagliato non viene scelto.
    for candidate in singular_candidates(&cleaned) {
        if let Some(record) = db
            .symbol_by_label(&candidate, &filters)
            .map_err(|e| e.to_string())?
        {
            return Ok(Some((
                record.id,
                record.concept_id.or_else(|| concept_hint.map(str::to_string)),
                record.svg,
                record.image,
                Some(record.label),
                None,
            )));
        }
    }
    Ok(None)
}

/// Varianti singolari prudenti per etichette di sezione. Ogni variante deve
/// esistere come etichetta esatta nel repertorio per essere usata.
fn singular_candidates(label: &str) -> Vec<String> {
    fn variants(word: &str) -> Vec<String> {
        let mut out = Vec::new();
        if word.ends_with('i') && word.chars().count() > 2 {
            for ending in ["o", "e", "a"] {
                let mut candidate = word.to_string();
                candidate.pop();
                candidate.push_str(ending);
                out.push(candidate);
            }
        } else if word.ends_with('e') && word.chars().count() > 2 {
            for ending in ["a", "o"] {
                let mut candidate = word.to_string();
                candidate.pop();
                candidate.push_str(ending);
                out.push(candidate);
            }
        }
        out
    }
    let words: Vec<&str> = label.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    // Prima l'ultima parola (il caso più frequente), poi l'intera etichetta
    // con tutte le parole rese singolari.
    if let Some(last) = words.last() {
        for variant in variants(last) {
            let mut parts: Vec<String> = words[..words.len() - 1]
                .iter()
                .map(|word| word.to_string())
                .collect();
            parts.push(variant);
            out.push(parts.join(" "));
        }
    }
    // Per le etichette di sezione («Momenti della giornata», «Oggetti
    // personali») il simbolo utile è spesso nel primo sostantivo, non
    // nell'ultima parola. Si provano forme singolari di ogni parola.
    const STOP: &[&str] = &["di", "del", "della", "dei", "delle", "e", "il", "lo", "la", "i", "gli", "le", "per", "con", "in", "a", "da", "su"];
    for word in &words {
        if STOP.contains(&word.to_lowercase().as_str()) {
            continue;
        }
        for variant in variants(word) {
            if variant != *word && !out.contains(&variant) {
                out.push(variant);
            }
        }
    }
    if words.len() > 1 {
        let mut combinations = vec![String::new()];
        for word in &words {
            let options = std::iter::once(word.to_string())
                .chain(variants(word))
                .collect::<Vec<_>>();
            let mut next = Vec::new();
            for prefix in &combinations {
                for option in &options {
                    next.push(if prefix.is_empty() {
                        option.clone()
                    } else {
                        format!("{prefix} {option}")
                    });
                }
            }
            combinations = next;
        }
        for candidate in combinations {
            if candidate != label && !out.contains(&candidate) {
                out.push(candidate);
            }
        }
    }
    out.truncate(24);
    out
}

fn action_from(value: &Value) -> Option<UniversalAction> {
    let object = value.as_object()?;
    let kind = object
        .get("type")
        .and_then(|item| item.as_str())
        .unwrap_or("append")
        .to_string();
    let target = object
        .get("target")
        .or_else(|| object.get("target_page"))
        .or_else(|| object.get("target_page_id"))
        .and_then(|item| item.as_str())
        .map(str::to_string);
    Some(UniversalAction {
        action_type: kind,
        target_page: target,
        target_document: object
            .get("target_document")
            .and_then(|item| item.as_str())
            .map(str::to_string),
        url: object.get("url").and_then(|item| item.as_str()).map(str::to_string),
    })
}

fn add_asset(
    assets: &mut HashMap<String, DocumentAsset>,
    symbol_id: Option<&str>,
    svg: Option<&str>,
) {
    let (Some(symbol_id), Some(svg)) = (symbol_id, svg) else {
        return;
    };
    if symbol_id.trim().is_empty() || svg.trim().is_empty() {
        return;
    }
    assets
        .entry(symbol_id.to_string())
        .or_insert_with(|| DocumentAsset {
            symbol_id: symbol_id.to_string(),
            svg: Some(svg.to_string()),
            png_base64: None,
        });
}

fn attach_licenses(engine: &Engine, document: &mut UniversalDocument) {
    let Ok(db) = engine.db() else {
        return;
    };
    let mut seen = std::collections::HashSet::new();
    let ids: Vec<String> = document
        .pages
        .iter()
        .flat_map(|page| page.elements.iter())
        .flat_map(|element| {
            let mut out = Vec::new();
            if let Some(id) = &element.symbol_id {
                out.push(id.clone());
            }
            if let Some(grid) = &element.grid {
                out.extend(grid.cells.iter().filter_map(|cell| cell.symbol_id.clone()));
            }
            out
        })
        .collect();
    for id in ids {
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Ok(Some(record)) = db.symbol_by_id(&id) {
            document.licenses.push(crate::universal::DocumentLicense {
                symbol_id: record.id,
                label: record.label,
                author: record.author.unwrap_or_default(),
                license: record.license,
                license_url: record.license_url.unwrap_or_default(),
                source_url: record.source_url.unwrap_or_default(),
            });
        }
    }
}

/// Converte una specifica JSON in documento universale. La specifica è un
/// input della CLI, non un formato della biblioteca: l'output è sempre un
/// `UniversalDocument` completo.
pub fn build_from_spec(engine: &Engine, spec: &Value) -> Result<BuildOutcome, String> {
    let id = string(spec, "id").ok_or_else(|| "specifica senza id".to_string())?;
    let title = string(spec, "title").unwrap_or_else(|| id.clone());
    let kind = string(spec, "kind").unwrap_or_else(|| "mixed".to_string());
    let mut document = UniversalDocument::empty(&id, &title, &kind);
    document.description = string(spec, "description").unwrap_or_default();
    document.metadata = json!({
        "category": string(spec, "category").unwrap_or_default(),
        "source_type": string(spec, "source_type").unwrap_or_else(|| "template".to_string()),
        "interactive": spec.get("interactive").and_then(|v| v.as_bool()).unwrap_or(false),
        "license": string(spec, "license").unwrap_or_else(|| "CC BY-NC-SA 4.0".to_string()),
    });
    if let Some(size) = string(spec, "page_size") {
        document.page.size = size;
    }
    if let Some(orientation) = string(spec, "orientation") {
        document.page.orientation = orientation;
    }
    if let Some(margin) = number(spec, "margin_mm") {
        document.page.margin_mm = margin;
    }
    document.pages.clear();

    let pages = spec
        .get("pages")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let mut assets: HashMap<String, DocumentAsset> = HashMap::new();
    let mut notes: Vec<String> = Vec::new();

    for (page_index, page_spec) in pages.iter().enumerate() {
        let fallback = format!("page-{}", page_index + 1);
        let page_identifier = page_id(page_spec, &fallback);
        let layout = string(page_spec, "layout").unwrap_or_else(|| "flow".to_string());
        if !matches!(layout.as_str(), "flow" | "absolute" | "grid") {
            return Err(format!("{id}/{page_identifier}: layout di pagina non valido «{layout}»"));
        }
        let mut page = UniversalPage {
            id: page_identifier.clone(),
            title: string(page_spec, "title").unwrap_or_else(|| format!("Pagina {}", page_index + 1)),
            layout: layout.clone(),
            background: string(page_spec, "background").unwrap_or_default(),
            flow: universal::FlowSettings::default(),
            grid: universal::GridSettings::default(),
            elements: Vec::new(),
            is_home: page_spec.get("home").and_then(|v| v.as_bool()).unwrap_or(page_index == 0),
            notes: string(page_spec, "notes").unwrap_or_default(),
        };
        if let Some(grid) = page_spec.get("grid") {
            page.grid.columns = grid
                .get("columns")
                .and_then(|v| v.as_u64())
                .unwrap_or(3) as u32;
            page.grid.rows = grid.get("rows").and_then(|v| v.as_u64()).unwrap_or(3) as u32;
            page.grid.gap_mm = grid
                .get("gap_mm")
                .and_then(|v| v.as_f64())
                .unwrap_or(3.0) as f32;
            page.grid.content_width_mm = grid
                .get("content_width_mm")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32);
            page.grid.content_height_mm = grid
                .get("content_height_mm")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32);
            page.grid.align = string(grid, "align").unwrap_or_default();
            page.grid.valign = string(grid, "valign").unwrap_or_default();
            page.grid.origin_x_mm = grid
                .get("origin_x_mm")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32);
            page.grid.origin_y_mm = grid
                .get("origin_y_mm")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32);
            page.grid.column_widths = grid
                .get("column_widths")
                .and_then(|v| v.as_array())
                .map(|values| values.iter().filter_map(|v| v.as_f64().map(|n| n as f32)).collect())
                .unwrap_or_default();
            page.grid.row_heights = grid
                .get("row_heights")
                .and_then(|v| v.as_array())
                .map(|values| values.iter().filter_map(|v| v.as_f64().map(|n| n as f32)).collect())
                .unwrap_or_default();
        }
        if let Some(flow) = page_spec.get("flow") {
            page.flow.columns = flow
                .get("columns")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u32;
            page.flow.gap_mm = flow
                .get("gap_mm")
                .and_then(|v| v.as_f64())
                .unwrap_or(3.0) as f32;
        }

        let elements = page_spec
            .get("elements")
            .or_else(|| page_spec.get("cells"))
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        let columns = page.grid.columns.max(1) as usize;
        let grid_elements = elements
            .iter()
            .filter(|element| {
                !element
                    .get("absolute")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
            })
            .count();
        let needed_rows = grid_elements.div_ceil(columns).max(1) as u32;
        if layout == "grid" {
            page.grid.rows = page.grid.rows.max(needed_rows);
        }

        let mut grid_index = 0usize;
        for (index, element_spec) in elements.iter().enumerate() {
            let element_id = page_id(
                element_spec,
                &format!("{}-el-{}", page.id, index + 1),
            );
            let element_absolute = element_spec
                .get("absolute")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let label = string(element_spec, "text")
                .or_else(|| string(element_spec, "label"))
                .unwrap_or_default();
            let requested_kind = string(element_spec, "kind");
            let explicit_symbol = string(element_spec, "symbol_id");
            let has_symbol_field = element_spec.get("symbol").is_some()
                || element_spec.get("symbol_label").is_some()
                || explicit_symbol.is_some();
            let symbol_bearing = requested_kind
                .as_deref()
                .map(|kind| matches!(kind, "symbol" | "image" | "communication_cell"))
                .unwrap_or(false)
                || has_symbol_field;
            let symbol_label = string(element_spec, "symbol")
                .or_else(|| string(element_spec, "symbol_label"))
                .unwrap_or_else(|| label.clone());
            let concept_hint = string(element_spec, "concept")
                .or_else(|| string(element_spec, "concept_id"));
            let resolved = if symbol_bearing
                && (!symbol_label.trim().is_empty() || explicit_symbol.is_some())
            {
                resolve_symbol(
                    engine,
                    &symbol_label,
                    explicit_symbol.as_deref(),
                    concept_hint.as_deref(),
                )?
            } else {
                None
            };
            let (symbol_id, concept_id, svg, image, symbol_label_resolved, lemma) = match resolved {
                Some((symbol_id, concept_id, svg, image, label, lemma)) => {
                    add_asset(&mut assets, Some(&symbol_id), svg.as_deref());
                    (Some(symbol_id), concept_id, svg, image, label, lemma)
                }
                None => {
                    if symbol_bearing && !symbol_label.trim().is_empty() {
                        notes.push(format!(
                            "«{}» non ha un pittogramma: resta il testo",
                            symbol_label
                        ));
                    }
                    (None, concept_hint, None, None, None, None)
                }
            };
            let kind = requested_kind.unwrap_or_else(|| {
                if symbol_id.is_some() {
                    "symbol".to_string()
                } else if label.trim().is_empty() {
                    "placeholder".to_string()
                } else {
                    "text".to_string()
                }
            });
            let mut element = UniversalElement {
                id: element_id.clone(),
                kind,
                text: label.clone(),
                spoken_text: string(element_spec, "spoken_text")
                    .or_else(|| string(element_spec, "spokenText"))
                    .unwrap_or_else(|| label.clone()),
                concept_id,
                symbol_id,
                lemma,
                svg,
                image_asset: image,
                x_mm: number(element_spec, "x_mm").unwrap_or(0.0),
                y_mm: number(element_spec, "y_mm").unwrap_or(0.0),
                w_mm: number(element_spec, "w_mm").unwrap_or(0.0),
                h_mm: number(element_spec, "h_mm").unwrap_or(0.0),
                rotation: number(element_spec, "rotation").unwrap_or(0.0),
                z: element_spec.get("z").and_then(|v| v.as_i64()).unwrap_or(index as i64) as i32,
                row: element_spec
                    .get("row")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(if element_absolute {
                        0
                    } else {
                        (grid_index / columns) as u64
                    }) as u32,
                column: element_spec
                    .get("column")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(if element_absolute {
                        0
                    } else {
                        (grid_index % columns) as u64
                    }) as u32,
                row_span: element_spec
                    .get("row_span")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as u32,
                column_span: element_spec
                    .get("column_span")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as u32,
                action: element_spec.get("action").and_then(action_from),
                link_page: string(element_spec, "link_page"),
                metadata: element_spec
                    .get("metadata")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                visible: element_spec
                    .get("visible")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                locked: element_spec
                    .get("locked")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                absolute: element_spec
                    .get("absolute")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                opacity: element_spec
                    .get("opacity")
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(1.0),
                ..UniversalElement::default()
            };
            if let Some(accessibility) = element_spec.get("accessibility") {
                element.accessibility = serde_json::from_value(accessibility.clone())
                    .map_err(|e| format!("{id}/{element_id}: accessibilità non valida: {e}"))?;
            }
            if let Some(style) = element_spec.get("style") {
                element.style = serde_json::from_value(style.clone())
                    .map_err(|e| format!("{id}/{element_id}: stile non valido: {e}"))?;
            }
            if symbol_bearing && element.symbol_id.is_none() && !symbol_label.trim().is_empty() {
                if !element.metadata.is_object() {
                    element.metadata = json!({});
                }
                if let Some(object) = element.metadata.as_object_mut() {
                    object.insert("missing_symbol".to_string(), json!(true));
                    object.insert("missing_label".to_string(), json!(symbol_label));
                }
            }
            if let Some(grid) = element_spec.get("cells").and_then(|v| v.as_array()) {
                let cell_columns = element_spec
                    .get("columns")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(columns as u64)
                    .max(1) as usize;
                let mut cells = Vec::new();
                for (cell_index, cell_spec) in grid.iter().enumerate() {
                    let text = string(cell_spec, "text")
                        .or_else(|| string(cell_spec, "label"))
                        .unwrap_or_default();
                    let symbol_label = string(cell_spec, "symbol")
                        .or_else(|| string(cell_spec, "symbol_label"))
                        .unwrap_or_else(|| text.clone());
                    let resolved = resolve_symbol(
                        engine,
                        &symbol_label,
                        string(cell_spec, "symbol_id").as_deref(),
                        string(cell_spec, "concept").as_deref(),
                    )?;
                    let (symbol_id, concept_id, svg, image) = match resolved {
                        Some((symbol_id, concept_id, svg, image, _, _)) => {
                            add_asset(&mut assets, Some(&symbol_id), svg.as_deref());
                            (Some(symbol_id), concept_id, svg, image)
                        }
                        None => (None, None, None, None),
                    };
                    let cell_missing = symbol_id.is_none() && !text.trim().is_empty();
                    cells.push(GridCell {
                        id: page_id(cell_spec, &format!("{}-cell-{}", element_id, cell_index + 1)),
                        row: (cell_index / cell_columns) as u32,
                        column: (cell_index % cell_columns) as u32,
                        row_span: 1,
                        column_span: 1,
                        text,
                        spoken_text: string(cell_spec, "spoken_text")
                            .or_else(|| string(cell_spec, "spokenText"))
                            .unwrap_or_default(),
                        concept_id,
                        symbol_id,
                        lemma: None,
                        svg,
                        image_asset: image,
                        action: cell_spec.get("action").and_then(action_from),
                        style: cell_spec
                            .get("style")
                            .and_then(|style| serde_json::from_value(style.clone()).ok())
                            .unwrap_or_default(),
                        hidden: cell_spec
                            .get("hidden")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        metadata: cell_spec
                            .get("metadata")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        missing_symbol: cell_missing,
                    });
                }
                element.grid = Some(universal::GridData {
                    columns: element_spec
                        .get("columns")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(columns as u64) as u32,
                    rows: element_spec
                        .get("rows")
                        .and_then(|v| v.as_u64())
                        .unwrap_or((cells.len().div_ceil(cell_columns)) as u64)
                        as u32,
                    gap_mm: element_spec
                        .get("gap_mm")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(3.0) as f32,
                    column_widths: element_spec
                        .get("column_widths")
                        .and_then(|v| v.as_array())
                        .map(|values| {
                            values.iter().filter_map(|v| v.as_f64().map(|n| n as f32)).collect()
                        })
                        .unwrap_or_default(),
                    row_heights: element_spec
                        .get("row_heights")
                        .and_then(|v| v.as_array())
                        .map(|values| {
                            values.iter().filter_map(|v| v.as_f64().map(|n| n as f32)).collect()
                        })
                        .unwrap_or_default(),
                    cells,
                });
            }
            let _ = symbol_label_resolved;
            if !element_absolute {
                grid_index += 1;
            }
            page.elements.push(element);
        }
        // Anche la generazione dei modelli passa dalle stesse operazioni
        // tipizzate usate dall'editor e dalla CLI.
        universal::apply_command(
            &mut document,
            &universal::DocumentCommand::AddPage {
                id: Some(page.id.clone()),
                title: Some(page.title.clone()),
                layout: Some(page.layout.clone()),
                after: None,
            },
        )?;
        if page.layout == "grid" {
            universal::apply_command(
                &mut document,
                &universal::DocumentCommand::SetPageGrid {
                    page: page.id.clone(),
                    columns: page.grid.columns,
                    rows: page.grid.rows,
                    gap_mm: Some(page.grid.gap_mm),
                    column_widths: if page.grid.column_widths.is_empty() {
                        None
                    } else {
                        Some(page.grid.column_widths.clone())
                    },
                    row_heights: if page.grid.row_heights.is_empty() {
                        None
                    } else {
                        Some(page.grid.row_heights.clone())
                    },
                    content_width_mm: page.grid.content_width_mm,
                    content_height_mm: page.grid.content_height_mm,
                    align: if page.grid.align.is_empty() {
                        None
                    } else {
                        Some(page.grid.align.clone())
                    },
                    valign: if page.grid.valign.is_empty() {
                        None
                    } else {
                        Some(page.grid.valign.clone())
                    },
                    origin_x_mm: page.grid.origin_x_mm,
                    origin_y_mm: page.grid.origin_y_mm,
                },
            )?;
        }
        universal::apply_command(
            &mut document,
            &universal::DocumentCommand::UpdatePage {
                id: page.id.clone(),
                patch: json!({
                    "background": page.background,
                    "is_home": page.is_home,
                    "flow": page.flow,
                    "notes": page.notes,
                }),
            },
        )?;
        for element in page.elements {
            universal::apply_command(
                &mut document,
                &universal::DocumentCommand::AddElement {
                    page: page.id.clone(),
                    element: serde_json::to_value(element).map_err(|e| e.to_string())?,
                    index: None,
                },
            )?;
        }
    }

    if document.pages.is_empty() {
        document.pages.push(UniversalPage {
            id: "page-1".to_string(),
            title: title.clone(),
            layout: "flow".to_string(),
            background: String::new(),
            flow: Default::default(),
            grid: Default::default(),
            elements: Vec::new(),
            is_home: true,
            notes: String::new(),
        });
    }
    attach_licenses(engine, &mut document);
    universal::validate(&document)?;
    document.render()?;
    Ok(BuildOutcome {
        document,
        assets: assets.into_values().collect(),
        notes,
    })
}

/// Converte un piano dello Studio (anche prodotto dal modello neurale) in un
/// documento universale nativo. Non genera HTML: usa gli elementi e le azioni
/// dello stesso motore dell'editor.
pub fn from_plan(plan: &crate::studio::ResourcePlan) -> Result<UniversalDocument, String> {
    let mut document = UniversalDocument::empty(
        &format!("studio-{}", crate::pictogram_import::slugify(&plan.title)),
        &plan.title,
        if plan.kind == "schedule" { "visual_schedule" } else { "communication_grid" },
    );
    document.description = plan.notes.join(" ");
    document.metadata = json!({
        "category": plan.kind,
        "source_type": "generated",
        "interactive": true,
        "license": "CC BY-NC-SA 4.0",
    });
    document.pages.clear();
    let columns = plan.grid.columns.max(1);
    let rows = plan
        .items
        .len()
        .div_ceil(columns as usize)
        .max(1) as u32;
    let mut page = UniversalPage {
        id: "home".to_string(),
        title: plan.title.clone(),
        layout: "grid".to_string(),
        background: String::new(),
        flow: Default::default(),
        grid: universal::GridSettings {
            columns,
            rows,
            gap_mm: 3.0,
            ..Default::default()
        },
        elements: Vec::new(),
        is_home: true,
        notes: String::new(),
    };
    for (index, item) in plan.items.iter().enumerate() {
        page.elements.push(UniversalElement {
            id: format!("gen-{}", index + 1),
            kind: if item.symbol_id.is_some() { "symbol" } else { "text" }.to_string(),
            text: item.label.clone(),
            spoken_text: item.label.clone(),
            concept_id: item.concept_id.clone(),
            symbol_id: item.symbol_id.clone(),
            lemma: None,
            svg: item.svg.clone(),
            image_asset: None,
            audio_asset: None,
            x_mm: 0.0,
            y_mm: 0.0,
            w_mm: 0.0,
            h_mm: 0.0,
            rotation: 0.0,
            z: index as i32,
            visible: true,
            locked: false,
            absolute: false,
            opacity: 1.0,
            row: (index / columns as usize) as u32,
            column: (index % columns as usize) as u32,
            row_span: 1,
            column_span: 1,
            style: universal::ElementStyle {
                font_size: 4.2,
                align: "center".to_string(),
                border_width: 1.0,
                border_color: "#5b6575".to_string(),
                ..Default::default()
            },
            action: Some(UniversalAction {
                action_type: "append".to_string(),
                target_page: None,
                target_document: None,
                url: None,
            }),
            link_page: None,
            group_id: None,
            children: Vec::new(),
            grid: None,
            accessibility: Default::default(),
            metadata: Value::Null,
            disposition: String::new(),
        });
    }
    document.pages.push(page);
    universal::validate(&document)?;
    document.render()?;
    Ok(document)
}

pub fn plain_text(document: &UniversalDocument) -> String {
    let mut lines = Vec::new();
    for page in &document.pages {
        lines.push(page.title.clone());
        for element in &page.elements {
            if !element.text.trim().is_empty() {
                lines.push(element.text.clone());
            }
            if let Some(grid) = &element.grid {
                for cell in &grid.cells {
                    if !cell.text.trim().is_empty() {
                        lines.push(cell.text.clone());
                    }
                }
            }
        }
    }
    lines.join("\n")
}

pub fn assets_from_document(document: &UniversalDocument) -> Vec<DocumentAsset> {
    let mut map: HashMap<String, DocumentAsset> = HashMap::new();
    for page in &document.pages {
        for element in &page.elements {
            if let (Some(symbol_id), Some(svg)) = (&element.symbol_id, &element.svg) {
                add_asset(&mut map, Some(symbol_id), Some(svg));
            }
            if let Some(grid) = &element.grid {
                for cell in &grid.cells {
                    if let (Some(symbol_id), Some(svg)) = (&cell.symbol_id, &cell.svg) {
                        add_asset(&mut map, Some(symbol_id), Some(svg));
                    }
                }
            }
        }
    }
    map.into_values().collect()
}

pub fn save_universal(
    path: &Path,
    document: &UniversalDocument,
    assets: &[DocumentAsset],
) -> Result<ExportResult, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cartella documento: {e}"))?;
    }
    let payload = DocumentPayload {
        title: document.title.clone(),
        content: json!({"type": "doc", "content": []}),
        plain_text: plain_text(document),
        settings: json!({ "universal": true }),
        assets: assets.to_vec(),
        preferences: Vec::new(),
        layout: None,
        universal: Some(serde_json::to_value(document).map_err(|e| e.to_string())?),
    };
    caadoc::save(path, &payload, &[])
}

pub fn load_universal(path: &Path) -> Result<UniversalDocument, String> {
    universal::load_from_caadoc(path)
}

pub fn preview_svg(document: &UniversalDocument) -> Result<String, String> {
    let rendered = document.render()?;
    let page = rendered
        .pages
        .first()
        .ok_or_else(|| "documento senza pagine".to_string())?;
    Ok(universal::render_page_svg(page))
}

pub fn export_html(document: &UniversalDocument) -> Result<String, String> {
    let rendered = document.render()?;
    Ok(universal::render_html(&rendered))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LibraryBuildReport {
    pub templates_expected: usize,
    pub boards_expected: usize,
    pub templates_built: usize,
    pub boards_built: usize,
    pub built: usize,
    pub valid: usize,
    pub modifiable: usize,
    pub failed: usize,
    pub missing_symbols: usize,
    pub missing_examples: Vec<String>,
    pub failures: Vec<Value>,
    pub generated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BuildAllResult {
    pub manifest: LibraryManifest,
    pub report: LibraryBuildReport,
}

/// Rigenera l'intera biblioteca dalle specifiche versionate. Scrive documenti,
/// anteprime, manifest e report; non pubblica nulla se una specifica fallisce
/// la validazione o il rendering.
pub fn build_all(
    engine: &Engine,
    sources: &Path,
    out: &Path,
    resources_root: &Path,
) -> Result<BuildAllResult, String> {
    let mut specs: Vec<(PathBuf, Value)> = Vec::new();
    collect_specs(sources, &mut specs)?;
    specs.sort_by(|a, b| a.0.cmp(&b.0));
    let templates_expected = specs
        .iter()
        .filter(|(_, spec)| {
            string(spec, "source_type").unwrap_or_else(|| "template".to_string()) == "template"
        })
        .count();
    let boards_expected = specs.len().saturating_sub(templates_expected);

    let mut entries = Vec::new();
    let mut failures = Vec::new();
    let mut built = 0usize;
    let mut valid = 0usize;
    let mut templates_built = 0usize;
    let mut boards_built = 0usize;
    let mut missing_total = 0usize;
    let mut missing_examples: Vec<String> = Vec::new();
    let cwd = std::env::current_dir().map_err(|e| format!("cartella corrente: {e}"))?;
    let out_root = if out.is_absolute() { out.to_path_buf() } else { cwd.join(out) };
    let resources_abs = if resources_root.is_absolute() {
        resources_root.to_path_buf()
    } else {
        cwd.join(resources_root)
    };

    std::fs::create_dir_all(out_root.join("templates"))
        .map_err(|e| format!("cartella templates: {e}"))?;
    std::fs::create_dir_all(out_root.join("boards")).map_err(|e| format!("cartella boards: {e}"))?;
    std::fs::create_dir_all(out_root.join("previews"))
        .map_err(|e| format!("cartella previews: {e}"))?;

    for (path, spec) in &specs {
        let source_type = string(spec, "source_type").unwrap_or_else(|| "template".to_string());
        let id = match string(spec, "id") {
            Some(id) => id,
            None => {
                failures.push(json!({"source": path, "error": "specifica senza id"}));
                continue;
            }
        };
        let outcome = match build_from_spec(engine, spec) {
            Ok(outcome) => outcome,
            Err(error) => {
                failures.push(json!({"id": id, "source": path, "error": error}));
                continue;
            }
        };
        if let Err(error) = universal::validate(&outcome.document) {
            failures.push(json!({"id": id, "source": path, "error": error}));
            continue;
        }
        let rendered = match outcome.document.render() {
            Ok(rendered) => rendered,
            Err(error) => {
                failures.push(json!({"id": id, "source": path, "error": error}));
                continue;
            }
        };
        valid += 1;
        let folder = if source_type == "board" { "boards" } else { "templates" };
        let target = out_root.join(folder).join(format!("{id}.caadoc"));
        if let Err(error) = save_universal(&target, &outcome.document, &outcome.assets) {
            failures.push(json!({"id": id, "source": path, "error": error}));
            valid -= 1;
            continue;
        }
        if let Some(page) = rendered.pages.first() {
            let preview = out_root.join("previews").join(format!("{id}.svg"));
            if let Err(error) = std::fs::write(&preview, universal::render_page_svg(page)) {
                failures.push(json!({"id": id, "source": path, "error": format!("anteprima: {error}")}));
            }
        }
        for page in &rendered.pages {
            let preview = out_root
                .join("previews")
                .join(format!("{}-p{}.svg", id, page.index + 1));
            let _ = std::fs::write(&preview, universal::render_page_svg(page));
        }
        let relative = target
            .strip_prefix(&resources_abs)
            .unwrap_or(&target)
            .to_string_lossy()
            .to_string();
        let taxonomy = spec.get("taxonomy").cloned().unwrap_or(Value::Null);
        let taxonomy_string = |key: &str| {
            taxonomy
                .get(key)
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .unwrap_or_default()
        };
        let taxonomy_bool = |key: &str| taxonomy.get(key).and_then(|value| value.as_bool());
        let mut entry = LibraryEntry {
            id: id.clone(),
            title: outcome.document.title.clone(),
            category: outcome
                .document
                .metadata
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            description: outcome.document.description.clone(),
            source_type: source_type.clone(),
            kind: outcome.document.kind.clone(),
            pages: outcome.document.pages.len(),
            elements: outcome
                .document
                .pages
                .iter()
                .map(|page| page.elements.len())
                .sum(),
            interactive: outcome
                .document
                .metadata
                .get("interactive")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            missing_symbols: outcome.notes.len(),
            path: relative,
            preview: format!("library/previews/{id}.svg"),
            category_id: taxonomy_string("category_id"),
            subcategory_id: taxonomy_string("subcategory_id"),
            subcategory: taxonomy_string("subcategory"),
            audience: taxonomy_string("audience"),
            objective: taxonomy_string("objective"),
            tags: taxonomy
                .get("tags")
                .and_then(|value| value.as_array())
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|value| value.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            complexity: taxonomy_string("complexity"),
            needs_symbols: taxonomy_bool("needs_symbols")
                .unwrap_or_else(|| outcome.notes.len() > 0),
            printable: taxonomy_bool("printable").unwrap_or(true),
            customizable: taxonomy_bool("customizable").unwrap_or(true),
            featured: taxonomy_bool("featured").unwrap_or(false),
        };
        let class = crate::template_catalog::classify(&entry);
        if entry.category_id.is_empty() {
            entry.category_id = class.category_id.clone();
        }
        if entry.subcategory_id.is_empty() {
            entry.subcategory_id = class.subcategory_id.clone();
        }
        if entry.subcategory.is_empty() {
            entry.subcategory = class.subcategory.clone();
        }
        if entry.audience.is_empty() {
            entry.audience = class.audience.clone();
        }
        if entry.objective.is_empty() {
            entry.objective = class.objective.clone();
        }
        if entry.tags.is_empty() {
            entry.tags = class.tags.clone();
        }
        if entry.complexity.is_empty() {
            entry.complexity = class.complexity.clone();
        }
        entries.push(entry);
        missing_total += outcome.notes.len();
        for note in outcome.notes.iter().take(3) {
            if missing_examples.len() < 30 {
                missing_examples.push(format!("{}: {note}", id));
            }
        }
        built += 1;
        if source_type == "board" {
            boards_built += 1;
        } else {
            templates_built += 1;
        }
    }

    let generated_at = crate::util::now_iso();
    let report = LibraryBuildReport {
        templates_expected,
        boards_expected,
        templates_built,
        boards_built,
        built,
        valid,
        modifiable: valid,
        failed: failures.len(),
        missing_symbols: missing_total,
        missing_examples: missing_examples.clone(),
        failures: failures.clone(),
        generated_at: generated_at.clone(),
    };
    let manifest = LibraryManifest {
        version: 1,
        generated_at,
        entries,
        report: serde_json::to_value(&report).unwrap_or(Value::Null),
    };
    std::fs::write(
        out_root.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("manifest: {e}"))?;
    std::fs::write(
        out_root.join("report.json"),
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("report: {e}"))?;
    Ok(BuildAllResult { manifest, report })
}

fn collect_specs(dir: &Path, out: &mut Vec<(PathBuf, Value)>) -> Result<(), String> {
    if !dir.exists() {
        return Err(format!("cartella specifiche non trovata: {}", dir.display()));
    }
    for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json")
            || path.file_name().and_then(|name| name.to_str()) == Some("manifest.json")
        {
            continue;
        }
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let value: Value = serde_json::from_str(&raw)
            .map_err(|e| format!("{}: specifica non valida: {e}", path.display()))?;
        out.push((path.to_path_buf(), value));
    }
    Ok(())
}
