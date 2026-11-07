//! Esportazione DOCX del documento universale.
//!
//! Strategia dichiarata: ogni pagina viene rasterizzata dal renderer canonico e
//! inserita come immagine a piena pagina. Il DOCX è valido e conserva l'aspetto
//! (pittogrammi, testi, bordi, colori), ma non è un documento Word nativo
//! modificabile elemento per elemento: per la modifica completa resta `.caadoc`.
//! Questa perdita è esplicita e viene riportata negli avvisi.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::export::universal_pdf::page_to_png;
use crate::export::emu;
use crate::model::ExportResult;
use crate::universal::UniversalDocument;

const TWIPS_PER_MM: f32 = 56.692_913;

pub fn write(path: &Path, document: &UniversalDocument) -> Result<ExportResult, String> {
    let rendered = document.render()?;
    let mut media: Vec<(String, Vec<u8>, u32, u32, f32, f32)> = Vec::new();
    for (index, page) in rendered.pages.iter().enumerate() {
        let png = page_to_png(page, 1.6)?;
        let image = image::load_from_memory(&png).map_err(|e| format!("PNG pagina: {e}"))?;
        media.push((
            format!("page{}.png", index + 1),
            png,
            image.width(),
            image.height(),
            page.width_mm,
            page.height_mm,
        ));
    }

    let mut body = String::new();
    for (index, (_name, _png, _pixel_w, _pixel_h, width_mm, height_mm)) in media.iter().enumerate() {
        let rid = format!("rIdImg{}", index + 1);
        let cx = emu(*width_mm / 25.4);
        let cy = emu(*height_mm / 25.4);
        body.push_str(&format!(
            "<w:p><w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\">\
<wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:docPr id=\"{id}\" name=\"Pagina {number}\"/>\
<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:pic><pic:nvPicPr><pic:cNvPr id=\"{id}\" name=\"page{number}.png\"/><pic:cNvPicPr/></pic:nvPicPr>\
<pic:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>\
<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm>\
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic>\
</wp:inline></w:drawing></w:r>",
            id = index + 1,
            number = index + 1
        ));
        if index + 1 < media.len() {
            body.push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");
        }
    }
    body.push_str(&format!(
        "<w:p><w:r><w:rPr><w:sz w:val=\"16\"/><w:color w:val=\"555555\"/></w:rPr>\
         <w:t xml:space=\"preserve\">Esportato da CAA Writer. Per la modifica completa usare il documento nativo .caadoc.</w:t>\
         </w:r></w:p>"
    ));
    let (page_w_mm, page_h_mm) = document.page.dimensions()?;
    let page_w = (page_w_mm * TWIPS_PER_MM).round() as u32;
    let page_h = (page_h_mm * TWIPS_PER_MM).round() as u32;
    let document_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" \
xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<w:body>{body}<w:sectPr><w:pgSz w:w=\"{page_w}\" w:h=\"{page_h}\"/>\
<w:pgMar w:top=\"0\" w:right=\"0\" w:bottom=\"0\" w:left=\"0\" w:header=\"0\" w:footer=\"0\" w:gutter=\"0\"/>\
</w:sectPr></w:body></w:document>"
    );

    let mut rels = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );
    for (index, _) in media.iter().enumerate() {
        rels.push_str(&format!(
            "<Relationship Id=\"rIdImg{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/page{}.png\"/>",
            index + 1,
            index + 1
        ));
    }
    rels.push_str("</Relationships>");

    let content_types = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Default Extension=\"png\" ContentType=\"image/png\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
</Types>",
    );

    let styles = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:rPr><w:sz w:val=\"20\"/></w:rPr></w:style>\
</w:styles>";

    let mut zip = ZipWriter::new(
        std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?,
    );
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let entries: HashMap<&str, Vec<u8>> = HashMap::from([
        (
            "[Content_Types].xml",
            content_types.into_bytes(),
        ),
        (
            "_rels/.rels",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/>\
</Relationships>"
                .as_bytes()
                .to_vec(),
        ),
        ("word/document.xml", document_xml.into_bytes()),
        ("word/styles.xml", styles.as_bytes().to_vec()),
        ("word/_rels/document.xml.rels", rels.into_bytes()),
    ]);
    for (name, bytes) in entries {
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    for (name, png, _, _, _, _) in &media {
        zip.start_file(format!("word/media/{name}"), options)
            .map_err(|e| e.to_string())?;
        zip.write_all(png).map_err(|e| e.to_string())?;
    }
    zip.finish().map_err(|e| format!("chiusura DOCX: {e}"))?;

    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
        warnings: vec![
            "DOCX: pagine esportate come immagini; testo e pittogrammi non sono singolarmente modificabili in Word. Usare .caadoc per la modifica completa.".to_string(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::{build_document, UniversalElement, UniversalPage};

    #[test]
    fn docx_contains_document_and_media() {
        let mut document = build_document("docx", "DOCX", "document", "A4", "portrait");
        let page = UniversalPage {
            id: "p1".to_string(),
            title: "Pagina".to_string(),
            layout: "absolute".to_string(),
            background: "#fff".to_string(),
            flow: Default::default(),
            grid: Default::default(),
            elements: vec![UniversalElement {
                id: "t1".to_string(),
                kind: "text".to_string(),
                text: "Ciao".to_string(),
                x_mm: 20.0,
                y_mm: 20.0,
                w_mm: 80.0,
                h_mm: 20.0,
                ..UniversalElement::default()
            }],
            is_home: true,
            notes: String::new(),
        };
        document.pages = vec![page];
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.docx");
        write(&path, &document).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert!(archive.by_name("word/document.xml").is_ok());
        assert!(archive.by_name("word/media/page1.png").is_ok());
    }
}
