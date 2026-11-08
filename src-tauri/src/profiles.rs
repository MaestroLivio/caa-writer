//! Profili utilizzatore, permessi e sistemi comunicativi multilivello.
//!
//! I profili sono locali e non richiedono diagnosi o dati sanitari. Le
//! preferenze (tema, livello editor, funzionalità, voce, accessibilità) sono
//! per profilo; le tabelle assegnate sono un elenco ordinato di sistemi in
//! `resources/boards`.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::database::Database;
use crate::engine::Engine;
use crate::util::now_iso;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default = "default_theme")]
    pub age_theme: String,
    #[serde(default = "default_level")]
    pub editor_level: u32,
    /// Funzionalità abilitate; l'elenco è validato dal frontend e usato per
    /// nascondere/bloccare i comandi.
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub voice: Value,
    #[serde(default)]
    pub accessibility: Value,
    #[serde(default)]
    pub navigation: Value,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

fn default_theme() -> String {
    "primaria".to_string()
}
fn default_level() -> u32 {
    2
}

pub const FEATURES: &[&str] = &[
    "boards",
    "editor",
    "agenda",
    "calendar",
    "social",
    "didactic",
    "speech",
    "compose",
    "search",
    "edit_symbols",
    "save",
    "export",
    "print",
    "online",
    "ai",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardAction {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub target_page_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardCell {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub symbol_label: String,
    #[serde(default)]
    pub spoken_text: String,
    #[serde(default)]
    pub action: Option<BoardAction>,
    // Campi aggiunti dal motore alla lettura.
    #[serde(default, skip_deserializing)]
    pub symbol_id: Option<String>,
    #[serde(default, skip_deserializing)]
    pub svg: Option<String>,
    /// PNG come data URL, alternativa raster a `svg`.
    #[serde(default, skip_deserializing)]
    pub image_asset: Option<String>,
    #[serde(default, skip_deserializing)]
    pub approximate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardPage {
    pub id: String,
    pub title: String,
    pub cells: Vec<BoardCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardSystem {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub age_theme: String,
    pub root_page_id: String,
    pub pages: Vec<BoardPage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardMeta {
    pub id: String,
    pub title: String,
    pub description: String,
    pub age_theme: String,
    pub pages: usize,
    pub cells: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileBoard {
    pub profile_id: String,
    pub board_id: String,
    pub position: i64,
    pub enabled: bool,
    pub title: String,
    pub description: String,
    pub pages: usize,
}

fn profile_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Profile> {
    let features: String = row.get(5)?;
    let voice: String = row.get(6)?;
    let accessibility: String = row.get(7)?;
    let navigation: String = row.get(8)?;
    Ok(Profile {
        id: row.get(0)?,
        name: row.get(1)?,
        avatar: row.get(2)?,
        age_theme: row.get(3)?,
        editor_level: row.get(4)?,
        features: serde_json::from_str(&features).unwrap_or_default(),
        voice: serde_json::from_str(&voice).unwrap_or_else(|_| Value::Object(Default::default())),
        accessibility: serde_json::from_str(&accessibility)
            .unwrap_or_else(|_| Value::Object(Default::default())),
        navigation: serde_json::from_str(&navigation)
            .unwrap_or_else(|_| Value::Object(Default::default())),
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub fn list_profiles(db: &Database) -> Result<Vec<Profile>, String> {
    let mut stmt = db
        .conn()
        .prepare(
            "SELECT id,name,avatar,age_theme,editor_level,features,voice,accessibility,
                    navigation,created_at,updated_at FROM profiles ORDER BY lower(name)",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], profile_from_row)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn get_profile(db: &Database, id: &str) -> Result<Option<Profile>, String> {
    let mut stmt = db
        .conn()
        .prepare(
            "SELECT id,name,avatar,age_theme,editor_level,features,voice,accessibility,
                    navigation,created_at,updated_at FROM profiles WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query_map(rusqlite::params![id], profile_from_row)
        .map_err(|e| e.to_string())?;
    if let Some(row) = rows.next() {
        return Ok(Some(row.map_err(|e| e.to_string())?));
    }
    Ok(None)
}

pub fn save_profile(db: &Database, profile: &Profile) -> Result<(), String> {
    let id = profile.id.trim();
    let name = profile.name.trim();
    if id.is_empty() || name.is_empty() {
        return Err("il profilo deve avere identificativo e nome".to_string());
    }
    let features: Vec<String> = profile
        .features
        .iter()
        .filter(|feature| FEATURES.contains(&feature.as_str()))
        .cloned()
        .collect();
    let now = now_iso();
    db.conn()
        .execute(
            "INSERT INTO profiles(id,name,avatar,age_theme,editor_level,features,voice,
                accessibility,navigation,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)
             ON CONFLICT(id) DO UPDATE SET
                name=excluded.name, avatar=excluded.avatar, age_theme=excluded.age_theme,
                editor_level=excluded.editor_level, features=excluded.features,
                voice=excluded.voice, accessibility=excluded.accessibility,
                navigation=excluded.navigation, updated_at=excluded.updated_at",
            rusqlite::params![
                id,
                name,
                profile.avatar,
                profile.age_theme,
                profile.editor_level.clamp(1, 3),
                serde_json::to_string(&features).map_err(|e| e.to_string())?,
                serde_json::to_string(&profile.voice).map_err(|e| e.to_string())?,
                serde_json::to_string(&profile.accessibility).map_err(|e| e.to_string())?,
                serde_json::to_string(&profile.navigation).map_err(|e| e.to_string())?,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_profile(db: &Database, id: &str) -> Result<(), String> {
    db.conn()
        .execute("DELETE FROM profiles WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    if active_profile(db).as_deref() == Some(id) {
        let _ = db.set_setting("profile.active", "");
    }
    Ok(())
}

pub fn active_profile(db: &Database) -> Option<String> {
    db.setting("profile.active")
        .filter(|value| !value.trim().is_empty())
}

pub fn set_active_profile(db: &Database, id: &str) -> Result<(), String> {
    if get_profile(db, id)?.is_none() {
        return Err("profilo non trovato".to_string());
    }
    db.set_setting("profile.active", id)
        .map_err(|e| e.to_string())
}

pub fn list_profile_boards(db: &Database, profile_id: &str) -> Result<Vec<ProfileBoard>, String> {
    let mut stmt = db
        .conn()
        .prepare(
            "SELECT profile_id, board_id, position, enabled FROM profile_boards
             WHERE profile_id = ?1 ORDER BY position, board_id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        let (profile_id, board_id, position, enabled) = row.map_err(|e| e.to_string())?;
        out.push(ProfileBoard {
            profile_id,
            board_id: board_id.clone(),
            position,
            enabled: enabled != 0,
            title: board_id.clone(),
            description: String::new(),
            pages: 0,
        });
    }
    Ok(out)
}

pub fn assign_board(
    db: &Database,
    profile_id: &str,
    board_id: &str,
    enabled: bool,
    position: i64,
) -> Result<(), String> {
    db.conn()
        .execute(
            "INSERT INTO profile_boards(profile_id,board_id,position,enabled)
             VALUES (?1,?2,?3,?4)
             ON CONFLICT(profile_id,board_id) DO UPDATE SET
                position=excluded.position, enabled=excluded.enabled",
            rusqlite::params![profile_id, board_id, position, if enabled { 1 } else { 0 }],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn remove_board(db: &Database, profile_id: &str, board_id: &str) -> Result<(), String> {
    db.conn()
        .execute(
            "DELETE FROM profile_boards WHERE profile_id = ?1 AND board_id = ?2",
            rusqlite::params![profile_id, board_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn board_path(dir: &Path, id: &str) -> Option<std::path::PathBuf> {
    let candidate = dir.join(format!("{id}.json"));
    if candidate.is_file() {
        return Some(candidate);
    }
    // Accetta anche un percorso relativo senza estensione.
    let candidate = dir.join(id);
    candidate.is_file().then_some(candidate)
}

pub fn list_boards(dir: &Path) -> Result<Vec<BoardMeta>, String> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(out),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(board) = serde_json::from_str::<BoardSystem>(&raw) else {
            continue;
        };
        out.push(BoardMeta {
            id: board.id,
            title: board.title,
            description: board.description,
            age_theme: board.age_theme,
            pages: board.pages.len(),
            cells: board.pages.iter().map(|page| page.cells.len()).sum(),
        });
    }
    out.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(out)
}

/// Carica un sistema e risolve i pittogrammi delle celle con il motore
/// condiviso. La risoluzione avviene una volta per apertura, non a ogni tocco.
pub fn build_board(engine: &Engine, dir: &Path, id: &str) -> Result<BoardSystem, String> {
    let path = board_path(dir, id).ok_or_else(|| format!("sistema comunicativo {id} non trovato"))?;
    let raw = fs::read_to_string(&path).map_err(|e| format!("lettura sistema: {e}"))?;
    let mut board: BoardSystem =
        serde_json::from_str(&raw).map_err(|e| format!("sistema non valido: {e}"))?;
    for page in &mut board.pages {
        for cell in &mut page.cells {
            if cell.symbol_label.trim().is_empty() {
                continue;
            }
            let options = crate::model::SymbolizeOptions {
                intensity: crate::model::Intensity::Completa,
                mode: crate::model::CaaMode::Sopra,
                paragraph_id: format!("board:{}", board.id),
                ..Default::default()
            };
            let query = cell.symbol_label.replace('_', " ");
            if let Ok(result) = engine.symbolize(&query, &options) {
                if let Some(token) = result
                    .tokens
                    .iter()
                    .find(|token| !token.is_punct && token.symbol_id.is_some())
                {
                    cell.symbol_id = token.symbol_id.clone();
                    cell.svg = token.svg.clone();
                    cell.image_asset = token.image.clone();
                    cell.approximate = token.approximate;
                }
            }
        }
    }
    Ok(board)
}

pub fn pin_hash(pin: &str) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(format!("caa-local-creator:{pin}").as_bytes());
    crate::asset_manager::hex(&digest)
}

pub fn pin_enabled(db: &Database) -> bool {
    db.setting("creator.pin_enabled")
        .map(|value| value == "1")
        .unwrap_or(false)
}

pub fn verify_pin(db: &Database, pin: &str) -> bool {
    if !pin_enabled(db) {
        return true;
    }
    db.setting("creator.pin_hash")
        .map(|stored| stored == pin_hash(pin))
        .unwrap_or(false)
}

pub fn set_pin(db: &Database, pin: Option<&str>) -> Result<(), String> {
    match pin {
        Some(pin) if pin.len() >= 4 => {
            db.set_setting("creator.pin_hash", &pin_hash(pin))
                .map_err(|e| e.to_string())?;
            db.set_setting("creator.pin_enabled", "1")
                .map_err(|e| e.to_string())?;
        }
        Some(_) => return Err("il PIN deve avere almeno 4 caratteri".to_string()),
        None => {
            db.set_setting("creator.pin_enabled", "0")
                .map_err(|e| e.to_string())?;
            db.set_setting("creator.pin_hash", "")
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Controllo amministrativo riutilizzato dai comandi di profilo.
pub fn require_creator(db: &Database, pin: &str) -> Result<(), String> {
    if verify_pin(db, pin) {
        Ok(())
    } else {
        Err("PIN Creator non valido".to_string())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_library_has_30_complete_systems() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/boards");
        let boards = list_boards(&dir).expect("catalogo boards");
        assert_eq!(boards.len(), 30, "devono esserci 30 sistemi comunicativi");
        let mut total_pages = 0usize;
        let mut total_cells = 0usize;
        for meta in &boards {
            let raw = fs::read_to_string(dir.join(format!("{}.json", meta.id))).unwrap();
            let board: BoardSystem = serde_json::from_str(&raw).unwrap();
            assert!(board.pages.len() >= 5, "{} ha troppe poche pagine", board.id);
            assert!(
                board.pages.iter().any(|page| page.id == board.root_page_id),
                "{} non ha la home",
                board.id
            );
            let ids: std::collections::HashSet<&str> =
                board.pages.iter().map(|page| page.id.as_str()).collect();
            for page in &board.pages {
                assert!(page.cells.len() >= 8, "{}: pagina {} troppo piccola", board.id, page.id);
                for cell in &page.cells {
                    assert!(!cell.label.trim().is_empty());
                    assert!(!cell.symbol_label.trim().is_empty());
                    if let Some(action) = &cell.action {
                        if action.kind == "navigate" {
                            let target = action.target_page_id.as_deref().unwrap_or("");
                            assert!(ids.contains(target), "{}: collegamento rotto {}", board.id, target);
                        }
                    }
                }
            }
            total_pages += board.pages.len();
            total_cells += board.pages.iter().map(|page| page.cells.len()).sum::<usize>();
        }
        assert!(total_pages >= 200);
        assert!(total_cells >= 1500);
    }

    #[test]
    fn profile_roundtrip_and_board_assignment() {
        let mut db = Database::open_memory().unwrap();
        db.migrate().unwrap();
        let profile = Profile {
            id: "p1".to_string(),
            name: "Luca".to_string(),
            avatar: "L".to_string(),
            age_theme: "primaria".to_string(),
            editor_level: 2,
            features: vec!["boards".to_string(), "speech".to_string()],
            voice: Value::Object(Default::default()),
            accessibility: Value::Object(Default::default()),
            navigation: Value::Object(Default::default()),
            created_at: String::new(),
            updated_at: String::new(),
        };
        save_profile(&db, &profile).unwrap();
        set_active_profile(&db, "p1").unwrap();
        assert_eq!(active_profile(&db).as_deref(), Some("p1"));
        let loaded = get_profile(&db, "p1").unwrap().unwrap();
        assert_eq!(loaded.name, "Luca");
        assert!(loaded.features.contains(&"speech".to_string()));
        assign_board(&db, "p1", "boards.famiglia", true, 0).unwrap();
        let boards = list_profile_boards(&db, "p1").unwrap();
        assert_eq!(boards.len(), 1);
        assert!(boards[0].enabled);
        remove_board(&db, "p1", "boards.famiglia").unwrap();
        assert!(list_profile_boards(&db, "p1").unwrap().is_empty());
    }
}
