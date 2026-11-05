//! Sanificazione e validazione degli SVG.
//!
//! Un pittogramma è un asset eseguibile dal renderer: deve essere XML puro,
//! senza script, senza riferimenti esterni e senza elementi attivi.
//! Questo modulo è usato sia all'import sia prima di mostrare un simbolo.

use roxmltree::{Document, Node};

const FORBIDDEN_TAGS: &[&str] = &[
    "script",
    "foreignObject",
    "iframe",
    "audio",
    "video",
    "animate",
    "animateTransform",
    "animateMotion",
    "set",
    "discard",
    "handler",
    "use-script",
];

const FORBIDDEN_PREFIXES: &[&str] = &["javascript:", "http://", "https://", "file:", "data:", "//"];

#[derive(Debug, Clone)]
pub struct SanitizedSvg {
    pub svg: String,
    pub bytes: usize,
    pub warnings: Vec<String>,
    pub view_box: Option<String>,
}

/// Valida e riscrive un SVG rimuovendo commenti, DOCTYPE e costrutti attivi.
pub fn sanitize(input: &str, max_bytes: usize) -> Result<SanitizedSvg, String> {
    if input.len() > max_bytes {
        return Err(format!(
            "SVG troppo grande: {} byte (limite {max_bytes})",
            input.len()
        ));
    }
    if !input.trim_start().starts_with('<') {
        return Err("il file non sembra un SVG".to_string());
    }
    let doc = Document::parse(input).map_err(|e| format!("XML non valido: {e}"))?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err(format!(
            "elemento radice non supportato: {}",
            root.tag_name().name()
        ));
    }

    let has_namespace = root
        .tag_name()
        .namespace()
        .map(|n| n == "http://www.w3.org/2000/svg")
        .unwrap_or(false);
    let mut warnings = Vec::new();
    if !has_namespace {
        warnings.push("namespace SVG mancante: aggiunto".to_string());
    }

    let view_box = root
        .attribute("viewBox")
        .map(|v| v.to_string())
        .or_else(|| root.attribute("viewbox").map(|v| v.to_string()));
    if view_box.is_none() {
        let w = root.attribute("width").unwrap_or("100");
        let h = root.attribute("height").unwrap_or("100");
        warnings.push(format!("viewBox assente: creato 0 0 {w} {h}"));
    }

    for node in root.descendants() {
        if !node.is_element() {
            continue;
        }
        let name = node.tag_name().name();
        if FORBIDDEN_TAGS.iter().any(|t| t.eq_ignore_ascii_case(name)) {
            return Err(format!("elemento non ammesso in un pittogramma: <{name}>"));
        }
        for attr in node.attributes() {
            let attr_name = attr.name().to_ascii_lowercase();
            if attr_name.starts_with("on") {
                return Err(format!("gestore di evento non ammesso: {attr_name}"));
            }
            if attr_name == "href" || attr_name.ends_with(":href") {
                let value = attr.value().trim().to_ascii_lowercase();
                if value.starts_with('#') {
                    continue;
                }
                // I pittogrammi ARASAAC sono PNG ufficiali incorporati nel SVG:
                // è ammesso solo il data URI di immagine, mai una risorsa esterna.
                if value.starts_with("data:image/png;base64,")
                    || value.starts_with("data:image/jpeg;base64,")
                    || value.starts_with("data:image/webp;base64,")
                {
                    continue;
                }
                if FORBIDDEN_PREFIXES.iter().any(|p| value.starts_with(p)) {
                    return Err(format!("riferimento esterno non ammesso: {}", attr.value()));
                }
            }
            if attr_name == "style" {
                let value = attr.value().to_ascii_lowercase();
                if value.contains("url(") || value.contains("expression") {
                    return Err("stile con url()/expression non ammesso".to_string());
                }
            }
        }
    }

    // Controllo di complessità: un pittogramma non deve essere un'illustrazione
    // arbitrariamente complessa (illeggibile a 48 px e pesante da renderizzare).
    let element_count = root.descendants().filter(|n| n.is_element()).count();
    if element_count > 400 {
        return Err(format!(
            "SVG troppo complesso per un pittogramma: {element_count} elementi (limite 400)"
        ));
    }

    let mut out = String::with_capacity(input.len());
    write_root(&root, &mut out);

    Ok(SanitizedSvg {
        bytes: out.len(),
        svg: out,
        warnings,
        view_box,
    })
}

fn write_root(root: &Node, out: &mut String) {
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\"");
    for attr in root.attributes() {
        let name = attr.name();
        if name.starts_with("xmlns") {
            continue;
        }
        out.push(' ');
        out.push_str(name);
        out.push_str("=\"");
        escape_into(attr.value(), out);
        out.push('"');
    }
    if root.attribute("viewBox").is_none() {
        let w = root.attribute("width").unwrap_or("100");
        let h = root.attribute("height").unwrap_or("100");
        out.push_str(&format!(" viewBox=\"0 0 {w} {h}\""));
    }
    write_children(root, out);
}

fn write_children(node: &Node, out: &mut String) {
    let children: Vec<Node> = node.children().filter(|n| n.is_element() || n.is_text()).collect();
    if children.is_empty() {
        out.push_str("/>");
        return;
    }
    out.push('>');
    for child in children {
        if child.is_element() {
            let name = child.tag_name().name();
            out.push('<');
            out.push_str(name);
            for attr in child.attributes() {
                let attr_name = attr.name();
                if attr_name.starts_with("xmlns") {
                    continue;
                }
                out.push(' ');
                out.push_str(attr_name);
                out.push_str("=\"");
                escape_into(attr.value(), out);
                out.push('"');
            }
            write_children(&child, out);
        } else if let Some(text) = child.text() {
            escape_into(text.trim(), out);
        }
    }
    out.push_str("</");
    out.push_str(node.tag_name().name());
    out.push('>');
}

fn escape_into(value: &str, out: &mut String) {
    for c in value.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
}

/// Controllo leggero: l'SVG è già sanificato e ha un viewBox?
pub fn is_renderable(svg: &str) -> bool {
    svg.contains("<svg") && svg.contains("viewBox")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_and_normalizes_valid_svg() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><!-- c --><circle cx="50" cy="50" r="40" fill="#F4C430"/></svg>"##;
        let out = sanitize(svg, 100_000).unwrap();
        assert!(out.svg.contains("<circle"));
        assert!(!out.svg.contains("<!--"));
        assert!(out.svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    }

    #[test]
    fn rejects_script() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><script>alert(1)</script></svg>"#;
        assert!(sanitize(svg, 100_000).is_err());
    }

    #[test]
    fn accepts_embedded_png_data_uri() {
        // Forma usata dal pacchetto ARASAAC: PNG ufficiale dentro un SVG.
        let png = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500 500"><image x="0" y="0" width="500" height="500" href="{png}"/></svg>"#
        );
        let out = sanitize(&svg, 200_000).expect("data URI ammesso");
        assert!(out.svg.contains("data:image/png;base64,"));
    }

    #[test]
    fn rejects_external_image() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><image href="https://example.com/x.png"/></svg>"#;
        assert!(sanitize(svg, 100_000).is_err());
    }

    #[test]
    fn rejects_event_handlers() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="5" height="5" onclick="x()"/></svg>"#;
        assert!(sanitize(svg, 100_000).is_err());
    }

    #[test]
    fn adds_viewbox_when_missing() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect width="4" height="4"/></svg>"#;
        let out = sanitize(svg, 100_000).unwrap();
        assert!(out.svg.contains("viewBox=\"0 0 24 24\""));
    }

    #[test]
    fn rejects_too_large() {
        let svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\">{}</svg>",
            "<rect width=\"1\" height=\"1\"/>".repeat(10)
        );
        assert!(sanitize(&svg, 100).is_err());
    }
}
