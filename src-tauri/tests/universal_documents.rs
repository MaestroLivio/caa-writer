//! Verifica strutturale della biblioteca universale generata dalla CLI.
//!
//! Non avvia la GUI: usa lo stesso motore documentale e gli stessi file
//! `.caadoc` che l'editor apre. Ogni modello viene caricato, validato,
//! modificato, salvato e riletto.

use std::path::PathBuf;

use caa_writer_lib::universal::{
    self, DocumentCommand, UniversalDocument,
};
use caa_writer_lib::universal_build;

fn resources() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources")
}

fn manifest() -> universal::LibraryManifest {
    universal::load_library_manifest(&resources())
}

#[test]
fn library_contains_at_least_500_templates_and_30_systems() {
    let manifest = manifest();
    let (templates, boards) = manifest.counts();
    assert!(templates >= 500, "template universali: {templates}");
    assert_eq!(boards, 30, "sistemi comunicativi: {boards}");
    assert_eq!(manifest.report.get("failed").and_then(|v| v.as_u64()), Some(0));
}

#[test]
fn every_library_document_loads_validates_and_renders() {
    let resources = resources();
    let manifest = manifest();
    assert!(manifest.entries.len() >= 330);
    for entry in &manifest.entries {
        let path = resources.join(&entry.path);
        let document = universal_build::load_universal(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", entry.id));
        universal::validate(&document)
            .unwrap_or_else(|error| panic!("{}: {error}", entry.id));
        let rendered = document
            .render()
            .unwrap_or_else(|error| panic!("{}: {error}", entry.id));
        assert_eq!(document.pages.len(), rendered.pages.len());
        assert!(document.pages.len() >= 1);
        if entry.source_type == "board" {
            assert_eq!(document.kind, "communication_board");
            assert!(document.pages.iter().any(|page| page.is_home));
        }
    }
}

#[test]
fn every_library_document_renders_without_overflow_warnings() {
    let resources = resources();
    let mut issues = Vec::new();
    for entry in manifest().entries {
        let document = universal_build::load_universal(&resources.join(&entry.path)).unwrap();
        let rendered = document.render().unwrap();
        if !rendered.warnings.is_empty() {
            issues.push((entry.id, rendered.warnings));
        }
    }
    assert!(
        issues.is_empty(),
        "documenti con avvisi di rendering: {:?}",
        issues.into_iter().take(10).collect::<Vec<_>>()
    );
}

#[test]
fn each_document_can_be_edited_saved_and_reopened_without_loss() {
    let resources = resources();
    let manifest = manifest();
    let temp = tempfile::tempdir().unwrap();
    for entry in &manifest.entries {
        let path = resources.join(&entry.path);
        let original = universal_build::load_universal(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", entry.id));
        let page = original.pages.first().expect("pagina");
        let target = page
            .elements
            .iter()
            .find(|element| !element.locked)
            .or_else(|| page.elements.first());
        let Some(target) = target else {
            continue;
        };
        let new_text = format!("{} · modificato", target.text);
        let commands = vec![
            DocumentCommand::SetText {
                page: page.id.clone(),
                id: target.id.clone(),
                text: new_text.clone(),
                spoken_text: None,
            },
            DocumentCommand::SetPosition {
                page: page.id.clone(),
                id: target.id.clone(),
                x_mm: 17.0,
                y_mm: 23.0,
            },
            DocumentCommand::SetSize {
                page: page.id.clone(),
                id: target.id.clone(),
                w_mm: 44.0,
                h_mm: 31.0,
            },
        ];
        let result = universal::apply_transaction(&original, &commands)
            .unwrap_or_else(|error| panic!("{}: {error}", entry.id));
        let output = temp.path().join(format!("{}.caadoc", entry.id));
        let assets = universal_build::assets_from_document(&result.document);
        universal_build::save_universal(&output, &result.document, &assets)
            .unwrap_or_else(|error| panic!("{}: {error}", entry.id));
        let reopened = universal_build::load_universal(&output)
            .unwrap_or_else(|error| panic!("{}: {error}", entry.id));
        assert_eq!(reopened.pages.len(), original.pages.len(), "{}", entry.id);
        let reopened_element = reopened
            .element(&page.id, &target.id)
            .unwrap_or_else(|| panic!("{}: elemento perso", entry.id));
        assert_eq!(reopened_element.text, new_text, "{}", entry.id);
        assert_eq!(reopened_element.x_mm, 17.0, "{}", entry.id);
        assert_eq!(reopened_element.w_mm, 44.0, "{}", entry.id);
        universal::validate(&reopened).unwrap();
    }
}

#[test]
fn communication_links_survive_edit_and_reopen() {
    let resources = resources();
    let manifest = manifest();
    let board = manifest
        .entries
        .iter()
        .find(|entry| entry.source_type == "board")
        .expect("almeno un sistema comunicativo");
    let document = universal_build::load_universal(&resources.join(&board.path)).unwrap();
    let page = document.pages.iter().find(|page| page.is_home).unwrap();
    let linked = page
        .elements
        .iter()
        .find(|element| element.action.as_ref().map(|a| a.action_type.as_str()) == Some("navigate"))
        .expect("un collegamento nella home");
    let target_page = linked.action.as_ref().unwrap().target_page.clone().unwrap();
    assert!(document.page(&target_page).is_some());

    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("linked.caadoc");
    let commands = vec![DocumentCommand::SetText {
        page: page.id.clone(),
        id: linked.id.clone(),
        text: "Categoria modificata".to_string(),
        spoken_text: Some("Apri la categoria".to_string()),
    }];
    let result = universal::apply_transaction(&document, &commands).unwrap();
    let assets = universal_build::assets_from_document(&result.document);
    universal_build::save_universal(&path, &result.document, &assets).unwrap();
    let reopened = universal_build::load_universal(&path).unwrap();
    let element = reopened.page(&page.id).unwrap().elements.iter()
        .find(|element| element.id == linked.id)
        .unwrap();
    assert_eq!(element.action.as_ref().unwrap().target_page.as_deref(), Some(target_page.as_str()));
    assert_eq!(element.text, "Categoria modificata");
}

#[test]
fn library_paths_are_resolved_inside_the_library() {
    let resources = std::fs::canonicalize(resources()).unwrap();
    let manifest = manifest();
    let entry = manifest.entries.first().expect("una voce");
    let path = universal::resolve_library_path(
        &manifest,
        &entry.id,
        Some(&entry.source_type),
        &resources,
    )
    .unwrap();
    assert!(path.starts_with(resources.join("library")));
    assert!(path.exists());

    let mut tampered = manifest.clone();
    tampered.entries[0].path = "library/../../etc/passwd".to_string();
    assert!(universal::resolve_library_path(
        &tampered,
        &tampered.entries[0].id,
        None,
        &resources
    )
    .is_err());
}

#[test]
fn library_documents_export_real_pdf_and_png() {
    let resources = resources();
    let manifest = manifest();
    let temp = tempfile::tempdir().unwrap();
    for source_type in ["template", "board"] {
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.source_type == source_type)
            .unwrap();
        let document = universal_build::load_universal(&resources.join(&entry.path)).unwrap();
        let pages = document.pages.len();
        let pdf = temp.path().join(format!("{source_type}.pdf"));
        caa_writer_lib::export::universal_pdf::export_pdf(&document, &pdf).unwrap();
        let bytes = std::fs::read(&pdf).unwrap();
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert!(bytes.ends_with(b"%%EOF\n"));
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains(&format!("/Count {pages}")));
        assert_eq!(text.matches("/Type /Page").count(), pages + 1);
        let png = temp.path().join(format!("{source_type}.png"));
        caa_writer_lib::export::universal_pdf::export_first_page_png(&document, &png).unwrap();
        let image = image::load_from_memory(&std::fs::read(&png).unwrap()).unwrap();
        assert!(image.width() > 300);
        assert!(image.height() > 300);
    }
}

#[test]
fn library_documents_export_docx_pptx_and_interactive_html() {
    let resources = resources();
    let manifest = manifest();
    let temp = tempfile::tempdir().unwrap();
    for source_type in ["template", "board"] {
        let entry = manifest.entries.iter().find(|e| e.source_type == source_type).unwrap();
        let document = universal_build::load_universal(&resources.join(&entry.path)).unwrap();
        let has_svg = document.pages.iter().flat_map(|page| page.elements.iter()).any(|element| {
            element.svg.is_some()
                || element
                    .grid
                    .as_ref()
                    .map(|grid| grid.cells.iter().any(|cell| cell.svg.is_some()))
                    .unwrap_or(false)
        });
        let docx = temp.path().join(format!("{source_type}.docx"));
        let result = caa_writer_lib::export::universal_docx::write(&docx, &document).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("modificabili")));
        let file = std::fs::File::open(&docx).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut docx_xml = String::new();
        {
            use std::io::Read;
            archive
                .by_name("word/document.xml")
                .unwrap()
                .read_to_string(&mut docx_xml)
                .unwrap();
        }
        assert!(docx_xml.contains("wps:txbx") || docx_xml.contains("<w:tbl>"));
        if has_svg {
            assert!(archive.file_names().any(|name| name.starts_with("word/media/")));
        }

        let pptx = temp.path().join(format!("{source_type}.pptx"));
        let result = caa_writer_lib::export::universal_pptx::write(&pptx, &document).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("modificabili")));
        let file = std::fs::File::open(&pptx).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert!(archive.by_name("ppt/presentation.xml").is_ok());
        let mut pptx_xml = String::new();
        {
            use std::io::Read;
            archive
                .by_name("ppt/slides/slide1.xml")
                .unwrap()
                .read_to_string(&mut pptx_xml)
                .unwrap();
        }
        assert!(pptx_xml.contains("<p:sp>") || pptx_xml.contains("<p:pic>"));
        if has_svg {
            assert!(archive.file_names().any(|name| name.starts_with("ppt/media/")));
        }

        let html = universal_build::export_html(&document).unwrap();
        assert!(html.contains("caa-bar"));
        assert!(html.contains("caaAppend"));
        assert!(html.contains("data-page"));
    }
}

#[test]
fn legacy_layout_is_migrated_without_losing_symbols() {
    let mut spec = caa_writer_lib::layout::LayoutSpecification::default();
    spec.title = "Legacy".to_string();
    spec.elements.push(caa_writer_lib::layout::ElementSpec {
        id: "old-1".to_string(),
        label: "Mela".to_string(),
        symbol_id: Some("core.fruit.apple".to_string()),
        svg: Some("<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_string()),
        ..Default::default()
    });
    let document: UniversalDocument = universal::from_layout(&spec);
    universal::validate(&document).unwrap();
    let element = &document.pages[0].elements[0];
    assert_eq!(element.symbol_id.as_deref(), Some("core.fruit.apple"));
}
