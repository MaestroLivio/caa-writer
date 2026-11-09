//! CAA Universal Document Engine.
//!
//! Un solo modello documentale per Word Processor, presentazioni, tabelle
//! comunicative, agende, storie sociali, sequenze, materiali didattici e
//! sistemi comunicativi. Il motore non dipende da Svelte, dal DOM o dalla GUI:
//! la CLI e l'editor usano le stesse strutture, le stesse operazioni e lo
//! stesso calcolo geometrico.
//!
//! Tre layout di pagina:
//! - `flow`: elementi in sequenza verticale (modalità documento);
//! - `absolute`: coordinate libere (modalità presentazione);
//! - `grid`: righe/colonne con celle (tabelle comunicative e materiali).
//!
//! Ogni elemento ha un ID stabile; i collegamenti usano l'ID della pagina, mai
//! il suo indice.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SCHEMA_VERSION: u32 = 1;

fn default_schema() -> u32 {
    SCHEMA_VERSION
}
fn default_true() -> bool {
    true
}
fn default_locale() -> String {
    "it".to_string()
}
fn default_page_size() -> String {
    "A4".to_string()
}
fn default_orientation() -> String {
    "portrait".to_string()
}
fn default_margin() -> f32 {
    12.0
}
fn default_gap() -> f32 {
    3.0
}
fn default_columns() -> u32 {
    3
}
fn default_rows() -> u32 {
    3
}
fn default_opacity() -> f32 {
    1.0
}
fn default_font_size() -> f32 {
    4.2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSettings {
    #[serde(default = "default_page_size")]
    pub size: String,
    #[serde(default = "default_orientation")]
    pub orientation: String,
    #[serde(default = "default_margin")]
    pub margin_mm: f32,
    /// Larghezza/altezza esplicite per formati personalizzati o 16:9.
    #[serde(default)]
    pub width_mm: Option<f32>,
    #[serde(default)]
    pub height_mm: Option<f32>,
    #[serde(default)]
    pub background: String,
}

impl Default for PageSettings {
    fn default() -> Self {
        Self {
            size: default_page_size(),
            orientation: default_orientation(),
            margin_mm: default_margin(),
            width_mm: None,
            height_mm: None,
            background: "#ffffff".to_string(),
        }
    }
}

impl PageSettings {
    pub fn dimensions(&self) -> Result<(f32, f32), String> {
        if let (Some(w), Some(h)) = (self.width_mm, self.height_mm) {
            if w.is_finite() && h.is_finite() && w > 20.0 && h > 20.0 {
                return Ok((w, h));
            }
            return Err("dimensioni personalizzate non valide".to_string());
        }
        let (w, h) = match self.size.to_uppercase().as_str() {
            "A4" => (210.0, 297.0),
            "A3" => (297.0, 420.0),
            "A5" => (148.0, 210.0),
            "LETTER" => (215.9, 279.4),
            "16:9" => (338.7, 190.5),
            "4:3" => (254.0, 190.5),
            other => return Err(format!("formato pagina non supportato: {other}")),
        };
        match self.orientation.as_str() {
            "portrait" => Ok((w, h)),
            "landscape" => Ok((h, w)),
            other => Err(format!("orientamento non supportato: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowSettings {
    #[serde(default = "default_columns")]
    pub columns: u32,
    #[serde(default = "default_gap")]
    pub gap_mm: f32,
    #[serde(default)]
    pub padding_mm: f32,
}

impl Default for FlowSettings {
    fn default() -> Self {
        Self {
            columns: 1,
            gap_mm: 3.0,
            padding_mm: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridSettings {
    #[serde(default = "default_columns")]
    pub columns: u32,
    #[serde(default = "default_rows")]
    pub rows: u32,
    #[serde(default = "default_gap")]
    pub gap_mm: f32,
    /// Pesi delle colonne; vuoto = colonne uguali.
    #[serde(default)]
    pub column_widths: Vec<f32>,
    /// Pesi delle righe; vuoto = righe uguali.
    #[serde(default)]
    pub row_heights: Vec<f32>,
    /// Area occupata dalla griglia. Se assente resta l'intera area utile:
    /// i documenti precedenti mantengono la geometria di prima.
    #[serde(default)]
    pub content_width_mm: Option<f32>,
    #[serde(default)]
    pub content_height_mm: Option<f32>,
    /// left | center | right
    #[serde(default)]
    pub align: String,
    /// top | middle | bottom
    #[serde(default)]
    pub valign: String,
    /// Origine dell'area griglia, in millimetri dal bordo pagina. Se assente
    /// vale il margine; permette intestazioni e piedi senza sovrapposizioni.
    #[serde(default)]
    pub origin_x_mm: Option<f32>,
    #[serde(default)]
    pub origin_y_mm: Option<f32>,
}

impl Default for GridSettings {
    fn default() -> Self {
        Self {
            columns: 3,
            rows: 3,
            gap_mm: 3.0,
            column_widths: Vec::new(),
            row_heights: Vec::new(),
            content_width_mm: None,
            content_height_mm: None,
            align: String::new(),
            valign: String::new(),
            origin_x_mm: None,
            origin_y_mm: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ElementStyle {
    #[serde(default)]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub background: String,
    #[serde(default)]
    pub border_color: String,
    #[serde(default)]
    pub border_width: f32,
    #[serde(default)]
    pub border_style: String,
    #[serde(default)]
    pub border_radius: f32,
    #[serde(default)]
    pub padding_mm: f32,
    #[serde(default)]
    pub align: String,
    #[serde(default)]
    pub valign: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Accessibility {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub large_text: bool,
    #[serde(default)]
    pub high_contrast: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentLicense {
    #[serde(default)]
    pub symbol_id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub license_url: String,
    #[serde(default)]
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalAction {
    #[serde(rename = "type", default)]
    pub action_type: String,
    /// ID stabile della pagina di destinazione (non un indice).
    #[serde(default)]
    pub target_page: Option<String>,
    #[serde(default)]
    pub target_document: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

impl Default for UniversalAction {
    fn default() -> Self {
        Self {
            action_type: "append".to_string(),
            target_page: None,
            target_document: None,
            url: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GridCell {
    pub id: String,
    #[serde(default)]
    pub row: u32,
    #[serde(default)]
    pub column: u32,
    #[serde(default = "default_span")]
    pub row_span: u32,
    #[serde(default = "default_span")]
    pub column_span: u32,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub spoken_text: String,
    #[serde(default)]
    pub concept_id: Option<String>,
    #[serde(default)]
    pub symbol_id: Option<String>,
    #[serde(default)]
    pub lemma: Option<String>,
    #[serde(default)]
    pub svg: Option<String>,
    /// PNG come data URL; alternativa raster a `svg`.
    #[serde(default)]
    pub image_asset: Option<String>,
    #[serde(default)]
    pub action: Option<UniversalAction>,
    #[serde(default)]
    pub style: ElementStyle,
    #[serde(default)]
    pub hidden: bool,
    /// Vero quando il testo è presente ma il repertorio non offre un
    /// pittogramma: il dato non viene nascosto, viene dichiarato.
    #[serde(default)]
    pub missing_symbol: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GridData {
    #[serde(default)]
    pub columns: u32,
    #[serde(default)]
    pub rows: u32,
    #[serde(default)]
    pub gap_mm: f32,
    /// Pesi delle colonne; vuoto = colonne uguali. Valori positivi.
    #[serde(default)]
    pub column_widths: Vec<f32>,
    /// Pesi delle righe; vuoto = righe uguali.
    #[serde(default)]
    pub row_heights: Vec<f32>,
    #[serde(default)]
    pub cells: Vec<GridCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalElement {
    pub id: String,
    /// text | caa_text | symbol | image | shape | line | arrow | text_box |
    /// table | grid | communication_cell | button | link | audio | sequence |
    /// container | group | placeholder
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub spoken_text: String,
    #[serde(default)]
    pub concept_id: Option<String>,
    #[serde(default)]
    pub symbol_id: Option<String>,
    #[serde(default)]
    pub lemma: Option<String>,
    #[serde(default)]
    pub svg: Option<String>,
    #[serde(default)]
    pub image_asset: Option<String>,
    #[serde(default)]
    pub audio_asset: Option<String>,
    #[serde(default)]
    pub x_mm: f32,
    #[serde(default)]
    pub y_mm: f32,
    #[serde(default)]
    pub w_mm: f32,
    #[serde(default)]
    pub h_mm: f32,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub z: i32,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    /// Su una pagina a griglia, l'elemento conserva le coordinate assolute:
    /// serve per intestazioni, barre della frase e piè di pagina.
    #[serde(default)]
    pub absolute: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub row: u32,
    #[serde(default)]
    pub column: u32,
    #[serde(default = "default_span")]
    pub row_span: u32,
    #[serde(default = "default_span")]
    pub column_span: u32,
    #[serde(default)]
    pub style: ElementStyle,
    #[serde(default)]
    pub action: Option<UniversalAction>,
    /// ID stabile della pagina collegata (per pulsanti e collegamenti).
    #[serde(default)]
    pub link_page: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub grid: Option<GridData>,
    #[serde(default)]
    pub accessibility: Accessibility,
    #[serde(default)]
    pub metadata: Value,
}

fn default_kind() -> String {
    "text".to_string()
}
fn default_span() -> u32 {
    1
}

impl Default for UniversalElement {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: default_kind(),
            text: String::new(),
            spoken_text: String::new(),
            concept_id: None,
            symbol_id: None,
            lemma: None,
            svg: None,
            image_asset: None,
            audio_asset: None,
            x_mm: 0.0,
            y_mm: 0.0,
            w_mm: 0.0,
            h_mm: 0.0,
            rotation: 0.0,
            z: 0,
            visible: true,
            locked: false,
            absolute: false,
            opacity: 1.0,
            row: 0,
            column: 0,
            row_span: 1,
            column_span: 1,
            style: ElementStyle::default(),
            action: None,
            link_page: None,
            group_id: None,
            children: Vec::new(),
            grid: None,
            accessibility: Accessibility::default(),
            metadata: Value::Null,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalPage {
    pub id: String,
    #[serde(default)]
    pub title: String,
    /// flow | absolute | grid
    #[serde(default = "default_flow")]
    pub layout: String,
    #[serde(default)]
    pub background: String,
    #[serde(default)]
    pub flow: FlowSettings,
    #[serde(default)]
    pub grid: GridSettings,
    #[serde(default)]
    pub elements: Vec<UniversalElement>,
    #[serde(default)]
    pub is_home: bool,
    #[serde(default)]
    pub notes: String,
}

fn default_flow() -> String {
    "absolute".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalDocument {
    #[serde(default = "default_schema")]
    pub schema_version: u32,
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    /// document | presentation | mixed | communication_board
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default)]
    pub page: PageSettings,
    #[serde(default)]
    pub pages: Vec<UniversalPage>,
    #[serde(default)]
    pub styles: HashMap<String, ElementStyle>,
    #[serde(default)]
    pub accessibility: Accessibility,
    #[serde(default)]
    pub licenses: Vec<DocumentLicense>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

impl Default for UniversalDocument {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            id: String::new(),
            title: String::new(),
            description: String::new(),
            kind: "document".to_string(),
            locale: "it".to_string(),
            metadata: Value::Null,
            page: PageSettings::default(),
            pages: vec![UniversalPage {
                id: "page-1".to_string(),
                title: "Pagina 1".to_string(),
                layout: "flow".to_string(),
                background: String::new(),
                flow: FlowSettings::default(),
                grid: GridSettings::default(),
                elements: Vec::new(),
                is_home: true,
                notes: String::new(),
            }],
            styles: HashMap::new(),
            accessibility: Accessibility::default(),
            licenses: Vec::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

impl UniversalDocument {
    pub fn empty(id: &str, title: &str, kind: &str) -> Self {
        let mut document = Self::default();
        document.id = id.to_string();
        document.title = title.to_string();
        document.kind = kind.to_string();
        document.created_at = crate::util::now_iso();
        document.updated_at = document.created_at.clone();
        document
    }

    pub fn page(&self, id: &str) -> Option<&UniversalPage> {
        self.pages.iter().find(|page| page.id == id)
    }

    pub fn page_mut(&mut self, id: &str) -> Option<&mut UniversalPage> {
        self.pages.iter_mut().find(|page| page.id == id)
    }

    pub fn element(&self, page_id: &str, element_id: &str) -> Option<&UniversalElement> {
        self.page(page_id)?
            .elements
            .iter()
            .find(|element| element.id == element_id)
    }

    pub fn render(&self) -> Result<RenderedDocument, String> {
        validate(self)?;
        render_document(self)
    }
}

// ------------------------------------------------------------------ validazione

pub fn validate(document: &UniversalDocument) -> Result<(), String> {
    if document.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "versione schema non supportata: {} (attesa {SCHEMA_VERSION})",
            document.schema_version
        ));
    }
    if document.id.trim().is_empty() {
        return Err("documento senza identificativo".to_string());
    }
    if document.pages.is_empty() {
        return Err("il documento deve avere almeno una pagina".to_string());
    }
    let mut page_ids = HashSet::new();
    for page in &document.pages {
        if page.id.trim().is_empty() {
            return Err("pagina senza identificativo".to_string());
        }
        if !page_ids.insert(page.id.as_str()) {
            return Err(format!("pagina duplicata: {}", page.id));
        }
        if !matches!(page.layout.as_str(), "flow" | "absolute" | "grid") {
            return Err(format!("{}: layout di pagina sconosciuto «{}»", page.id, page.layout));
        }
        if page.layout == "grid" {
            if page.grid.columns == 0 || page.grid.columns > 12 {
                return Err(format!("{}: colonne fuori intervallo (1-12)", page.id));
            }
            if page.grid.rows == 0 || page.grid.rows > 60 {
                return Err(format!("{}: righe fuori intervallo (1-60)", page.id));
            }
            if let Some(width) = page.grid.content_width_mm {
                if !width.is_finite() || width <= 0.0 {
                    return Err(format!("{}: larghezza dell'area griglia non valida", page.id));
                }
            }
            if let Some(height) = page.grid.content_height_mm {
                if !height.is_finite() || height <= 0.0 {
                    return Err(format!("{}: altezza dell'area griglia non valida", page.id));
                }
            }
            if !page.grid.column_widths.is_empty()
                && (page.grid.column_widths.len() != page.grid.columns as usize
                    || page.grid.column_widths.iter().any(|v| !v.is_finite() || *v <= 0.0))
            {
                return Err(format!("{}: pesi delle colonne non validi", page.id));
            }
            if !page.grid.row_heights.is_empty()
                && (page.grid.row_heights.len() != page.grid.rows as usize
                    || page.grid.row_heights.iter().any(|v| !v.is_finite() || *v <= 0.0))
            {
                return Err(format!("{}: pesi delle righe non validi", page.id));
            }
        }
        if page.flow.columns == 0 || page.flow.columns > 4 {
            return Err(format!("{}: colonne di flusso fuori intervallo (1-4)", page.id));
        }
        let mut element_ids = HashSet::new();
        for element in &page.elements {
            if element.id.trim().is_empty() {
                return Err(format!("{}: elemento senza identificativo", page.id));
            }
            if !element_ids.insert(element.id.as_str()) {
                return Err(format!("{}: elemento duplicato {}", page.id, element.id));
            }
            validate_element(element, page)?;
        }
    }
    // I collegamenti usano ID stabili di pagina e devono esistere davvero.
    for page in &document.pages {
        for element in &page.elements {
            if let Some(target) = &element.link_page {
                if !page_ids.contains(target.as_str()) {
                    return Err(format!(
                        "{}: collegamento alla pagina inesistente «{}»",
                        element.id, target
                    ));
                }
            }
            if let Some(action) = &element.action {
                if action.action_type == "navigate" {
                    let target = action.target_page.as_deref().unwrap_or("");
                    if !page_ids.contains(target) {
                        return Err(format!(
                            "{}: azione verso una pagina inesistente «{}»",
                            element.id, target
                        ));
                    }
                }
            }
            if let Some(grid) = &element.grid {
                let mut cell_ids = HashSet::new();
                let legacy_positions = grid.cells.iter().all(|cell| cell.row == 0 && cell.column == 0)
                    && grid.cells.len() > 1;
                for cell in &grid.cells {
                    if cell.id.trim().is_empty() || !cell_ids.insert(cell.id.as_str()) {
                        return Err(format!("{}: cella senza ID o duplicata", element.id));
                    }
                    if !legacy_positions {
                        if cell.column + cell.column_span.max(1) > grid.columns {
                            return Err(format!("{}: cella fuori dalle colonne", cell.id));
                        }
                        if cell.row + cell.row_span.max(1) > grid.rows {
                            return Err(format!("{}: cella fuori dalle righe", cell.id));
                        }
                    }
                    if let Some(action) = &cell.action {
                        if action.action_type == "navigate"
                            && !page_ids.contains(
                                action.target_page.as_deref().unwrap_or(""),
                            )
                        {
                            return Err(format!(
                                "{}: cella collegata a una pagina inesistente",
                                cell.id
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_element(element: &UniversalElement, page: &UniversalPage) -> Result<(), String> {
    let allowed = [
        "text",
        "caa_text",
        "symbol",
        "image",
        "shape",
        "line",
        "arrow",
        "text_box",
        "table",
        "grid",
        "communication_cell",
        "button",
        "link",
        "audio",
        "sequence",
        "container",
        "group",
        "placeholder",
        "title",
        "subtitle",
        "instruction",
        "question",
        "answer_line",
        "handwriting_line",
        "checkbox",
        "number_line",
        "ten_frame",
        "timeline",
        "token_slot",
        "badge",
        "note",
        "matching",
        "sequence",
        "word_bank",
        "example",
        "speech",
    ];
    if !allowed.contains(&element.kind.as_str()) {
        return Err(format!("{}: tipo di elemento sconosciuto «{}»", element.id, element.kind));
    }
    if !element.x_mm.is_finite()
        || !element.y_mm.is_finite()
        || !element.w_mm.is_finite()
        || !element.h_mm.is_finite()
    {
        return Err(format!("{}: coordinate non numeriche", element.id));
    }
    if element.w_mm < 0.0 || element.h_mm < 0.0 {
        return Err(format!("{}: dimensioni negative", element.id));
    }
    if !(-360.0..=360.0).contains(&element.rotation) {
        return Err(format!("{}: rotazione fuori intervallo", element.id));
    }
    if !(0.0..=1.0).contains(&element.opacity) {
        return Err(format!("{}: opacità fuori intervallo", element.id));
    }
    if let Some(action) = &element.action {
        let kind = action.action_type.as_str();
        if !matches!(
            kind,
            "" | "append" | "speak" | "navigate" | "back" | "home" | "clear" | "toggle"
                | "open_document"
        ) {
            return Err(format!("{}: azione non supportata «{}»", element.id, kind));
        }
        if kind == "navigate" && action.target_page.is_none() {
            return Err(format!("{}: «navigate» richiede una pagina", element.id));
        }
    }
    if page.layout == "grid" && !element.absolute {
        if element.column + element.column_span.max(1) > page.grid.columns {
            return Err(format!("{}: esce dalle colonne della griglia", element.id));
        }
        if element.row + element.row_span.max(1) > page.grid.rows {
            return Err(format!("{}: esce dalle righe della griglia", element.id));
        }
    }
    if let Some(grid) = &element.grid {
        if grid.columns == 0 || grid.rows == 0 {
            return Err(format!("{}: griglia con zero righe o colonne", element.id));
        }
        if grid.cells.len() > (grid.columns * grid.rows) as usize {
            return Err(format!("{}: più celle dello spazio disponibile", element.id));
        }
    }
    Ok(())
}

// -------------------------------------------------------------------- rendering

#[derive(Debug, Clone, Serialize)]
pub struct RenderedElement {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub spoken_text: String,
    pub concept_id: Option<String>,
    pub symbol_id: Option<String>,
    pub svg: Option<String>,
    pub image_asset: Option<String>,
    pub audio_asset: Option<String>,
    pub x_mm: f32,
    pub y_mm: f32,
    pub w_mm: f32,
    pub h_mm: f32,
    pub rotation: f32,
    pub z: i32,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub style: ElementStyle,
    pub action: Option<UniversalAction>,
    pub link_page: Option<String>,
    pub group_id: Option<String>,
    pub cells: Vec<RenderedGridCell>,
    pub accessibility: Accessibility,
    /// Metadati del componente (es. `lines`, `min`, `max`, `filled`).
    pub metadata: Value,
}

impl RenderedElement {
    /// Contenuto grafico del pittogramma: SVG oppure PNG come data URL.
    pub fn media(&self) -> Option<&str> {
        self.svg.as_deref().or(self.image_asset.as_deref())
    }
}

impl RenderedGridCell {
    pub fn media(&self) -> Option<&str> {
        self.svg.as_deref().or(self.image_asset.as_deref())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderedGridCell {
    pub id: String,
    pub row: u32,
    pub column: u32,
    pub row_span: u32,
    pub column_span: u32,
    pub text: String,
    pub spoken_text: String,
    pub symbol_id: Option<String>,
    pub concept_id: Option<String>,
    pub svg: Option<String>,
    pub image_asset: Option<String>,
    pub action: Option<UniversalAction>,
    pub hidden: bool,
    pub missing_symbol: bool,
    pub x_mm: f32,
    pub y_mm: f32,
    pub w_mm: f32,
    pub h_mm: f32,
    pub style: ElementStyle,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderedPage {
    pub id: String,
    pub title: String,
    pub index: usize,
    pub layout: String,
    pub width_mm: f32,
    pub height_mm: f32,
    pub background: String,
    pub is_home: bool,
    pub elements: Vec<RenderedElement>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderedDocument {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub schema_version: u32,
    pub pages: Vec<RenderedPage>,
    pub warnings: Vec<String>,
}

fn round2(value: f32) -> f32 {
    (value * 100.0).round() / 100.0
}

fn render_document(document: &UniversalDocument) -> Result<RenderedDocument, String> {
    let (page_w, page_h) = document.page.dimensions()?;
    let margin = document.page.margin_mm.max(0.0);
    let mut warnings = Vec::new();
    let mut pages = Vec::new();

    for (index, page) in document.pages.iter().enumerate() {
        let mut rendered: Vec<RenderedElement> = Vec::new();
        let mut flow_y = margin + page.flow.padding_mm;
        let flow_w = page_w - 2.0 * margin - 2.0 * page.flow.padding_mm;
        let column_w = if page.flow.columns > 1 {
            (flow_w - page.flow.gap_mm * (page.flow.columns.saturating_sub(1)) as f32)
                / page.flow.columns as f32
        } else {
            flow_w
        };
        // L'area della griglia può essere più piccola della pagina: è questo
        // che impedisce a una riga di cinque gettoni di diventare alta 186 mm.
        let area_x = page
            .grid
            .origin_x_mm
            .filter(|value| value.is_finite())
            .unwrap_or(margin)
            .max(margin);
        let area_y = page
            .grid
            .origin_y_mm
            .filter(|value| value.is_finite())
            .unwrap_or(margin)
            .max(margin);
        let available_w = (page_w - margin - area_x).max(1.0);
        let available_h = (page_h - margin - area_y).max(1.0);
        let content_w = page
            .grid
            .content_width_mm
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(available_w)
            .min(available_w);
        let content_h = page
            .grid
            .content_height_mm
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(available_h)
            .min(available_h);
        let grid_area_x = area_x + align_offset(available_w, content_w, &page.grid.align);
        let grid_area_y = area_y + align_offset(available_h, content_h, &page.grid.valign);
        let (grid_columns, grid_column_offsets) =
            axis_sizes(content_w, page.grid.gap_mm.max(0.0), page.grid.columns, &page.grid.column_widths);
        let (grid_rows, grid_row_offsets) =
            axis_sizes(content_h, page.grid.gap_mm.max(0.0), page.grid.rows, &page.grid.row_heights);

        let mut sorted: Vec<&UniversalElement> = page.elements.iter().collect();
        sorted.sort_by(|a, b| {
            a.z.cmp(&b.z)
                .then_with(|| a.y_mm.partial_cmp(&b.y_mm).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| a.x_mm.partial_cmp(&b.x_mm).unwrap_or(std::cmp::Ordering::Equal))
        });

        for element in sorted {
            let absolute_override = element.absolute && page.layout == "grid";
            let (x, y, w, h) = if absolute_override {
                let w = if element.w_mm > 0.0 {
                    element.w_mm
                } else {
                    (page_w - 2.0 * margin).min(120.0)
                };
                let h = if element.h_mm > 0.0 {
                    element.h_mm
                } else {
                    text_height(element)
                };
                (element.x_mm, element.y_mm, w, h)
            } else {
                match page.layout.as_str() {
                "flow" => {
                    let width = if element.w_mm > 0.0 {
                        element.w_mm.min(flow_w)
                    } else {
                        column_w
                    };
                    let height = if element.h_mm > 0.0 {
                        element.h_mm
                    } else if matches!(element.kind.as_str(), "symbol" | "button" | "grid" | "image") {
                        28.0
                    } else {
                        text_height(element)
                    };
                    let column = element.column.min(page.flow.columns.saturating_sub(1));
                    let x = margin + column as f32 * (column_w + page.flow.gap_mm);
                    let y = flow_y;
                    flow_y += height + page.flow.gap_mm;
                    (x, y, width, height)
                }
                "grid" => {
                    let col = element.column.min(page.grid.columns.saturating_sub(1));
                    let row = element.row.min(page.grid.rows.saturating_sub(1));
                    let span_cols = element.column_span.max(1).min(page.grid.columns - col);
                    let span_rows = element.row_span.max(1).min(page.grid.rows - row);
                    let w = grid_columns
                        .iter()
                        .skip(col as usize)
                        .take(span_cols as usize)
                        .sum::<f32>()
                        + page.grid.gap_mm * (span_cols.saturating_sub(1)) as f32;
                    let h = grid_rows
                        .iter()
                        .skip(row as usize)
                        .take(span_rows as usize)
                        .sum::<f32>()
                        + page.grid.gap_mm * (span_rows.saturating_sub(1)) as f32;
                    let x = grid_area_x
                        + grid_column_offsets.get(col as usize).copied().unwrap_or(0.0);
                    let y = grid_area_y
                        + grid_row_offsets.get(row as usize).copied().unwrap_or(0.0);
                    (x, y, w, h)
                }
                _ => {
                    let w = if element.w_mm > 0.0 {
                        element.w_mm
                    } else {
                        (page_w - 2.0 * margin).min(120.0)
                    };
                    let h = if element.h_mm > 0.0 {
                        element.h_mm
                    } else {
                        text_height(element)
                    };
                    (element.x_mm, element.y_mm, w, h)
                }
                }
            };
            if page.layout == "absolute" || absolute_override {
                if x + w > page_w - margin + 0.1 {
                    warnings.push(format!(
                        "{}: elemento oltre il margine destro della pagina {}",
                        element.id,
                        page.title
                    ));
                }
                if y + h > page_h - margin + 0.1 {
                    warnings.push(format!(
                        "{}: elemento oltre il margine inferiore della pagina {}",
                        element.id,
                        page.title
                    ));
                }
            }
            let cells = render_grid_cells(element, w, h);
            rendered.push(RenderedElement {
                id: element.id.clone(),
                kind: element.kind.clone(),
                text: element.text.clone(),
                spoken_text: if element.spoken_text.is_empty() {
                    element.text.clone()
                } else {
                    element.spoken_text.clone()
                },
                concept_id: element.concept_id.clone(),
                symbol_id: element.symbol_id.clone(),
                svg: element.svg.clone(),
                image_asset: element.image_asset.clone(),
                audio_asset: element.audio_asset.clone(),
                x_mm: round2(x),
                y_mm: round2(y),
                w_mm: round2(w),
                h_mm: round2(h),
                rotation: element.rotation,
                z: element.z,
                visible: element.visible,
                locked: element.locked,
                opacity: element.opacity,
                style: element.style.clone(),
                action: element.action.clone(),
                link_page: element.link_page.clone(),
                group_id: element.group_id.clone(),
                cells,
                accessibility: element.accessibility.clone(),
                metadata: element.metadata.clone(),
            });
        }
        if page.layout == "flow" && flow_y > page_h - margin {
            warnings.push(format!(
                "{}: il contenuto supera l'altezza della pagina",
                page.title
            ));
        }
        pages.push(RenderedPage {
            id: page.id.clone(),
            title: page.title.clone(),
            index,
            layout: page.layout.clone(),
            width_mm: round2(page_w),
            height_mm: round2(page_h),
            background: if page.background.is_empty() {
                document.page.background.clone()
            } else {
                page.background.clone()
            },
            is_home: page.is_home,
            elements: rendered,
        });
    }

    Ok(RenderedDocument {
        id: document.id.clone(),
        title: document.title.clone(),
        kind: document.kind.clone(),
        schema_version: document.schema_version,
        pages,
        warnings,
    })
}

fn text_height(element: &UniversalElement) -> f32 {
    let font = element.style.font_size.max(2.0);
    let chars = element.text.chars().count().max(1) as f32;
    let lines = (chars / 32.0).ceil().max(1.0);
    (lines * font * 1.5 + element.style.padding_mm * 2.0).max(8.0)
}

fn align_offset(available: f32, content: f32, align: &str) -> f32 {
    let free = (available - content).max(0.0);
    match align {
        "center" | "middle" => free / 2.0,
        "right" | "bottom" => free,
        _ => 0.0,
    }
}

fn axis_sizes(total: f32, gap: f32, count: u32, weights: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let count = count.max(1) as usize;
    let usable = (total - gap * (count.saturating_sub(1)) as f32).max(1.0);
    let weight_sum: f32 = if weights.len() == count
        && weights.iter().all(|value| value.is_finite() && *value > 0.0)
    {
        weights.iter().sum()
    } else {
        count as f32
    };
    let normalized: Vec<f32> = if weights.len() == count
        && weights.iter().all(|value| value.is_finite() && *value > 0.0)
    {
        weights.iter().map(|value| value / weight_sum).collect()
    } else {
        vec![1.0 / count as f32; count]
    };
    let mut sizes = Vec::with_capacity(count);
    let mut offsets = Vec::with_capacity(count);
    let mut cursor = 0.0;
    for (index, weight) in normalized.iter().enumerate() {
        let size = if index + 1 == count {
            (usable - sizes.iter().sum::<f32>()).max(1.0)
        } else {
            (usable * weight).max(1.0)
        };
        offsets.push(cursor);
        sizes.push(size);
        cursor += size + gap;
    }
    (sizes, offsets)
}

fn render_grid_cells(element: &UniversalElement, w: f32, h: f32) -> Vec<RenderedGridCell> {
    let Some(grid) = &element.grid else {
        return Vec::new();
    };
    let gap = grid.gap_mm.max(0.0);
    let (column_sizes, column_offsets) =
        axis_sizes(w, gap, grid.columns, &grid.column_widths);
    let (row_sizes, row_offsets) = axis_sizes(h, gap, grid.rows, &grid.row_heights);
    // I documenti creati prima del supporto alle posizioni esplicite hanno
    // tutte le celle in (0,0): in quel caso si usa l'ordine dell'array.
    let legacy_positions = grid.cells.iter().all(|cell| cell.row == 0 && cell.column == 0)
        && grid.cells.len() > 1;
    grid.cells
        .iter()
        .enumerate()
        .map(|(index, cell)| {
            let row = if legacy_positions {
                (index / grid.columns.max(1) as usize) as u32
            } else {
                cell.row
            };
            let column = if legacy_positions {
                (index % grid.columns.max(1) as usize) as u32
            } else {
                cell.column
            };
            let column_span = cell.column_span.max(1).min(grid.columns.saturating_sub(column).max(1));
            let row_span = cell.row_span.max(1).min(grid.rows.saturating_sub(row).max(1));
            let x = column_offsets.get(column as usize).copied().unwrap_or(0.0);
            let y = row_offsets.get(row as usize).copied().unwrap_or(0.0);
            let width = column_sizes
                .iter()
                .skip(column as usize)
                .take(column_span as usize)
                .sum::<f32>()
                + gap * column_span.saturating_sub(1) as f32;
            let height = row_sizes
                .iter()
                .skip(row as usize)
                .take(row_span as usize)
                .sum::<f32>()
                + gap * row_span.saturating_sub(1) as f32;
            RenderedGridCell {
                id: cell.id.clone(),
                row,
                column,
                row_span,
                column_span,
                text: cell.text.clone(),
                spoken_text: if cell.spoken_text.is_empty() {
                    cell.text.clone()
                } else {
                    cell.spoken_text.clone()
                },
                symbol_id: cell.symbol_id.clone(),
                concept_id: cell.concept_id.clone(),
                svg: cell.svg.clone(),
                image_asset: cell.image_asset.clone(),
                action: cell.action.clone(),
                hidden: cell.hidden,
                missing_symbol: cell.missing_symbol,
                x_mm: round2(x),
                y_mm: round2(y),
                w_mm: round2(width),
                h_mm: round2(height),
                style: cell.style.clone(),
            }
        })
        .collect()
}

// -------------------------------------------------------------------- comandi

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum DocumentCommand {
    AddPage {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        layout: Option<String>,
        #[serde(default)]
        after: Option<String>,
    },
    RemovePage {
        id: String,
    },
    MovePage {
        id: String,
        index: usize,
    },
    UpdatePage {
        id: String,
        patch: Value,
    },
    UpdateDocument {
        patch: Value,
    },
    AddElement {
        page: String,
        element: Value,
        #[serde(default)]
        index: Option<usize>,
    },
    UpdateElement {
        page: String,
        id: String,
        patch: Value,
    },
    RemoveElement {
        page: String,
        id: String,
    },
    SetPosition {
        page: String,
        id: String,
        x_mm: f32,
        y_mm: f32,
    },
    SetSize {
        page: String,
        id: String,
        w_mm: f32,
        h_mm: f32,
    },
    SetRotation {
        page: String,
        id: String,
        rotation: f32,
    },
    SetStyle {
        page: String,
        id: String,
        style: Value,
    },
    SetSymbol {
        page: String,
        id: String,
        symbol_id: String,
        #[serde(default)]
        concept_id: Option<String>,
        #[serde(default)]
        svg: Option<String>,
        /// PNG come data URL, alternativa raster a `svg`.
        #[serde(default)]
        image_asset: Option<String>,
        #[serde(default)]
        label: Option<String>,
    },
    SetText {
        page: String,
        id: String,
        text: String,
        #[serde(default)]
        spoken_text: Option<String>,
    },
    SetAction {
        page: String,
        id: String,
        action: Option<UniversalAction>,
    },
    CreateLink {
        page: String,
        id: String,
        target_page: String,
    },
    GroupElements {
        page: String,
        ids: Vec<String>,
        group_id: String,
    },
    UngroupElements {
        page: String,
        group_id: String,
    },
    CreateGrid {
        page: String,
        id: String,
        columns: u32,
        rows: u32,
        #[serde(default)]
        gap_mm: Option<f32>,
        #[serde(default)]
        x_mm: Option<f32>,
        #[serde(default)]
        y_mm: Option<f32>,
        #[serde(default)]
        w_mm: Option<f32>,
        #[serde(default)]
        h_mm: Option<f32>,
    },
    UpdateGrid {
        page: String,
        id: String,
        #[serde(default)]
        columns: Option<u32>,
        #[serde(default)]
        rows: Option<u32>,
        #[serde(default)]
        gap_mm: Option<f32>,
        #[serde(default)]
        cells: Option<Vec<GridCell>>,
    },
    SetGridSizes {
        page: String,
        id: String,
        #[serde(default)]
        column_widths: Option<Vec<f32>>,
        #[serde(default)]
        row_heights: Option<Vec<f32>>,
    },
    MergeGridCell {
        page: String,
        id: String,
        cell_id: String,
        /// `right` | `down`
        direction: String,
    },
    SplitGridCell {
        page: String,
        id: String,
        cell_id: String,
    },
    SetPageGrid {
        page: String,
        columns: u32,
        rows: u32,
        #[serde(default)]
        gap_mm: Option<f32>,
        #[serde(default)]
        column_widths: Option<Vec<f32>>,
        #[serde(default)]
        row_heights: Option<Vec<f32>>,
        #[serde(default)]
        content_width_mm: Option<f32>,
        #[serde(default)]
        content_height_mm: Option<f32>,
        #[serde(default)]
        align: Option<String>,
        #[serde(default)]
        valign: Option<String>,
        #[serde(default)]
        origin_x_mm: Option<f32>,
        #[serde(default)]
        origin_y_mm: Option<f32>,
    },
    ConvertLayout {
        page: String,
        to: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyResult {
    pub document: UniversalDocument,
    pub rendered: RenderedDocument,
    pub changed: usize,
}

pub fn apply_transaction(
    document: &UniversalDocument,
    commands: &[DocumentCommand],
) -> Result<ApplyResult, String> {
    let mut next = document.clone();
    for (index, command) in commands.iter().enumerate() {
        apply_command(&mut next, command)
            .map_err(|error| format!("operazione {}: {error}", index + 1))?;
    }
    next.updated_at = crate::util::now_iso();
    let rendered = next.render()?;
    Ok(ApplyResult {
        document: next,
        rendered,
        changed: commands.len(),
    })
}

pub fn apply_command(document: &mut UniversalDocument, command: &DocumentCommand) -> Result<(), String> {
    match command {
        DocumentCommand::AddPage {
            id,
            title,
            layout,
            after,
        } => {
            let id = id.clone().unwrap_or_else(|| free_id(document, "page"));
            if document.page(&id).is_some() {
                return Err(format!("pagina già esistente: {id}"));
            }
            let layout = layout.clone().unwrap_or_else(|| "flow".to_string());
            if !matches!(layout.as_str(), "flow" | "absolute" | "grid") {
                return Err(format!("layout di pagina sconosciuto: {layout}"));
            }
            let page = UniversalPage {
                id,
                title: title
                    .clone()
                    .unwrap_or_else(|| format!("Pagina {}", document.pages.len() + 1)),
                layout,
                background: String::new(),
                flow: FlowSettings::default(),
                grid: GridSettings::default(),
                elements: Vec::new(),
                is_home: false,
                notes: String::new(),
            };
            match after {
                Some(after_id) => {
                    let position = document
                        .pages
                        .iter()
                        .position(|item| &item.id == after_id)
                        .ok_or_else(|| format!("pagina di riferimento inesistente: {after_id}"))?;
                    document.pages.insert(position + 1, page);
                }
                None => document.pages.push(page),
            }
        }
        DocumentCommand::RemovePage { id } => {
            if document.pages.len() == 1 {
                return Err("non si può eliminare l'unica pagina".to_string());
            }
            let position = document
                .pages
                .iter()
                .position(|page| &page.id == id)
                .ok_or_else(|| format!("pagina inesistente: {id}"))?;
            document.pages.remove(position);
            // I collegamenti orfani vengono rimossi esplicitamente, non
            // lasciati a puntare a una pagina che non esiste più.
            for page in &mut document.pages {
                for element in &mut page.elements {
                    if element.link_page.as_deref() == Some(id) {
                        element.link_page = None;
                    }
                    if let Some(action) = &mut element.action {
                        if action.target_page.as_deref() == Some(id) {
                            action.action_type = "append".to_string();
                            action.target_page = None;
                        }
                    }
                    if let Some(grid) = &mut element.grid {
                        for cell in &mut grid.cells {
                            if let Some(action) = &mut cell.action {
                                if action.target_page.as_deref() == Some(id) {
                                    action.action_type = "append".to_string();
                                    action.target_page = None;
                                }
                            }
                        }
                    }
                }
            }
        }
        DocumentCommand::MovePage { id, index } => {
            let position = document
                .pages
                .iter()
                .position(|page| &page.id == id)
                .ok_or_else(|| format!("pagina inesistente: {id}"))?;
            if *index >= document.pages.len() {
                return Err(format!("indice pagina fuori intervallo: {index}"));
            }
            let page = document.pages.remove(position);
            document.pages.insert(*index, page);
        }
        DocumentCommand::UpdateDocument { patch } => {
            let current_id = document.id.clone();
            let merged = merge_value(
                serde_json::to_value(&*document).map_err(|e| e.to_string())?,
                patch,
            )?;
            *document = serde_json::from_value(merged)
                .map_err(|e| format!("documento non valido: {e}"))?;
            // L'identificativo non cambia con una modifica di contenuto.
            document.id = current_id;
        }
        DocumentCommand::UpdatePage { id, patch } => {
            let page = document
                .page_mut(id)
                .ok_or_else(|| format!("pagina inesistente: {id}"))?;
            let merged = merge_value(
                serde_json::to_value(&*page).map_err(|e| e.to_string())?,
                patch,
            )?;
            *page = serde_json::from_value(merged).map_err(|e| format!("pagina non valida: {e}"))?;
        }
        DocumentCommand::AddElement {
            page,
            element,
            index,
        } => {
            let target = document
                .page_mut(page)
                .ok_or_else(|| format!("pagina inesistente: {page}"))?;
            let mut element: UniversalElement =
                serde_json::from_value(element.clone()).map_err(|e| format!("elemento non valido: {e}"))?;
            if element.id.trim().is_empty() {
                element.id = free_id_for_page(target, "el");
            }
            if target.elements.iter().any(|item| item.id == element.id) {
                return Err(format!("elemento già esistente: {}", element.id));
            }
            match index {
                Some(index) if *index <= target.elements.len() => {
                    target.elements.insert(*index, element)
                }
                _ => target.elements.push(element),
            }
        }
        DocumentCommand::UpdateElement { page, id, patch } => {
            let target = document
                .page_mut(page)
                .ok_or_else(|| format!("pagina inesistente: {page}"))?;
            let element = target
                .elements
                .iter_mut()
                .find(|element| &element.id == id)
                .ok_or_else(|| format!("elemento inesistente: {id}"))?;
            let merged = merge_value(
                serde_json::to_value(&*element).map_err(|e| e.to_string())?,
                patch,
            )?;
            *element = serde_json::from_value(merged).map_err(|e| format!("elemento non valido: {e}"))?;
            element.id = id.clone();
        }
        DocumentCommand::RemoveElement { page, id } => {
            let target = document
                .page_mut(page)
                .ok_or_else(|| format!("pagina inesistente: {page}"))?;
            let before = target.elements.len();
            target.elements.retain(|element| &element.id != id);
            if target.elements.len() == before {
                return Err(format!("elemento inesistente: {id}"));
            }
            for element in &mut target.elements {
                element.children.retain(|child| child != id);
            }
        }
        DocumentCommand::SetPosition { page, id, x_mm, y_mm } => {
            if !x_mm.is_finite() || !y_mm.is_finite() {
                return Err("posizione non numerica".to_string());
            }
            let element = element_mut(document, page, id)?;
            element.x_mm = *x_mm;
            element.y_mm = *y_mm;
        }
        DocumentCommand::SetSize { page, id, w_mm, h_mm } => {
            if !w_mm.is_finite() || !h_mm.is_finite() || *w_mm < 0.0 || *h_mm < 0.0 {
                return Err("dimensioni non valide".to_string());
            }
            let element = element_mut(document, page, id)?;
            element.w_mm = *w_mm;
            element.h_mm = *h_mm;
        }
        DocumentCommand::SetRotation { page, id, rotation } => {
            if !(-360.0..=360.0).contains(rotation) {
                return Err("rotazione fuori intervallo".to_string());
            }
            element_mut(document, page, id)?.rotation = *rotation;
        }
        DocumentCommand::SetStyle { page, id, style } => {
            let element = element_mut(document, page, id)?;
            let merged = merge_value(serde_json::to_value(&element.style).map_err(|e| e.to_string())?, style)?;
            element.style = serde_json::from_value(merged).map_err(|e| format!("stile non valido: {e}"))?;
        }
        DocumentCommand::SetSymbol {
            page,
            id,
            symbol_id,
            concept_id,
            svg,
            image_asset,
            label,
        } => {
            let element = element_mut(document, page, id)?;
            element.kind = "symbol".to_string();
            element.symbol_id = Some(symbol_id.clone());
            element.concept_id = concept_id.clone();
            element.svg = svg.clone();
            element.image_asset = image_asset.clone();
            if let Some(label) = label {
                // La sostituzione del pittogramma non deve cancellare il testo
                // scritto dall'utente; viene usata solo per un elemento vuoto.
                if element.text.trim().is_empty() {
                    element.text = label.clone();
                }
                if element.spoken_text.trim().is_empty() {
                    element.spoken_text = label.clone();
                }
            }
        }
        DocumentCommand::SetText {
            page,
            id,
            text,
            spoken_text,
        } => {
            let element = element_mut(document, page, id)?;
            element.text = text.clone();
            if let Some(spoken) = spoken_text {
                element.spoken_text = spoken.clone();
            } else if element.spoken_text.is_empty() {
                element.spoken_text = text.clone();
            }
        }
        DocumentCommand::SetAction { page, id, action } => {
            element_mut(document, page, id)?.action = action.clone();
        }
        DocumentCommand::CreateLink {
            page,
            id,
            target_page,
        } => {
            let element = element_mut(document, page, id)?;
            element.link_page = Some(target_page.clone());
            element.action = Some(UniversalAction {
                action_type: "navigate".to_string(),
                target_page: Some(target_page.clone()),
                target_document: None,
                url: None,
            });
        }
        DocumentCommand::GroupElements {
            page,
            ids,
            group_id,
        } => {
            if ids.len() < 2 {
                return Err("un gruppo richiede almeno due elementi".to_string());
            }
            let target = document
                .page_mut(page)
                .ok_or_else(|| format!("pagina inesistente: {page}"))?;
            for id in ids {
                let element = target
                    .elements
                    .iter_mut()
                    .find(|element| &element.id == id)
                    .ok_or_else(|| format!("elemento inesistente: {id}"))?;
                element.group_id = Some(group_id.clone());
            }
        }
        DocumentCommand::UngroupElements { page, group_id } => {
            let target = document
                .page_mut(page)
                .ok_or_else(|| format!("pagina inesistente: {page}"))?;
            for element in &mut target.elements {
                if element.group_id.as_deref() == Some(group_id) {
                    element.group_id = None;
                }
            }
        }
        DocumentCommand::CreateGrid {
            page,
            id,
            columns,
            rows,
            gap_mm,
            x_mm,
            y_mm,
            w_mm,
            h_mm,
        } => {
            if *columns == 0 || *rows == 0 || *columns > 20 || *rows > 40 {
                return Err("dimensioni della griglia fuori intervallo".to_string());
            }
            let (width, height) = document.page.dimensions()?;
            let margin = document.page.margin_mm;
            let w = w_mm.unwrap_or((width - 2.0 * margin).max(20.0));
            let h = h_mm.unwrap_or((height - 2.0 * margin).max(20.0));
            let target = document
                .page_mut(page)
                .ok_or_else(|| format!("pagina inesistente: {page}"))?;
            if target.elements.iter().any(|element| &element.id == id) {
                return Err(format!("elemento già esistente: {id}"));
            }
            target.elements.push(UniversalElement {
                id: id.clone(),
                kind: "grid".to_string(),
                x_mm: x_mm.unwrap_or(margin),
                y_mm: y_mm.unwrap_or(margin),
                w_mm: w,
                h_mm: h,
                grid: Some(GridData {
                    columns: *columns,
                    rows: *rows,
                    gap_mm: gap_mm.unwrap_or(3.0),
                    column_widths: Vec::new(),
                    row_heights: Vec::new(),
                    cells: (0..(*columns * *rows))
                        .map(|index| GridCell {
                            id: format!("{id}-c{}", index + 1),
                            row: (index / *columns) as u32,
                            column: (index % *columns) as u32,
                            row_span: 1,
                            column_span: 1,
                            ..GridCell::default()
                        })
                        .collect(),
                }),
                ..UniversalElement::default()
            });
        }
        DocumentCommand::UpdateGrid {
            page,
            id,
            columns,
            rows,
            gap_mm,
            cells,
        } => {
            let element = element_mut(document, page, id)?;
            let grid = element
                .grid
                .as_mut()
                .ok_or_else(|| format!("{id} non è una griglia"))?;
            if let Some(columns) = columns {
                if *columns == 0 || *columns > 20 {
                    return Err("colonne della griglia fuori intervallo".to_string());
                }
                grid.columns = *columns;
            }
            if let Some(rows) = rows {
                if *rows == 0 || *rows > 40 {
                    return Err("righe della griglia fuori intervallo".to_string());
                }
                grid.rows = *rows;
            }
            if let Some(gap) = gap_mm {
                grid.gap_mm = *gap;
            }
            let capacity = (grid.columns * grid.rows) as usize;
            if let Some(cells) = cells {
                if cells.len() > capacity {
                    return Err("più celle dello spazio disponibile".to_string());
                }
                grid.cells = cells.clone();
            }
            // Ridimensionare la griglia non deve lasciare celle fantasma né
            // celle senza ID: le nuove celle sono reali e modificabili.
            let mut used: std::collections::HashSet<String> =
                grid.cells.iter().map(|cell| cell.id.clone()).collect();
            while grid.cells.len() < capacity {
                let mut index = grid.cells.len() + 1;
                let id = loop {
                    let candidate = format!("{id}-c{index}");
                    if used.insert(candidate.clone()) {
                        break candidate;
                    }
                    index += 1;
                };
                let position = grid.cells.len();
                grid.cells.push(GridCell {
                    id,
                    row: (position / grid.columns.max(1) as usize) as u32,
                    column: (position % grid.columns.max(1) as usize) as u32,
                    row_span: 1,
                    column_span: 1,
                    ..GridCell::default()
                });
            }
            if grid.cells.len() > capacity {
                grid.cells.truncate(capacity);
            }
            for index in 0..grid.cells.len() {
                if grid.cells[index].id.trim().is_empty() {
                    grid.cells[index].id = format!("{id}-c{}", index + 1);
                }
            }
            for cell in &mut grid.cells {
                cell.column = cell.column.min(grid.columns.saturating_sub(1));
                cell.row = cell.row.min(grid.rows.saturating_sub(1));
                cell.column_span = cell.column_span.max(1).min(grid.columns - cell.column);
                cell.row_span = cell.row_span.max(1).min(grid.rows - cell.row);
            }
        }
        DocumentCommand::SetGridSizes {
            page,
            id,
            column_widths,
            row_heights,
        } => {
            let element = element_mut(document, page, id)?;
            let grid = element
                .grid
                .as_mut()
                .ok_or_else(|| format!("{id} non è una griglia"))?;
            if let Some(widths) = column_widths {
                if widths.len() != grid.columns as usize
                    || widths.iter().any(|value| !value.is_finite() || *value <= 0.0)
                {
                    return Err("larghezze delle colonne non valide".to_string());
                }
                grid.column_widths = widths.clone();
            }
            if let Some(heights) = row_heights {
                if heights.len() != grid.rows as usize
                    || heights.iter().any(|value| !value.is_finite() || *value <= 0.0)
                {
                    return Err("altezze delle righe non valide".to_string());
                }
                grid.row_heights = heights.clone();
            }
        }
        DocumentCommand::MergeGridCell {
            page,
            id,
            cell_id,
            direction,
        } => {
            let element = element_mut(document, page, id)?;
            let grid = element
                .grid
                .as_mut()
                .ok_or_else(|| format!("{id} non è una griglia"))?;
            normalize_grid_positions(grid);
            let position = grid
                .cells
                .iter()
                .position(|cell| &cell.id == cell_id)
                .ok_or_else(|| format!("cella inesistente: {cell_id}"))?;
            let (row, column, row_span, column_span) = {
                let cell = &grid.cells[position];
                (cell.row, cell.column, cell.row_span.max(1), cell.column_span.max(1))
            };
            let (target_row, target_column) = match direction.as_str() {
                "right" => (row, column + column_span),
                "down" => (row + row_span, column),
                other => return Err(format!("direzione non supportata: {other}")),
            };
            if target_column >= grid.columns || target_row >= grid.rows {
                return Err("la cella non ha una vicina in quella direzione".to_string());
            }
            let Some(target) = grid
                .cells
                .iter_mut()
                .find(|cell| cell.row == target_row && cell.column == target_column)
            else {
                return Err("cella vicina non trovata".to_string());
            };
            let target_id = target.id.clone();
            let target_row_span = target.row_span.max(1);
            let target_column_span = target.column_span.max(1);
            if direction == "right" {
                if target_row_span != row_span {
                    return Err("le celle devono avere la stessa altezza per unirsi".to_string());
                }
                grid.cells[position].column_span += target_column_span;
            } else {
                if target_column_span != column_span {
                    return Err("le celle devono avere la stessa larghezza per unirsi".to_string());
                }
                grid.cells[position].row_span += target_row_span;
            }
            grid.cells.retain(|cell| cell.id != target_id);
        }
        DocumentCommand::SplitGridCell {
            page,
            id,
            cell_id,
        } => {
            let element = element_mut(document, page, id)?;
            let grid = element
                .grid
                .as_mut()
                .ok_or_else(|| format!("{id} non è una griglia"))?;
            normalize_grid_positions(grid);
            let position = grid
                .cells
                .iter()
                .position(|cell| &cell.id == cell_id)
                .ok_or_else(|| format!("cella inesistente: {cell_id}"))?;
            let cell = &grid.cells[position];
            if cell.row_span <= 1 && cell.column_span <= 1 {
                return Err("la cella non è unita".to_string());
            }
            let (row, column, row_span, column_span, text, spoken, style) = (
                cell.row,
                cell.column,
                cell.row_span.max(1),
                cell.column_span.max(1),
                cell.text.clone(),
                cell.spoken_text.clone(),
                cell.style.clone(),
            );
            if column_span > 1 {
                grid.cells[position].column_span -= 1;
                grid.cells.push(GridCell {
                    id: format!("{cell_id}-split-{}", grid.cells.len() + 1),
                    row,
                    column: column + column_span - 1,
                    row_span,
                    column_span: 1,
                    text,
                    spoken_text: spoken,
                    style,
                    ..GridCell::default()
                });
            } else {
                grid.cells[position].row_span -= 1;
                grid.cells.push(GridCell {
                    id: format!("{cell_id}-split-{}", grid.cells.len() + 1),
                    row: row + row_span - 1,
                    column,
                    row_span: 1,
                    column_span,
                    text,
                    spoken_text: spoken,
                    style,
                    ..GridCell::default()
                });
            }
        }
        DocumentCommand::SetPageGrid {
            page,
            columns,
            rows,
            gap_mm,
            column_widths,
            row_heights,
            content_width_mm,
            content_height_mm,
            align,
            valign,
            origin_x_mm,
            origin_y_mm,
        } => {
            let target = document
                .page_mut(page)
                .ok_or_else(|| format!("pagina inesistente: {page}"))?;
            if *columns == 0 || *columns > 12 || *rows == 0 || *rows > 60 {
                return Err("griglia di pagina fuori intervallo".to_string());
            }
            target.layout = "grid".to_string();
            target.grid.columns = *columns;
            target.grid.rows = *rows;
            if let Some(gap) = gap_mm {
                target.grid.gap_mm = *gap;
            }
            if let Some(widths) = column_widths {
                if widths.len() != *columns as usize
                    || widths.iter().any(|value| !value.is_finite() || *value <= 0.0)
                {
                    return Err("pesi delle colonne non validi".to_string());
                }
                target.grid.column_widths = widths.clone();
            }
            if let Some(heights) = row_heights {
                if heights.len() != *rows as usize
                    || heights.iter().any(|value| !value.is_finite() || *value <= 0.0)
                {
                    return Err("pesi delle righe non validi".to_string());
                }
                target.grid.row_heights = heights.clone();
            }
            if let Some(width) = content_width_mm {
                if !width.is_finite() || *width <= 0.0 {
                    return Err("larghezza dell'area griglia non valida".to_string());
                }
                target.grid.content_width_mm = Some(*width);
            }
            if let Some(height) = content_height_mm {
                if !height.is_finite() || *height <= 0.0 {
                    return Err("altezza dell'area griglia non valida".to_string());
                }
                target.grid.content_height_mm = Some(*height);
            }
            if let Some(value) = align {
                target.grid.align = value.clone();
            }
            if let Some(value) = valign {
                target.grid.valign = value.clone();
            }
            if let Some(value) = origin_x_mm {
                if !value.is_finite() || *value < 0.0 {
                    return Err("origine orizzontale della griglia non valida".to_string());
                }
                target.grid.origin_x_mm = Some(*value);
            }
            if let Some(value) = origin_y_mm {
                if !value.is_finite() || *value < 0.0 {
                    return Err("origine verticale della griglia non valida".to_string());
                }
                target.grid.origin_y_mm = Some(*value);
            }
            for element in &mut target.elements {
                if element.absolute {
                    continue;
                }
                element.column = element.column.min(columns.saturating_sub(1));
                element.row = element.row.min(rows.saturating_sub(1));
                element.column_span = element.column_span.min(*columns - element.column);
                element.row_span = element.row_span.min(*rows - element.row);
            }
        }
        DocumentCommand::ConvertLayout { page, to } => {
            if !matches!(to.as_str(), "flow" | "absolute" | "grid") {
                return Err(format!("layout sconosciuto: {to}"));
            }
            let target = document
                .page_mut(page)
                .ok_or_else(|| format!("pagina inesistente: {page}"))?;
            if to == "grid" {
                let mut max_row = 0;
                let mut max_column = 0;
                for element in &target.elements {
                    max_row = max_row.max(element.row + element.row_span.max(1));
                    max_column = max_column.max(element.column + element.column_span.max(1));
                }
                target.grid.columns = target.grid.columns.max(max_column).max(1);
                target.grid.rows = target.grid.rows.max(max_row).max(1);
            }
            target.layout = to.clone();
        }
    }
    Ok(())
}

fn normalize_grid_positions(grid: &mut GridData) {
    let legacy = grid.cells.iter().all(|cell| cell.row == 0 && cell.column == 0)
        && grid.cells.len() > 1;
    if !legacy {
        return;
    }
    let columns = grid.columns.max(1);
    for (index, cell) in grid.cells.iter_mut().enumerate() {
        cell.row = (index as u32) / columns;
        cell.column = (index as u32) % columns;
        cell.row_span = 1;
        cell.column_span = 1;
    }
}

fn element_mut<'a>(
    document: &'a mut UniversalDocument,
    page: &str,
    id: &str,
) -> Result<&'a mut UniversalElement, String> {
    document
        .page_mut(page)
        .ok_or_else(|| format!("pagina inesistente: {page}"))?
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or_else(|| format!("elemento inesistente: {id}"))
}

fn merge_object(target: &mut Value, patch: &Value) -> Result<(), String> {
    let Some(target_object) = target.as_object_mut() else {
        return Err("destinazione non è un oggetto JSON".to_string());
    };
    let Some(patch_object) = patch.as_object() else {
        return Err("patch non è un oggetto JSON".to_string());
    };
    for (key, value) in patch_object {
        target_object.insert(key.clone(), value.clone());
    }
    Ok(())
}

fn merge_value(base: Value, patch: &Value) -> Result<Value, String> {
    let mut target = base;
    merge_object(&mut target, patch)?;
    Ok(target)
}

fn free_id(document: &UniversalDocument, prefix: &str) -> String {
    let mut index = document.pages.len() + 1;
    loop {
        let candidate = format!("{prefix}-{index}");
        if document.page(&candidate).is_none() {
            return candidate;
        }
        index += 1;
    }
}

fn free_id_for_page(page: &UniversalPage, prefix: &str) -> String {
    let mut index = page.elements.len() + 1;
    loop {
        let candidate = format!("{prefix}-{index}");
        if page.elements.iter().all(|element| element.id != candidate) {
            return candidate;
        }
        index += 1;
    }
}

// ------------------------------------------------------- migrazione e utilità

/// Converte una vecchia specifica di layout in documento universale. Non perde
/// contenuti: le pagine diventano pagine, gli elementi mantengono ID, simboli,
/// stato e azioni (la pagina di destinazione viene convertita in ID stabile).
pub fn from_layout(spec: &crate::layout::LayoutSpecification) -> UniversalDocument {
    let mut document = UniversalDocument::empty(
        &format!("legacy-{}", crate::pictogram_import::slugify(&spec.title)),
        &spec.title,
        "presentation",
    );
    document.page.size = spec.page.size.clone();
    document.page.orientation = spec.page.orientation.clone();
    document.page.margin_mm = spec.page.margin_mm;
    let mut pages: Vec<UniversalPage> = Vec::new();
    let max_page = spec
        .elements
        .iter()
        .map(|element| element.page as usize)
        .max()
        .unwrap_or(0);
    for index in 0..=max_page {
        pages.push(UniversalPage {
            id: if index == 0 {
                "page-1".to_string()
            } else {
                format!("page-{}", index + 1)
            },
            title: if index == 0 {
                spec.title.clone()
            } else {
                format!("Pagina {}", index + 1)
            },
            layout: "grid".to_string(),
            background: String::new(),
            flow: FlowSettings::default(),
            grid: GridSettings {
                columns: spec.layout.columns,
                rows: spec.layout.rows,
                gap_mm: spec.layout.gap_mm,
                ..Default::default()
            },
            elements: Vec::new(),
            is_home: index == 0,
            notes: String::new(),
        });
    }
    for element in &spec.elements {
        let page_id = pages
            .get(element.page as usize)
            .map(|page| page.id.clone())
            .unwrap_or_else(|| "page-1".to_string());
        let target_page = element.action.as_ref().and_then(|action| {
            action
                .target_page
                .and_then(|index| pages.get(index as usize).map(|page| page.id.clone()))
        });
        let universal = UniversalElement {
            id: element.id.clone(),
            kind: match element.kind.as_str() {
                "box" => "placeholder",
                "label" => "text",
                other => other,
            }
            .to_string(),
            text: element.label.clone(),
            spoken_text: element.spoken_text.clone().unwrap_or_else(|| element.label.clone()),
            concept_id: element.concept_id.clone(),
            symbol_id: element.symbol_id.clone(),
            lemma: None,
            svg: element.svg.clone(),
            image_asset: None,
            audio_asset: None,
            x_mm: 0.0,
            y_mm: 0.0,
            w_mm: 0.0,
            h_mm: 0.0,
            rotation: 0.0,
            z: 0,
            visible: !element.hidden,
            locked: element.locked,
            absolute: false,
            opacity: 1.0,
            row: element.row,
            column: element.column,
            row_span: element.row_span,
            column_span: element.column_span,
            style: ElementStyle::default(),
            action: element.action.as_ref().map(|action| UniversalAction {
                action_type: action.action_type.clone(),
                target_page: target_page.clone(),
                target_document: None,
                url: None,
            }),
            link_page: target_page,
            group_id: None,
            children: Vec::new(),
            grid: None,
            accessibility: Accessibility::default(),
            metadata: Value::Null,
        };
        if let Some(page) = pages.iter_mut().find(|page| page.id == page_id) {
            page.elements.push(universal);
        }
    }
    document.pages = pages;
    document
}

/// Utility per costruire documenti nelle pipeline CLI e nei test.
pub fn build_document(
    id: &str,
    title: &str,
    kind: &str,
    page_size: &str,
    orientation: &str,
) -> UniversalDocument {
    let mut document = UniversalDocument::empty(id, title, kind);
    document.page.size = page_size.to_string();
    document.page.orientation = orientation.to_string();
    document
}

pub fn add_flow_page(document: &mut UniversalDocument, id: &str, title: &str) -> Result<(), String> {
    apply_command(
        document,
        &DocumentCommand::AddPage {
            id: Some(id.to_string()),
            title: Some(title.to_string()),
            layout: Some("flow".to_string()),
            after: None,
        },
    )
}

pub fn add_grid_page(
    document: &mut UniversalDocument,
    id: &str,
    title: &str,
    columns: u32,
    rows: u32,
) -> Result<(), String> {
    apply_command(
        document,
        &DocumentCommand::AddPage {
            id: Some(id.to_string()),
            title: Some(title.to_string()),
            layout: Some("grid".to_string()),
            after: None,
        },
    )?;
    apply_command(
        document,
        &DocumentCommand::SetPageGrid {
            page: id.to_string(),
            columns,
            rows,
            gap_mm: Some(3.0),
            column_widths: None,
            row_heights: None,
            content_width_mm: None,
            content_height_mm: None,
            align: None,
            valign: None,
            origin_x_mm: None,
            origin_y_mm: None,
        },
    )
}

pub fn new_element(id: &str, kind: &str, text: &str) -> UniversalElement {
    UniversalElement {
        id: id.to_string(),
        kind: kind.to_string(),
        text: text.to_string(),
        spoken_text: text.to_string(),
        ..UniversalElement::default()
    }
}

pub fn grid_cell(id: &str, text: &str, symbol_id: Option<String>, svg: Option<String>) -> GridCell {
    GridCell {
        id: id.to_string(),
        text: text.to_string(),
        spoken_text: text.to_string(),
        symbol_id,
        svg,
        ..GridCell::default()
    }
}

// ------------------------------------------------------------------ export

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn remove_attribute(tag: &str, name: &str) -> String {
    let needle = format!("{name}=");
    let mut out = tag.to_string();
    let mut search = 0usize;
    while let Some(relative) = out[search..].find(&needle) {
        let start = search + relative;
        let before_ok = start == 0
            || out[..start]
                .chars()
                .next_back()
                .map(|c| c.is_whitespace() || c == '<')
                .unwrap_or(true);
        if !before_ok {
            search = start + needle.len();
            continue;
        }
        let value_start = start + needle.len();
        let quote = out[value_start..].chars().next().unwrap_or('"');
        if quote == '"' || quote == '\'' {
            if let Some(close_rel) = out[value_start + 1..].find(quote) {
                let end = value_start + 1 + close_rel + 1;
                out.replace_range(start..end, "");
                search = start;
                continue;
            }
        }
        search = value_start;
    }
    out
}

fn embed_symbol(svg: &str, x: f32, y: f32, w: f32, h: f32) -> String {
    let trimmed = svg.trim();
    // I pittogrammi raster arrivano già come data URL PNG: si inseriscono
    // direttamente come <image>, senza costruire un SVG di contenimento.
    if trimmed.starts_with("data:image/") {
        return format!(
            "<image x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" preserveAspectRatio=\"xMidYMid meet\" href=\"{}\" />",
            escape_xml(trimmed)
        );
    }
    if !trimmed.starts_with("<svg") && !trimmed.starts_with("<?xml") {
        return String::new();
    }
    // Inserisce l'SVG come SVG annidato: conserva viewBox e contenuto, senza
    // dipendere da un browser o da una risorsa esterna. Le dimensioni già
    // presenti nell'SVG originale vengono rimosse per evitare attributi
    // duplicati (errore reale di resvg con i pittogrammi ARASAAC).
    let start = trimmed.find("<svg").unwrap_or(0);
    let head = &trimmed[start..];
    let mut opening_end = head.find('>').unwrap_or(head.len());
    let mut opening = head[..opening_end].to_string();
    for attribute in ["x", "y", "width", "height", "preserveAspectRatio"] {
        opening = remove_attribute(&opening, attribute);
    }
    let rest = &head[opening_end..];
    opening_end = head.len();
    let _ = opening_end;
    format!(
        "{opening} x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" preserveAspectRatio=\"xMidYMid meet\"{rest}"
    )
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let limit = max_chars.max(1);
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let words: Vec<&str> = paragraph.split_whitespace().collect();
        if words.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in words {
            let word_len = word.chars().count();
            if current.is_empty() {
                if word_len <= limit {
                    current.push_str(word);
                } else {
                    // Una parola più lunga della riga viene spezzata, non persa.
                    let mut chunk = String::new();
                    for (index, ch) in word.chars().enumerate() {
                        chunk.push(ch);
                        if (index + 1) % limit == 0 {
                            lines.push(chunk.clone());
                            chunk.clear();
                        }
                    }
                    if !chunk.is_empty() {
                        current = chunk;
                    }
                }
            } else if current.chars().count() + 1 + word_len <= limit {
                current.push(' ');
                current.push_str(word);
            } else {
                lines.push(current.clone());
                current.clear();
                if word_len <= limit {
                    current.push_str(word);
                } else {
                    let mut chunk = String::new();
                    for (index, ch) in word.chars().enumerate() {
                        chunk.push(ch);
                        if (index + 1) % limit == 0 {
                            lines.push(chunk.clone());
                            chunk.clear();
                        }
                    }
                    current = chunk;
                }
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn text_attributes(style: &ElementStyle, font: f32) -> String {
    let family = if style.font_family.trim().is_empty() {
        String::new()
    } else {
        format!(" font-family=\"{}\"", escape_xml(&style.font_family))
    };
    let weight = if style.bold { " font-weight=\"700\"" } else { "" };
    let italic = if style.italic { " font-style=\"italic\"" } else { "" };
    let underline = if style.underline {
        " text-decoration=\"underline\""
    } else {
        ""
    };
    format!(
        " font-size=\"{}\" fill=\"{}\"{family}{weight}{italic}{underline}",
        font,
        escape_xml(if style.color.is_empty() { "#111111" } else { &style.color })
    )
}

fn render_text_lines(
    out: &mut String,
    lines: &[String],
    x: f32,
    y: f32,
    width: f32,
    align: &str,
    font: f32,
    style: &ElementStyle,
) {
    let (anchor, text_x) = match align {
        "center" => ("middle", x + width / 2.0),
        "right" => ("end", x + width),
        _ => ("start", x),
    };
    let attrs = text_attributes(style, font);
    let line_height = font * 1.28;
    for (index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"{}\"{}>{}</text>",
            text_x,
            y + index as f32 * line_height,
            anchor,
            attrs,
            escape_xml(line)
        ));
    }
}

/// Disegna i componenti didattici che non sono semplici caselle di testo.
/// La geometria resta nel documento: stampa, PDF e anteprima usano questa
/// stessa funzione.
fn render_special_kind(out: &mut String, element: &RenderedElement, w: f32, h: f32) {
    let ink = if element.style.color.is_empty() {
        "#1f2937"
    } else {
        &element.style.color
    };
    let stroke = if element.style.border_color.is_empty() {
        "#94a3b8"
    } else {
        &element.style.border_color
    };
    let font = element.style.font_size.max(2.0);
    match element.kind.as_str() {
        "answer_line" | "handwriting_line" => {
            let lines = element
                .metadata
                .get("lines")
                .and_then(|value| value.as_u64())
                .unwrap_or(1)
                .clamp(1, 12) as usize;
            let handwriting = element.kind == "handwriting_line";
            let label_h = if element.text.is_empty() { 0.0 } else { font * 1.4 };
            if !element.text.is_empty() {
                render_text_lines(
                    out,
                    &[element.text.clone()],
                    0.0,
                    font,
                    w,
                    &element.style.align,
                    font,
                    &element.style,
                );
            }
            let available = (h - label_h).max(4.0);
            let step = available / lines as f32;
            for index in 0..lines {
                let y = label_h + step * (index as f32 + 1.0) - 1.2;
                out.push_str(&format!(
                    "<line x1=\"0\" y1=\"{y}\" x2=\"{w}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"0.35\"/>",
                    escape_xml(stroke)
                ));
                if handwriting {
                    out.push_str(&format!(
                        "<line x1=\"0\" y1=\"{}\" x2=\"{w}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.2\" stroke-dasharray=\"1.5 1.5\"/>",
                        y - step / 2.0,
                        y - step / 2.0,
                        escape_xml(stroke)
                    ));
                }
            }
        }
        "checkbox" => {
            let size = (h - 2.0).min(5.2).max(3.0);
            let y = (h - size) / 2.0;
            out.push_str(&format!(
                "<rect x=\"0\" y=\"{y}\" width=\"{size}\" height=\"{size}\" rx=\"0.7\" fill=\"#ffffff\" stroke=\"{}\" stroke-width=\"0.45\"/>",
                escape_xml(stroke)
            ));
            if !element.text.is_empty() {
                let max_chars = ((w - size - 2.0) / (font * 0.55)).max(3.0) as usize;
                let lines = wrap_text(&element.text, max_chars);
                let start_y = (h - lines.len() as f32 * font * 1.28) / 2.0 + font;
                render_text_lines(
                    out,
                    &lines,
                    (size + 1.8).min(w),
                    start_y,
                    (w - size - 2.0).max(2.0),
                    &element.style.align,
                    font,
                    &element.style,
                );
            }
        }
        "token_slot" => {
            let diameter = (w.min(h) - 2.0).max(3.0);
            let cx = w / 2.0;
            let cy = h / 2.0;
            let filled = element
                .metadata
                .get("filled")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            out.push_str(&format!(
                "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0.6\"{} />",
                diameter / 2.0,
                if filled { "#fbbf24" } else { "#ffffff" },
                escape_xml(stroke),
                if filled { "" } else { " stroke-dasharray=\"1.8 1.4\"" }
            ));
            if !element.text.is_empty() {
                render_text_lines(
                    out,
                    &[element.text.clone()],
                    0.0,
                    cy + font / 3.0,
                    w,
                    "center",
                    font,
                    &element.style,
                );
            }
        }
        "number_line" => {
            let min = element.metadata.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let max = element.metadata.get("max").and_then(|v| v.as_f64()).unwrap_or(10.0);
            let step = element
                .metadata
                .get("step")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0)
                .max(0.5);
            let count = (((max - min) / step).round() as i64 + 1).max(2) as usize;
            let y = h * 0.55;
            out.push_str(&format!(
                "<line x1=\"0\" y1=\"{y}\" x2=\"{w}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"0.45\"/>",
                escape_xml(stroke)
            ));
            let font_small = (font * 0.7).max(2.0);
            for index in 0..count {
                let x = if count == 1 { 0.0 } else { w * index as f32 / (count - 1) as f32 };
                out.push_str(&format!(
                    "<line x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.4\"/>",
                    y - 1.4,
                    y + 1.4,
                    escape_xml(stroke)
                ));
                let value = min + step * index as f64;
                out.push_str(&format!(
                    "<text x=\"{x}\" y=\"{}\" font-size=\"{font_small}\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    y + font_small + 1.0,
                    escape_xml(ink),
                    escape_xml(&format!("{}", value as i64))
                ));
            }
        }
        "ten_frame" => {
            let columns = 5usize;
            let rows = 2usize;
            let filled = element
                .metadata
                .get("filled")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                .min(10) as usize;
            let cell_w = w / columns as f32;
            let cell_h = h / rows as f32;
            for index in 0..columns * rows {
                let col = index % columns;
                let row = index / columns;
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{cell_w}\" height=\"{cell_h}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0.35\"/>",
                    col as f32 * cell_w,
                    row as f32 * cell_h,
                    if index < filled { "#a7f3d0" } else { "#ffffff" },
                    escape_xml(stroke)
                ));
            }
        }
        "timeline" => {
            let steps = element
                .metadata
                .get("steps")
                .and_then(|value| value.as_u64())
                .unwrap_or(4)
                .clamp(2, 8) as usize;
            let y = h * 0.4;
            out.push_str(&format!(
                "<line x1=\"0\" y1=\"{y}\" x2=\"{w}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                escape_xml(stroke)
            ));
            for index in 0..steps {
                let x = if steps == 1 { 0.0 } else { w * index as f32 / (steps - 1) as f32 };
                out.push_str(&format!(
                    "<circle cx=\"{x}\" cy=\"{y}\" r=\"1.8\" fill=\"#ffffff\" stroke=\"{}\" stroke-width=\"0.45\"/>",
                    escape_xml(stroke)
                ));
            }
        }
        _ => {}
    }
}

/// Tratteggio del bordo: serve alle linee di taglio dei materiali velcro.
fn border_dash(style: &ElementStyle) -> &'static str {
    match style.border_style.as_str() {
        "dashed" => " stroke-dasharray=\"2 1.5\"",
        "dotted" => " stroke-dasharray=\"0.6 1.2\" stroke-linecap=\"round\"",
        _ => "",
    }
}

fn render_element_svg(element: &RenderedElement) -> String {
    let mut out = String::new();
    let background = if element.style.background.is_empty() {
        "none".to_string()
    } else {
        element.style.background.clone()
    };
    let border = if element.style.border_width > 0.0 && !element.style.border_color.is_empty() {
        format!(
            " stroke=\"{}\" stroke-width=\"{}\"{}",
            escape_xml(&element.style.border_color),
            element.style.border_width,
            border_dash(&element.style)
        )
    } else {
        String::new()
    };
    out.push_str(&format!(
        "<g transform=\"translate({} {}) rotate({} {} {})\" opacity=\"{}\">",
        element.x_mm, element.y_mm, element.rotation, element.w_mm / 2.0, element.h_mm / 2.0, element.opacity
    ));
    if background != "none" || !border.is_empty() {
        out.push_str(&format!(
            "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"{}/>",
            element.w_mm,
            element.h_mm,
            element.style.border_radius,
            escape_xml(&background),
            border
        ));
    }
    let padding = element.style.padding_mm.max(0.0);
    let inner_w = (element.w_mm - 2.0 * padding).max(1.0);
    let inner_h = (element.h_mm - 2.0 * padding).max(1.0);
    let font = element.style.font_size.max(2.0);
    let special = matches!(
        element.kind.as_str(),
        "answer_line" | "handwriting_line" | "checkbox" | "token_slot" | "number_line"
            | "ten_frame" | "timeline"
    );
    if special {
        out.push_str(&format!(
            "<g transform=\"translate({padding} {padding})\">"
        ));
        render_special_kind(&mut out, element, inner_w, inner_h);
        out.push_str("</g>");
    } else if element.kind == "speech" {
        // Fumetto: riquadro arrotondato, coda in basso a sinistra e testo
        // centrato. Serve per schede illustrate e strisce a vignette.
        let fill = if element.style.background.is_empty() {
            "#ffffff".to_string()
        } else {
            element.style.background.clone()
        };
        let stroke = if element.style.border_color.is_empty() {
            "#172033".to_string()
        } else {
            element.style.border_color.clone()
        };
        let stroke_width = element.style.border_width.max(0.35).min(1.5);
        let radius = element.style.border_radius.max(2.0).min(inner_h / 2.0);
        out.push_str(&format!(
            "<rect x=\"0\" y=\"0\" width=\"{inner_w}\" height=\"{inner_h}\" rx=\"{radius}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{stroke_width}\"{} />",
            escape_xml(&fill),
            escape_xml(&stroke),
            border_dash(&element.style)
        ));
        let tail_x = inner_w * 0.22;
        let tail_y = inner_h;
        out.push_str(&format!(
            "<path d=\"M {tail_x} {tail_y} L {} {} L {} {tail_y} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{stroke_width}\"/>",
            tail_x + inner_w * 0.06,
            tail_y + 4.0,
            tail_x + inner_w * 0.14,
            escape_xml(&fill),
            escape_xml(&stroke)
        ));
        if element.cells.is_empty() {
            let font = element.style.font_size.max(2.5);
            let max_chars = (inner_w / (font * 0.55)).max(4.0) as usize;
            let lines = wrap_text(&element.text, max_chars);
            let line_height = font * 1.24;
            let text_h = lines.len() as f32 * line_height;
            let start_y = ((inner_h - 4.0 - text_h) / 2.0 + font).max(font);
            render_text_lines(
                &mut out,
                &lines,
                padding,
                start_y,
                inner_w,
                if element.style.align.is_empty() {
                    "center"
                } else {
                    &element.style.align
                },
                font,
                &element.style,
            );
        }
    } else if !element.cells.is_empty() {
        // Le celle della frase CAA (pittogramma + parola) sono disegnate dopo:
        // non sovrapporre il testo o il simbolo dell'elemento contenitore.
    } else if let Some(svg) = element
        .svg
        .as_deref()
        .or(element.image_asset.as_deref())
        .filter(|value| !value.trim().is_empty())
    {
        let caption_h = if element.text.is_empty() {
            0.0
        } else {
            let max_chars = (inner_w / (font * 0.55)).max(3.0) as usize;
            let lines = wrap_text(&element.text, max_chars);
            (lines.len() as f32 * font * 1.26 + 1.0).min(inner_h * 0.55)
        };
        let symbol_h = (inner_h - caption_h).max(2.0);
        out.push_str(&embed_symbol(svg, padding, padding, inner_w, symbol_h));
        if !element.text.is_empty() {
            let max_chars = (inner_w / (font * 0.55)).max(3.0) as usize;
            let lines = wrap_text(&element.text, max_chars);
            let line_height = font * 1.26;
            let text_h = lines.len() as f32 * line_height;
            let start_y = (element.h_mm - padding - text_h + font * 0.92).max(font);
            render_text_lines(
                &mut out,
                &lines,
                padding,
                start_y,
                inner_w,
                if element.style.align.is_empty() {
                    "center"
                } else {
                    &element.style.align
                },
                font,
                &element.style,
            );
        }
    } else if !element.text.is_empty() {
        let max_chars = (inner_w / (font * 0.55)).max(3.0) as usize;
        let lines = wrap_text(&element.text, max_chars);
        let line_height = font * 1.28;
        let text_h = lines.len() as f32 * line_height;
        let start_y = match element.style.valign.as_str() {
            "bottom" => (element.h_mm - padding - text_h + font).max(font),
            "middle" => ((element.h_mm - text_h) / 2.0 + font).max(font),
            _ => padding + font,
        };
        render_text_lines(
            &mut out,
            &lines,
            padding,
            start_y,
            inner_w,
            &element.style.align,
            font,
            &element.style,
        );
    }
    // Celle di una griglia interna.
    if !element.cells.is_empty() {
        for cell in element.cells.iter() {
            if cell.hidden {
                continue;
            }
            let x = cell.x_mm;
            let y = cell.y_mm;
            out.push_str(&format!(
                "<g transform=\"translate({} {})\">",
                x, y
            ));
            let cell_border = if cell.style.border_width > 0.0 {
                format!(
                    " stroke=\"{}\" stroke-width=\"{}\"{}",
                    escape_xml(if cell.style.border_color.is_empty() {
                        "#94a3b8"
                    } else {
                        &cell.style.border_color
                    }),
                    cell.style.border_width,
                    border_dash(&cell.style)
                )
            } else {
                String::new()
            };
            if !cell.style.background.is_empty() || !cell_border.is_empty() {
                out.push_str(&format!(
                    "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"{}/>",
                    cell.w_mm,
                    cell.h_mm,
                    cell.style.border_radius,
                    escape_xml(if cell.style.background.is_empty() {
                        "#ffffff"
                    } else {
                        &cell.style.background
                    }),
                    cell_border
                ));
            }
            let cell_font = cell.style.font_size.max(2.4);
            let caption_h = if cell.text.is_empty() {
                0.0
            } else {
                let max_chars = ((cell.w_mm - 2.0) / (cell_font * 0.55)).max(2.0) as usize;
                let lines = wrap_text(&cell.text, max_chars);
                (lines.len() as f32 * cell_font * 1.22 + 0.8).min(cell.h_mm * 0.5)
            };
            if let Some(svg) = cell
                .svg
                .as_deref()
                .or(cell.image_asset.as_deref())
                .filter(|value| !value.trim().is_empty())
            {
                out.push_str(&embed_symbol(
                    svg,
                    1.0,
                    1.0,
                    (cell.w_mm - 2.0).max(2.0),
                    (cell.h_mm - caption_h - 1.0).max(2.0),
                ));
            }
            if !cell.text.is_empty() {
                let max_chars = ((cell.w_mm - 2.0) / (cell_font * 0.55)).max(2.0) as usize;
                let lines = wrap_text(&cell.text, max_chars);
                let line_height = cell_font * 1.22;
                let text_h = lines.len() as f32 * line_height;
                let start_y = (cell.h_mm - 0.8 - text_h + cell_font * 0.92).max(cell_font);
                render_text_lines(
                    &mut out,
                    &lines,
                    1.0,
                    start_y,
                    (cell.w_mm - 2.0).max(2.0),
                    "center",
                    cell_font,
                    &cell.style,
                );
            }
            out.push_str("</g>");
        }
    }
    out.push_str("</g>");
    out
}

pub fn render_page_svg(page: &RenderedPage) -> String {
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}mm\" height=\"{}mm\">",
        page.width_mm, page.height_mm, page.width_mm, page.height_mm
    );
    out.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        page.width_mm,
        page.height_mm,
        escape_xml(if page.background.is_empty() { "#ffffff" } else { &page.background })
    ));
    for element in &page.elements {
        if element.visible {
            out.push_str(&render_element_svg(element));
        }
    }
    out.push_str("</svg>");
    out
}

pub fn render_html(document: &RenderedDocument) -> String {
    let mut body = String::new();
    for (index, page) in document.pages.iter().enumerate() {
        body.push_str(&format!(
            "<section class=\"caa-page\" data-page=\"{}\" id=\"page-{index}\"{}>",
            escape_xml(&page.id),
            if index == 0 { "" } else { " hidden" }
        ));
        for element in &page.elements {
            if !element.visible {
                continue;
            }
            let action = element
                .action
                .as_ref()
                .map(|action| action.action_type.as_str())
                .unwrap_or("");
            let data = format!(
                " data-element=\"{}\" data-action=\"{}\" data-target=\"{}\" data-speech=\"{}\"",
                escape_xml(&element.id),
                escape_xml(action),
                escape_xml(element.action.as_ref().and_then(|a| a.target_page.as_deref()).unwrap_or("")),
                escape_xml(&element.spoken_text)
            );
            let style = format!(
                "left:{:.2}mm;top:{:.2}mm;width:{:.2}mm;height:{:.2}mm;transform:rotate({}deg);opacity:{};",
                element.x_mm,
                element.y_mm,
                element.w_mm,
                element.h_mm,
                element.rotation,
                element.opacity
            );
            if !element.cells.is_empty() {
                body.push_str(&format!("<div class=\"caa-grid\" style=\"{}\"{}>", style, data));
                for cell in &element.cells {
                    if cell.hidden {
                        continue;
                    }
                    let action = cell
                        .action
                        .as_ref()
                        .map(|action| action.action_type.as_str())
                        .unwrap_or("");
                    body.push_str(&format!(
                        "<button class=\"caa-cell\" data-element=\"{}\" data-action=\"{}\" data-target=\"{}\" data-speech=\"{}\">",
                        escape_xml(&cell.id),
                        escape_xml(action),
                        escape_xml(cell.action.as_ref().and_then(|a| a.target_page.as_deref()).unwrap_or("")),
                        escape_xml(&cell.spoken_text)
                    ));
                    if let Some(svg) = cell
                        .svg
                        .as_deref()
                        .or(cell.image_asset.as_deref())
                        .filter(|value| !value.trim().is_empty())
                    {
                        body.push_str(&embed_symbol(svg, 0.0, 0.0, 20.0, 20.0));
                    }
                    body.push_str(&format!("<span>{}</span></button>", escape_xml(&cell.text)));
                }
                body.push_str("</div>");
            } else if element.kind == "token_slot" {
                body.push_str(&format!(
                    "<button class=\"caa-el caa-token\" style=\"{}\"{}><span class=\"caa-token__circle\"></span>{}</button>",
                    style,
                    data,
                    if element.text.is_empty() {
                        String::new()
                    } else {
                        format!("<span>{}</span>", escape_xml(&element.text))
                    }
                ));
            } else {
                body.push_str(&format!("<div class=\"caa-el caa-{}\" style=\"{}\"{}>", escape_xml(&element.kind), style, data));
                if let Some(svg) = element
                    .svg
                    .as_deref()
                    .or(element.image_asset.as_deref())
                    .filter(|value| !value.trim().is_empty())
                {
                    body.push_str(&embed_symbol(svg, 0.0, 0.0, element.w_mm.max(4.0), (element.h_mm * 0.7).max(4.0)));
                }
                if !element.text.is_empty() {
                    body.push_str(&format!("<span>{}</span>", escape_xml(&element.text)));
                }
                body.push_str("</div>");
            }
        }
        body.push_str("</section>");
    }
    format!(
        "<!doctype html><html lang=\"it\"><meta charset=\"utf-8\"><title>{}</title>\
<style>body{{margin:0;background:#e9edf3;font-family:system-ui,sans-serif}}\
.caa-page{{position:relative;background:#fff;width:210mm;height:297mm;margin:8mm auto;box-shadow:0 2px 12px #0002;overflow:hidden}}\
.caa-el,.caa-grid{{position:absolute;box-sizing:border-box;border:1px solid #d0d7e2;border-radius:3mm;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:2mm;font-size:4mm;padding:2mm;background:#fff;color:#111}}\
.caa-grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(20mm,1fr));gap:2mm;border:0;background:transparent;padding:0}}\
.caa-cell{{display:flex;flex-direction:column;align-items:center;justify-content:center;gap:1mm;border:1px solid #9aa7bb;border-radius:3mm;background:#fff;font:inherit;color:#111;cursor:pointer;min-height:18mm}}\
.caa-cell svg,.caa-el svg{{width:14mm;height:14mm}}\
.caa-speech{{border-radius:6mm;background:#fff;border:0.6mm solid #172033;overflow:visible}}\
.caa-speech::after{{content:'';position:absolute;left:18%;bottom:-3mm;width:6mm;height:6mm;background:#fff;border-right:0.6mm solid #172033;border-bottom:0.6mm solid #172033;transform:rotate(45deg)}}\
.caa-token{{border:0;background:transparent;padding:0;display:flex;align-items:center;justify-content:center}}\
.caa-token__circle{{display:block;width:82%;max-width:34mm;aspect-ratio:1;border:2px dashed #0f766e;border-radius:50%;background:#fff}}\
.caa-token.is-on .caa-token__circle{{border-style:solid;background:#fbbf24}}\
.caa-nav{{position:fixed;top:4mm;left:4mm;z-index:5}}\
.caa-nav button{{font:inherit;padding:2mm 4mm;margin-right:2mm}}\
.caa-bar{{position:fixed;bottom:0;left:0;right:0;display:flex;gap:2mm;align-items:center;background:#fff;border-top:1px solid #ccd;padding:2mm;z-index:6}}\
.caa-bar #msg{{flex:1;min-height:8mm;border:1px dashed #99a;border-radius:2mm;padding:1mm 2mm}}\
.caa-bar button{{font:inherit;padding:2mm 4mm}}\
@media print{{.caa-nav,.caa-bar{{display:none}}.caa-page{{box-shadow:none;margin:0;page-break-after:always}}}}\
</style><body><nav class=\"caa-nav\"><button onclick=\"caaGo(0)\">Home</button><button onclick=\"caaBack()\">Indietro</button></nav>{}\
<div class=\"caa-bar\"><span id=\"msg\">Tocca i pittogrammi per comporre la frase.</span>\
<button onclick=\"caaSpeakMessage()\">Parla frase</button><button onclick=\"caaClear()\">Svuota</button></div>\
<script>const pages=[...document.querySelectorAll('.caa-page')];let current=0,stack=[],msg=[];\
function show(i){{if(i<0||i>=pages.length)return;pages[current].hidden=true;pages[i].hidden=false;current=i;}}\
function caaGo(i){{stack.push(current);show(i);}}\
function caaBack(){{const i=stack.pop();if(i!==undefined)show(i);}}\
function speak(t){{if(!t)return;if('speechSynthesis' in window){{speechSynthesis.cancel();speechSynthesis.speak(new SpeechSynthesisUtterance(t));}}}}\
function renderMsg(){{const el=document.getElementById('msg');if(!el)return;el.textContent=msg.length?msg.join(' '):'Tocca i pittogrammi per comporre la frase.';}}\
function caaAppend(t){{if(!t)return;msg.push(t);renderMsg();speak(t);}}\
function caaClear(){{msg=[];renderMsg();}}\
function caaSpeakMessage(){{if(msg.length)speak(msg.join(' ')+'.');}}\
document.querySelectorAll('[data-element]').forEach((el)=>{{el.addEventListener('click',()=>{{const action=el.dataset.action;const target=el.dataset.target;\
if(action==='navigate'&&target){{const p=pages.findIndex(p=>p.dataset.page===target);if(p>=0)caaGo(p);}}else if(action==='back')caaBack();else if(action==='home')caaGo(0);\
else if(action==='speak')speak(el.dataset.speech);else if(action==='toggle'){{el.classList.toggle('is-on');speak(el.dataset.speech);}}else if(action==='clear')caaClear();\
else if(action==='append')caaAppend(el.dataset.speech||el.textContent);}});}});</script></body></html>",
        escape_xml(&document.title),
        body
    )
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibraryEntry {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    /// `template` | `board`
    #[serde(default)]
    pub source_type: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub pages: usize,
    #[serde(default)]
    pub elements: usize,
    #[serde(default)]
    pub interactive: bool,
    /// Pittogrammi richiesti ma non trovati nel repertorio.
    #[serde(default)]
    pub missing_symbols: usize,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub preview: String,
    #[serde(default)]
    pub category_id: String,
    #[serde(default)]
    pub subcategory_id: String,
    #[serde(default)]
    pub subcategory: String,
    #[serde(default)]
    pub audience: String,
    #[serde(default)]
    pub objective: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub complexity: String,
    #[serde(default)]
    pub needs_symbols: bool,
    #[serde(default)]
    pub printable: bool,
    #[serde(default)]
    pub customizable: bool,
    #[serde(default)]
    pub featured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LibraryManifest {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub generated_at: String,
    #[serde(default)]
    pub entries: Vec<LibraryEntry>,
    #[serde(default)]
    pub report: Value,
}

impl LibraryManifest {
    pub fn counts(&self) -> (usize, usize) {
        let templates = self
            .entries
            .iter()
            .filter(|entry| entry.source_type == "template")
            .count();
        let boards = self
            .entries
            .iter()
            .filter(|entry| entry.source_type == "board")
            .count();
        (templates, boards)
    }
}

/// Legge un documento universale da un `.caadoc`. Accetta anche i documenti
/// legacy: se manca `universal`, la specifica `layout` viene migrata.
pub fn load_from_caadoc(path: &std::path::Path) -> Result<UniversalDocument, String> {
    let loaded = crate::export::caadoc::load(path)?;
    if let Some(value) = loaded.universal {
        let document: UniversalDocument =
            serde_json::from_value(value).map_err(|e| format!("documento universale non valido: {e}"))?;
        validate(&document)?;
        return Ok(document);
    }
    if let Some(value) = loaded.layout {
        let spec: crate::layout::LayoutSpecification =
            serde_json::from_value(value).map_err(|e| format!("layout legacy non valido: {e}"))?;
        let document = from_layout(&spec);
        validate(&document)?;
        return Ok(document);
    }
    Err("il file non contiene un documento universale né un layout migrabile".to_string())
}

/// Risolve un documento della biblioteca con protezione dal path traversal.
pub fn resolve_library_path(
    manifest: &LibraryManifest,
    id: &str,
    source_type: Option<&str>,
    resources_root: &std::path::Path,
) -> Result<PathBuf, String> {
    let entry = manifest
        .entries
        .iter()
        .find(|entry| {
            entry.id == id && source_type.map(|kind| entry.source_type == kind).unwrap_or(true)
        })
        .ok_or_else(|| format!("documento «{id}» non trovato nella biblioteca universale"))?;
    let raw = std::path::Path::new(&entry.path);
    let path = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        let relative = entry.path.strip_prefix("resources/").unwrap_or(&entry.path);
        resources_root.join(relative)
    };
    let library_root = resources_root.join("library");
    // canonicalize risolve `..` e i collegamenti simbolici: un manifest
    // manomesso non può uscire dalla biblioteca.
    let canonical = std::fs::canonicalize(&path)
        .map_err(|_| format!("documento non trovato: {}", path.display()))?;
    let root = std::fs::canonicalize(&library_root)
        .map_err(|_| format!("biblioteca non trovata: {}", library_root.display()))?;
    if !canonical.starts_with(&root) {
        return Err("percorso del documento fuori dalla biblioteca".to_string());
    }
    Ok(canonical)
}

pub fn load_library_manifest(resources: &std::path::Path) -> LibraryManifest {
    let path = resources.join("library/manifest.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

// -------------------------------------------------------------------- test

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> UniversalDocument {
        let mut document = build_document("doc-1", "Prova", "mixed", "A4", "portrait");
        document.pages.clear();
        add_grid_page(&mut document, "home", "Home", 3, 2).unwrap();
        add_flow_page(&mut document, "story", "Storia").unwrap();
        apply_command(
            &mut document,
            &DocumentCommand::CreateGrid {
                page: "home".to_string(),
                id: "grid-1".to_string(),
                columns: 3,
                rows: 2,
                gap_mm: Some(3.0),
                x_mm: None,
                y_mm: None,
                w_mm: None,
                h_mm: None,
            },
        )
        .unwrap();
        apply_command(
            &mut document,
            &DocumentCommand::UpdateGrid {
                page: "home".to_string(),
                id: "grid-1".to_string(),
                columns: None,
                rows: None,
                gap_mm: None,
                cells: Some(vec![
                    GridCell {
                        id: "c1".to_string(),
                        text: "Frutta".to_string(),
                        spoken_text: "Apri la frutta".to_string(),
                        action: Some(UniversalAction {
                            action_type: "navigate".to_string(),
                            target_page: Some("story".to_string()),
                            ..UniversalAction::default()
                        }),
                        ..GridCell::default()
                    },
                    GridCell {
                        id: "c2".to_string(),
                        text: "Mela".to_string(),
                        action: Some(UniversalAction {
                            action_type: "speak".to_string(),
                            ..UniversalAction::default()
                        }),
                        ..GridCell::default()
                    },
                ]),
            },
        )
        .unwrap();
        document
    }

    #[test]
    fn document_roundtrip_and_render() {
        let document = sample();
        validate(&document).unwrap();
        let rendered = document.render().unwrap();
        assert_eq!(rendered.pages.len(), 2);
        assert_eq!(rendered.pages[0].layout, "grid");
        // La griglia 3×2 ha sempre sei celle reali; le due compilate sono le prime.
        assert_eq!(rendered.pages[0].elements[0].cells.len(), 6);
        assert_eq!(rendered.pages[0].elements[0].cells[0].text, "Frutta");
        let encoded = serde_json::to_string(&document).unwrap();
        let decoded: UniversalDocument = serde_json::from_str(&encoded).unwrap();
        validate(&decoded).unwrap();
    }

    #[test]
    fn links_use_stable_page_ids_and_are_validated() {
        let mut document = sample();
        validate(&document).unwrap();
        document.pages[0].elements[0].grid.as_mut().unwrap().cells[0]
            .action
            .as_mut()
            .unwrap()
            .target_page = Some("missing".to_string());
        assert!(validate(&document).unwrap_err().contains("inesistente"));
    }

    #[test]
    fn image_elements_keep_embedded_svg() {
        let mut document = sample();
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 8 8\"><image href=\"data:image/png;base64,AAAA\"/></svg>";
        let mut element = new_element("img1", "image", "Immagine generata");
        element.svg = Some(svg.to_string());
        apply_command(
            &mut document,
            &DocumentCommand::AddElement {
                page: "story".to_string(),
                element: serde_json::to_value(&element).unwrap(),
                index: None,
            },
        )
        .unwrap();
        let stored = document.element("story", "img1").unwrap();
        assert_eq!(stored.kind, "image");
        assert_eq!(stored.svg.as_deref(), Some(svg));
    }

    #[test]
    fn speech_bubble_renders_caa_cells_without_duplicate_sentence() {
        // Una frase CAA dentro un fumetto: le celle mostrano pittogramma e
        // parola, mentre il testo del contenitore non viene disegnato una
        // seconda volta sopra le celle.
        let mut document = sample();
        let mut element = new_element("speech-cells", "speech", "Ciao come stai");
        element.x_mm = 10.0;
        element.y_mm = 10.0;
        element.w_mm = 90.0;
        element.h_mm = 36.0;
        element.visible = true;
        element.style.font_size = 3.2;
        element.grid = Some(GridData {
            columns: 3,
            rows: 1,
            gap_mm: 1.0,
            cells: vec![
                grid_cell("c1", "Ciao", Some("s1".into()), None),
                grid_cell("c2", "come", Some("s2".into()), None),
                grid_cell("c3", "stai", Some("s3".into()), None),
            ],
            ..GridData::default()
        });
        apply_command(
            &mut document,
            &DocumentCommand::AddElement {
                page: "story".to_string(),
                element: serde_json::to_value(&element).unwrap(),
                index: None,
            },
        )
        .unwrap();
        let rendered = document.render().unwrap();
        let page = rendered.pages.iter().find(|page| page.id == "story").unwrap();
        let svg = render_page_svg(page);
        assert!(svg.contains("Ciao"));
        assert!(svg.contains("come"));
        assert!(svg.contains("stai"));
        assert_eq!(svg.matches("Ciao come stai").count(), 0);
        assert!(svg.contains("<path"));
    }

    #[test]
    fn speech_bubble_renders_box_tail_and_text() {
        let mut document = sample();
        let mut element = new_element("speech-1", "speech", "Ciao, sono un fumetto");
        element.x_mm = 20.0;
        element.y_mm = 20.0;
        element.w_mm = 70.0;
        element.h_mm = 30.0;
        element.visible = true;
        element.style.font_size = 5.0;
        apply_command(
            &mut document,
            &DocumentCommand::AddElement {
                page: "story".to_string(),
                element: serde_json::to_value(&element).unwrap(),
                index: None,
            },
        )
        .unwrap();
        let rendered = document.render().unwrap();
        let page = rendered.pages.iter().find(|page| page.id == "story").unwrap();
        let svg = render_page_svg(page);
        assert!(svg.contains("Ciao, sono un fumetto"));
        assert!(svg.contains("<path"));
        assert!(svg.contains("rx="));
        let html = render_html(&rendered);
        assert!(html.contains("caa-speech"));
        assert!(html.contains("caa-speech::after"));
        let pdf = crate::export::universal_pdf::build_pdf(&document, 1.0).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn png_symbols_are_embedded_as_image_not_wrapped_svg() {
        let markup = embed_symbol("data:image/png;base64,AAAA", 1.0, 2.0, 3.0, 4.0);
        assert!(markup.starts_with("<image "));
        assert!(markup.contains("href=\"data:image/png;base64,AAAA\""));
        assert!(!markup.contains("<svg"));
    }

    #[test]
    fn commands_move_resize_and_group() {
        let mut document = sample();
        apply_command(
            &mut document,
            &DocumentCommand::AddElement {
                page: "story".to_string(),
                element: serde_json::to_value(new_element("t1", "text", "Ciao")).unwrap(),
                index: None,
            },
        )
        .unwrap();
        apply_command(
            &mut document,
            &DocumentCommand::SetPosition {
                page: "story".to_string(),
                id: "t1".to_string(),
                x_mm: 20.0,
                y_mm: 30.0,
            },
        )
        .unwrap();
        apply_command(
            &mut document,
            &DocumentCommand::SetSize {
                page: "story".to_string(),
                id: "t1".to_string(),
                w_mm: 40.0,
                h_mm: 20.0,
            },
        )
        .unwrap();
        assert_eq!(document.element("story", "t1").unwrap().x_mm, 20.0);
        apply_command(
            &mut document,
            &DocumentCommand::AddElement {
                page: "story".to_string(),
                element: serde_json::to_value(new_element("t2", "text", "Mondo")).unwrap(),
                index: None,
            },
        )
        .unwrap();
        apply_command(
            &mut document,
            &DocumentCommand::GroupElements {
                page: "story".to_string(),
                ids: vec!["t1".to_string(), "t2".to_string()],
                group_id: "g1".to_string(),
            },
        )
        .unwrap();
        assert_eq!(
            document.element("story", "t1").unwrap().group_id.as_deref(),
            Some("g1")
        );
    }

    #[test]
    fn grid_4_by_5_creates_twenty_real_cells() {
        let mut document = build_document("g", "Griglia", "mixed", "A4", "portrait");
        document.pages.clear();
        add_grid_page(&mut document, "home", "Home", 4, 5).unwrap();
        apply_command(
            &mut document,
            &DocumentCommand::CreateGrid {
                page: "home".to_string(),
                id: "grid".to_string(),
                columns: 4,
                rows: 5,
                gap_mm: Some(2.0),
                x_mm: Some(12.0),
                y_mm: Some(12.0),
                w_mm: Some(180.0),
                h_mm: Some(220.0),
            },
        )
        .unwrap();
        let grid = document.element("home", "grid").unwrap().grid.as_ref().unwrap();
        assert_eq!(grid.cells.len(), 20);
        assert!(grid.cells.iter().all(|cell| !cell.id.is_empty()));
        let ids: HashSet<&str> = grid.cells.iter().map(|cell| cell.id.as_str()).collect();
        assert_eq!(ids.len(), 20);

        apply_command(
            &mut document,
            &DocumentCommand::UpdateGrid {
                page: "home".to_string(),
                id: "grid".to_string(),
                columns: Some(5),
                rows: Some(4),
                gap_mm: None,
                cells: None,
            },
        )
        .unwrap();
        let grid = document.element("home", "grid").unwrap().grid.as_ref().unwrap();
        assert_eq!(grid.cells.len(), 20);
        apply_command(
            &mut document,
            &DocumentCommand::UpdateGrid {
                page: "home".to_string(),
                id: "grid".to_string(),
                columns: Some(3),
                rows: Some(3),
                gap_mm: None,
                cells: None,
            },
        )
        .unwrap();
        let grid = document.element("home", "grid").unwrap().grid.as_ref().unwrap();
        assert_eq!(grid.cells.len(), 9);
    }

    #[test]
    fn grid_content_box_prevents_page_height_cells() {
        let mut document = sample();
        document.pages.clear();
        add_grid_page(&mut document, "home", "Home", 4, 1).unwrap();
        let page = document.page_mut("home").unwrap();
        page.grid.origin_y_mm = Some(50.0);
        page.grid.content_height_mm = Some(36.0);
        page.elements.push(new_element("cell-1", "text", "uno"));
        let rendered = document.render().unwrap();
        let element = &rendered.pages[0].elements[0];
        assert!((element.y_mm - 50.0).abs() < 0.1, "origine verticale: {}", element.y_mm);
        assert!(element.h_mm <= 36.1, "altezza non limitata: {}", element.h_mm);
    }

    #[test]
    fn absolute_element_on_grid_page_keeps_its_coordinates() {
        let mut document = sample();
        document.pages.clear();
        add_grid_page(&mut document, "home", "Home", 4, 4).unwrap();
        let page = document.page_mut("home").unwrap();
        page.grid.content_height_mm = Some(100.0);
        let mut header = new_element("header", "title", "Titolo");
        header.absolute = true;
        header.x_mm = 12.0;
        header.y_mm = 8.0;
        header.w_mm = 120.0;
        header.h_mm = 10.0;
        page.elements.push(header);
        page.elements.push(new_element("cell-1", "symbol", "cane"));
        let rendered = document.render().unwrap();
        let header = rendered.pages[0]
            .elements
            .iter()
            .find(|element| element.id == "header")
            .unwrap();
        assert!((header.x_mm - 12.0).abs() < 0.1);
        assert!((header.y_mm - 8.0).abs() < 0.1);
        assert!((header.w_mm - 120.0).abs() < 0.1);
    }

    #[test]
    fn long_text_is_wrapped_in_multiple_lines() {
        let mut document = sample();
        document.pages.clear();
        add_flow_page(&mut document, "story", "Storia").unwrap();
        let page = document.page_mut("story").unwrap();
        let mut paragraph = new_element(
            "paragraph",
            "text",
            "Questa frase è abbastanza lunga da richiedere più righe nel rendering della pagina.",
        );
        paragraph.w_mm = 90.0;
        paragraph.h_mm = 30.0;
        paragraph.style.font_size = 4.0;
        paragraph.style.align = "left".to_string();
        page.elements.push(paragraph);
        let rendered = document.render().unwrap();
        let svg = render_page_svg(&rendered.pages[0]);
        let count = svg.matches("<text ").count();
        assert!(count >= 2, "il testo lungo non è stato mandato a capo: {count} righe");
    }

    #[test]
    fn grid_cells_can_be_merged_split_and_resized() {
        let mut document = build_document("g", "Griglia", "mixed", "A4", "portrait");
        document.pages.clear();
        add_grid_page(&mut document, "home", "Home", 3, 2).unwrap();
        apply_command(
            &mut document,
            &DocumentCommand::CreateGrid {
                page: "home".to_string(),
                id: "grid".to_string(),
                columns: 3,
                rows: 2,
                gap_mm: Some(2.0),
                x_mm: Some(10.0),
                y_mm: Some(10.0),
                w_mm: Some(180.0),
                h_mm: Some(100.0),
            },
        )
        .unwrap();
        apply_command(
            &mut document,
            &DocumentCommand::MergeGridCell {
                page: "home".to_string(),
                id: "grid".to_string(),
                cell_id: "grid-c1".to_string(),
                direction: "right".to_string(),
            },
        )
        .unwrap();
        let grid = document.element("home", "grid").unwrap().grid.as_ref().unwrap();
        assert_eq!(grid.cells.len(), 5);
        assert_eq!(grid.cells[0].column_span, 2);
        apply_command(
            &mut document,
            &DocumentCommand::SplitGridCell {
                page: "home".to_string(),
                id: "grid".to_string(),
                cell_id: "grid-c1".to_string(),
            },
        )
        .unwrap();
        let grid = document.element("home", "grid").unwrap().grid.as_ref().unwrap();
        assert_eq!(grid.cells.len(), 6);
        apply_command(
            &mut document,
            &DocumentCommand::SetGridSizes {
                page: "home".to_string(),
                id: "grid".to_string(),
                column_widths: Some(vec![2.0, 1.0, 1.0]),
                row_heights: Some(vec![1.0, 2.0]),
            },
        )
        .unwrap();
        let rendered = document.render().unwrap();
        let cells = &rendered.pages[0].elements[0].cells;
        assert!(cells[0].w_mm > cells[1].w_mm);
    }

    #[test]
    fn transaction_is_atomic() {
        let document = sample();
        let commands = vec![
            DocumentCommand::SetText {
                page: "home".to_string(),
                id: "grid-1".to_string(),
                text: "Nuovo".to_string(),
                spoken_text: None,
            },
            DocumentCommand::MovePage {
                id: "missing".to_string(),
                index: 0,
            },
        ];
        let error = apply_transaction(&document, &commands).unwrap_err();
        assert!(error.contains("pagina inesistente"));
        // L'originale non è stato modificato.
        assert_ne!(document.title, "rotto");
    }

    #[test]
    fn layout_migration_keeps_links_and_symbols() {
        let mut spec = crate::layout::LayoutSpecification::default();
        spec.title = "Vecchio".to_string();
        spec.elements.push(crate::layout::ElementSpec {
            id: "a".to_string(),
            label: "Mela".to_string(),
            symbol_id: Some("core.fruit.apple".to_string()),
            action: Some(crate::layout::ElementAction {
                action_type: "navigate".to_string(),
                target_page: Some(0),
            }),
            ..Default::default()
        });
        let document = from_layout(&spec);
        validate(&document).unwrap();
        assert_eq!(document.pages[0].elements[0].symbol_id.as_deref(), Some("core.fruit.apple"));
        assert_eq!(document.pages[0].elements[0].link_page.as_deref(), Some("page-1"));
    }
}
