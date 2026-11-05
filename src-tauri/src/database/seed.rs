//! Popolamento del database dal lessico e dal pacchetto simboli `core`.
//!
//! L'import è idempotente: si può rieseguire dopo un aggiornamento delle
//! risorse senza duplicare simboli o concetti.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::params;
use serde::Deserialize;

use super::{Database, DbResult};
use crate::asset_manager::svg;
use crate::linguistics::lexicon::LexiconFile;
use crate::linguistics::morphology::generate_forms;
use crate::util::now_iso;

const MAX_SVG_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SeedStats {
    pub concepts: usize,
    pub lemmas: usize,
    pub forms: usize,
    pub expressions: usize,
    pub symbols: usize,
    pub skipped: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PackManifest {
    pack: PackMeta,
    symbols: Vec<ManifestSymbol>,
}

#[derive(Debug, Deserialize)]
struct PackMeta {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    rights_holder: Option<String>,
    license: String,
    #[serde(default)]
    license_url: Option<String>,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default)]
    constraints: Option<String>,
    #[serde(default)]
    commercial_ok: bool,
    #[serde(default)]
    share_alike: bool,
    #[serde(default)]
    no_derivatives: bool,
    #[serde(default)]
    builtin: bool,
}

#[derive(Debug, Deserialize)]
struct ManifestSymbol {
    id: String,
    #[serde(default)]
    concept: Option<String>,
    label: String,
    #[serde(default)]
    meaning: Option<String>,
    #[serde(default)]
    file: Option<String>,
    /// Parole chiave del repertorio (per esempio i sinonimi italiani di
    /// ARASAAC): entrano nell'indice di ricerca insieme all'etichetta.
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    status: Option<String>,
}

/// Popola concetti, lemmi, flessioni, sinonimi, espressioni e simboli.
pub fn seed_core_pack(db: &mut Database, dictionary: &Path, pack_dir: &Path) -> DbResult<SeedStats> {
    let mut stats = SeedStats::default();

    let raw = std::fs::read_to_string(dictionary).map_err(|e| {
        super::DbError::Other(format!("lessico non leggibile ({}): {e}", dictionary.display()))
    })?;
    let lexicon: LexiconFile = serde_json::from_str(&raw)
        .map_err(|e| super::DbError::Other(format!("lessico non valido: {e}")))?;

    let manifest_path = pack_dir.join("manifest.json");
    let manifest_raw = std::fs::read_to_string(&manifest_path).map_err(|e| {
        super::DbError::Other(format!(
            "manifest dei simboli non leggibile ({}): {e}",
            manifest_path.display()
        ))
    })?;
    let manifest: PackManifest = serde_json::from_str(&manifest_raw)
        .map_err(|e| super::DbError::Other(format!("manifest non valido: {e}")))?;

    let tx = db.conn_mut().transaction()?;
    let pack = &manifest.pack;

    tx.execute(
        "INSERT INTO symbol_packs(id, name, version, description, author, rights_holder, license,
            license_url, source_url, constraints, commercial_ok, share_alike, no_derivatives, builtin, installed_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
         ON CONFLICT(id) DO UPDATE SET
            name=excluded.name, version=excluded.version, description=excluded.description,
            author=excluded.author, rights_holder=excluded.rights_holder, license=excluded.license,
            license_url=excluded.license_url, source_url=excluded.source_url,
            constraints=excluded.constraints, commercial_ok=excluded.commercial_ok,
            share_alike=excluded.share_alike, no_derivatives=excluded.no_derivatives,
            builtin=excluded.builtin, installed_at=excluded.installed_at",
        params![
            pack.id,
            pack.name,
            pack.version,
            pack.description,
            pack.author,
            pack.rights_holder,
            pack.license,
            pack.license_url,
            pack.source_url,
            pack.constraints,
            pack.commercial_ok as i64,
            pack.share_alike as i64,
            pack.no_derivatives as i64,
            pack.builtin as i64,
            now_iso()
        ],
    )?;

    // Concetti + significati.
    // `broader` ha una foreign key: un riferimento a un concetto assente
    // farebbe fallire l'intera transazione, quindi viene filtrato.
    let concept_ids: std::collections::HashSet<&str> =
        lexicon.concepts.iter().map(|c| c.id.as_str()).collect();
    // Primo passaggio: tutti i concetti senza gerarchia. La foreign key su
    // `broader` è immediata, quindi i padri devono esistere prima del
    // collegamento: la gerarchia viene applicata in un secondo passaggio.
    for concept in &lexicon.concepts {
        tx.execute(
            "INSERT INTO concepts(id, label, meaning, category, broader, pos, source, commercial_ok)
             VALUES (?1,?2,?3,?4,NULL,?5,'lexicon',?6)
             ON CONFLICT(id) DO UPDATE SET
                label=excluded.label, meaning=excluded.meaning, category=excluded.category,
                pos=excluded.pos, commercial_ok=excluded.commercial_ok",
            params![
                concept.id,
                concept.label,
                concept.meaning,
                concept.category,
                concept.pos,
                pack.commercial_ok as i64
            ],
        )?;
        stats.concepts += 1;
    }

    // Secondo passaggio: gerarchia e relazioni `broader`.
    for concept in &lexicon.concepts {
        let broader = concept
            .broader
            .as_deref()
            .filter(|parent| concept_ids.contains(parent));
        tx.execute(
            "UPDATE concepts SET broader = ?2 WHERE id = ?1",
            params![concept.id, broader],
        )?;
        if let Some(broader) = broader {
            tx.execute(
                "INSERT INTO semantic_relations(from_concept, to_concept, kind, weight)
                 SELECT ?1, ?2, 'broader', 1.0
                 WHERE EXISTS (SELECT 1 FROM concepts WHERE id = ?2)",
                params![concept.id, broader],
            )?;
        }
        // Le cue sono informative anche dal database (ricerca e script).
        let _ = (concept.cues.prefer.len(), concept.cues.avoid.len());
    }

    // Lemmi e flessioni.
    let mut form_index: HashMap<String, String> = HashMap::new();
    for lemma in &lexicon.lemmas {
        tx.execute(
            "INSERT INTO lemmas(text, pos, gender, number) VALUES (?1,?2,?3,?4)
             ON CONFLICT(text, pos) DO UPDATE SET gender=excluded.gender, number=excluded.number",
            params![lemma.lemma, lemma.pos, lemma.gender, lemma.number],
        )?;
        stats.lemmas += 1;
        for concept_id in &lemma.concepts {
            // Il seed può essere rieseguito quando cambia la versione del
            // pacchetto: senza il controllo si accumulano righe duplicate.
            tx.execute(
                "INSERT INTO meanings(concept_id, lemma_text, pos, meaning, priority, cues_prefer, cues_avoid)
                 SELECT ?1, ?2, ?3, meaning, 100, '', '' FROM concepts
                 WHERE id = ?1 AND NOT EXISTS (
                    SELECT 1 FROM meanings
                    WHERE concept_id = ?1 AND lemma_text = ?2 AND pos = ?3
                 )",
                params![concept_id, lemma.lemma, lemma.pos],
            )?;
        }
        for form in generate_forms(&lemma.lemma, &lemma.pos, lemma.gender.as_deref()) {
            form_index.insert(form.form.clone(), lemma.lemma.clone());
            tx.execute(
                "INSERT INTO inflections(lemma_text, form, mood, tense, person, number, gender)
                 SELECT ?1,?2,?3,?4,?5,?6,?7
                 WHERE NOT EXISTS (
                    SELECT 1 FROM inflections WHERE lemma_text = ?1 AND form = ?2
                 )",
                params![
                    lemma.lemma,
                    form.form,
                    form.mood,
                    form.tense,
                    form.person,
                    form.number,
                    form.gender
                ],
            )?;
            stats.forms += 1;
        }
    }

    // Sinonimi.
    for synonym in &lexicon.synonyms {
        for concept_id in &synonym.concepts {
            tx.execute(
                "INSERT INTO synonyms(concept_id, term, weight) SELECT ?1, ?2, 1.0
                 WHERE EXISTS (SELECT 1 FROM concepts WHERE id = ?1)
                   AND NOT EXISTS (
                    SELECT 1 FROM synonyms WHERE concept_id = ?1 AND term = ?2
                   )",
                params![concept_id, synonym.term],
            )?;
        }
    }

    // Espressioni multiword.
    for mwe in &lexicon.mwe {
        let normalized = crate::linguistics::normalize::normalize_word(&mwe.form);
        tx.execute(
            "INSERT INTO expressions(form, normalized, concept_id, kind) VALUES (?1,?2,?3,'mwe')
             ON CONFLICT(normalized) DO UPDATE SET concept_id=excluded.concept_id",
            params![mwe.form, normalized, mwe.concept],
        )?;
        stats.expressions += 1;
    }

    // Simboli del pacchetto.
    for symbol in &manifest.symbols {
        let Some(file) = &symbol.file else {
            stats.skipped += 1;
            stats
                .warnings
                .push(format!("simbolo {} senza file", symbol.id));
            continue;
        };
        let path = pack_dir.join(file);
        let raw_svg = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(e) => {
                stats
                    .warnings
                    .push(format!("{}: {e}", path.display()));
                stats.skipped += 1;
                continue;
            }
        };
        let sanitized = match svg::sanitize(&raw_svg, MAX_SVG_BYTES) {
            Ok(value) => value,
            Err(e) => {
                stats.warnings.push(format!("{}: {e}", symbol.id));
                stats.skipped += 1;
                continue;
            }
        };
        for warning in &sanitized.warnings {
            stats
                .warnings
                .push(format!("{}: {warning}", symbol.id));
        }
        let checksum = {
            use sha2::Digest;
            let digest = sha2::Sha256::digest(sanitized.svg.as_bytes());
            crate::asset_manager::hex(&digest)
        };
        let status = symbol
            .status
            .clone()
            .unwrap_or_else(|| "approved".to_string());

        // I simboli legati a un concetto sono quelli che la simbolizzazione
        // legge a ogni parola: restano nel database (sono un centinaio). La
        // libreria ARASAAC, migliaia di simboli consultati solo cercando,
        // resta su disco per non gonfiare il database di centinaia di MB.
        let concept_bound = symbol.concept.is_some();
        let storage_kind = if concept_bound { "db" } else { "file" };
        let inline_svg = concept_bound.then(|| sanitized.svg.clone());

        tx.execute(
            "INSERT INTO symbols(id, concept_id, pack_id, label, meaning, author, rights_holder,
                license, license_url, source_url, constraints, modifications, format, storage,
                data_path, status, hide, checksum, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'svg',?13,?14,?15,0,?16,?17)
             ON CONFLICT(id) DO UPDATE SET
                concept_id=excluded.concept_id, label=excluded.label, meaning=excluded.meaning,
                license=excluded.license, license_url=excluded.license_url,
                source_url=excluded.source_url, constraints=excluded.constraints,
                status=excluded.status, checksum=excluded.checksum",
            params![
                symbol.id,
                symbol.concept,
                pack.id,
                symbol.label,
                symbol.meaning,
                pack.author,
                pack.rights_holder,
                pack.license,
                pack.license_url,
                pack.source_url,
                pack.constraints,
                "Nessuna modifica: pittogramma originale del pacchetto.",
                storage_kind,
                path.to_string_lossy().to_string(),
                status,
                checksum,
                now_iso()
            ],
        )?;

        tx.execute("DELETE FROM symbol_assets WHERE symbol_id = ?1", params![symbol.id])?;
        // Gli asset su disco vengono riletti e sanificati a ogni lettura
        // (vedi `Database::map_symbol`), quindi non si perde la validazione.
        tx.execute(
            "INSERT INTO symbol_assets(symbol_id, kind, storage, data, path, width, height, bytes)
             VALUES (?1,'original',?2,?3,?4,500,500,?5)",
            params![
                symbol.id,
                storage_kind,
                inline_svg,
                path.to_string_lossy().to_string(),
                sanitized.bytes as i64
            ],
        )?;

        tx.execute(
            "INSERT INTO symbol_licenses(symbol_id, license, license_url, attribution, commercial_ok,
                share_alike, no_derivatives, source_url)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(symbol_id) DO UPDATE SET license=excluded.license,
                license_url=excluded.license_url, attribution=excluded.attribution,
                commercial_ok=excluded.commercial_ok, share_alike=excluded.share_alike,
                no_derivatives=excluded.no_derivatives, source_url=excluded.source_url",
            params![
                symbol.id,
                pack.license,
                pack.license_url,
                format!("{} — {}", pack.name, pack.author.clone().unwrap_or_default()),
                pack.commercial_ok as i64,
                pack.share_alike as i64,
                pack.no_derivatives as i64,
                pack.source_url
            ],
        )?;

        // Indice di ricerca: etichetta, significato, concetto, categoria, sinonimi.
        tx.execute("DELETE FROM symbol_fts WHERE symbol_id = ?1", params![symbol.id])?;
        let mut text = format!("{} {}", symbol.label, symbol.meaning.clone().unwrap_or_default());
        for keyword in &symbol.keywords {
            text.push(' ');
            text.push_str(keyword);
        }
        if let Some(concept_id) = &symbol.concept {
            if let Some(concept) = lexicon.concepts.iter().find(|c| &c.id == concept_id) {
                text.push_str(&format!(
                    " {} {} {}",
                    concept.label, concept.meaning, concept.category
                ));
            }
            let mut stmt = tx.prepare("SELECT term FROM synonyms WHERE concept_id = ?1")?;
            let terms = stmt.query_map(params![concept_id], |row| row.get::<_, String>(0))?;
            for term in terms {
                text.push(' ');
                text.push_str(&term?);
            }
        }
        tx.execute(
            "INSERT INTO symbol_fts(symbol_id, text) VALUES (?1, ?2)",
            params![symbol.id, text],
        )?;
        stats.symbols += 1;
    }

    // Relazioni semantiche aggiuntive dichiarate nel lessico.
    for relation in &lexicon.relations {
        tx.execute(
            "INSERT INTO semantic_relations(from_concept, to_concept, kind, weight)
             SELECT ?1, ?2, ?3, ?4
             WHERE EXISTS (SELECT 1 FROM concepts WHERE id = ?1)
               AND EXISTS (SELECT 1 FROM concepts WHERE id = ?2)",
            params![
                relation.from,
                relation.to,
                relation.kind,
                relation.weight as f64
            ],
        )?;
    }

    // Legami concetto → pittogramma di repertorio dichiarati nel lessico.
    // Il manifest ARASAAC non è versionato: senza questo passaggio i concetti
    // con un simbolo esterno dipenderebbero da modifiche locali al pacchetto.
    for concept in &lexicon.concepts {
        if let Some(symbol_id) = &concept.symbol_ref {
            tx.execute(
                "UPDATE symbols SET concept_id = ?1 WHERE id = ?2",
                params![concept.id, symbol_id],
            )?;
        }
    }

    tx.execute(
        "INSERT INTO settings(key, value) VALUES ('seed.lexicon_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![lexicon.version.to_string()],
    )?;
    tx.commit()?;

    db.insert_import_log(
        Some(&pack.id),
        "seed",
        &format!(
            "concetti={} lemmi={} forme={} simboli={}",
            stats.concepts, stats.lemmas, stats.forms, stats.symbols
        ),
        &stats.warnings.join(" | "),
    );

    Ok(stats)
}

impl Database {
    pub(crate) fn conn_mut(&mut self) -> &mut rusqlite::Connection {
        &mut self.conn
    }
}
