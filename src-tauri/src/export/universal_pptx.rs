//! Esportazione PPTX del documento universale.
//!
//! Ogni pagina diventa una diapositiva. Gli elementi del documento diventano
//! caselle di testo e immagini indipendenti, quindi restano selezionabili e
//! modificabili in PowerPoint; le celle delle griglie sono forme separate.
//! Azioni e collegamenti interni del player non hanno equivalente e restano
//! disponibili solo in `.caadoc`.

use std::io::Write;
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::export::universal_pdf::svg_to_png;
use crate::export::xml_escape;
use crate::model::ExportResult;
use crate::universal::UniversalDocument;

const EMU_PER_MM: f32 = 36_000.0;

/// Colore esadecimale valido per DrawingML, con un default leggibile.
fn color_hex(value: &str) -> String {
    let candidate = value.trim().trim_start_matches('#');
    if candidate.len() == 6 && candidate.chars().all(|c| c.is_ascii_hexdigit()) {
        candidate.to_string()
    } else {
        "111111".to_string()
    }
}

fn rels(entries: &[(String, String, String)]) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );
    for (id, kind, target) in entries {
        out.push_str(&format!(
            "<Relationship Id=\"{id}\" Type=\"http://schemas.openxmlformats.org/{kind}\" Target=\"{target}\"/>"
        ));
    }
    out.push_str("</Relationships>");
    out
}

fn empty_tree() -> &'static str {
    "<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>\
     <p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/>\
     <a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>"
}

fn text_shape(
    id: usize,
    name: &str,
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
    rotation: f32,
) -> String {
    let x = (x_mm * EMU_PER_MM).round() as i64;
    let y = (y_mm * EMU_PER_MM).round() as i64;
    let cx = ((w_mm.max(3.0)) * EMU_PER_MM).round() as i64;
    let cy = ((h_mm.max(3.0)) * EMU_PER_MM).round() as i64;
    let size = ((font_mm.max(2.0) * 2.8346 * 100.0).round() as u32).clamp(600, 9600);
    let rot = (rotation * 60_000.0).round() as i64;
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
        "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{name_text}\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm rot=\"{rot}\"><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>{fill_xml}{line_xml}</p:spPr><p:txBody><a:bodyPr wrap=\"square\" anchor=\"ctr\" lIns=\"45720\" tIns=\"45720\" rIns=\"45720\" bIns=\"45720\"/><a:lstStyle/><a:p><a:pPr algn=\"ctr\"/><a:r><a:rPr lang=\"it-IT\" sz=\"{size}\"{bold_attr}><a:solidFill><a:srgbClr val=\"{color_text}\"/></a:solidFill></a:rPr><a:t>{text_text}</a:t></a:r></a:p></p:txBody></p:sp>",
        name_text = xml_escape(name),
        text_text = xml_escape(text),
        color_text = color_hex(color),
        bold_attr = if bold { " b=\"1\"" } else { "" }
    )
}

fn picture_shape(
    id: usize,
    name: &str,
    rid: &str,
    x_mm: f32,
    y_mm: f32,
    w_mm: f32,
    h_mm: f32,
    rotation: f32,
) -> String {
    let x = (x_mm * EMU_PER_MM).round() as i64;
    let y = (y_mm * EMU_PER_MM).round() as i64;
    let cx = ((w_mm.max(3.0)) * EMU_PER_MM).round() as i64;
    let cy = ((h_mm.max(3.0)) * EMU_PER_MM).round() as i64;
    let rot = (rotation * 60_000.0).round() as i64;
    format!(
        "<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"{}\"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm rot=\"{rot}\"><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic>",
        xml_escape(name)
    )
}

pub fn write(path: &Path, document: &UniversalDocument) -> Result<ExportResult, String> {
    let rendered = document.render()?;
    if rendered.pages.is_empty() {
        return Err("documento senza pagine".to_string());
    }
    let (width_mm, height_mm) = document.page.dimensions()?;
    let cx = (width_mm * EMU_PER_MM).round() as i64;
    let cy = (height_mm * EMU_PER_MM).round() as i64;

    let mut zip =
        ZipWriter::new(std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut put = |name: &str, body: Vec<u8>| -> Result<(), String> {
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        zip.write_all(&body).map_err(|e| e.to_string())
    };

    let mut content_types = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Default Extension=\"png\" ContentType=\"image/png\"/>\
<Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/>\
<Override PartName=\"/ppt/slideMasters/slideMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml\"/>\
<Override PartName=\"/ppt/slideLayouts/slideLayout1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml\"/>\
<Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>",
    );
    for index in 0..rendered.pages.len() {
        content_types.push_str(&format!(
            "<Override PartName=\"/ppt/slides/slide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>",
            index + 1
        ));
    }
    content_types.push_str("</Types>");
    put("[Content_Types].xml", content_types.into_bytes())?;
    put(
        "_rels/.rels",
        rels(&[(
            "rId1".to_string(),
            "officeDocument/2006/relationships/officeDocument".to_string(),
            "ppt/presentation.xml".to_string(),
        )])
        .into_bytes(),
    )?;

    let mut presentation = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<p:presentation xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\
<p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst>\
<p:sldIdLst>"
    );
    for index in 0..rendered.pages.len() {
        presentation.push_str(&format!(
            "<p:sldId id=\"{}\" r:id=\"rId{}\"/>",
            256 + index,
            index + 2
        ));
    }
    presentation.push_str(&format!(
        "</p:sldIdLst><p:sldSz cx=\"{cx}\" cy=\"{cy}\" type=\"custom\"/>\
         <p:notesSz cx=\"6858000\" cy=\"9144000\"/></p:presentation>"
    ));
    put("ppt/presentation.xml", presentation.into_bytes())?;

    let mut presentation_rels: Vec<(String, String, String)> = vec![
        (
            "rId1".to_string(),
            "officeDocument/2006/relationships/slideMaster".to_string(),
            "slideMasters/slideMaster1.xml".to_string(),
        ),
        (
            format!("rId{}", rendered.pages.len() + 2),
            "officeDocument/2006/relationships/theme".to_string(),
            "theme/theme1.xml".to_string(),
        ),
    ];
    for index in 0..rendered.pages.len() {
        presentation_rels.push((
            format!("rId{}", index + 2),
            "officeDocument/2006/relationships/slide".to_string(),
            format!("slides/slide{}.xml", index + 1),
        ));
    }
    put(
        "ppt/_rels/presentation.xml.rels",
        rels(&presentation_rels).into_bytes(),
    )?;

    let master = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<p:sldMaster xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\
<p:cSld><p:spTree>{}</p:spTree></p:cSld>\
<p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" \
accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>\
<p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rId1\"/></p:sldLayoutIdLst></p:sldMaster>",
        empty_tree()
    );
    put("ppt/slideMasters/slideMaster1.xml", master.into_bytes())?;
    put(
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        rels(&[
            (
                "rId1".to_string(),
                "officeDocument/2006/relationships/slideLayout".to_string(),
                "../slideLayouts/slideLayout1.xml".to_string(),
            ),
            (
                "rId2".to_string(),
                "officeDocument/2006/relationships/theme".to_string(),
                "../theme/theme1.xml".to_string(),
            ),
        ])
        .into_bytes(),
    )?;

    let layout = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<p:sldLayout xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" type=\"blank\" preserve=\"1\">\
<p:cSld name=\"Blank\"><p:spTree>{}</p:spTree></p:cSld>\
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>",
        empty_tree()
    );
    put("ppt/slideLayouts/slideLayout1.xml", layout.into_bytes())?;
    put(
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        rels(&[(
            "rId1".to_string(),
            "officeDocument/2006/relationships/slideMaster".to_string(),
            "../slideMasters/slideMaster1.xml".to_string(),
        )])
        .into_bytes(),
    )?;

    let theme = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"CAA Writer\">\
<a:themeElements><a:clrScheme name=\"CAA\"><a:dk1><a:srgbClr val=\"000000\"/></a:dk1>\
<a:lt1><a:srgbClr val=\"FFFFFF\"/></a:lt1><a:dk2><a:srgbClr val=\"222222\"/></a:dk2>\
<a:lt2><a:srgbClr val=\"EEEEEE\"/></a:lt2><a:accent1><a:srgbClr val=\"2463EB\"/></a:accent1>\
<a:accent2><a:srgbClr val=\"E0663A\"/></a:accent2><a:accent3><a:srgbClr val=\"2F9E6F\"/></a:accent3>\
<a:accent4><a:srgbClr val=\"8A5BD6\"/></a:accent4><a:accent5><a:srgbClr val=\"C9A227\"/></a:accent5>\
<a:accent6><a:srgbClr val=\"667085\"/></a:accent6><a:hlink><a:srgbClr val=\"2463EB\"/></a:hlink>\
<a:folHlink><a:srgbClr val=\"8A5BD6\"/></a:folHlink></a:clrScheme>\
<a:fontScheme name=\"CAA\"><a:majorFont><a:latin typeface=\"Arial\"/><a:ea typeface=\"\"/>\
<a:cs typeface=\"\"/></a:majorFont><a:minorFont><a:latin typeface=\"Arial\"/><a:ea typeface=\"\"/>\
<a:cs typeface=\"\"/></a:minorFont></a:fontScheme>\
<a:fmtScheme name=\"CAA\"><a:fillStyleLst>\
<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
<a:solidFill><a:schemeClr val=\"phClr\"><a:lumMod val=\"50000\"/></a:schemeClr></a:solidFill>\
<a:solidFill><a:schemeClr val=\"phClr\"><a:lumMod val=\"80000\"/></a:schemeClr></a:solidFill>\
</a:fillStyleLst>\
<a:lnStyleLst><a:ln w=\"6350\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>\
<a:ln w=\"12700\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>\
<a:ln w=\"19050\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln></a:lnStyleLst>\
<a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle>\
<a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst>\
<a:bgFillStyleLst><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:bgFillStyleLst>\
</a:fmtScheme></a:themeElements></a:theme></a:themeElements></a:theme>";
    put("ppt/theme/theme1.xml", theme.as_bytes().to_vec())?;

    let mut slide_counter = 0usize;
    for (_index, page) in rendered.pages.iter().enumerate() {
        slide_counter += 1;
        let mut shapes = String::new();
        let mut images: Vec<(String, Vec<u8>)> = Vec::new();
        let mut next_id = 10usize;
        let add_image = |images: &mut Vec<(String, Vec<u8>)>, svg: &str, w: f32, h: f32| -> Option<String> {
            let png = svg_to_png(svg, w, h, 2.0).ok()?;
            let name = format!("slide{}_img{}.png", slide_counter, images.len() + 1);
            let rid = format!("rIdImg{}", images.len() + 1);
            images.push((name, png));
            Some(rid)
        };
        for element in &page.elements {
            if !element.visible {
                continue;
            }
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
                    if let Some(svg) = cell.media() {
                        if let Some(rid) = add_image(&mut images, svg, cell.w_mm, image_h) {
                            shapes.push_str(&picture_shape(
                                next_id,
                                &format!("{} immagine", cell.id),
                                &rid,
                                x,
                                y,
                                cell.w_mm,
                                image_h,
                                0.0,
                            ));
                            next_id += 1;
                        }
                    }
                    if !cell.text.trim().is_empty() {
                        shapes.push_str(&text_shape(
                            next_id,
                            &format!("{} testo", cell.id),
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
                            0.0,
                        ));
                        next_id += 1;
                    }
                }
                continue;
            }
            if let Some(svg) = element.media() {
                let text_h = if element.text.trim().is_empty() {
                    0.0
                } else {
                    (element.h_mm * 0.28).max(5.0)
                };
                let image_h = (element.h_mm - text_h).max(5.0);
                if let Some(rid) = add_image(&mut images, svg, element.w_mm, image_h) {
                    shapes.push_str(&picture_shape(
                        next_id,
                        &format!("{} immagine", element.id),
                        &rid,
                        element.x_mm,
                        element.y_mm,
                        element.w_mm,
                        image_h,
                        element.rotation,
                    ));
                    next_id += 1;
                }
                if !element.text.trim().is_empty() {
                    shapes.push_str(&text_shape(
                        next_id,
                        &format!("{} testo", element.id),
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
                        element.rotation,
                    ));
                    next_id += 1;
                }
                continue;
            }
            if !element.text.trim().is_empty() {
                shapes.push_str(&text_shape(
                    next_id,
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
                    element.rotation,
                ));
                next_id += 1;
            }
        }
        let background = if page.background.trim().is_empty()
            || page.background == "transparent"
        {
            String::new()
        } else {
            format!(
                "<p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>",
                color_hex(&page.background)
            )
        };
        let slide = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\
<p:cSld>{background}<p:spTree>{}{}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>",
            empty_tree(),
            shapes
        );
        put(&format!("ppt/slides/slide{}.xml", slide_counter), slide.into_bytes())?;
        let mut slide_rels: Vec<(String, String, String)> = Vec::new();
        for (index, (name, png)) in images.iter().enumerate() {
            put(&format!("ppt/media/{name}"), png.clone())?;
            slide_rels.push((
                format!("rIdImg{}", index + 1),
                "officeDocument/2006/relationships/image".to_string(),
                format!("../media/{name}"),
            ));
        }
        put(
            &format!("ppt/slides/_rels/slide{}.xml.rels", slide_counter),
            rels(&slide_rels).into_bytes(),
        )?;
    }
    zip.finish().map_err(|e| format!("chiusura PPTX: {e}"))?;
    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
        warnings: vec![
            "PPTX: testo e immagini sono forme modificabili separate; azioni e collegamenti del player non sono rappresentati. Usare .caadoc per la versione completa.".to_string(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::{build_document, UniversalElement, UniversalPage};

    #[test]
    fn pptx_contains_slides_and_media() {
        let mut document = build_document("pptx", "PPTX", "presentation", "16:9", "landscape");
        document.pages = vec![UniversalPage {
            id: "p1".to_string(),
            title: "Slide".to_string(),
            layout: "absolute".to_string(),
            background: "#fff".to_string(),
            flow: Default::default(),
            grid: Default::default(),
            elements: vec![UniversalElement {
                id: "t1".to_string(),
                kind: "text".to_string(),
                text: "Ciao".to_string(),
                x_mm: 10.0,
                y_mm: 10.0,
                w_mm: 100.0,
                h_mm: 20.0,
                ..UniversalElement::default()
            }],
            is_home: true,
            notes: String::new(),
        }];
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.pptx");
        write(&path, &document).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert!(archive.by_name("ppt/presentation.xml").is_ok());
        let mut xml = String::new();
        use std::io::Read;
        archive.by_name("ppt/slides/slide1.xml").unwrap().read_to_string(&mut xml).unwrap();
        assert!(xml.contains("p:sp"));
        assert!(xml.contains("Ciao"));
    }
}
