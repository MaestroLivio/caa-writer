//! Esportazione DOCX (Office Open XML) con pittogrammi incorporati.
//!
//! Scelta: il PNG del pittogramma viene rasterizzato dal frontend (canvas
//! WebView) perché DOCX non gestisce SVG come vettoriale in modo affidabile.
//! Il formato nativo `.caadoc` conserva invece sempre l'SVG originale.
//! Testo e didascalie restano testo modificabile in Word.

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use base64::Engine;
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::export::{emu, xml_escape};
use crate::model::{ExportDoc, ExportResult};

const SYMBOL_INCHES: f32 = 0.6;

pub fn write(path: &Path, doc: &ExportDoc) -> Result<ExportResult, String> {
    let mut warnings = Vec::new();
    let mut media: Vec<(String, Vec<u8>)> = Vec::new();
    let mut rels: HashMap<String, (String, usize)> = HashMap::new(); // symbol_id -> (rId, index)

    if doc.include_symbols {
        for paragraph in &doc.paragraphs {
            for unit in &paragraph.units {
                if let (Some(symbol_id), Some(png)) = (&unit.symbol_id, &unit.png_base64) {
                    if rels.contains_key(symbol_id) {
                        continue;
                    }
                    match base64::engine::general_purpose::STANDARD.decode(png.trim()) {
                        Ok(bytes) => {
                            let index = media.len() + 1;
                            let file_name = format!("image{index}.png");
                            let rid = format!("rIdImg{index}");
                            rels.insert(symbol_id.clone(), (rid, index));
                            media.push((file_name, bytes));
                        }
                        Err(e) => warnings.push(format!(
                            "pittogramma {symbol_id} ignorato (PNG non valido: {e})"
                        )),
                    }
                } else if unit.symbol_id.is_some() {
                    warnings.push(format!(
                        "pittogramma {} esportato senza immagine (manca la rasterizzazione)",
                        unit.symbol_id.clone().unwrap_or_default()
                    ));
                }
            }
        }
    }

    let mut body = String::new();
    body.push_str(&format!(
        "<w:p><w:pPr><w:pStyle w:val=\"Title\"/></w:pPr><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        xml_escape(&doc.title)
    ));

    let mut doc_pr = 1usize;
    for paragraph in &doc.paragraphs {
        let units_with_images = paragraph
            .units
            .iter()
            .filter(|u| u.symbol_id.as_ref().is_some_and(|id| rels.contains_key(id)))
            .count();

        if doc.include_symbols && units_with_images > 0 {
            body.push_str("<w:p>");
            for unit in &paragraph.units {
                let Some(symbol_id) = &unit.symbol_id else {
                    continue;
                };
                let Some((rid, _index)) = rels.get(symbol_id) else {
                    continue;
                };
                body.push_str(&image_run(rid, &mut doc_pr));
                if doc.include_captions && !unit.caption.is_empty() {
                    body.push_str(&format!(
                        "<w:r><w:rPr><w:i/><w:sz w:val=\"16\"/><w:color w:val=\"555555\"/></w:rPr>\
                         <w:t xml:space=\"preserve\"> {}</w:t></w:r>",
                        xml_escape(&unit.caption)
                    ));
                }
                body.push_str("<w:r><w:t xml:space=\"preserve\">   </w:t></w:r>");
            }
            body.push_str("</w:p>");
        }

        let style = if paragraph.style == "heading" {
            "<w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr>"
        } else {
            ""
        };
        body.push_str(&format!(
            "<w:p>{style}<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
            xml_escape(&paragraph.plain_text)
        ));
    }

    if doc.attributions.is_empty() {
        // Un documento può non usare pittogrammi di terzi: si inserisce comunque
        // una riga di credito generica, senza trattarlo come un errore.
        body.push_str(
            "<w:p><w:r><w:rPr><w:sz w:val=\"16\"/><w:color w:val=\"555555\"/></w:rPr>\
             <w:t xml:space=\"preserve\">Pittogrammi: repertorio incluso in CAA Writer (CC BY-NC-SA 4.0). \
             Registro completo delle licenze esportabile dall'applicazione.</w:t></w:r></w:p>",
        );
    } else {
        body.push_str(
            "<w:p><w:r><w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">Crediti dei pittogrammi</w:t></w:r></w:p>",
        );
        for entry in &doc.attributions {
            body.push_str(&format!(
                "<w:p><w:r><w:rPr><w:sz w:val=\"16\"/><w:color w:val=\"555555\"/></w:rPr>\
                 <w:t xml:space=\"preserve\">{} — {} ({})</w:t></w:r></w:p>",
                xml_escape(&entry.label),
                xml_escape(&entry.pack_name),
                xml_escape(&entry.license)
            ));
        }
    }

    let document_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" \
xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<w:body>{body}\
<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/>\
<w:pgMar w:top=\"1134\" w:right=\"1134\" w:bottom=\"1134\" w:left=\"1134\" w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/>\
</w:sectPr></w:body></w:document>"
    );

    let mut document_rels = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rIdStyles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>",
    );
    for (_symbol_id, (rid, index)) in rels.iter() {
        document_rels.push_str(&format!(
            "<Relationship Id=\"{rid}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/image{index}.png\"/>"
        ));
    }
    document_rels.push_str("</Relationships>");

    let content_types = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Default Extension=\"png\" ContentType=\"image/png\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
<Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>\
</Types>"
    );

    let root_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/>\
<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/>\
</Relationships>";

    let core = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" \
xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
<dc:title>{}</dc:title><dc:creator>CAA Writer</dc:creator></cp:coreProperties>",
        xml_escape(&doc.title)
    );

    let metadata = doc.metadata.clone();
    let _ = json!(metadata);

    let tmp = path.with_extension("docx.tmp");
    {
        let file = File::create(&tmp).map_err(|e| format!("impossibile creare il DOCX: {e}"))?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut put = |name: &str, bytes: &[u8]| -> Result<(), String> {
            zip.start_file(name, options)
                .map_err(|e| format!("{name}: {e}"))?;
            zip.write_all(bytes).map_err(|e| format!("{name}: {e}"))?;
            Ok(())
        };
        put("[Content_Types].xml", content_types.as_bytes())?;
        put("_rels/.rels", root_rels.as_bytes())?;
        put("docProps/core.xml", core.as_bytes())?;
        put("word/document.xml", document_xml.as_bytes())?;
        put("word/_rels/document.xml.rels", document_rels.as_bytes())?;
        put("word/styles.xml", STYLES.as_bytes())?;
        for (name, bytes) in &media {
            put(&format!("word/media/{name}"), bytes)?;
        }
        zip.finish().map_err(|e| format!("chiusura DOCX: {e}"))?;
    }

    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("impossibile completare l'esportazione: {e}")
    })?;

    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
        warnings,
    })
}

fn image_run(rid: &str, doc_pr: &mut usize) -> String {
    let size = emu(SYMBOL_INCHES);
    let id = *doc_pr;
    *doc_pr += 1;
    format!(
        "<w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\">\
<wp:extent cx=\"{size}\" cy=\"{size}\"/><wp:docPr id=\"{id}\" name=\"Pittogramma {id}\"/>\
<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:pic><pic:nvPicPr><pic:cNvPr id=\"{id}\" name=\"Pittogramma {id}\"/><pic:cNvPicPr/></pic:nvPicPr>\
<pic:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>\
<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{size}\" cy=\"{size}\"/></a:xfrm>\
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic>\
</a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"
    )
}

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:docDefaults><w:rPrDefault><w:rPr>
<w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/><w:sz w:val="22"/></w:rPr></w:rPrDefault>
<w:pPrDefault><w:pPr><w:spacing w:after="120"/></w:pPr></w:pPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
<w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/>
<w:rPr><w:b/><w:sz w:val="36"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/>
<w:rPr><w:b/><w:sz w:val="28"/></w:rPr></w:style>
</w:styles>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CaaMode, ExportParagraph, ExportUnit};

    fn png_base64() -> String {
        // PNG 1x1 trasparente.
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="
            .to_string()
    }

    #[test]
    fn docx_contains_document_and_media() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.docx");
        let doc = ExportDoc {
            title: "Prova".to_string(),
            paragraphs: vec![ExportParagraph {
                plain_text: "Luca mangia una pesca.".to_string(),
                style: "paragraph".to_string(),
                units: vec![ExportUnit {
                    surface: "pesca".to_string(),
                    caption: "pesca".to_string(),
                    symbol_id: Some("core.fruit.peach".to_string()),
                    svg: None,
                    image: None,
                    png_base64: Some(png_base64()),
                    approximate: false,
                }],
            }],
            include_symbols: true,
            include_captions: true,
            mode: CaaMode::Sopra,
            attributions: vec![],
            metadata: serde_json::json!({}),
        };
        let result = write(&path, &doc).unwrap();
        assert!(path.exists());
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);

        let file = File::open(&path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut names = Vec::new();
        for i in 0..archive.len() {
            names.push(archive.by_index(i).unwrap().name().to_string());
        }
        assert!(names.contains(&"word/document.xml".to_string()));
        assert!(names.contains(&"word/media/image1.png".to_string()));

        let mut xml = String::new();
        std::io::Read::read_to_string(&mut archive.by_name("word/document.xml").unwrap(), &mut xml)
            .unwrap();
        assert!(xml.contains("Luca mangia una pesca."));
        assert!(xml.contains("r:embed=\"rIdImg1\""));
    }

    #[test]
    fn caption_is_omitted_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.docx");
        let doc = ExportDoc {
            title: "Prova".to_string(),
            paragraphs: vec![ExportParagraph {
                plain_text: "Luca mangia una pesca.".to_string(),
                style: "paragraph".to_string(),
                units: vec![ExportUnit {
                    surface: "pesca".to_string(),
                    caption: "PESCA-REPERTORIO".to_string(),
                    symbol_id: Some("core.fruit.peach".to_string()),
                    svg: None,
                    image: None,
                    png_base64: Some(png_base64()),
                    approximate: false,
                }],
            }],
            include_symbols: true,
            include_captions: false,
            mode: CaaMode::Sopra,
            attributions: vec![],
            metadata: serde_json::json!({}),
        };
        write(&path, &doc).unwrap();
        let file = File::open(&path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut archive.by_name("word/document.xml").unwrap(), &mut xml)
            .unwrap();
        assert!(!xml.contains("PESCA-REPERTORIO"));
        assert!(xml.contains("Luca mangia una pesca."));
    }
}
