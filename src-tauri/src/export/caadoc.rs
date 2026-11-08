//! Contenitore nativo `.caadoc`.
//!
//! È uno ZIP con:
//! - `manifest.json`: formato, versione, data, applicazione;
//! - `document.json`: contenuto strutturato (testo + modelli dell'editor);
//! - `preferences.json`: scelte manuali legate al documento;
//! - `licenses.json`: attribuzioni dei simboli usati;
//! - `assets/symbols/<id>.svg`: pittogrammi vettoriali incorporati.
//!
//! Regola: il documento non dipende da URL esterni. Il salvataggio è atomico
//! (file temporaneo + rename) per non corrompere il documento in caso di
//! chiusura imprevista.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::model::{
    AttributionEntry, DocumentAsset, DocumentPayload, DocumentPreference, ExportResult,
    LoadedDocument,
};
use crate::util::now_iso;

pub const FORMAT_NAME: &str = "caadoc";
/// v3: aggiunge il documento universale accanto al testo e al layout legacy.
/// I documenti v1/v2 continuano a caricarsi e vengono migrati all'apertura.
pub const FORMAT_VERSION: u32 = 3;

pub fn save(
    path: &Path,
    payload: &DocumentPayload,
    attributions: &[AttributionEntry],
) -> Result<ExportResult, String> {
    let tmp: PathBuf = path.with_extension("caadoc.tmp");
    let mut warnings: Vec<String> = Vec::new();

    {
        let file = File::create(&tmp).map_err(|e| format!("impossibile creare il documento: {e}"))?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        let manifest = json!({
            "format": FORMAT_NAME,
            "version": FORMAT_VERSION,
            "created_at": now_iso(),
            "app": { "name": "CAA Writer", "version": env!("CARGO_PKG_VERSION") },
            "title": payload.title,
            "plain_text_chars": payload.plain_text.chars().count(),
            "assets": payload.assets.len(),
            "preferences": payload.preferences.len(),
            "license": "CC BY-NC-SA 4.0 (contenuti originali) — i pittogrammi di terzi conservano la licenza d'origine",
        });
        write_entry(&mut zip, "manifest.json", &manifest.to_string(), options, &mut warnings)?;

        let document = json!({
            "title": payload.title,
            "content": payload.content,
            "plain_text": payload.plain_text,
            "settings": payload.settings,
            "layout": payload.layout,
            "universal": payload.universal,
            "saved_at": now_iso(),
        });
        write_entry(&mut zip, "document.json", &document.to_string(), options, &mut warnings)?;
        write_entry(
            &mut zip,
            "preferences.json",
            &serde_json::to_string(&payload.preferences).map_err(|e| e.to_string())?,
            options,
            &mut warnings,
        )?;
        write_entry(
            &mut zip,
            "licenses.json",
            &serde_json::to_string(attributions).map_err(|e| e.to_string())?,
            options,
            &mut warnings,
        )?;
        // Testo semplice: garantisce il recupero del contenuto anche senza
        // l'applicazione (requisito: nessuna perdita di informazione).
        write_entry(&mut zip, "plain.txt", &payload.plain_text, options, &mut warnings)?;

        for asset in &payload.assets {
            match &asset.svg {
                Some(svg) => {
                    let name = format!("assets/symbols/{}.svg", sanitize_id(&asset.symbol_id));
                    write_entry(&mut zip, &name, svg, options, &mut warnings)?;
                }
                None => {
                    if let Some(png) = &asset.png_base64 {
                        let name = format!("assets/symbols/{}.png.b64", sanitize_id(&asset.symbol_id));
                        write_entry(&mut zip, &name, png, options, &mut warnings)?;
                    } else {
                        warnings.push(format!(
                            "simbolo {} senza dati incorporati",
                            asset.symbol_id
                        ));
                    }
                }
            }
        }

        zip.finish().map_err(|e| format!("chiusura archivio: {e}"))?;
    }

    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("impossibile completare il salvataggio: {e}")
    })?;

    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
        warnings,
    })
}

pub fn load(path: &Path) -> Result<LoadedDocument, String> {
    let file = File::open(path).map_err(|e| format!("documento non leggibile: {e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("archivio non valido: {e}"))?;
    let mut warnings: Vec<String> = Vec::new();

    let mut manifest_raw = String::new();
    archive
        .by_name("manifest.json")
        .map_err(|_| "manca manifest.json: non è un documento CAA Writer".to_string())?
        .read_to_string(&mut manifest_raw)
        .map_err(|e| e.to_string())?;
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_raw).map_err(|e| format!("manifest non valido: {e}"))?;

    if manifest.get("format").and_then(|v| v.as_str()) != Some(FORMAT_NAME) {
        return Err("il file non è un documento CAA Writer".to_string());
    }
    let version = manifest
        .get("version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if version > FORMAT_VERSION {
        warnings.push(format!(
            "documento creato con una versione più recente ({version}): alcune funzioni potrebbero non essere supportate"
        ));
    }

    let mut document_raw = String::new();
    archive
        .by_name("document.json")
        .map_err(|_| "manca document.json".to_string())?
        .read_to_string(&mut document_raw)
        .map_err(|e| e.to_string())?;
    let document: serde_json::Value =
        serde_json::from_str(&document_raw).map_err(|e| format!("documento non valido: {e}"))?;

    let title = document
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Documento senza titolo")
        .to_string();
    let content = document
        .get("content")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let plain_text = document
        .get("plain_text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let settings = document
        .get("settings")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let layout = document.get("layout").cloned().filter(|v| !v.is_null());
    let universal = document.get("universal").cloned().filter(|v| !v.is_null());

    let preferences: Vec<DocumentPreference> = read_json_entry(&mut archive, "preferences.json")
        .unwrap_or_else(|| {
            warnings.push("preferenze del documento assenti: uso le preferenze globali".to_string());
            Vec::new()
        });

    // Asset incorporati.
    let mut assets: Vec<DocumentAsset> = Vec::new();
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    for name in names {
        if let Some(rest) = name.strip_prefix("assets/symbols/") {
            if let Some(symbol_id) = rest.strip_suffix(".svg") {
                let mut content = String::new();
                if let Ok(mut entry) = archive.by_name(&name) {
                    if entry.read_to_string(&mut content).is_ok() {
                        assets.push(DocumentAsset {
                            symbol_id: symbol_id.to_string(),
                            svg: Some(content),
                            png_base64: None,
                        });
                    }
                }
            } else if let Some(symbol_id) = rest.strip_suffix(".png.b64") {
                let mut content = String::new();
                if let Ok(mut entry) = archive.by_name(&name) {
                    if entry.read_to_string(&mut content).is_ok() {
                        assets.push(DocumentAsset {
                            symbol_id: symbol_id.to_string(),
                            svg: None,
                            png_base64: Some(content),
                        });
                    }
                }
            }
        }
    }

    Ok(LoadedDocument {
        title,
        content,
        plain_text,
        settings,
        assets,
        preferences,
        format_version: version,
        warnings,
        path: path.to_string_lossy().to_string(),
        layout,
        universal,
    })
}

fn read_json_entry<T: serde::de::DeserializeOwned>(
    archive: &mut ZipArchive<File>,
    name: &str,
) -> Option<T> {
    let mut raw = String::new();
    archive.by_name(name).ok()?.read_to_string(&mut raw).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_entry(
    zip: &mut ZipWriter<File>,
    name: &str,
    content: &str,
    options: SimpleFileOptions,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    if let Err(e) = zip.start_file(name, options) {
        warnings.push(format!("{name}: {e}"));
        return Ok(());
    }
    zip.write_all(content.as_bytes())
        .map_err(|e| format!("scrittura {name}: {e}"))?;
    Ok(())
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DocumentPayload;

    fn payload() -> DocumentPayload {
        DocumentPayload {
            title: "La mia giornata".to_string(),
            content: serde_json::json!({"type": "doc", "content": []}),
            plain_text: "Il bambino mangia una pesca.".to_string(),
            settings: serde_json::json!({"mode": "sopra"}),
            assets: vec![DocumentAsset {
                symbol_id: "core.fruit.peach".to_string(),
                svg: Some("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\"></svg>".to_string()),
                png_base64: None,
            }],
            universal: Some(serde_json::json!({
                "schema_version": 1,
                "id": "doc-test",
                "title": "La mia giornata",
                "kind": "mixed",
                "pages": []
            })),
            preferences: vec![DocumentPreference {
                unit_key: "p1:4".to_string(),
                symbol_id: Some("core.fruit.peach".to_string()),
                concept_id: Some("fruit.peach".to_string()),
                locked: true,
                hidden: false,
                modifiers: Default::default(),
            }],
            layout: Some(serde_json::json!({
                "schemaVersion": 1,
                "documentType": "visual_schedule",
                "elements": []
            })),
        }
    }

    #[test]
    fn roundtrip_preserves_text_assets_and_preferences() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.caadoc");
        save(&path, &payload(), &[]).unwrap();
        assert!(path.exists());
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.title, "La mia giornata");
        assert_eq!(loaded.plain_text, "Il bambino mangia una pesca.");
        assert_eq!(loaded.assets.len(), 1);
        assert_eq!(loaded.preferences.len(), 1);
        assert_eq!(loaded.format_version, FORMAT_VERSION);
        assert_eq!(
            loaded.layout.as_ref().unwrap()["documentType"],
            "visual_schedule"
        );
        assert_eq!(loaded.universal.as_ref().unwrap()["id"], "doc-test");
    }

    #[test]
    fn png_assets_are_saved_and_loaded_without_svg_wrapper() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("png.caadoc");
        let mut data = payload();
        data.assets = vec![DocumentAsset {
            symbol_id: "arasaac.picto.2359".to_string(),
            svg: None,
            png_base64: Some("iVBORw0KGgo=".to_string()),
        }];
        save(&path, &data, &[]).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.assets.len(), 1);
        assert!(loaded.assets[0].svg.is_none());
        assert_eq!(
            loaded.assets[0].png_base64.as_deref(),
            Some("iVBORw0KGgo=")
        );
    }

    #[test]
    fn free_board_and_communication_actions_survive_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("free.caadoc");
        let mut data = payload();
        data.layout = Some(serde_json::json!({
            "schema_version": 1,
            "document_type": "free_board",
            "title": "Progetto libero",
            "layout": { "kind": "grid", "columns": 3, "rows": 2, "gap_mm": 3 },
            "elements": [{
                "id": "c1",
                "kind": "symbol",
                "label": "Parla",
                "spoken_text": "Voglio parlare",
                "action": { "type": "speak", "target_page": null },
                "page": 0, "row": 0, "column": 0
            }]
        }));
        save(&path, &data, &[]).unwrap();
        let loaded = load(&path).unwrap();
        let layout = loaded.layout.unwrap();
        assert_eq!(layout["document_type"], "free_board");
        assert_eq!(layout["elements"][0]["action"]["type"], "speak");
        assert_eq!(layout["elements"][0]["spoken_text"], "Voglio parlare");
    }

    #[test]
    fn v1_document_without_layout_still_loads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.caadoc");
        let file = File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file("manifest.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(format!("{{\"format\":\"{FORMAT_NAME}\",\"version\":1}}").as_bytes())
            .unwrap();
        zip.start_file("document.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            br#"{"title":"Vecchio","content":{"type":"doc"},"plain_text":"Ciao","settings":{}}"#,
        )
        .unwrap();
        zip.finish().unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.title, "Vecchio");
        assert_eq!(loaded.format_version, 1);
        assert!(loaded.layout.is_none());
        assert!(loaded.warnings.iter().any(|w| w.contains("preferenze")));
    }

    #[test]
    fn atomic_save_leaves_no_tmp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("atomic.caadoc");
        save(&path, &payload(), &[]).unwrap();
        assert!(!path.with_extension("caadoc.tmp").exists());
    }

    #[test]
    fn rejects_non_caadoc_zip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("other.zip");
        let file = File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file("hello.txt", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"hello").unwrap();
        zip.finish().unwrap();
        assert!(load(&path).is_err());
    }
}
