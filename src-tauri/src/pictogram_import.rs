//! Importazione di pittogrammi personali da file e directory.
//!
//! Il file originale viene copiato nella cartella dati dell'applicazione; per
//! il rendering nel documento viene creato un asset SVG autonomo (vettoriale
//! per gli SVG, con anteprima PNG incorporata per le immagini raster). Questo
//! permette di riusare l'indice simboli, la ricerca FTS e il motore di
//! simbolizzazione esistenti senza introdurre un secondo modello dati.

use std::collections::HashSet;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use base64::Engine as _;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::database::Database;
use crate::util::now_iso;

const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SVG_BYTES: usize = 1024 * 1024;
const MAX_FILES: usize = 5000;
const PREVIEW_PIXELS: u32 = 96;
const STORED_PIXELS: u32 = 512;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "svg", "webp", "gif", "bmp", "tif", "tiff", "avif", "heic", "heif", "ico",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedFormat {
    Png,
    Jpeg,
    Gif,
    Webp,
    Bmp,
    Tiff,
    Ico,
    Avif,
    Heic,
    Svg,
    Unknown,
}

impl DetectedFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Gif => "gif",
            Self::Webp => "webp",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::Ico => "ico",
            Self::Avif => "avif",
            Self::Heic => "heic",
            Self::Svg => "svg",
            Self::Unknown => "unknown",
        }
    }

    /// Formati che il decoder incluso nell'applicazione sa aprire davvero.
    pub fn decoder_available(self) -> bool {
        match self {
            Self::Png
            | Self::Jpeg
            | Self::Gif
            | Self::Webp
            | Self::Bmp
            | Self::Tiff
            | Self::Ico
            | Self::Avif
            | Self::Svg => true,
            // Il decoder HEIC non è incluso: dichiararlo evita di mostrare un
            // formato "supportato" che poi fallisce.
            Self::Heic | Self::Unknown => false,
        }
    }
}

/// Riconosce il formato dai byte, non dall'estensione: un PNG rinominato
/// `.jpg` non deve ingannare l'importazione.
pub fn detect_format(bytes: &[u8]) -> DetectedFormat {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return DetectedFormat::Png;
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return DetectedFormat::Jpeg;
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return DetectedFormat::Gif;
    }
    if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return DetectedFormat::Webp;
    }
    if bytes.starts_with(b"BM") {
        return DetectedFormat::Bmp;
    }
    if bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00]) || bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]) {
        return DetectedFormat::Tiff;
    }
    if bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
        return DetectedFormat::Ico;
    }
    if bytes.len() > 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"avif" || brand == b"avis" {
            return DetectedFormat::Avif;
        }
        if matches!(
            brand,
            b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1"
        ) {
            return DetectedFormat::Heic;
        }
    }
    // L'SVG è testo: si accetta un BOM e spazi iniziali, ma deve contenere un
    // elemento <svg>.
    if let Ok(text) = std::str::from_utf8(&bytes[..bytes.len().min(4096)]) {
        let trimmed = text.trim_start_matches('\u{feff}').trim_start();
        if trimmed.starts_with('<') && trimmed.to_ascii_lowercase().contains("<svg") {
            return DetectedFormat::Svg;
        }
    }
    DetectedFormat::Unknown
}

/// Propone un lemma dal nome del file: rimuove estensione, separatori
/// tecnici e suffissi numerici puramente tecnici, senza cancellare numeri
/// interni che possono avere significato («3 desideri»).
pub fn lemma_from_filename(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name);
    let mut cleaned = stem.replace(['_', '-', '.'], " ");
    // «mangiare 01», «gatto (2)», «cane copia»: suffissi tecnici frequenti
    // nelle cartelle di materiali. Non si toccano numeri all'inizio o in mezzo.
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    let mut keep: Vec<&str> = tokens.clone();
    while let Some(last) = keep.last() {
        let lower = last.to_ascii_lowercase();
        let numeric_tail = lower.chars().all(|c| c.is_ascii_digit())
            && tokens.len() > 1
            && keep.len() > 1;
        let copy_tail = matches!(lower.as_str(), "copia" | "copy" | "finale" | "final");
        if numeric_tail || copy_tail {
            keep.pop();
        } else {
            break;
        }
    }
    cleaned = keep.join(" ");
    cleaned = cleaned
        .trim_matches(|c: char| !c.is_alphanumeric() && c != ' ' && c != '\'')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if cleaned.is_empty() {
        stem.to_string()
    } else {
        // La prima lettera maiuscola rende leggibile il campo modificabile,
        // ma il confronto semantico usa comunque la forma normalizzata.
        let mut chars = cleaned.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => cleaned,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewEntry {
    pub path: String,
    pub relative_path: String,
    pub original_name: String,
    pub suggested_lemma: String,
    pub format: String,
    pub supported: bool,
    pub reason: Option<String>,
    pub duplicate: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub checksum: Option<String>,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportPreview {
    pub suggested_collection: String,
    pub root: Option<String>,
    pub entries: Vec<ImportPreviewEntry>,
    pub total: usize,
    pub supported: usize,
    pub duplicates: usize,
    pub truncated: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportItem {
    pub path: String,
    pub lemma: String,
    #[serde(default)]
    pub collection: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub synonyms: Vec<String>,
    #[serde(default)]
    pub relative_path: Option<String>,
    #[serde(default)]
    pub original_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ImportLicense {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub commercial_ok: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportOutcome {
    pub path: String,
    pub symbol_id: Option<String>,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: usize,
    pub failed: usize,
    pub items: Vec<ImportOutcome>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub symbols: i64,
    pub updated_at: String,
}

pub fn slugify(value: &str) -> String {
    let mut out = String::new();
    for c in value.to_lowercase().chars() {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn checksum(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    crate::asset_manager::hex(&digest)
}

fn read_limited(path: &Path) -> Result<Vec<u8>, String> {
    let meta = fs::metadata(path).map_err(|e| format!("file non leggibile: {e}"))?;
    if meta.len() > MAX_IMAGE_BYTES {
        return Err(format!(
            "file troppo grande: {:.1} MB (limite {} MB)",
            meta.len() as f64 / 1_048_576.0,
            MAX_IMAGE_BYTES / 1_048_576
        ));
    }
    let mut file = fs::File::open(path).map_err(|e| format!("apertura: {e}"))?;
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    file.read_to_end(&mut bytes)
        .map_err(|e| format!("lettura: {e}"))?;
    Ok(bytes)
}

fn svg_dimensions(view_box: Option<&str>) -> (Option<u32>, Option<u32>) {
    let Some(view_box) = view_box else {
        return (None, None);
    };
    let parts: Vec<f32> = view_box
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|part| part.parse::<f32>().ok())
        .collect();
    if parts.len() == 4 {
        (Some(parts[2].round() as u32), Some(parts[3].round() as u32))
    } else {
        (None, None)
    }
}

fn raster_asset(img: &image::DynamicImage) -> Result<(String, u32, u32), String> {
    let (width, height) = (img.width(), img.height());
    let thumb = img.thumbnail(STORED_PIXELS, STORED_PIXELS);
    let mut png = Vec::new();
    thumb
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| format!("codifica anteprima: {e}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&png);
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\"><image href=\"data:image/png;base64,{encoded}\" x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" preserveAspectRatio=\"xMidYMid meet\"/></svg>"
    );
    let sanitized = crate::asset_manager::svg::sanitize(&svg, 4 * 1024 * 1024)?;
    Ok((sanitized.svg, width, height))
}

fn preview_for(
    kind: DetectedFormat,
    bytes: &[u8],
) -> Option<(String, Option<u32>, Option<u32>)> {
    match kind {
        DetectedFormat::Svg => {
            let text = std::str::from_utf8(bytes).ok()?;
            let sanitized = crate::asset_manager::svg::sanitize(text, MAX_SVG_BYTES).ok()?;
            let (w, h) = svg_dimensions(sanitized.view_box.as_deref());
            let encoded = base64::engine::general_purpose::STANDARD.encode(sanitized.svg.as_bytes());
            Some((format!("data:image/svg+xml;base64,{encoded}"), w, h))
        }
        DetectedFormat::Heic | DetectedFormat::Unknown => None,
        other if other.decoder_available() => {
            let img = image::load_from_memory(bytes).ok()?;
            let (w, h) = (img.width(), img.height());
            let thumb = img.thumbnail(PREVIEW_PIXELS, PREVIEW_PIXELS);
            let mut png = Vec::new();
            thumb
                .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
                .ok()?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&png);
            Some((format!("data:image/png;base64,{encoded}"), Some(w), Some(h)))
        }
        _ => None,
    }
}

fn collect_paths(
    paths: &[String],
    warnings: &mut Vec<String>,
) -> (Vec<PathBuf>, Option<PathBuf>, bool) {
    let mut files = Vec::new();
    let mut root: Option<PathBuf> = None;
    let mut truncated = false;

    for raw in paths {
        let path = PathBuf::from(raw);
        if path.is_dir() {
            if root.is_none() {
                root = Some(path.clone());
            }
            for entry in WalkDir::new(&path).follow_links(false).into_iter() {
                if files.len() >= MAX_FILES {
                    truncated = true;
                    break;
                }
                let Ok(entry) = entry else { continue };
                if entry.file_type().is_symlink() {
                    warnings.push(format!(
                        "Collegamento simbolico ignorato: {}",
                        entry.path().display()
                    ));
                    continue;
                }
                if !entry.file_type().is_file() {
                    continue;
                }
                let ext = entry
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                    continue;
                }
                files.push(entry.path().to_path_buf());
            }
        } else if path.is_file() {
            if root.is_none() {
                root = path.parent().map(|parent| parent.to_path_buf());
            }
            files.push(path);
        } else {
            warnings.push(format!("Percorso non trovato: {raw}"));
        }
    }
    (files, root, truncated)
}

/// Analizza file e directory senza importare: restituisce l'anteprima,
/// i lemmi proposti, i duplicati e le miniature (solo per i primi elementi).
pub fn scan(paths: &[String], preview_limit: usize) -> ImportPreview {
    let mut warnings = Vec::new();
    let (files, root, truncated) = collect_paths(paths, &mut warnings);
    let suggested_collection = root
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("Pittogrammi importati")
        .to_string();

    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    let mut supported_count = 0usize;
    let mut duplicates = 0usize;
    let limit = preview_limit.min(files.len());

    for (index, path) in files.iter().enumerate() {
        let original_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("immagine")
            .to_string();
        let relative_path = root
            .as_ref()
            .and_then(|root| path.strip_prefix(root).ok())
            .map(|value| value.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| original_name.clone());
        let suggested_lemma = lemma_from_filename(&original_name);

        let meta = fs::metadata(path).ok();
        if meta.map(|m| m.len() > MAX_IMAGE_BYTES).unwrap_or(false) {
            entries.push(ImportPreviewEntry {
                path: path.to_string_lossy().to_string(),
                relative_path,
                original_name,
                suggested_lemma,
                format: "unknown".to_string(),
                supported: false,
                reason: Some(format!(
                    "File troppo grande (limite {} MB)",
                    MAX_IMAGE_BYTES / 1_048_576
                )),
                duplicate: false,
                width: None,
                height: None,
                checksum: None,
                preview: None,
            });
            continue;
        }
        let bytes = match read_limited(path) {
            Ok(bytes) => bytes,
            Err(reason) => {
                entries.push(ImportPreviewEntry {
                    path: path.to_string_lossy().to_string(),
                    relative_path,
                    original_name,
                    suggested_lemma,
                    format: "unknown".to_string(),
                    supported: false,
                    reason: Some(reason),
                    duplicate: false,
                    width: None,
                    height: None,
                    checksum: None,
                    preview: None,
                });
                continue;
            }
        };
        let detected = detect_format(&bytes);
        let format = detected.as_str().to_string();
        let supported = detected.decoder_available();
        let reason = if supported {
            None
        } else if detected == DetectedFormat::Heic {
            Some("HEIC/HEIF: decoder non incluso in questa build".to_string())
        } else {
            Some("Formato non riconosciuto o non supportato".to_string())
        };
        let hash = supported.then(|| checksum(&bytes));
        let mut duplicate = false;
        if let Some(hash) = &hash {
            if !seen.insert(hash.clone()) {
                duplicate = true;
            }
        }
        if supported {
            supported_count += 1;
        }
        if duplicate {
            duplicates += 1;
        }
        let preview = if index < limit && supported {
            preview_for(detected, &bytes)
        } else {
            None
        };
        entries.push(ImportPreviewEntry {
            path: path.to_string_lossy().to_string(),
            relative_path,
            original_name,
            suggested_lemma,
            format,
            supported,
            reason,
            duplicate,
            width: preview.as_ref().and_then(|(_, w, _)| *w),
            height: preview.as_ref().and_then(|(_, _, h)| *h),
            checksum: hash,
            preview: preview.map(|(data, _, _)| data),
        });
    }

    if truncated {
        warnings.push(format!(
            "Scansione limitata a {MAX_FILES} file: importa la cartella in più passaggi."
        ));
    }
    ImportPreview {
        suggested_collection,
        root: root.map(|path| path.to_string_lossy().to_string()),
        total: entries.len(),
        supported: supported_count,
        duplicates,
        truncated,
        warnings,
        entries,
    }
}

fn ensure_pack(db: &Database) -> Result<(), String> {
    db.conn()
        .execute(
            "INSERT INTO symbol_packs(id, name, version, description, author, rights_holder, license,
                license_url, source_url, constraints, commercial_ok, share_alike, no_derivatives,
                builtin, installed_at)
             VALUES ('personal','Collezioni personali','1.0',
                'Pittogrammi importati dall''utente nelle proprie collezioni',
                'Utente','Utente','Licenza dichiarata dall''utente',NULL,NULL,
                'I metadati di licenza sono quelli indicati al momento dell''importazione',
                0,0,0,0,?1)
             ON CONFLICT(id) DO NOTHING",
            rusqlite::params![now_iso()],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn ensure_collection(db: &Database, name: &str) -> Result<(String, String), String> {
    let trimmed = name.trim();
    let name = if trimmed.is_empty() {
        "Pittogrammi importati"
    } else {
        trimmed
    };
    let existing: Option<(String, String)> = db
        .conn()
        .query_row(
            "SELECT id, name FROM pictogram_collections WHERE lower(name) = lower(?1)",
            rusqlite::params![name],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some((id, existing)) = existing {
        return Ok((id, existing));
    }
    let slug = slugify(name);
    let id = format!(
        "col-{}-{}",
        if slug.is_empty() { "import" } else { &slug },
        &checksum(name.as_bytes())[..8]
    );
    let now = now_iso();
    db.conn()
        .execute(
            "INSERT INTO pictogram_collections(id, name, description, created_at, updated_at)
             VALUES (?1, ?2, NULL, ?3, ?3)",
            rusqlite::params![id, name, now],
        )
        .map_err(|e| e.to_string())?;
    Ok((id, name.to_string()))
}

fn insert_index(
    db: &Database,
    symbol_id: &str,
    label: &str,
    meaning: &str,
    category: &str,
    collection: &str,
    synonyms: &[String],
) -> Result<(), String> {
    db.conn()
        .execute(
            "DELETE FROM symbol_fts WHERE symbol_id = ?1",
            rusqlite::params![symbol_id],
        )
        .map_err(|e| e.to_string())?;
    let mut text = format!("{label} {meaning} {category} {collection}");
    for synonym in synonyms {
        text.push(' ');
        text.push_str(synonym);
    }
    db.conn()
        .execute(
            "INSERT INTO symbol_fts(symbol_id, text) VALUES (?1, ?2)",
            rusqlite::params![symbol_id, text],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn import_single(
    db: &Database,
    assets_dir: &Path,
    item: &ImportItem,
    license: &ImportLicense,
    fallback_collection: &str,
) -> Result<(String, String), String> {
    let lemma = item.lemma.trim();
    if lemma.is_empty() {
        return Err("lemma mancante".to_string());
    }
    let collection_name = item
        .collection
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(fallback_collection);
    let (collection_id, collection_name) = ensure_collection(db, collection_name)?;
    ensure_pack(db)?;

    let path = PathBuf::from(&item.path);
    let bytes = read_limited(&path)?;
    let detected = detect_format(&bytes);
    if !detected.decoder_available() {
        return Err(if detected == DetectedFormat::Heic {
            "HEIC/HEIF: decoder non incluso".to_string()
        } else {
            "formato non supportato".to_string()
        });
    }
    let hash = checksum(&bytes);
    let duplicate: Option<String> = db
        .conn()
        .query_row(
            "SELECT id FROM symbols WHERE checksum = ?1 AND collection_id = ?2",
            rusqlite::params![hash, collection_id],
            |row| row.get(0),
        )
        .ok();
    if let Some(existing) = duplicate {
        return Ok((existing, "duplicate".to_string()));
    }

    let (asset_svg, width, height) = if detected == DetectedFormat::Svg {
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| "SVG non codificato in UTF-8".to_string())?;
        let sanitized = crate::asset_manager::svg::sanitize(text, MAX_SVG_BYTES)?;
        let (width, height) = svg_dimensions(sanitized.view_box.as_deref());
        (sanitized.svg, width, height)
    } else {
        let img = image::load_from_memory(&bytes)
            .map_err(|e| format!("decodifica {} non riuscita: {e}", detected.as_str()))?;
        let (svg, width, height) = raster_asset(&img)?;
        (svg, Some(width), Some(height))
    };

    let extension = detected.as_str();
    let original_dir = assets_dir.join("imported").join(&collection_id);
    fs::create_dir_all(&original_dir).map_err(|e| format!("cartella importazione: {e}"))?;
    let original_path = original_dir.join(format!("{}.{}", &hash[..16], extension));
    if !original_path.exists() {
        fs::write(&original_path, &bytes).map_err(|e| format!("copia del file: {e}"))?;
    }

    let lemma_slug = {
        let slug = slugify(lemma);
        if slug.is_empty() {
            "pittogramma".to_string()
        } else {
            slug
        }
    };
    let concept_id = format!("custom.{lemma_slug}");
    let meaning = item
        .category
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|category| format!("pittogramma personale — {category}"))
        .unwrap_or_else(|| "pittogramma personale".to_string());
    let category = item
        .category
        .as_deref()
        .unwrap_or("personalizzato")
        .to_string();

    let now = now_iso();
    db.conn()
        .execute(
            "INSERT INTO concepts(id, label, meaning, category, broader, pos, source, commercial_ok)
             VALUES (?1, ?2, ?3, ?4, NULL, 'noun', 'custom', ?5)
             ON CONFLICT(id) DO UPDATE SET label=excluded.label, meaning=excluded.meaning,
                category=excluded.category",
            rusqlite::params![
                concept_id,
                lemma,
                meaning,
                category,
                if license.commercial_ok { 1 } else { 0 }
            ],
        )
        .map_err(|e| e.to_string())?;

    let symbol_id = format!("custom.{lemma_slug}.{}", &hash[..8]);
    let license_name = license
        .name
        .clone()
        .unwrap_or_else(|| "Licenza non specificata".to_string());
    let author = license.author.clone().unwrap_or_else(|| "Utente".to_string());
    db.conn()
        .execute(
            "INSERT INTO symbols(id, concept_id, pack_id, label, meaning, author, rights_holder,
                license, license_url, source_url, constraints, modifications, format, storage,
                data_path, width, height, status, hide, checksum, created_at, collection_id,
                relative_path, original_name, category)
             VALUES (?1, ?2, 'personal', ?3, ?4, ?5, ?5, ?6, ?7, ?8, NULL,
                'Importato dall''utente', ?9, 'db', ?10, ?11, ?12, 'approved', 0, ?13, ?14,
                ?15, ?16, ?17, ?18)
             ON CONFLICT(id) DO NOTHING",
            rusqlite::params![
                symbol_id,
                concept_id,
                lemma,
                meaning,
                author,
                license_name,
                license.url.clone(),
                license.source_url.clone(),
                detected.as_str(),
                original_path.to_string_lossy().to_string(),
                width.map(|value| value as i64),
                height.map(|value| value as i64),
                hash,
                now,
                collection_id,
                item.relative_path
                    .clone()
                    .unwrap_or_else(|| item.original_name.clone().unwrap_or_default()),
                item.original_name.clone(),
                category,
            ],
        )
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "DELETE FROM symbol_assets WHERE symbol_id = ?1",
            rusqlite::params![symbol_id],
        )
        .map_err(|e| e.to_string())?;
    let asset_bytes = asset_svg.len() as i64;
    db.conn()
        .execute(
            "INSERT INTO symbol_assets(symbol_id, kind, storage, data, path, width, height, bytes)
             VALUES (?1, 'original', 'db', ?2, NULL, ?3, ?4, ?5)",
            rusqlite::params![
                symbol_id,
                asset_svg,
                width.map(|value| value as i64),
                height.map(|value| value as i64),
                asset_bytes
            ],
        )
        .map_err(|e| e.to_string())?;
    db.conn()
        .execute(
            "INSERT INTO symbol_licenses(symbol_id, license, license_url, attribution, commercial_ok,
                share_alike, no_derivatives, source_url)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6)
             ON CONFLICT(symbol_id) DO UPDATE SET license=excluded.license,
                license_url=excluded.license_url, attribution=excluded.attribution,
                commercial_ok=excluded.commercial_ok",
            rusqlite::params![
                symbol_id,
                license_name,
                license.url.clone(),
                format!("{author} — {license_name}"),
                if license.commercial_ok { 1 } else { 0 },
                license.source_url.clone()
            ],
        )
        .map_err(|e| e.to_string())?;

    for synonym in &item.synonyms {
        let term = synonym.trim();
        if term.is_empty() {
            continue;
        }
        db.conn()
            .execute(
                "INSERT OR IGNORE INTO symbol_keywords(symbol_id, term) VALUES (?1, ?2)",
                rusqlite::params![symbol_id, term],
            )
            .map_err(|e| e.to_string())?;
    }
    insert_index(
        db,
        &symbol_id,
        lemma,
        &meaning,
        &category,
        &collection_name,
        &item.synonyms,
    )?;
    db.conn()
        .execute(
            "UPDATE pictogram_collections SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, collection_id],
        )
        .ok();
    let _ = db.insert_import_log(
        Some("personal"),
        "import_pictogram",
        &symbol_id,
        &format!("{collection_name}: {}", item.path),
    );
    Ok((symbol_id, "imported".to_string()))
}

pub fn import_batch(
    db: &Database,
    assets_dir: &Path,
    entries: &[ImportItem],
    license: &ImportLicense,
    fallback_collection: &str,
) -> ImportReport {
    let mut report = ImportReport {
        imported: 0,
        skipped: 0,
        failed: 0,
        items: Vec::new(),
        warnings: Vec::new(),
    };
    for item in entries {
        match import_single(db, assets_dir, item, license, fallback_collection) {
            Ok((symbol_id, status)) => {
                if status == "duplicate" {
                    report.skipped += 1;
                    report.items.push(ImportOutcome {
                        path: item.path.clone(),
                        symbol_id: Some(symbol_id),
                        status,
                        reason: Some("già presente nella collezione".to_string()),
                    });
                } else {
                    report.imported += 1;
                    report.items.push(ImportOutcome {
                        path: item.path.clone(),
                        symbol_id: Some(symbol_id),
                        status,
                        reason: None,
                    });
                }
            }
            Err(reason) => {
                report.failed += 1;
                report.warnings.push(format!("{}: {reason}", item.path));
                report.items.push(ImportOutcome {
                    path: item.path.clone(),
                    symbol_id: None,
                    status: "failed".to_string(),
                    reason: Some(reason),
                });
            }
        }
    }
    if report.failed > 0 {
        report.warnings.push(format!(
            "{} file non importati: controlla l'elenco.",
            report.failed
        ));
    }
    report
}

pub fn list_collections(db: &Database) -> Result<Vec<CollectionInfo>, String> {
    let mut stmt = db
        .conn()
        .prepare(
            "SELECT c.id, c.name, c.description, c.updated_at,
                    (SELECT COUNT(*) FROM symbols s WHERE s.collection_id = c.id AND s.hide = 0)
             FROM pictogram_collections c ORDER BY lower(c.name)",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(CollectionInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                updated_at: row.get(3)?,
                symbols: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn rename_collection(db: &Database, id: &str, name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("il nome della collezione non può essere vuoto".to_string());
    }
    db.conn()
        .execute(
            "UPDATE pictogram_collections SET name = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![trimmed, now_iso(), id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_collection(db: &Database, assets_dir: &Path, id: &str) -> Result<usize, String> {
    let mut files: Vec<String> = Vec::new();
    {
        let mut stmt = db
            .conn()
            .prepare("SELECT data_path FROM symbols WHERE collection_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![id], |row| row.get::<_, Option<String>>(0))
            .map_err(|e| e.to_string())?;
        for row in rows.flatten() {
            if let Some(path) = row {
                files.push(path);
            }
        }
    }
    let count = db
        .conn()
        .execute(
            "DELETE FROM symbols WHERE collection_id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| e.to_string())?;
    db.conn()
        .execute(
            "DELETE FROM pictogram_collections WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| e.to_string())?;
    for file in files {
        if file.starts_with(assets_dir.to_string_lossy().as_ref()) {
            let _ = fs::remove_file(file);
        }
    }
    let _ = fs::remove_dir_all(assets_dir.join("imported").join(id));
    db.conn()
        .execute("DELETE FROM symbol_fts WHERE symbol_id NOT IN (SELECT id FROM symbols)", [])
        .ok();
    Ok(count)
}

pub fn delete_symbols(db: &Database, assets_dir: &Path, ids: &[String]) -> Result<usize, String> {
    let mut removed = 0usize;
    for id in ids {
        let path: Option<String> = db
            .conn()
            .query_row(
                "SELECT data_path FROM symbols WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .ok()
            .flatten();
        let changed = db
            .conn()
            .execute("DELETE FROM symbols WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| e.to_string())?;
        db.conn()
            .execute(
                "DELETE FROM symbol_fts WHERE symbol_id = ?1",
                rusqlite::params![id],
            )
            .ok();
        if let Some(path) = path {
            if path.starts_with(assets_dir.to_string_lossy().as_ref()) {
                let _ = fs::remove_file(path);
            }
        }
        removed += changed;
    }
    Ok(removed)
}

pub fn move_symbols(db: &Database, ids: &[String], collection_id: &str) -> Result<usize, String> {
    let mut moved = 0usize;
    for id in ids {
        moved += db
            .conn()
            .execute(
                "UPDATE symbols SET collection_id = ?1 WHERE id = ?2",
                rusqlite::params![collection_id, id],
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_formats_from_magic_bytes() {
        assert_eq!(detect_format(b"\x89PNG\r\n\x1a\nrest"), DetectedFormat::Png);
        assert_eq!(detect_format(b"\xFF\xD8\xFFrest"), DetectedFormat::Jpeg);
        assert_eq!(detect_format(b"GIF89a..."), DetectedFormat::Gif);
        assert_eq!(
            detect_format(b"RIFF\0\0\0\0WEBPVP8 "),
            DetectedFormat::Webp
        );
        assert_eq!(detect_format(b"BM123456"), DetectedFormat::Bmp);
        assert_eq!(detect_format(b"II*\0rest"), DetectedFormat::Tiff);
        assert_eq!(detect_format(b"\0\0\x01\0rest"), DetectedFormat::Ico);
        assert_eq!(
            detect_format(b"\0\0\0\x20ftypavifrest"),
            DetectedFormat::Avif
        );
        assert_eq!(
            detect_format(b"\0\0\0\x20ftypheicrest"),
            DetectedFormat::Heic
        );
        assert_eq!(
            detect_format(b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
            DetectedFormat::Svg
        );
        assert_eq!(detect_format(b"hello"), DetectedFormat::Unknown);
        assert!(!DetectedFormat::Heic.decoder_available());
        assert!(DetectedFormat::Avif.decoder_available());
    }

    #[test]
    fn lemmas_come_from_clean_file_names() {
        assert_eq!(lemma_from_filename("gatto.png"), "Gatto");
        assert_eq!(
            lemma_from_filename("computer_portatile.svg"),
            "Computer portatile"
        );
        assert_eq!(lemma_from_filename("mangiare-01.webp"), "Mangiare");
        assert_eq!(lemma_from_filename("principesse.jpg"), "Principesse");
        assert_eq!(lemma_from_filename("3_desideri.png"), "3 desideri");
        assert_eq!(lemma_from_filename("cane copia.png"), "Cane");
        assert_eq!(lemma_from_filename("___"), "___");
    }

    fn temp_fn() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open_memory().unwrap();
        db.migrate().unwrap();
        (dir, db)
    }

    fn write_image(dir: &Path, name: &str, format: image::ImageFormat) -> PathBuf {
        let path = dir.join(name);
        // Il colore dipende dal nome: file diversi devono avere impronte
        // diverse anche quando hanno le stesse dimensioni.
        let seed = name.bytes().fold(0u8, |acc, byte| acc.wrapping_add(byte));
        // Il decoder ICO incluso accetta solo PNG RGBA: il test usa quel
        // formato, mentre un ICO non-RGBA viene segnalato come non importato.
        let image = if format == image::ImageFormat::Ico {
            image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(64, 64, |x, y| {
                image::Rgba([(x * 4) as u8, (y * 4) as u8, seed, 255])
            }))
        } else {
            image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(64, 64, |x, y| {
                image::Rgb([(x * 4) as u8, (y * 4) as u8, seed])
            }))
        };
        image.save_with_format(&path, format).unwrap();
        path
    }

    fn entry(path: &Path, lemma: &str) -> ImportItem {
        ImportItem {
            path: path.to_string_lossy().to_string(),
            lemma: lemma.to_string(),
            collection: Some("Prova".to_string()),
            category: Some("animali".to_string()),
            synonyms: vec!["micio".to_string()],
            relative_path: Some("Animali/gatto.png".to_string()),
            original_name: Some("gatto.png".to_string()),
        }
    }

    #[test]
    fn imports_png_jpeg_webp_gif_bmp_tiff_and_svg() {
        let (dir, db) = temp_fn();
        let assets = dir.path().join("assets");
        fs::create_dir_all(&assets).unwrap();
        let license = ImportLicense {
            name: Some("CC BY 4.0".to_string()),
            url: Some("https://creativecommons.org/licenses/by/4.0/".to_string()),
            author: Some("Test".to_string()),
            source_url: None,
            commercial_ok: true,
        };
        let mut items = Vec::new();
        for (name, format) in [
            ("gatto.png", image::ImageFormat::Png),
            ("cane.jpg", image::ImageFormat::Jpeg),
            ("uccello.webp", image::ImageFormat::WebP),
            ("pesce.gif", image::ImageFormat::Gif),
            ("topo.bmp", image::ImageFormat::Bmp),
            ("volpe.tiff", image::ImageFormat::Tiff),
            ("icona.ico", image::ImageFormat::Ico),
            ("aquila.avif", image::ImageFormat::Avif),
        ] {
            items.push(entry(&write_image(dir.path(), name, format), &name[..name.find('.').unwrap()]));
        }
        let svg = dir.path().join("laptop.svg");
        fs::write(
            &svg,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 80\"><rect width=\"100\" height=\"80\" fill=\"#123\"/></svg>",
        )
        .unwrap();
        items.push(entry(&svg, "computer portatile"));

        let report = import_batch(&db, &assets, &items, &license, "Prova");
        assert_eq!(report.imported, items.len(), "avvisi: {:?}", report.warnings);
        assert_eq!(report.failed, 0);

        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM symbols WHERE pack_id='personal'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count as usize, items.len());
        let collections = list_collections(&db).unwrap();
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].name, "Prova");
        assert_eq!(collections[0].symbols as usize, items.len());

        let page = db
            .search_symbols("gatto", 10, 0, &crate::database::SymbolFilters::default())
            .unwrap();
        assert!(page.items.iter().any(|item| item.label == "gatto"));
        let synonyms = db
            .search_symbols("micio", 10, 0, &crate::database::SymbolFilters::default())
            .unwrap();
        assert!(synonyms.items.iter().any(|item| item.label == "gatto"));

        // Reimportare lo stesso file non duplica il pittogramma.
        let again = import_batch(&db, &assets, &items[..1], &license, "Prova");
        assert_eq!(again.imported, 0);
        assert_eq!(again.skipped, 1);
    }

    #[test]
    fn scans_nested_directories_and_reports_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Pittogrammi scuola");
        fs::create_dir_all(root.join("Animali")).unwrap();
        fs::create_dir_all(root.join("Azioni")).unwrap();
        let gatto = write_image(&root.join("Animali"), "gatto.png", image::ImageFormat::Png);
        fs::copy(&gatto, root.join("Animali").join("gatto copia.png")).unwrap();
        write_image(&root.join("Azioni"), "mangiare.png", image::ImageFormat::Png);
        fs::write(root.join("Azioni").join("note.txt"), "non è un'immagine").unwrap();

        let preview = scan(&[root.to_string_lossy().to_string()], 10);
        assert_eq!(preview.suggested_collection, "Pittogrammi scuola");
        assert_eq!(preview.total, 3);
        assert_eq!(preview.duplicates, 1);
        assert!(preview
            .entries
            .iter()
            .any(|entry| entry.relative_path == "Animali/gatto.png"));
        assert!(preview.entries.iter().all(|entry| entry.supported));
        assert!(preview.entries.iter().filter(|entry| entry.preview.is_some()).count() >= 1);
    }

    #[test]
    fn heic_is_reported_as_unsupported_without_faking_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("foto.heic");
        fs::write(&path, b"\0\0\0\x18ftypheicrest").unwrap();
        let preview = scan(&[dir.path().to_string_lossy().to_string()], 10);
        let entry = preview.entries.first().unwrap();
        assert!(!entry.supported);
        assert!(entry.reason.as_deref().unwrap_or("").contains("HEIC"));
    }

    #[test]
    fn deleting_a_collection_removes_its_symbols() {
        let (dir, db) = temp_fn();
        let assets = dir.path().join("assets");
        fs::create_dir_all(&assets).unwrap();
        let path = write_image(dir.path(), "gatto.png", image::ImageFormat::Png);
        let report = import_batch(
            &db,
            &assets,
            &[entry(&path, "gatto")],
            &ImportLicense::default(),
            "Da eliminare",
        );
        assert_eq!(report.imported, 1);
        let collections = list_collections(&db).unwrap();
        let removed = delete_collection(&db, &assets, &collections[0].id).unwrap();
        assert_eq!(removed, 1);
        assert!(list_collections(&db).unwrap().is_empty());
    }

    #[test]
    fn avif_and_heic_are_not_confused() {
        assert_eq!(
            detect_format(b"\0\0\0\x18ftypmif1rest"),
            DetectedFormat::Heic
        );
        assert_eq!(
            detect_format(b"\0\0\0\x18ftypavisrest"),
            DetectedFormat::Avif
        );
    }
}
