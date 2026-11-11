//! Foglio di pregrafismo: lettere a punti con guide di scrittura.
//!
//! Il foglio è un SVG autonomo, quindi si stampa, si converte in PNG/PDF con
//! resvg e si inserisce in un documento universale come immagine modificabile.
//! Il riempimento delle lettere è un motivo di punti vero (non un carattere
//! tratteggiato): dimensione dei punti, spaziatura, spessore del contorno e
//! colore sono parametri indipendenti. Nessun font viene inventato: se il
//! carattere scelto non è installato, resvg usa il ripiego di sistema.

use serde::{Deserialize, Serialize};

fn default_font_family() -> String {
    "Arial, Helvetica, sans-serif".to_string()
}

fn default_color() -> String {
    "#1f2937".to_string()
}

fn default_title() -> String {
    "Pregrafismo".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConfig {
    /// Testo da tracciare. Gli spazi separano le parole, `\n` va a capo.
    pub text: String,
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// Larghezza del foglio in millimetri.
    #[serde(default = "default_page_width")]
    pub width_mm: f32,
    /// Altezza del foglio in millimetri.
    #[serde(default = "default_page_height")]
    pub height_mm: f32,
    /// Corpo delle lettere in millimetri.
    #[serde(default = "default_font_size")]
    pub font_size_mm: f32,
    /// Raggio dei punti in millimetri.
    #[serde(default = "default_dot_radius")]
    pub dot_radius_mm: f32,
    /// Distanza fra i centri dei punti in millimetri.
    #[serde(default = "default_dot_spacing")]
    pub dot_spacing_mm: f32,
    /// Spessore del contorno delle lettere in millimetri (0 = nessuno).
    #[serde(default)]
    pub outline_mm: f32,
    /// Contorno tratteggiato invece che continuo.
    #[serde(default)]
    pub dashed_outline: bool,
    #[serde(default = "default_color")]
    pub color: String,
    /// Mostra le guide (base, altezza x, ascendenti, discendenti).
    #[serde(default = "default_true")]
    pub guides: bool,
    /// Trasforma il testo in maiuscolo.
    #[serde(default)]
    pub uppercase: bool,
    /// Margine del foglio in millimetri.
    #[serde(default = "default_margin")]
    pub margin_mm: f32,
}

fn default_page_width() -> f32 {
    297.0
}
fn default_page_height() -> f32 {
    210.0
}
fn default_font_size() -> f32 {
    40.0
}
fn default_dot_radius() -> f32 {
    0.55
}
fn default_dot_spacing() -> f32 {
    2.2
}
fn default_margin() -> f32 {
    15.0
}
fn default_true() -> bool {
    true
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            text: String::new(),
            title: default_title(),
            font_family: default_font_family(),
            width_mm: default_page_width(),
            height_mm: default_page_height(),
            font_size_mm: default_font_size(),
            dot_radius_mm: default_dot_radius(),
            dot_spacing_mm: default_dot_spacing(),
            outline_mm: 0.35,
            dashed_outline: false,
            color: default_color(),
            guides: true,
            uppercase: false,
            margin_mm: default_margin(),
        }
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Accetta solo colori CSS semplici: evita di iniettare markup arbitrario
/// nell'SVG. Un valore non riconosciuto torna al grigio scuro predefinito.
fn safe_color(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with('#') && trimmed.len() <= 9 {
        let hex = trimmed.trim_start_matches('#');
        if hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return trimmed.to_string();
        }
    }
    if trimmed.chars().all(|c| c.is_ascii_alphabetic()) && !trimmed.is_empty() {
        return trimmed.to_string();
    }
    "#1f2937".to_string()
}

/// Il nome del carattere è una pila CSS: si conservano lettere, virgole,
/// spazi e trattini; si eliminano le virgolette che romperebbero l'attributo.
fn safe_font(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | ',' | '-' | '_' | '\''))
        .take(160)
        .collect();
    if cleaned.trim().is_empty() {
        default_font_family()
    } else {
        cleaned
    }
}

pub fn trace_sheet_svg(config: &TraceConfig) -> String {
    let width = config.width_mm.clamp(60.0, 2000.0);
    let height = config.height_mm.clamp(60.0, 2000.0);
    let margin = config.margin_mm.clamp(5.0, width.min(height) / 3.0);
    let font = config.font_size_mm.clamp(8.0, height / 2.0);
    let dot_radius = config.dot_radius_mm.clamp(0.15, font / 4.0);
    let spacing = config.dot_spacing_mm.clamp(dot_radius * 2.4, font);
    let outline = config.outline_mm.clamp(0.0, font / 8.0);
    let color = safe_color(&config.color);
    let family = safe_font(&config.font_family);
    let text = if config.uppercase {
        config.text.to_uppercase()
    } else {
        config.text.clone()
    };

    let usable = width - margin * 2.0;
    let line_height = font * 1.65;
    let rows_available = (((height - margin * 2.0) / line_height).floor() as usize).max(1);
    let chars_per_line = ((usable / (font * 0.95)).floor() as usize).max(1);

    let mut out = String::new();
    out.push_str(&format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{width}mm" height="{height}mm" viewBox="0 0 {width} {height}">
<defs>
<pattern id="trace-dots" patternUnits="userSpaceOnUse" width="{spacing}" height="{spacing}">
<circle cx="{cx}" cy="{cy}" r="{dot_radius}" fill="{color}"/>
</pattern>
</defs>
<rect x="0" y="0" width="{width}" height="{height}" fill="#ffffff"/>
"##,
        cx = spacing / 2.0,
        cy = spacing / 2.0,
    ));

    if !config.title.trim().is_empty() {
        out.push_str(&format!(
            r##"<text x="{mx}" y="{ty}" font-family="{family}" font-size="{title_size}" fill="#475569">{title}</text>
"##,
            mx = margin,
            ty = margin.max(6.0) + 4.0,
            title_size = (margin * 0.55).clamp(4.0, 9.0),
            title = escape_xml(config.title.trim()),
        ));
    }

    let start_y = margin + (margin * 0.8).max(8.0);
    let mut row = 0usize;
    let mut column = 0usize;
    let lines: Vec<&str> = text.split('\n').collect();
    let mut chars = lines.into_iter().flat_map(|line| {
        let mut parts: Vec<Option<char>> = line.chars().map(Some).collect();
        parts.push(None); // a capo dopo ogni riga del testo
        parts
    });

    let stroke = if outline > 0.0 {
        let dash = if config.dashed_outline {
            format!(" stroke-dasharray=\"{d} {g}\"", d = font * 0.06, g = font * 0.04)
        } else {
            String::new()
        };
        format!(
            " stroke=\"{color}\" stroke-width=\"{outline}\" stroke-opacity=\"0.55\"{dash}"
        )
    } else {
        String::new()
    };

    loop {
        if row >= rows_available {
            break;
        }
        let baseline = start_y + row as f32 * line_height + font;
        let cell_w = usable / chars_per_line as f32;
        if config.guides {
            // Ascendenti, altezza x, base e discendenti: tratto sottile, mai
            // sopra le lettere.
            let lines = [
                baseline - font * 0.72,
                baseline - font * 0.48,
                baseline,
                baseline + font * 0.22,
            ];
            for (index, y) in lines.iter().enumerate() {
                let stroke_width = if index == 2 { 0.25 } else { 0.16 };
                out.push_str(&format!(
                    r##"<line x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="#93a4b8" stroke-width="{stroke_width}" stroke-dasharray="2 2"/>
"##,
                    x1 = margin,
                    x2 = width - margin,
                    y = y,
                ));
            }
        }

        let mut wrote_any = false;
        for _ in 0..chars_per_line {
            let Some(next) = chars.next() else {
                break;
            };
            let Some(character) = next else {
                // Fine riga del testo: si passa alla riga successiva del foglio.
                wrote_any = true;
                break;
            };
            if character == '\r' {
                continue;
            }
            let x = margin + column as f32 * cell_w + cell_w * 0.08;
            wrote_any = true;
            if character == ' ' {
                column += 1;
                continue;
            }
            out.push_str(&format!(
                r#"<text x="{x}" y="{baseline}" font-family="{family}" font-size="{font}" font-weight="600" fill="url(#trace-dots)"{stroke}>{ch}</text>
"#,
                ch = escape_xml(&character.to_string()),
            ));
            // Punto di partenza del tratto: aiuta a capire da dove iniziare.
            out.push_str(&format!(
                r#"<circle cx="{cx}" cy="{cy}" r="{radius}" fill="{color}"/>
"#,
                cx = x + font * 0.12,
                cy = baseline - font * 0.52,
                radius = dot_radius * 1.4,
            ));
            column += 1;
        }
        column = 0;
        row += 1;
        if !wrote_any {
            break;
        }
    }

    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genera_un_svg_con_punti_e_guide() {
        let config = TraceConfig {
            text: "Casa".to_string(),
            ..TraceConfig::default()
        };
        let svg = trace_sheet_svg(&config);
        assert!(svg.starts_with("<?xml"));
        assert!(svg.contains("pattern id=\"trace-dots\""));
        assert!(svg.contains("fill=\"url(#trace-dots)\""));
        assert!(svg.contains(">C</text>"));
        assert!(svg.contains(">a</text>"));
        assert!(svg.contains("stroke-dasharray"));
        assert!(svg.contains("width=\"297mm\""));
    }

    #[test]
    fn i_colori_non_validi_tornano_al_predefinito() {
        let config = TraceConfig {
            text: "A".to_string(),
            color: "\" onload=\"alert(1)".to_string(),
            ..TraceConfig::default()
        };
        let svg = trace_sheet_svg(&config);
        assert!(!svg.contains("onload"));
        assert!(svg.contains("#1f2937"));
    }

    #[test]
    fn il_testo_viene_escapato() {
        let config = TraceConfig {
            text: "<&>".to_string(),
            ..TraceConfig::default()
        };
        let svg = trace_sheet_svg(&config);
        assert!(svg.contains("&lt;"));
        assert!(!svg.contains("><text"));
    }
}
