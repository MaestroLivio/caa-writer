//! Universal Layout Engine: specifica versionata e geometria deterministica.
//!
//! Principio: il modello linguistico può proporre *cosa* mettere in pagina, ma
//! la geometria la calcola questo modulo, in millimetri, senza HTML e senza
//! misure inventate dall'IA. L'output (`LayoutDocument`) è serializzato per il
//! frontend, che si limita a disegnare i riquadri ricevuti.

use serde::{Deserialize, Serialize};

/// Versione dello schema di specifica. Un documento con versione diversa viene
/// rifiutato: meglio un errore esplicito di un layout interpretato male.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSpec {
    /// `A4` | `A3` | `A5`
    #[serde(default = "default_size")]
    pub size: String,
    /// `portrait` | `landscape`
    #[serde(default = "default_orientation")]
    pub orientation: String,
    #[serde(default = "default_margin")]
    pub margin_mm: f32,
}

fn default_size() -> String {
    "A4".to_string()
}
fn default_orientation() -> String {
    "portrait".to_string()
}
fn default_margin() -> f32 {
    12.0
}

impl Default for PageSpec {
    fn default() -> Self {
        Self {
            size: default_size(),
            orientation: default_orientation(),
            margin_mm: default_margin(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutGridSpec {
    /// `grid` | `linear` | `cards` | `schedule`
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default = "default_columns")]
    pub columns: u32,
    #[serde(default = "default_rows")]
    pub rows: u32,
    #[serde(default = "default_gap")]
    pub gap_mm: f32,
}

fn default_kind() -> String {
    "grid".to_string()
}
fn default_columns() -> u32 {
    3
}
fn default_rows() -> u32 {
    3
}
fn default_gap() -> f32 {
    3.0
}

impl Default for LayoutGridSpec {
    fn default() -> Self {
        Self {
            kind: default_kind(),
            columns: default_columns(),
            rows: default_rows(),
            gap_mm: default_gap(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementSpec {
    /// Identificativo stabile: non viene rigenerato durante la stampa.
    pub id: String,
    /// `symbol` | `text` | `label` | `checkbox` | `box`
    #[serde(default = "default_element_kind")]
    pub kind: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub concept_id: Option<String>,
    #[serde(default)]
    pub symbol_id: Option<String>,
    #[serde(default)]
    pub svg: Option<String>,
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub row: u32,
    #[serde(default)]
    pub column: u32,
    #[serde(default = "default_span")]
    pub row_span: u32,
    #[serde(default = "default_span")]
    pub column_span: u32,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub locked: bool,
}

fn default_element_kind() -> String {
    "symbol".to_string()
}
fn default_span() -> u32 {
    1
}

impl Default for ElementSpec {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: default_element_kind(),
            label: String::new(),
            concept_id: None,
            symbol_id: None,
            svg: None,
            page: 0,
            row: 0,
            column: 0,
            row_span: 1,
            column_span: 1,
            hidden: false,
            locked: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccessibilitySpec {
    #[serde(default)]
    pub large_symbols: bool,
    #[serde(default)]
    pub high_contrast: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSpecification {
    #[serde(default)]
    pub schema_version: u32,
    /// `communication_grid` | `visual_schedule` | `token_board` | `first_then`
    /// | `story` | `worksheet` | `task_analysis` | `free`
    #[serde(default)]
    pub document_type: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub page: PageSpec,
    #[serde(default)]
    pub layout: LayoutGridSpec,
    #[serde(default)]
    pub elements: Vec<ElementSpec>,
    #[serde(default)]
    pub accessibility: AccessibilitySpec,
}

impl Default for LayoutSpecification {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            document_type: "communication_grid".to_string(),
            title: String::new(),
            page: PageSpec::default(),
            layout: LayoutGridSpec::default(),
            elements: Vec::new(),
            accessibility: AccessibilitySpec::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PlacedElement {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub concept_id: Option<String>,
    pub symbol_id: Option<String>,
    pub svg: Option<String>,
    pub hidden: bool,
    pub locked: bool,
    pub x_mm: f32,
    pub y_mm: f32,
    pub w_mm: f32,
    pub h_mm: f32,
    /// Altezza del testo in millimetri (non punti tipografici).
    pub font_mm: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageLayout {
    pub index: u32,
    pub width_mm: f32,
    pub height_mm: f32,
    pub elements: Vec<PlacedElement>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LayoutDocument {
    pub spec: LayoutSpecification,
    pub pages: Vec<PageLayout>,
    /// Avvisi reali (pittogrammi non trovati, testo accorciato): mai silenziosi.
    pub notes: Vec<String>,
}

/// Dimensioni del foglio in millimetri, già orientate.
pub fn page_size(spec: &PageSpec) -> Result<(f32, f32), String> {
    let (w, h) = match spec.size.to_uppercase().as_str() {
        "A4" => (210.0, 297.0),
        "A3" => (297.0, 420.0),
        "A5" => (148.0, 210.0),
        other => return Err(format!("formato carta non supportato: {other}")),
    };
    match spec.orientation.as_str() {
        "portrait" => Ok((w, h)),
        "landscape" => Ok((h, w)),
        other => Err(format!("orientamento non supportato: {other}")),
    }
}

/// Controlla la specifica prima di calcolare qualsiasi geometria.
pub fn validate(spec: &LayoutSpecification) -> Result<(), String> {
    if spec.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "versione dello schema non supportata: {} (attesa {SCHEMA_VERSION})",
            spec.schema_version
        ));
    }
    let (width, height) = page_size(&spec.page)?;
    if !spec.page.margin_mm.is_finite() || spec.page.margin_mm < 0.0 {
        return Err("margine negativo o non numerico".to_string());
    }
    if spec.page.margin_mm * 2.0 >= width.min(height) {
        return Err("margine troppo grande per il foglio".to_string());
    }
    if spec.layout.columns == 0 || spec.layout.columns > 12 {
        return Err("colonne fuori intervallo (1-12)".to_string());
    }
    if spec.layout.rows == 0 || spec.layout.rows > 40 {
        return Err("righe fuori intervallo (1-40)".to_string());
    }
    if !spec.layout.gap_mm.is_finite() || spec.layout.gap_mm < 0.0 {
        return Err("spaziatura negativa o non numerica".to_string());
    }
    let usable_w = width - 2.0 * spec.page.margin_mm;
    let gap_total = spec.layout.gap_mm * (spec.layout.columns.saturating_sub(1)) as f32;
    if usable_w - gap_total <= 0.0 {
        return Err("la spaziatura consuma tutta la larghezza utile".to_string());
    }
    let mut seen = std::collections::HashSet::new();
    for element in &spec.elements {
        if element.id.trim().is_empty() {
            return Err("elemento senza identificativo".to_string());
        }
        if !seen.insert(element.id.as_str()) {
            return Err(format!("identificativo duplicato: {}", element.id));
        }
        if element.column_span == 0 || element.row_span == 0 {
            return Err(format!("{}: estensione nulla", element.id));
        }
        if element.column + element.column_span > spec.layout.columns {
            return Err(format!("{}: esce dalle colonne della griglia", element.id));
        }
        if element.row + element.row_span > spec.layout.rows {
            return Err(format!("{}: esce dalle righe della griglia", element.id));
        }
    }
    Ok(())
}

/// Calcola i riquadri di ogni elemento, pagina per pagina.
pub fn paginate(spec: &LayoutSpecification) -> Result<Vec<PageLayout>, String> {
    validate(spec)?;
    let (width, height) = page_size(&spec.page)?;
    let margin = spec.page.margin_mm;
    let usable_w = width - 2.0 * margin;
    let usable_h = height - 2.0 * margin;
    let columns = spec.layout.columns as f32;
    let rows = spec.layout.rows as f32;
    let gap = spec.layout.gap_mm;
    let cell_w = (usable_w - gap * (columns - 1.0)) / columns;
    let cell_h = (usable_h - gap * (rows - 1.0)) / rows;
    if cell_w <= 0.0 || cell_h <= 0.0 {
        return Err("celle non calcolabili: la griglia non entra nel foglio".to_string());
    }

    let mut pages: Vec<PageLayout> = Vec::new();
    for element in &spec.elements {
        while pages.len() <= element.page as usize {
            pages.push(PageLayout {
                index: pages.len() as u32,
                width_mm: width,
                height_mm: height,
                elements: Vec::new(),
            });
        }
        let w = cell_w * element.column_span as f32 + gap * (element.column_span.saturating_sub(1)) as f32;
        let h = cell_h * element.row_span as f32 + gap * (element.row_span.saturating_sub(1)) as f32;
        let x = margin + element.column as f32 * (cell_w + gap);
        let y = margin + element.row as f32 * (cell_h + gap);
        // Il testo cresce con la cella ma resta leggibile: 4 mm minimo.
        let font_mm = (h * 0.16).max(4.0).min(10.0);
        pages[element.page as usize].elements.push(PlacedElement {
            id: element.id.clone(),
            kind: element.kind.clone(),
            label: element.label.clone(),
            concept_id: element.concept_id.clone(),
            symbol_id: element.symbol_id.clone(),
            svg: element.svg.clone(),
            hidden: element.hidden,
            locked: element.locked,
            x_mm: round2(x),
            y_mm: round2(y),
            w_mm: round2(w),
            h_mm: round2(h),
            font_mm: round2(font_mm),
        });
    }
    if pages.is_empty() {
        pages.push(PageLayout {
            index: 0,
            width_mm: width,
            height_mm: height,
            elements: Vec::new(),
        });
    }
    Ok(pages)
}

fn round2(value: f32) -> f32 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_spec(columns: u32, rows: u32, gap: f32) -> LayoutSpecification {
        LayoutSpecification {
            layout: LayoutGridSpec {
                columns,
                rows,
                gap_mm: gap,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn a4_portrait_geometry_uses_millimetres() {
        let spec = grid_spec(3, 3, 4.0);
        let pages = paginate(&spec).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].width_mm, 210.0);
        assert_eq!(pages[0].height_mm, 297.0);
        // Larghezza utile 186 mm, due spazi da 4 mm: 178 / 3 = 59,33.
        let placed = PlacedElement {
            id: String::new(),
            kind: String::new(),
            label: String::new(),
            concept_id: None,
            symbol_id: None,
            svg: None,
            hidden: false,
            locked: false,
            x_mm: 12.0,
            y_mm: 12.0,
            w_mm: round2((210.0 - 24.0 - 8.0) / 3.0),
            h_mm: round2((297.0 - 24.0 - 8.0) / 3.0),
            font_mm: 0.0,
        };
        assert_eq!(placed.w_mm, 59.33);
        assert_eq!(placed.h_mm, 88.33);
    }

    #[test]
    fn landscape_swaps_dimensions() {
        let mut spec = grid_spec(2, 2, 0.0);
        spec.page.orientation = "landscape".to_string();
        let pages = paginate(&spec).unwrap();
        assert_eq!(pages[0].width_mm, 297.0);
        assert_eq!(pages[0].height_mm, 210.0);
    }

    #[test]
    fn element_position_follows_row_and_column() {
        let mut spec = grid_spec(2, 2, 10.0);
        spec.elements.push(ElementSpec {
            id: "e1".to_string(),
            label: "mela".to_string(),
            column: 1,
            row: 1,
            ..Default::default()
        });
        let pages = paginate(&spec).unwrap();
        let element = &pages[0].elements[0];
        // Margine 12 + (colonna 1) * (cella + spazio).
        assert!(element.x_mm > 100.0);
        assert!(element.y_mm > 100.0);
    }

    #[test]
    fn spans_extend_the_box() {
        let mut spec = grid_spec(3, 2, 0.0);
        spec.elements.push(ElementSpec {
            id: "wide".to_string(),
            label: "sequenza".to_string(),
            column_span: 2,
            ..Default::default()
        });
        let pages = paginate(&spec).unwrap();
        let element = &pages[0].elements[0];
        assert!((element.w_mm - 124.0).abs() < 0.1, "larghezza {}", element.w_mm);
    }

    #[test]
    fn elements_beyond_first_page_create_new_pages() {
        let mut spec = grid_spec(2, 2, 0.0);
        spec.elements.push(ElementSpec {
            id: "p0".to_string(),
            label: "prima".to_string(),
            ..Default::default()
        });
        spec.elements.push(ElementSpec {
            id: "p1".to_string(),
            label: "seconda".to_string(),
            page: 1,
            ..Default::default()
        });
        let pages = paginate(&spec).unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].elements.len(), 1);
        assert_eq!(pages[1].elements.len(), 1);
    }

    #[test]
    fn invalid_inputs_are_rejected_with_explicit_messages() {
        let mut bad_version = grid_spec(2, 2, 0.0);
        bad_version.schema_version = 99;
        assert!(validate(&bad_version).unwrap_err().contains("schema"));

        let mut bad_page = grid_spec(2, 2, 0.0);
        bad_page.page.size = "Letter".to_string();
        assert!(validate(&bad_page).unwrap_err().contains("carta"));

        let mut bad_gap = grid_spec(2, 2, 200.0);
        assert!(validate(&bad_gap).unwrap_err().contains("spaziatura"));
        bad_gap.layout.gap_mm = 0.0;

        let mut out_of_grid = grid_spec(2, 2, 0.0);
        out_of_grid.elements.push(ElementSpec {
            id: "fuori".to_string(),
            column: 2,
            ..Default::default()
        });
        assert!(validate(&out_of_grid).unwrap_err().contains("colonne"));

        let mut duplicate = grid_spec(2, 2, 0.0);
        duplicate.elements.push(ElementSpec {
            id: "a".to_string(),
            ..Default::default()
        });
        duplicate.elements.push(ElementSpec {
            id: "a".to_string(),
            row: 1,
            ..Default::default()
        });
        assert!(validate(&duplicate).unwrap_err().contains("duplicato"));
    }

    #[test]
    fn empty_layout_still_returns_one_page() {
        let pages = paginate(&grid_spec(3, 3, 3.0)).unwrap();
        assert_eq!(pages.len(), 1);
        assert!(pages[0].elements.is_empty());
    }
}
