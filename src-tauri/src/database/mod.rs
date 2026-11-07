//! Database locale SQLite con FTS5.
//!
//! Scelte:
//! - SQLite compilato staticamente (`rusqlite/bundled`) → nessuna dipendenza
//!   dal sistema, FTS5 disponibile su Windows/macOS senza installazioni.
//! - I metadati dei simboli sono ricercabili senza caricare le immagini:
//!   l'SVG è in `symbol_assets` e viene letto solo quando serve.
//! - Le migrazioni sono versionate e applicate in ordine.

pub mod migrations;
pub mod seed;

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Row};
use thiserror::Error;

use crate::model::{
    AttributionEntry, LibraryStats, PackInfo, Preference, SearchResultPage, UnitModifiers,
};
use crate::util::now_iso;

pub use seed::{seed_core_pack, SeedStats};

#[derive(Debug, Error)]
pub enum DbError {
    #[error("errore database: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("{0}")]
    Other(String),
}

impl From<DbError> for String {
    fn from(value: DbError) -> Self {
        value.to_string()
    }
}

pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug, Clone, Default)]
pub struct SymbolFilters {
    pub pack_id: Option<String>,
    pub commercial_only: bool,
    /// Limita ai pittogrammi di una collezione personale.
    pub collection_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SymbolRecord {
    pub id: String,
    pub concept_id: Option<String>,
    pub pack_id: String,
    pub pack_name: String,
    pub label: String,
    pub meaning: Option<String>,
    pub format: String,
    pub license: String,
    pub license_url: Option<String>,
    pub source_url: Option<String>,
    pub author: Option<String>,
    pub commercial_ok: bool,
    pub status: String,
    pub svg: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConceptRecord {
    pub id: String,
    pub label: String,
    pub meaning: String,
    pub category: String,
    pub broader: Option<String>,
}

pub struct Database {
    conn: Connection,
}

const SYMBOL_SELECT: &str = "SELECT s.id, s.concept_id, s.pack_id, p.name, s.label, s.meaning, s.format,
        s.license, s.license_url, s.source_url, s.author, p.commercial_ok, s.status,
        (SELECT a.data FROM symbol_assets a WHERE a.symbol_id = s.id ORDER BY a.id LIMIT 1),
        (SELECT a.path FROM symbol_assets a WHERE a.symbol_id = s.id ORDER BY a.id LIMIT 1)
     FROM symbols s JOIN symbol_packs p ON p.id = s.pack_id ";

/// Legge e sanifica un SVG di pacchetto, tenendolo in una cache condivisa:
/// le alternative vengono richieste molte volte mentre si scrive, e senza
/// cache ogni parola rileggerebbe decine di file da disco.
fn svg_from_file(path: &str) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    // Mappa semplice, senza ordinamento: la cache LRU generica costa una
    // scansione lineare per accesso, inaccettabile quando le alternative
    // chiedono decine di simboli per parola. Al limite si svuota.
    const CAPACITY: usize = 512;
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(value) = guard.get(path) {
            return value.clone();
        }
    }
    let value = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| crate::asset_manager::svg::sanitize(&raw, 512 * 1024).ok())
        .map(|out| out.svg);
    if let Ok(mut guard) = cache.lock() {
        if guard.len() >= CAPACITY {
            guard.clear();
        }
        guard.insert(path.to_string(), value.clone());
    }
    value
}

impl Database {
    pub fn open(path: &Path) -> DbResult<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        let db = Self { conn };
        Ok(db)
    }

    pub fn open_memory() -> DbResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self { conn })
    }

    pub fn migrate(&mut self) -> DbResult<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );",
        )?;
        for migration in migrations::MIGRATIONS {
            let applied: Option<i64> = self
                .conn
                .query_row(
                    "SELECT version FROM schema_migrations WHERE version = ?1",
                    params![migration.version],
                    |row| row.get(0),
                )
                .optional()?;
            if applied.is_some() {
                continue;
            }
            let tx = self.conn.transaction()?;
            tx.execute_batch(migration.sql)?;
            tx.execute(
                "INSERT INTO schema_migrations(version, name, applied_at) VALUES (?1, ?2, ?3)",
                params![migration.version, migration.name, now_iso()],
            )?;
            tx.commit()?;
        }
        Ok(())
    }

    pub fn schema_version(&self) -> DbResult<i64> {
        let version: Option<i64> = self
            .conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .optional()?
            .flatten();
        Ok(version.unwrap_or(0))
    }

    fn map_symbol(row: &Row<'_>) -> rusqlite::Result<SymbolRecord> {
        let stored: Option<String> = row.get(13)?;
        let path: Option<String> = row.get(14)?;
        // I simboli dei pacchetti restano su disco: si rileggono e si
        // sanificano qui, così il contenuto restituito al frontend è sempre
        // quello validato, non il file grezzo.
        let svg = stored.or_else(|| svg_from_file(&path?));
        Ok(SymbolRecord {
            id: row.get(0)?,
            concept_id: row.get(1)?,
            pack_id: row.get(2)?,
            pack_name: row.get(3)?,
            label: row.get(4)?,
            meaning: row.get(5)?,
            format: row.get(6)?,
            license: row.get(7)?,
            license_url: row.get(8)?,
            source_url: row.get(9)?,
            author: row.get(10)?,
            commercial_ok: row.get::<_, i64>(11)? != 0,
            status: row.get(12)?,
            svg,
        })
    }

    /// Ripara i percorsi degli asset dopo uno spostamento dell'app o un
    /// aggiornamento che ha cambiato la cartella risorse. Senza questo, i
    /// simboli su disco risultano illeggibili e l'editor può bloccarsi.
    pub fn repair_asset_paths(&mut self, resources_dir: &Path) -> DbResult<usize> {
        let rows: Vec<(i64, String, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT id, symbol_id, path FROM symbol_assets
                 WHERE storage = 'file' AND path IS NOT NULL",
            )?;
            let mapped = stmt.query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;
            let mut out = Vec::new();
            for row in mapped {
                out.push(row?);
            }
            out
        };
        let mut updates: Vec<(i64, String, String)> = Vec::new();
        for (id, symbol_id, path) in rows {
            if Path::new(&path).exists() {
                continue;
            }
            let candidate = if let Some(index) = path.find("symbol-packs/") {
                resources_dir.join(&path[index..])
            } else if Path::new(&path).is_relative() {
                resources_dir.join(&path)
            } else {
                continue;
            };
            if candidate.exists() {
                updates.push((id, symbol_id, candidate.to_string_lossy().to_string()));
            }
        }
        if updates.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.transaction()?;
        for (id, symbol_id, path) in &updates {
            tx.execute(
                "UPDATE symbol_assets SET path = ?1 WHERE id = ?2",
                params![path, id],
            )?;
            tx.execute(
                "UPDATE symbols SET data_path = ?1 WHERE id = ?2",
                params![path, symbol_id],
            )?;
        }
        tx.commit()?;
        Ok(updates.len())
    }

    pub fn symbols_for_concept(
        &self,
        concept_id: &str,
        filters: &SymbolFilters,
    ) -> DbResult<Vec<SymbolRecord>> {
        let sql = format!(
            "{SYMBOL_SELECT}
             WHERE s.concept_id = ?1 AND s.hide = 0
               AND (?2 IS NULL OR s.pack_id = ?2)
               AND (?3 = 0 OR p.commercial_ok = 1)
             ORDER BY CASE s.status WHEN 'approved' THEN 0 ELSE 1 END, s.id
             LIMIT 24"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(
            params![concept_id, filters.pack_id, filters.commercial_only as i64],
            Self::map_symbol,
        )?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Simbolo con etichetta o parola chiave esattamente uguale al termine.
    /// Serve alle parole fuori lessico salvate dalle API: senza questo
    /// controllo la rianalisi cancellava il pittogramma appena inserito.
    pub fn symbol_by_label(
        &self,
        label: &str,
        filters: &SymbolFilters,
    ) -> DbResult<Option<SymbolRecord>> {
        let needle = label.trim().to_lowercase();
        if needle.is_empty() {
            return Ok(None);
        }
        let sql = format!(
            "{SYMBOL_SELECT}
             WHERE s.hide = 0
               AND (lower(trim(s.label)) = ?1
                    OR EXISTS (SELECT 1 FROM symbol_keywords k
                               WHERE k.symbol_id = s.id AND lower(trim(k.term)) = ?1))
               AND (?2 IS NULL OR s.pack_id = ?2)
               AND (?3 = 0 OR p.commercial_ok = 1)
             ORDER BY CASE WHEN s.pack_id = 'personal' THEN 0 ELSE 1 END,
                      CASE s.status WHEN 'approved' THEN 0 ELSE 1 END, s.id
             LIMIT 1"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query_map(
            params![needle, filters.pack_id, filters.commercial_only as i64],
            Self::map_symbol,
        )?;
        if let Some(row) = rows.next() {
            return Ok(Some(row?));
        }
        Ok(None)
    }

    pub fn symbol_by_id(&self, symbol_id: &str) -> DbResult<Option<SymbolRecord>> {
        let sql = format!("{SYMBOL_SELECT} WHERE s.id = ?1");
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query_map(params![symbol_id], Self::map_symbol)?;
        if let Some(row) = rows.next() {
            return Ok(Some(row?));
        }
        Ok(None)
    }

    /// Ricerca testuale: FTS5 per i termini, con ripiego su LIKE.
    pub fn search_symbols(
        &self,
        query: &str,
        limit: u32,
        offset: u32,
        filters: &SymbolFilters,
    ) -> DbResult<SearchResultPage> {
        let limit = limit.clamp(1, 200);
        let trimmed = query.trim();
        if trimmed.is_empty() {
            let total: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM symbols s JOIN symbol_packs p ON p.id = s.pack_id
                 WHERE s.hide = 0 AND (?1 IS NULL OR s.pack_id = ?1)
                   AND (?2 = 0 OR p.commercial_ok = 1)
                   AND (?3 IS NULL OR s.collection_id = ?3)",
                params![
                    filters.pack_id,
                    filters.commercial_only as i64,
                    filters.collection_id
                ],
                |row| row.get(0),
            )?;
            let sql = format!(
                "{SYMBOL_SELECT} WHERE s.hide = 0 AND (?1 IS NULL OR s.pack_id = ?1)
                 AND (?2 = 0 OR p.commercial_ok = 1)
                 AND (?3 IS NULL OR s.collection_id = ?3)
                 ORDER BY CASE s.pack_id WHEN 'personal' THEN 0 ELSE 1 END,
                          s.concept_id, s.id LIMIT ?4 OFFSET ?5"
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(
                params![
                    filters.pack_id,
                    filters.commercial_only as i64,
                    filters.collection_id,
                    limit,
                    offset
                ],
                Self::map_symbol,
            )?;
            let mut items = Vec::new();
            for row in rows {
                items.push(crate::asset_manager::alternative_from_record(&row?));
            }
            return Ok(SearchResultPage {
                items,
                total,
                page: offset / limit,
                page_size: limit,
            });
        }

        let match_query = fts_query(trimmed);
        if !match_query.is_empty() {
            let sql = format!(
                "{SYMBOL_SELECT} JOIN symbol_fts f ON f.symbol_id = s.id
                 WHERE symbol_fts MATCH ?1 AND s.hide = 0
                   AND (?2 IS NULL OR s.pack_id = ?2)
                   AND (?3 = 0 OR p.commercial_ok = 1)
                   AND (?4 IS NULL OR s.collection_id = ?4)
                 ORDER BY CASE s.pack_id WHEN 'personal' THEN 0 ELSE 1 END,
                          bm25(symbol_fts), s.id LIMIT ?5 OFFSET ?6"
            );
            if let Ok(mut stmt) = self.conn.prepare(&sql) {
                let rows = stmt.query_map(
                    params![
                        match_query,
                        filters.pack_id,
                        filters.commercial_only as i64,
                        filters.collection_id,
                        limit,
                        offset
                    ],
                    Self::map_symbol,
                );
                if let Ok(rows) = rows {
                    let mut items = Vec::new();
                    let mut failed = false;
                    for row in rows {
                        match row {
                            Ok(record) => items.push(crate::asset_manager::alternative_from_record(&record)),
                            Err(_) => {
                                failed = true;
                                break;
                            }
                        }
                    }
                    if !failed && !items.is_empty() {
                        let total = self.count_search(&match_query, filters).unwrap_or(items.len() as i64);
                        return Ok(SearchResultPage {
                            items,
                            total,
                            page: offset / limit,
                            page_size: limit,
                        });
                    }
                }
            }
        }

        // Ripiego LIKE: parole non indicizzate o forme parziali.
        let like = format!("%{}%", trimmed.to_lowercase());
        let sql = format!(
            "{SYMBOL_SELECT} WHERE s.hide = 0 AND (lower(s.label) LIKE ?1 OR lower(IFNULL(s.meaning,'')) LIKE ?1)
             AND (?2 IS NULL OR s.pack_id = ?2) AND (?3 = 0 OR p.commercial_ok = 1)
             AND (?4 IS NULL OR s.collection_id = ?4)
             ORDER BY CASE s.pack_id WHEN 'personal' THEN 0 ELSE 1 END,
                      length(s.label), s.id LIMIT ?5 OFFSET ?6"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(
            params![
                like,
                filters.pack_id,
                filters.commercial_only as i64,
                filters.collection_id,
                limit,
                offset
            ],
            Self::map_symbol,
        )?;
        let mut items = Vec::new();
        for row in rows {
            items.push(crate::asset_manager::alternative_from_record(&row?));
        }
        let total = items.len() as i64;
        Ok(SearchResultPage {
            items,
            total,
            page: offset / limit,
            page_size: limit,
        })
    }

    fn count_search(&self, match_query: &str, filters: &SymbolFilters) -> DbResult<i64> {
        let sql = "SELECT COUNT(*) FROM symbol_fts f
                   JOIN symbols s ON s.id = f.symbol_id
                   JOIN symbol_packs p ON p.id = s.pack_id
                   WHERE symbol_fts MATCH ?1 AND s.hide = 0
                     AND (?2 IS NULL OR s.pack_id = ?2)
                     AND (?3 = 0 OR p.commercial_ok = 1)
                     AND (?4 IS NULL OR s.collection_id = ?4)";
        let count: i64 = self.conn.query_row(
            sql,
            params![
                match_query,
                filters.pack_id,
                filters.commercial_only as i64,
                filters.collection_id
            ],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn concept(&self, concept_id: &str) -> DbResult<Option<ConceptRecord>> {
        let record = self
            .conn
            .query_row(
                "SELECT id, label, meaning, category, broader FROM concepts WHERE id = ?1",
                params![concept_id],
                |row| {
                    Ok(ConceptRecord {
                        id: row.get(0)?,
                        label: row.get(1)?,
                        meaning: row.get(2)?,
                        category: row.get(3)?,
                        broader: row.get(4)?,
                    })
                },
            )
            .optional()?;
        Ok(record)
    }

    pub fn broader_of(&self, concept_id: &str) -> DbResult<Option<String>> {
        let broader: Option<Option<String>> = self
            .conn
            .query_row(
                "SELECT broader FROM concepts WHERE id = ?1",
                params![concept_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(broader.flatten())
    }

    pub fn equivalent_concepts(&self, concept_id: &str, limit: u32) -> DbResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT to_concept FROM semantic_relations
             WHERE from_concept = ?1 AND kind = 'equivalent'
             ORDER BY weight DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![concept_id, limit], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn related_concepts(&self, concept_id: &str, limit: u32) -> DbResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT to_concept FROM semantic_relations
             WHERE from_concept = ?1 AND kind = 'related'
             ORDER BY weight DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![concept_id, limit], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Preferenza salvata: scope `document` usa `unit_key`, gli altri il lemma.
    pub fn preference(
        &self,
        scope: &str,
        lemma: &str,
        context_key: &str,
    ) -> DbResult<Option<Preference>> {
        let record = self
            .conn
            .query_row(
                "SELECT scope, lemma, context_key, symbol_id, concept_id, locked, created_at
                 FROM symbol_preferences
                 WHERE scope = ?1 AND lemma = ?2 AND IFNULL(context_key, '') = ?3
                 LIMIT 1",
                params![scope, lemma, context_key],
                |row| {
                    Ok(Preference {
                        scope: row.get(0)?,
                        lemma: row.get(1)?,
                        context_key: row.get(2)?,
                        symbol_id: row.get(3)?,
                        concept_id: row.get(4)?,
                        locked: row.get::<_, i64>(5)? != 0,
                        created_at: row.get(6)?,
                    })
                },
            )
            .optional()?;
        Ok(record)
    }

    pub fn upsert_preference(&self, pref: &Preference) -> DbResult<()> {
        self.conn.execute(
            "INSERT INTO symbol_preferences(scope, lemma, context_key, symbol_id, concept_id, locked, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(scope, lemma, IFNULL(context_key, '')) DO UPDATE SET
                symbol_id = excluded.symbol_id,
                concept_id = excluded.concept_id,
                locked = excluded.locked,
                created_at = excluded.created_at",
            params![
                pref.scope,
                pref.lemma,
                pref.context_key,
                pref.symbol_id,
                pref.concept_id,
                pref.locked as i64,
                pref.created_at
            ],
        )?;
        Ok(())
    }

    pub fn delete_preference(&self, scope: &str, lemma: &str, context_key: &str) -> DbResult<()> {
        self.conn.execute(
            "DELETE FROM symbol_preferences WHERE scope = ?1 AND lemma = ?2 AND IFNULL(context_key,'') = ?3",
            params![scope, lemma, context_key],
        )?;
        Ok(())
    }

    pub fn list_preferences(&self) -> DbResult<Vec<Preference>> {
        let mut stmt = self.conn.prepare(
            "SELECT scope, lemma, context_key, symbol_id, concept_id, locked, created_at
             FROM symbol_preferences ORDER BY scope, lemma",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Preference {
                scope: row.get(0)?,
                lemma: row.get(1)?,
                context_key: row.get(2)?,
                symbol_id: row.get(3)?,
                concept_id: row.get(4)?,
                locked: row.get::<_, i64>(5)? != 0,
                created_at: row.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn document_override(
        &self,
        doc_key: &str,
        unit_key: &str,
    ) -> DbResult<Option<(Option<String>, Option<String>)>> {
        let record = self
            .conn
            .query_row(
                "SELECT symbol_id, concept_id FROM document_overrides
                 WHERE doc_key = ?1 AND unit_key = ?2 AND locked = 1",
                params![doc_key, unit_key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        Ok(record)
    }

    pub fn set_document_override(
        &self,
        doc_key: &str,
        unit_key: &str,
        symbol_id: Option<&str>,
        concept_id: Option<&str>,
    ) -> DbResult<()> {
        self.conn.execute(
            "INSERT INTO document_overrides(doc_key, unit_key, symbol_id, concept_id, locked, updated_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5)
             ON CONFLICT(doc_key, unit_key) DO UPDATE SET
                symbol_id = excluded.symbol_id,
                concept_id = excluded.concept_id,
                locked = 1,
                updated_at = excluded.updated_at",
            params![doc_key, unit_key, symbol_id, concept_id, now_iso()],
        )?;
        Ok(())
    }


    /// Stato dell'unità indipendente dalla scelta del simbolo: visibilità e
    /// modificatori restano validi anche quando il simbolo viene ricalcolato.
    pub fn unit_state(
        &self,
        doc_key: &str,
        unit_key: &str,
    ) -> DbResult<Option<(bool, UnitModifiers)>> {
        let record = self
            .conn
            .query_row(
                "SELECT hidden, modifiers FROM document_overrides
                 WHERE doc_key = ?1 AND unit_key = ?2",
                params![doc_key, unit_key],
                |row| Ok((row.get::<_, i64>(0)? != 0, row.get::<_, String>(1)?)),
            )
            .optional()?;
        Ok(record.map(|(hidden, modifiers)| {
            let parsed = serde_json::from_str::<UnitModifiers>(&modifiers).unwrap_or_default();
            (hidden, parsed)
        }))
    }

    /// Aggiorna soltanto lo stato dell'unità (blocco, visibilità, modificatori).
    /// Il simbolo scelto non viene toccato: sbloccare un'unità la rende
    /// rivalutabile senza perdere il pittogramma finché non viene ricalcolato.
    pub fn set_unit_state(
        &self,
        doc_key: &str,
        unit_key: &str,
        locked: bool,
        hidden: bool,
        modifiers: &UnitModifiers,
    ) -> DbResult<()> {
        let modifiers = serde_json::to_string(modifiers).unwrap_or_else(|_| "{}".to_string());
        self.conn.execute(
            "INSERT INTO document_overrides(doc_key, unit_key, symbol_id, concept_id, locked, hidden, modifiers, updated_at)
             VALUES (?1, ?2, NULL, NULL, ?3, ?4, ?5, ?6)
             ON CONFLICT(doc_key, unit_key) DO UPDATE SET
                locked = excluded.locked,
                hidden = excluded.hidden,
                modifiers = excluded.modifiers,
                updated_at = excluded.updated_at",
            params![doc_key, unit_key, locked as i64, hidden as i64, modifiers, now_iso()],
        )?;
        Ok(())
    }

    pub fn clear_document_override(&self, doc_key: &str, unit_key: &str) -> DbResult<()> {
        self.conn.execute(
            "DELETE FROM document_overrides WHERE doc_key = ?1 AND unit_key = ?2",
            params![doc_key, unit_key],
        )?;
        Ok(())
    }

    pub fn record_search(&self, term: &str) -> DbResult<()> {
        self.conn.execute(
            "INSERT INTO search_statistics(term, hits, last_used) VALUES (?1, 1, ?2)
             ON CONFLICT(term) DO UPDATE SET hits = hits + 1, last_used = excluded.last_used",
            params![term.to_lowercase(), now_iso()],
        )?;
        Ok(())
    }

    pub fn frequency(&self, term: &str) -> i64 {
        self.conn
            .query_row(
                "SELECT hits FROM search_statistics WHERE term = ?1",
                params![term.to_lowercase()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or(0)
    }

    pub fn statistics(&self) -> DbResult<LibraryStats> {
        let count = |sql: &str| -> i64 {
            self.conn
                .query_row(sql, [], |row| row.get::<_, i64>(0))
                .unwrap_or(0)
        };
        Ok(LibraryStats {
            symbols: count("SELECT COUNT(*) FROM symbols WHERE hide = 0"),
            concepts: count("SELECT COUNT(*) FROM concepts"),
            lemmas: count("SELECT COUNT(*) FROM lemmas"),
            packs: count("SELECT COUNT(*) FROM symbol_packs"),
            expressions: count("SELECT COUNT(*) FROM expressions"),
            packs_detail: self.list_packs()?,
        })
    }

    pub fn list_packs(&self) -> DbResult<Vec<PackInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.name, p.version, p.license, p.license_url, p.source_url, p.author,
                    p.commercial_ok, p.description,
                    (SELECT COUNT(*) FROM symbols s WHERE s.pack_id = p.id AND s.hide = 0)
             FROM symbol_packs p ORDER BY p.builtin DESC, p.name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PackInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                license: row.get(3)?,
                license_url: row.get(4)?,
                source_url: row.get(5)?,
                author: row.get(6)?,
                commercial_ok: row.get::<_, i64>(7)? != 0,
                description: row.get(8)?,
                symbol_count: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn attributions(&self) -> DbResult<Vec<AttributionEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.label, s.pack_id, p.name, s.author, s.rights_holder, s.license,
                    s.license_url, s.source_url, s.constraints, s.modifications
             FROM symbols s JOIN symbol_packs p ON p.id = s.pack_id
             ORDER BY p.name, s.label",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(AttributionEntry {
                symbol_id: row.get(0)?,
                label: row.get(1)?,
                pack_id: row.get(2)?,
                pack_name: row.get(3)?,
                author: row.get(4)?,
                rights_holder: row.get(5)?,
                license: row.get(6)?,
                license_url: row.get(7)?,
                source_url: row.get(8)?,
                constraints: row.get(9)?,
                modifications: row.get(10)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> DbResult<()> {
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
    }

    /// Rimuove cache e statistiche d'uso (privacy, §30).
    pub fn clear_history(&self) -> DbResult<()> {
        self.conn.execute("DELETE FROM search_statistics", [])?;
        Ok(())
    }

    pub fn hide_symbol(&self, symbol_id: &str, hide: bool) -> DbResult<()> {
        self.conn.execute(
            "UPDATE symbols SET hide = ?2 WHERE id = ?1",
            params![symbol_id, hide as i64],
        )?;
        Ok(())
    }

    pub fn insert_import_log(&self, pack_id: Option<&str>, action: &str, detail: &str, warnings: &str) {
        let _ = self.conn.execute(
            "INSERT INTO import_log(pack_id, action, detail, warnings, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![pack_id, action, detail, warnings, now_iso()],
        );
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

/// Costruisce una query FTS5 tollerante: solo caratteri alfanumerici,
/// prefisso su ogni parola, AND tra le parole.
fn fts_query(query: &str) -> String {
    let words: Vec<String> = query
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        return String::new();
    }
    words
        .iter()
        .map(|w| format!("\"{}\"*", w.to_lowercase()))
        .collect::<Vec<_>>()
        .join(" AND ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_query_is_safe() {
        let q = fts_query("pesca \"frutto\" OR *");
        assert_eq!(q, "\"pesca\"* AND \"frutto\"* AND \"or\"*");
        assert!(!q.contains('('));
    }

    #[test]
    fn unit_state_roundtrip_keeps_symbol_and_flags() {
        let mut db = Database::open_memory().unwrap();
        db.migrate().unwrap();
        db.set_document_override("doc.caadoc", "p1:2", Some("core.a"), Some("a"))
            .unwrap();
        let modifiers = UnitModifiers {
            plural: true,
            negation: false,
            past: false,
            future: false,
            manual: true,
        };
        db.set_unit_state("doc.caadoc", "p1:2", true, true, &modifiers)
            .unwrap();
        let (hidden, stored) = db.unit_state("doc.caadoc", "p1:2").unwrap().unwrap();
        assert!(hidden);
        assert!(stored.plural && stored.manual);
        // Il simbolo scelto resta: cambiare visibilità non annulla la scelta.
        let (symbol, concept) = db.document_override("doc.caadoc", "p1:2").unwrap().unwrap();
        assert_eq!(symbol.as_deref(), Some("core.a"));
        assert_eq!(concept.as_deref(), Some("a"));
        // Sbloccare rende l'unità di nuovo automatica.
        db.set_unit_state("doc.caadoc", "p1:2", false, true, &modifiers)
            .unwrap();
        assert!(db.document_override("doc.caadoc", "p1:2").unwrap().is_none());
    }

    #[test]
    fn migrations_apply_and_are_idempotent() {
        let mut db = Database::open_memory().unwrap();
        db.migrate().unwrap();
        db.migrate().unwrap();
        // La versione attesa è l'ultima migrazione registrata.
        let latest = migrations::MIGRATIONS.last().map(|item| item.version).unwrap();
        assert_eq!(db.schema_version().unwrap(), latest);
    }

    #[test]
    fn settings_roundtrip() {
        let mut db = Database::open_memory().unwrap();
        db.migrate().unwrap();
        db.set_setting("theme", "dark").unwrap();
        assert_eq!(db.setting("theme").as_deref(), Some("dark"));
    }
}
