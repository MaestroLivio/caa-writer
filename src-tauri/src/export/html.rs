//! Esportazione HTML autosufficiente.
//!
//! Gli SVG sono incorporati nel file: il documento si apre offline, senza
//! dipendenze da servizi o CDN. Include i crediti dei repertori usati.

use std::collections::BTreeMap;
use std::path::Path;

use crate::export::xml_escape;
use crate::model::{CaaMode, ExportDoc, ExportResult};

pub fn write(path: &Path, doc: &ExportDoc) -> Result<ExportResult, String> {
    let mut warnings = Vec::new();
    let mode_class = match doc.mode {
        CaaMode::Testo => "mode-text",
        CaaMode::Sopra => "mode-above",
        CaaMode::Sotto => "mode-below",
        CaaMode::SoloSimboli => "mode-symbols",
        CaaMode::Semplificata => "mode-simple",
    };

    let mut body = String::new();
    for paragraph in &doc.paragraphs {
        body.push_str("<section class=\"caa-paragraph\">\n");
        if doc.include_symbols && !paragraph.units.is_empty() {
            body.push_str("<div class=\"caa-units\">\n");
            for unit in &paragraph.units {
                let mut media = String::new();
                if let Some(svg) = &unit.svg {
                    media.push_str("<span class=\"caa-symbol\">");
                    media.push_str(svg);
                    media.push_str("</span>");
                } else if unit.symbol_id.is_none() {
                    media.push_str("<span class=\"caa-symbol caa-symbol--empty\">?</span>");
                }
                let approximate = if unit.approximate {
                    " caa-unit--approx"
                } else {
                    ""
                };
                let caption = if doc.include_captions {
                    format!(
                        "<span class=\"caa-caption\">{}</span>",
                        xml_escape(&unit.caption)
                    )
                } else {
                    String::new()
                };
                body.push_str(&format!(
                    "<div class=\"caa-unit{approximate}\">{media}{caption}</div>\n"
                ));
            }
            body.push_str("</div>\n");
        }
        let tag = match paragraph.style.as_str() {
            "heading" => "h2",
            _ => "p",
        };
        body.push_str(&format!(
            "<{tag} class=\"caa-text\">{}</{tag}>\n",
            xml_escape(&paragraph.plain_text)
        ));
        body.push_str("</section>\n");
    }

    if doc.paragraphs.iter().all(|p| p.units.is_empty()) {
        warnings.push("il documento non contiene pittogrammi: esportato solo il testo".to_string());
    }

    let mut credits: BTreeMap<String, String> = BTreeMap::new();
    for entry in &doc.attributions {
        credits.insert(
            entry.symbol_id.clone(),
            format!(
                "{} — {} ({})",
                entry.label,
                entry.pack_name,
                entry.license
            ),
        );
    }
    let credits_html = if credits.is_empty() {
        String::new()
    } else {
        let items: String = credits
            .values()
            .map(|c| format!("<li>{}</li>", xml_escape(c)))
            .collect();
        format!(
            "<footer class=\"caa-credits\"><h3>Crediti dei pittogrammi</h3><ul>{items}</ul>\
             <p>Documento generato con CAA Writer. I pittogrammi mantengono la licenza del repertorio di origine.</p></footer>"
        )
    };

    let html = format!(
        "<!doctype html>\n<html lang=\"it\">\n<head>\n<meta charset=\"utf-8\">\n\
<title>{title}</title>\n<style>\n{style}\n</style>\n</head>\n\
<body class=\"{mode_class}\">\n<h1 class=\"caa-title\">{title}</h1>\n{body}\n{credits_html}\n</body>\n</html>\n",
        title = xml_escape(&doc.title),
        style = STYLE,
        mode_class = mode_class,
        body = body,
        credits_html = credits_html
    );

    crate::export::write_atomic(path, html.as_bytes())?;
    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ExportDoc, ExportParagraph, ExportUnit};

    fn document(include_captions: bool) -> ExportDoc {
        ExportDoc {
            title: "Prova".to_string(),
            paragraphs: vec![ExportParagraph {
                plain_text: "Luca mangia una pesca.".to_string(),
                style: "paragraph".to_string(),
                units: vec![ExportUnit {
                    surface: "pesca".to_string(),
                    caption: "pesca".to_string(),
                    symbol_id: None,
                    svg: None,
                    png_base64: None,
                    approximate: false,
                }],
            }],
            include_symbols: true,
            include_captions,
            mode: CaaMode::Sopra,
            attributions: vec![],
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn caption_is_omitted_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.html");
        write(&path, &document(false)).unwrap();
        let html = std::fs::read_to_string(&path).unwrap();
        assert!(!html.contains("<span class=\"caa-caption\">"));
        assert!(html.contains("Luca mangia una pesca."));
    }

    #[test]
    fn caption_is_present_when_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.html");
        write(&path, &document(true)).unwrap();
        let html = std::fs::read_to_string(&path).unwrap();
        assert!(html.contains("<span class=\"caa-caption\">pesca</span>"));
    }
}

const STYLE: &str = r#"
:root { color-scheme: light; }
body { font-family: "Helvetica Neue", Arial, sans-serif; color: #1f2933; margin: 2rem auto; max-width: 900px; line-height: 1.5; }
.caa-title { font-size: 1.8rem; margin-bottom: 1.5rem; }
.caa-paragraph { margin-bottom: 1.6rem; page-break-inside: avoid; }
.caa-units { display: flex; flex-wrap: wrap; gap: 0.6rem; margin-bottom: 0.4rem; }
.caa-unit { display: flex; flex-direction: column; align-items: center; width: 84px; }
.caa-symbol { display: block; width: 64px; height: 64px; }
.caa-symbol svg { width: 100%; height: 100%; }
.caa-symbol--empty { border: 2px dashed #9aa6b8; border-radius: 8px; display: flex; align-items: center; justify-content: center; color: #9aa6b8; font-weight: bold; }
.caa-caption { font-size: 0.75rem; text-align: center; word-break: break-word; }
.caa-unit--approx .caa-symbol { border: 2px dashed #d68b00; border-radius: 8px; padding: 2px; }
.caa-text { font-size: 1.05rem; }
.mode-below .caa-unit { flex-direction: column-reverse; }
.mode-symbols .caa-text { font-size: 0.7rem; color: #6b7280; }
.mode-text .caa-units { display: none; }
.caa-credits { margin-top: 3rem; border-top: 1px solid #d7dee8; padding-top: 1rem; font-size: 0.8rem; color: #4b5563; }
@media print {
  body { margin: 0; }
  .caa-paragraph { break-inside: avoid; }
}
"#;
