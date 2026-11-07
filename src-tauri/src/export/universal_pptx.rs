//! Esportazione PPTX del documento universale.
//!
//! Ogni pagina diventa una diapositiva con l'immagine renderizzata dal motore
//! canonico. Il file è modificabile come presentazione (diapositive, ordine,
//! sostituzione immagini), ma gli elementi non restano caselle di testo
//! separate: per la modifica fine resta `.caadoc`.

use std::io::Write;
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::export::universal_pdf::page_to_png;
use crate::model::ExportResult;
use crate::universal::UniversalDocument;

const EMU_PER_MM: f32 = 36_000.0;

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
<a:fmtScheme name=\"CAA\"><a:fillStyleLst><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:fillStyleLst>\
<a:lnStyleLst><a:ln w=\"6350\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln></a:lnStyleLst>\
<a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst>\
<a:bgFillStyleLst><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:bgFillStyleLst>\
</a:fmtScheme></a:themeElements></a:theme>";
    put("ppt/theme/theme1.xml", theme.as_bytes().to_vec())?;

    for (index, page) in rendered.pages.iter().enumerate() {
        let png = page_to_png(page, 1.6)?;
        let image = image::load_from_memory(&png).map_err(|e| format!("PNG slide: {e}"))?;
        let _ = (image.width(), image.height());
        let name = format!("slide{}.png", index + 1);
        put(&format!("ppt/media/{name}"), png)?;
        let slide = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\
<p:cSld><p:spTree>{}<p:pic><p:nvPicPr><p:cNvPr id=\"2\" name=\"{name}\"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>\
<p:blipFill><a:blip r:embed=\"rId1\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>\
<p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm>\
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic>\
</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>",
            empty_tree()
        );
        put(&format!("ppt/slides/slide{}.xml", index + 1), slide.into_bytes())?;
        put(
            &format!("ppt/slides/_rels/slide{}.xml.rels", index + 1),
            rels(&[(
                "rId1".to_string(),
                "officeDocument/2006/relationships/image".to_string(),
                format!("../media/{name}"),
            )])
            .into_bytes(),
        )?;
    }
    zip.finish().map_err(|e| format!("chiusura PPTX: {e}"))?;
    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
        warnings: vec![
            "PPTX: ogni pagina è una diapositiva-immagine; gli elementi non sono caselle modificabili separatamente. Usare .caadoc per la modifica completa.".to_string(),
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
        assert!(archive.by_name("ppt/slides/slide1.xml").is_ok());
        assert!(archive.by_name("ppt/media/slide1.png").is_ok());
        assert!(archive.by_name("ppt/presentation.xml").is_ok());
    }
}
