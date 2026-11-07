//! Esportazione DOCX con elementi nativi modificabili.
//!
//! - Le pagine a flusso diventano paragrafi Word (testo e immagini inline).
//! - Le pagine libere e a griglia usano caselle di testo e immagini ancorate
//!   alle coordinate della pagina: in Word restano selezionabili e modificabili
//!   singolarmente.
//! - Le celle delle griglie diventano caselle/immagini indipendenti.
//!
//! Resta una perdita dichiarata: azioni e collegamenti interni del player non
//! hanno un equivalente Word; il formato di riferimento resta `.caadoc`.

use std::io::Write;
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::export::universal_pdf::svg_to_png;
use crate::export::{emu, xml_escape};
use crate::model::ExportResult;
use crate::universal::{RenderedElement, UniversalDocument};

const TWIPS_PER_MM: f32 = 56.692_913;

/// Colore esadecimale valido per WordprocessingML, con default leggibile.
fn color_hex(value: &str) -> String {
    let candidate = value.trim().trim_start_matches('#');
    if candidate.len() == 6 && candidate.chars().all(|c| c.is_ascii_hexdigit()) {
        candidate.to_string()
    } else {
        "111111".to_string()
    }
}

struct Media {
    name: String,
    png: Vec<u8>,
}

struct Builder {
    body: String,
    media: Vec<Media>,
    next_id: usize,
}

impl Builder {
    fn new() -> Self {
        Self {
            body: String::new(),
            media: Vec::new(),
            next_id: 1,
        }
    }

    fn id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn image(&mut self, element_id: &str, svg: &str, w_mm: f32, h_mm: f32) -> Option<(String, usize)> {
        let png = svg_to_png(svg, w_mm, h_mm, 2.0).ok()?;
        let name = format!("el{}.png", self.media.len() + 1);
        let rid = format!("rIdImg{}", self.media.len() + 1);
        let id = self.id();
        self.media.push(Media {
            name: name.clone(),
            png,
        });
        let cx = emu(w_mm / 25.4);
        let cy = emu(h_mm / 25.4);
        let anchor = format!(
            "<w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\">\
<wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:docPr id=\"{id}\" name=\"{}\"/>\
<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:pic><pic:nvPicPr><pic:cNvPr id=\"{id}\" name=\"{name}\"/><pic:cNvPicPr/></pic:nvPicPr>\
<pic:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>\
<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm>\
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic>\
</wp:inline></w:drawing></w:r>",
            xml_escape(element_id)
        );
        Some((anchor, id))
    }

    fn anchored_text(
        &mut self,
        element_id: &str,
        text: &str,
        x_mm: f32,
        y_mm: f32,
        w_mm: f32,
        h_mm: f32,
        font_mm: f32,
        bold: bool,
        color: &str,
        fill: &str,
        border: &str,
        border_width: f32,
    ) -> String {
        let id = self.id();
        let cx = emu(w_mm / 25.4);
        let cy = emu(h_mm / 25.4);
        // `wp:posOffset` è in twips, non in EMU: `wp:extent` invece usa EMU.
        let ox = (x_mm * TWIPS_PER_MM).round() as i64;
        let oy = (y_mm * TWIPS_PER_MM).round() as i64;
        let size = ((font_mm * 5.67).round() as u32).clamp(10, 160);
        let fill_xml = if fill.trim().is_empty() || fill == "transparent" {
            "<a:noFill/>".to_string()
        } else {
            format!("<a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>", fill.trim_start_matches('#'))
        };
        let line_xml = if border_width > 0.0 && !border.trim().is_empty() {
            format!(
                "<a:ln w=\"{}\"><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill></a:ln>",
                (border_width * 12_700.0).round() as u32,
                border.trim_start_matches('#')
            )
        } else {
            "<a:ln><a:noFill/></a:ln>".to_string()
        };
        format!(
            "<w:r><w:drawing><wp:anchor distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\" simplePos=\"0\" \
relativeHeight=\"{id}\" behindDoc=\"0\" locked=\"0\" layoutInCell=\"1\" allowOverlap=\"1\">\
<wp:simplePos x=\"0\" y=\"0\"/><wp:positionH relativeFrom=\"page\"><wp:posOffset>{ox}</wp:posOffset></wp:positionH>\
<wp:positionV relativeFrom=\"page\"><wp:posOffset>{oy}</wp:posOffset></wp:positionV>\
<wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:effectExtent l=\"0\" t=\"0\" r=\"0\" b=\"0\"/><wp:wrapNone/>\
<wp:docPr id=\"{id}\" name=\"{}\"/><a:graphic><a:graphicData uri=\"http://schemas.microsoft.com/office/word/2010/wordprocessingShape\">\
<wps:wsp><wps:cNvSpPr txBox=\"1\"/><wps:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm>\
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>{fill_xml}{line_xml}</wps:spPr>\
<wps:txbx><w:txbxContent><w:p><w:pPr><w:jc w:val=\"center\"/></w:pPr><w:r><w:rPr><w:sz w:val=\"{size}\"/>{bold_xml}\
<w:color w:val=\"{color_text}\"/></w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p></w:txbxContent></wps:txbx>\
<wps:bodyPr rot=\"0\" wrap=\"square\" lIns=\"45720\" tIns=\"45720\" rIns=\"45720\" bIns=\"45720\"/></wps:wsp>\
</a:graphicData></a:graphic></wp:anchor></w:drawing></w:r>",
            xml_escape(element_id),
            xml_escape(text),
            color_text = color_hex(color),
            bold_xml = if bold { "<w:b/>" } else { "" }
        )
    }

    fn anchored_image(
        &mut self,
        element_id: &str,
        svg: &str,
        x_mm: f32,
        y_mm: f32,
        w_mm: f32,
        h_mm: f32,
    ) -> Option<String> {
        let png = svg_to_png(svg, w_mm, h_mm, 2.0).ok()?;
        let name = format!("el{}.png", self.media.len() + 1);
        let rid = format!("rIdImg{}", self.media.len() + 1);
        let id = self.id();
        self.media.push(Media {
            name: name.clone(),
            png,
        });
        let cx = emu(w_mm / 25.4);
        let cy = emu(h_mm / 25.4);
        // `wp:posOffset` è in twips, non in EMU: `wp:extent` invece usa EMU.
        let ox = (x_mm * TWIPS_PER_MM).round() as i64;
        let oy = (y_mm * TWIPS_PER_MM).round() as i64;
        Some(format!(
            "<w:r><w:drawing><wp:anchor distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\" simplePos=\"0\" \
relativeHeight=\"{id}\" behindDoc=\"0\" locked=\"0\" layoutInCell=\"1\" allowOverlap=\"1\">\
<wp:simplePos x=\"0\" y=\"0\"/><wp:positionH relativeFrom=\"page\"><wp:posOffset>{ox}</wp:posOffset></wp:positionH>\
<wp:positionV relativeFrom=\"page\"><wp:posOffset>{oy}</wp:posOffset></wp:positionV>\
<wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:effectExtent l=\"0\" t=\"0\" r=\"0\" b=\"0\"/><wp:wrapNone/>\
<wp:docPr id=\"{id}\" name=\"{}\"/><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:pic><pic:nvPicPr><pic:cNvPr id=\"{id}\" name=\"{name}\"/><pic:cNvPicPr/></pic:nvPicPr>\
<pic:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>\
<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm>\
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic>\
</wp:anchor></w:drawing></w:r>",
            xml_escape(element_id)
        ))
    }

    fn flow_paragraph(&mut self, element: &RenderedElement) {
        if element.cells.is_empty() {
            if let Some(svg) = &element.svg {
                if let Some((image, _)) = self.image(&element.id, svg, element.w_mm.max(18.0), element.h_mm.max(18.0)) {
                    self.body.push_str(&format!(
                        "<w:p><w:pPr><w:jc w:val=\"center\"/></w:pPr>{image}\
                         <w:r><w:t xml:space=\"preserve\"> {}</w:t></w:r></w:p>",
                        xml_escape(&element.text)
                    ));
                    return;
                }
            }
            let text = if element.text.trim().is_empty() {
                element.kind.clone()
            } else {
                element.text.clone()
            };
            self.body.push_str(&format!(
                "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                xml_escape(&text)
            ));
            return;
        }
        // Griglia dentro una pagina a flusso: una tabella Word reale.
        let columns = element.cells.iter().map(|cell| cell.column).max().unwrap_or(0) + 1;
        let rows = element.cells.iter().map(|cell| cell.row).max().unwrap_or(0) + 1;
        self.body.push_str("<w:tbl><w:tblPr><w:tblW w:w=\"0\" w:type=\"auto\"/><w:tblBorders>\
<w:top w:val=\"single\" w:sz=\"6\"/><w:left w:val=\"single\" w:sz=\"6\"/>\
<w:bottom w:val=\"single\" w:sz=\"6\"/><w:right w:val=\"single\" w:sz=\"6\"/>\
<w:insideH w:val=\"single\" w:sz=\"4\"/><w:insideV w:val=\"single\" w:sz=\"4\"/>\
</w:tblBorders></w:tblPr>");
        for row in 0..rows {
            self.body.push_str("<w:tr>");
            for column in 0..columns {
                self.body.push_str("<w:tc><w:tcPr><w:tcW w:w=\"2400\" w:type=\"dxa\"/></w:tcPr>");
                if let Some(cell) = element
                    .cells
                    .iter()
                    .find(|cell| cell.row == row && cell.column == column)
                {
                    self.body.push_str("<w:p>");
                    if let Some(svg) = &cell.svg {
                        if let Some((image, _)) = self.image(&cell.id, svg, 24.0, 24.0) {
                            self.body.push_str(&image);
                        }
                    }
                    self.body.push_str(&format!(
                        "<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                        xml_escape(&cell.text)
                    ));
                } else {
                    self.body.push_str("<w:p/>");
                }
                self.body.push_str("</w:tc>");
            }
            self.body.push_str("</w:tr>");
        }
        self.body.push_str("</w:tbl>");
    }

    fn absolute_element(&mut self, element: &RenderedElement) {
        if !element.cells.is_empty() {
            for cell in &element.cells {
                if cell.hidden {
                    continue;
                }
                let x = element.x_mm + cell.x_mm;
                let y = element.y_mm + cell.y_mm;
                let text_h = if cell.text.trim().is_empty() {
                    0.0
                } else {
                    (cell.h_mm * 0.3).max(5.0)
                };
                let image_h = (cell.h_mm - text_h).max(5.0);
                if let Some(svg) = &cell.svg {
                    if let Some(image) = self.anchored_image(&cell.id, svg, x, y, cell.w_mm, image_h) {
                        self.body.push_str(&image);
                    }
                }
                if !cell.text.trim().is_empty() {
                    let text_box = self.anchored_text(
                        &cell.id,
                        &cell.text,
                        x,
                        y + image_h,
                        cell.w_mm,
                        text_h,
                        cell.style.font_size.max(2.4),
                        cell.style.bold,
                        &cell.style.color,
                        &cell.style.background,
                        &cell.style.border_color,
                        cell.style.border_width,
                    );
                    self.body.push_str(&text_box);
                }
            }
            return;
        }
        if let Some(svg) = &element.svg {
            let text_h = if element.text.trim().is_empty() {
                0.0
            } else {
                (element.h_mm * 0.28).max(5.0)
            };
            let image_h = (element.h_mm - text_h).max(5.0);
            if let Some(image) = self.anchored_image(
                &element.id,
                svg,
                element.x_mm,
                element.y_mm,
                element.w_mm,
                image_h,
            ) {
                self.body.push_str(&image);
            }
            if !element.text.trim().is_empty() {
                let text_box = self.anchored_text(
                    &element.id,
                    &element.text,
                    element.x_mm,
                    element.y_mm + image_h,
                    element.w_mm,
                    text_h,
                    element.style.font_size.max(2.4),
                    element.style.bold,
                    &element.style.color,
                    "",
                    "",
                    0.0,
                );
                self.body.push_str(&text_box);
            }
            return;
        }
        if !element.text.trim().is_empty() {
            let text_box = self.anchored_text(
                &element.id,
                &element.text,
                element.x_mm,
                element.y_mm,
                element.w_mm,
                element.h_mm,
                element.style.font_size.max(2.4),
                element.style.bold,
                &element.style.color,
                &element.style.background,
                &element.style.border_color,
                element.style.border_width,
            );
            self.body.push_str(&text_box);
        }
    }
}

pub fn write(path: &Path, document: &UniversalDocument) -> Result<ExportResult, String> {
    let rendered = document.render()?;
    let mut builder = Builder::new();
    for (page_index, page) in rendered.pages.iter().enumerate() {
        builder.body.push_str("<w:p>");
        if page_index > 0 {
            builder.body.push_str("<w:r><w:br w:type=\"page\"/></w:r>");
        }
        builder.body.push_str("</w:p>");
        if page.layout == "flow" {
            for element in &page.elements {
                if element.visible {
                    builder.flow_paragraph(element);
                }
            }
        } else {
            builder.body.push_str("<w:p>");
            for element in &page.elements {
                if element.visible {
                    builder.absolute_element(element);
                }
            }
            builder.body.push_str("</w:p>");
        }
    }
    builder.body.push_str(
        "<w:p><w:r><w:rPr><w:sz w:val=\"16\"/><w:color w:val=\"555555\"/></w:rPr>\
         <w:t xml:space=\"preserve\">Esportato da CAA Writer. Le azioni del player non sono \
rappresentate in Word; usare .caadoc per la versione completa.</w:t></w:r></w:p>",
    );

    let (page_w_mm, page_h_mm) = document.page.dimensions()?;
    let page_w = (page_w_mm * TWIPS_PER_MM).round() as u32;
    let page_h = (page_h_mm * TWIPS_PER_MM).round() as u32;
    let document_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" \
xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\" \
xmlns:wps=\"http://schemas.microsoft.com/office/word/2010/wordprocessingShape\">\
<w:body>{}<w:sectPr><w:pgSz w:w=\"{page_w}\" w:h=\"{page_h}\"/>\
<w:pgMar w:top=\"0\" w:right=\"0\" w:bottom=\"0\" w:left=\"0\" w:header=\"0\" w:footer=\"0\" w:gutter=\"0\"/>\
</w:sectPr></w:body></w:document>",
        builder.body
    );

    let mut content_types = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Default Extension=\"png\" ContentType=\"image/png\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
</Types>",
    );
    let _ = &mut content_types;
    let mut rels = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );
    for (index, media) in builder.media.iter().enumerate() {
        rels.push_str(&format!(
            "<Relationship Id=\"rIdImg{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/{}\"/>",
            index + 1,
            media.name
        ));
    }
    rels.push_str("</Relationships>");
    let styles = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:rPr><w:sz w:val=\"20\"/></w:rPr></w:style>\
</w:styles>";

    let mut zip =
        ZipWriter::new(std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut put = |name: &str, body: Vec<u8>| -> Result<(), String> {
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        zip.write_all(&body).map_err(|e| e.to_string())
    };
    put("[Content_Types].xml", content_types.into_bytes())?;
    put(
        "_rels/.rels",
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/>\
</Relationships>"
            .as_bytes()
            .to_vec(),
    )?;
    put("word/document.xml", document_xml.into_bytes())?;
    put("word/styles.xml", styles.as_bytes().to_vec())?;
    put("word/_rels/document.xml.rels", rels.into_bytes())?;
    for media in &builder.media {
        put(&format!("word/media/{}", media.name), media.png.clone())?;
    }
    zip.finish().map_err(|e| format!("chiusura DOCX: {e}"))?;

    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
        warnings: vec![
            "DOCX: testo, immagini e griglie sono elementi Word modificabili; azioni e collegamenti del player non sono rappresentati. Usare .caadoc per la versione completa.".to_string(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::{build_document, UniversalElement, UniversalPage};

    #[test]
    fn docx_has_editable_text_and_media() {
        let mut document = build_document("docx", "DOCX", "document", "A4", "portrait");
        document.pages = vec![UniversalPage {
            id: "p1".to_string(),
            title: "Pagina".to_string(),
            layout: "absolute".to_string(),
            background: "#fff".to_string(),
            flow: Default::default(),
            grid: Default::default(),
            elements: vec![
                UniversalElement {
                    id: "t1".to_string(),
                    kind: "text".to_string(),
                    text: "Ciao DOCX".to_string(),
                    x_mm: 20.0,
                    y_mm: 20.0,
                    w_mm: 80.0,
                    h_mm: 20.0,
                    ..UniversalElement::default()
                },
                UniversalElement {
                    id: "s1".to_string(),
                    kind: "symbol".to_string(),
                    text: "casa".to_string(),
                    svg: Some("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"><rect width=\"10\" height=\"10\" fill=\"#2463eb\"/></svg>".to_string()),
                    x_mm: 20.0,
                    y_mm: 50.0,
                    w_mm: 30.0,
                    h_mm: 30.0,
                    ..UniversalElement::default()
                },
            ],
            is_home: true,
            notes: String::new(),
        }];
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.docx");
        write(&path, &document).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut xml = String::new();
        use std::io::Read;
        archive.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
        assert!(xml.contains("Ciao DOCX"));
        assert!(xml.contains("wps:txbx"));
        assert!(archive.by_name("word/media/el1.png").is_ok());
    }
}
