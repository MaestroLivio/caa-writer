//! Esportazione PDF e PNG del documento universale.
//!
//! Le pagine vengono rasterizzate dal medesimo SVG prodotto dal motore di
//! rendering (quindi con gli stessi pittogrammi, testi, bordi e geometrie) e
//! inserite in un PDF valido, multipagina, senza dipendenze da un browser
//! esterno. Il PDF è quindi una rappresentazione fedele ma non modificabile:
//! il formato di riferimento resta `.caadoc`.

use std::io::Write;
use std::path::Path;

use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::universal::{render_page_svg, RenderedPage, UniversalDocument};

const MM_TO_PT: f32 = 2.834_645_7;

/// Rasterizza un singolo SVG (pittogramma o elemento) alle dimensioni richieste.
/// Serve a DOCX/PPTX per incorporare immagini modificabili separatamente.
pub fn svg_to_png(svg: &str, width_mm: f32, height_mm: f32, scale: f32) -> Result<Vec<u8>, String> {
    let mut options = resvg::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_str(svg, &options)
        .map_err(|error| format!("SVG non valido: {error}"))?;
    let size = tree.size();
    let target_w = (width_mm.max(1.0) * 3.779_527_6 * scale).max(8.0);
    let target_h = (height_mm.max(1.0) * 3.779_527_6 * scale).max(8.0);
    let fit = (target_w / size.width()).min(target_h / size.height());
    let mut pixmap = resvg::tiny_skia::Pixmap::new(
        target_w.round().max(1.0) as u32,
        target_h.round().max(1.0) as u32,
    )
    .ok_or_else(|| "elemento troppo grande per la rasterizzazione".to_string())?;
    let offset_x = (target_w - size.width() * fit) / 2.0;
    let offset_y = (target_h - size.height() * fit) / 2.0;
    let transform = resvg::tiny_skia::Transform::from_translate(offset_x, offset_y)
        .pre_scale(fit, fit);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap
        .encode_png()
        .map_err(|error| format!("codifica PNG elemento: {error}"))
}

pub fn page_to_png(page: &RenderedPage, scale: f32) -> Result<Vec<u8>, String> {
    let svg = render_page_svg(page);
    let mut options = resvg::usvg::Options::default();
    // Senza i font di sistema resvg disegna solo forme e immagini: i testi
    // del documento sparirebbero dal PDF/PNG.
    options.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_str(&svg, &options)
        .map_err(|error| format!("SVG di pagina non valido: {error}"))?;
    let size = tree.size();
    let width = (size.width() * scale).round().max(1.0) as u32;
    let height = (size.height() * scale).round().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "pagina troppo grande per la rasterizzazione".to_string())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|error| format!("codifica PNG: {error}"))
}

fn rgb_pixels(png: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let image = image::load_from_memory(png).map_err(|error| format!("PNG non valido: {error}"))?;
    let image = image.to_rgb8();
    let (width, height) = image.dimensions();
    Ok((width, height, image.into_raw()))
}

fn pdf_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
        .replace('\r', " ")
        .replace('\n', " ")
}

/// Costruisce un PDF con una pagina-immagine per ogni pagina del documento.
pub fn build_pdf(document: &UniversalDocument, scale: f32) -> Result<Vec<u8>, String> {
    let rendered = document.render()?;
    if rendered.pages.is_empty() {
        return Err("documento senza pagine".to_string());
    }

    let mut pages: Vec<(f32, f32, u32, u32, Vec<u8>)> = Vec::new();
    for page in &rendered.pages {
        let png = page_to_png(page, scale)?;
        let (width, height, rgb) = rgb_pixels(&png)?;
        pages.push((
            page.width_mm * MM_TO_PT,
            page.height_mm * MM_TO_PT,
            width,
            height,
            rgb,
        ));
    }

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets: Vec<usize> = Vec::new();
    let object = |body: &[u8], out: &mut Vec<u8>, offsets: &mut Vec<usize>| {
        offsets.push(out.len());
        out.extend_from_slice(body);
    };

    let page_count = pages.len();
    let kids = (0..page_count)
        .map(|index| format!("{} 0 R", 3 + index * 3))
        .collect::<Vec<_>>()
        .join(" ");
    let catalog = b"<< /Type /Catalog /Pages 2 0 R >>";
    object(catalog, &mut out, &mut offsets);
    let pages_obj = format!(
        "<< /Type /Pages /Kids [{kids}] /Count {page_count} >>"
    );
    object(pages_obj.as_bytes(), &mut out, &mut offsets);

    for (index, (width_pt, height_pt, pixel_width, pixel_height, rgb)) in
        pages.iter().enumerate()
    {
        let page_object = 3 + index * 3;
        let content_object = page_object + 1;
        let image_object = page_object + 2;
        let page_body = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.3} {:.3}] \
             /Resources << /XObject << /Im0 {image_object} 0 R >> >> \
             /Contents {content_object} 0 R >>",
            width_pt, height_pt
        );
        object(page_body.as_bytes(), &mut out, &mut offsets);

        let content = format!(
            "q\n{:.3} 0 0 {:.3} 0 0 cm\n/Im0 Do\nQ\n",
            width_pt, height_pt
        );
        let content_body = format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            content
        );
        object(content_body.as_bytes(), &mut out, &mut offsets);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(rgb)
            .map_err(|error| format!("compressione immagine: {error}"))?;
        let compressed = encoder
            .finish()
            .map_err(|error| format!("compressione immagine: {error}"))?;
        let image_header = format!(
            "<< /Type /XObject /Subtype /Image /Width {pixel_width} /Height {pixel_height} \
             /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\n",
            compressed.len()
        );
        let mut image_body = Vec::new();
        image_body.extend_from_slice(image_header.as_bytes());
        image_body.extend_from_slice(b"stream\n");
        image_body.extend_from_slice(&compressed);
        image_body.extend_from_slice(b"\nendstream");
        object(&image_body, &mut out, &mut offsets);
    }

    let info_object = 3 + page_count * 3;
    let info = format!(
        "<< /Title ({}) /Producer (CAA Writer) /Creator (CAA Universal Document Engine) >>",
        pdf_string(&document.title)
    );
    object(info.as_bytes(), &mut out, &mut offsets);

    let xref_offset = out.len();
    let object_count = offsets.len() + 1;
    out.extend_from_slice(format!("xref\n0 {object_count}\n").as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {object_count} /Root 1 0 R /Info {info_object} 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
        )
        .as_bytes(),
    );
    Ok(out)
}

pub fn export_pdf(document: &UniversalDocument, path: &Path) -> Result<(), String> {
    let bytes = build_pdf(document, 1.8)?;
    if !bytes.starts_with(b"%PDF") || bytes.len() < 200 {
        return Err("il PDF generato non è valido".to_string());
    }
    std::fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
}

pub fn export_first_page_png(document: &UniversalDocument, path: &Path) -> Result<(), String> {
    let rendered = document.render()?;
    let page = rendered
        .pages
        .first()
        .ok_or_else(|| "documento senza pagine".to_string())?;
    let png = page_to_png(page, 2.0)?;
    std::fs::write(path, png).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::{build_document, UniversalElement, UniversalPage};

    fn document() -> UniversalDocument {
        let mut document = build_document("pdf-test", "PDF di prova", "document", "A4", "portrait");
        let mut page = UniversalPage {
            id: "p1".to_string(),
            title: "Pagina".to_string(),
            layout: "absolute".to_string(),
            background: "#ffffff".to_string(),
            flow: Default::default(),
            grid: Default::default(),
            elements: Vec::new(),
            is_home: true,
            notes: String::new(),
        };
        page.elements.push(UniversalElement {
            id: "t1".to_string(),
            kind: "text".to_string(),
            text: "Ciao PDF".to_string(),
            spoken_text: "Ciao PDF".to_string(),
            x_mm: 20.0,
            y_mm: 20.0,
            w_mm: 100.0,
            h_mm: 20.0,
            visible: true,
            ..UniversalElement::default()
        });
        document.pages = vec![page];
        document
    }

    #[test]
    fn pdf_is_real_and_has_one_page() {
        let bytes = build_pdf(&document(), 1.0).unwrap();
        assert!(bytes.starts_with(b"%PDF-1.4"));
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/Count 1"));
        assert!(text.ends_with("%%EOF\n"));
    }

    #[test]
    fn png_page_is_decodable() {
        let rendered = document().render().unwrap();
        let png = page_to_png(&rendered.pages[0], 1.0).unwrap();
        let image = image::load_from_memory(&png).unwrap();
        assert!(image.width() >= 100);
        assert!(image.height() >= 100);
    }
}
