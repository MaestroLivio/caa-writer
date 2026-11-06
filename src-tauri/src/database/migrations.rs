//! Migrazioni versionate del database SQLite.
//!
//! Regola: ogni modifica allo schema è una nuova migrazione. Le migrazioni
//! applicate sono registrate in `schema_migrations` e non vengono riscritte.

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial_schema",
    sql: r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS concepts (
    id              TEXT PRIMARY KEY,
    label           TEXT NOT NULL,
    meaning         TEXT NOT NULL,
    category        TEXT NOT NULL,
    broader         TEXT REFERENCES concepts(id),
    pos             TEXT,
    source          TEXT NOT NULL DEFAULT 'lexicon',
    commercial_ok   INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS lemmas (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    text            TEXT NOT NULL,
    pos             TEXT NOT NULL,
    gender          TEXT,
    number          TEXT,
    UNIQUE(text, pos)
);

CREATE TABLE IF NOT EXISTS meanings (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    concept_id      TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    lemma_text      TEXT NOT NULL,
    pos             TEXT,
    meaning         TEXT NOT NULL,
    priority        INTEGER NOT NULL DEFAULT 100,
    cues_prefer     TEXT NOT NULL DEFAULT '',
    cues_avoid      TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_meanings_lemma ON meanings(lemma_text);

CREATE TABLE IF NOT EXISTS inflections (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    lemma_text      TEXT NOT NULL,
    form            TEXT NOT NULL,
    mood            TEXT,
    tense           TEXT,
    person          TEXT,
    number          TEXT,
    gender          TEXT
);
CREATE INDEX IF NOT EXISTS idx_inflections_form ON inflections(form);

CREATE TABLE IF NOT EXISTS synonyms (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    concept_id      TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    term            TEXT NOT NULL,
    weight          REAL NOT NULL DEFAULT 1.0
);
CREATE INDEX IF NOT EXISTS idx_synonyms_term ON synonyms(term);

CREATE TABLE IF NOT EXISTS semantic_relations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    from_concept    TEXT NOT NULL,
    to_concept      TEXT NOT NULL,
    kind            TEXT NOT NULL,
    weight          REAL NOT NULL DEFAULT 1.0
);
CREATE INDEX IF NOT EXISTS idx_relations_from ON semantic_relations(from_concept);

CREATE TABLE IF NOT EXISTS translations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    concept_id      TEXT NOT NULL,
    lang            TEXT NOT NULL,
    text            TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS expressions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    form            TEXT NOT NULL,
    normalized      TEXT NOT NULL UNIQUE,
    concept_id      TEXT NOT NULL,
    kind            TEXT NOT NULL DEFAULT 'mwe'
);
CREATE INDEX IF NOT EXISTS idx_expressions_concept ON expressions(concept_id);

CREATE TABLE IF NOT EXISTS symbol_packs (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    version         TEXT NOT NULL,
    description     TEXT,
    author          TEXT,
    rights_holder   TEXT,
    license         TEXT NOT NULL,
    license_url     TEXT,
    source_url      TEXT,
    constraints     TEXT,
    commercial_ok   INTEGER NOT NULL DEFAULT 0,
    share_alike     INTEGER NOT NULL DEFAULT 0,
    no_derivatives  INTEGER NOT NULL DEFAULT 0,
    builtin         INTEGER NOT NULL DEFAULT 0,
    installed_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS symbols (
    id              TEXT PRIMARY KEY,
    concept_id      TEXT REFERENCES concepts(id),
    pack_id         TEXT NOT NULL REFERENCES symbol_packs(id) ON DELETE CASCADE,
    label           TEXT NOT NULL,
    meaning         TEXT,
    author          TEXT,
    rights_holder   TEXT,
    license         TEXT NOT NULL,
    license_url     TEXT,
    source_url      TEXT,
    constraints     TEXT,
    modifications   TEXT,
    format          TEXT NOT NULL DEFAULT 'svg',
    storage         TEXT NOT NULL DEFAULT 'inline',
    data_path       TEXT,
    width           INTEGER,
    height          INTEGER,
    status          TEXT NOT NULL DEFAULT 'approved',
    hide            INTEGER NOT NULL DEFAULT 0,
    checksum        TEXT,
    created_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_symbols_concept ON symbols(concept_id);
CREATE INDEX IF NOT EXISTS idx_symbols_pack ON symbols(pack_id);

CREATE TABLE IF NOT EXISTS symbol_assets (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_id       TEXT NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL DEFAULT 'original',
    storage         TEXT NOT NULL DEFAULT 'db',
    data            TEXT,
    path            TEXT,
    width           INTEGER,
    height          INTEGER,
    bytes           INTEGER
);
CREATE INDEX IF NOT EXISTS idx_assets_symbol ON symbol_assets(symbol_id);

CREATE TABLE IF NOT EXISTS symbol_licenses (
    symbol_id       TEXT PRIMARY KEY REFERENCES symbols(id) ON DELETE CASCADE,
    license         TEXT NOT NULL,
    license_url     TEXT,
    attribution     TEXT,
    commercial_ok   INTEGER NOT NULL DEFAULT 0,
    share_alike     INTEGER NOT NULL DEFAULT 0,
    no_derivatives  INTEGER NOT NULL DEFAULT 0,
    source_url      TEXT
);

CREATE TABLE IF NOT EXISTS symbol_preferences (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    scope           TEXT NOT NULL,
    lemma           TEXT NOT NULL,
    context_key     TEXT,
    symbol_id       TEXT NOT NULL,
    concept_id      TEXT,
    locked          INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_pref_unique
    ON symbol_preferences(scope, lemma, IFNULL(context_key, ''));

CREATE TABLE IF NOT EXISTS document_overrides (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_key         TEXT NOT NULL,
    unit_key        TEXT NOT NULL,
    symbol_id       TEXT,
    concept_id      TEXT,
    locked          INTEGER NOT NULL DEFAULT 1,
    updated_at      TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_override_unique ON document_overrides(doc_key, unit_key);

CREATE TABLE IF NOT EXISTS generated_symbols (
    id              TEXT PRIMARY KEY,
    concept_id      TEXT,
    label           TEXT NOT NULL,
    svg             TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'to_review',
    prompt          TEXT,
    source          TEXT NOT NULL DEFAULT 'manual',
    created_at      TEXT NOT NULL,
    reviewed_by     TEXT,
    reviewed_at     TEXT
);

CREATE TABLE IF NOT EXISTS search_statistics (
    term            TEXT PRIMARY KEY,
    hits            INTEGER NOT NULL DEFAULT 0,
    last_used       TEXT
);

CREATE VIRTUAL TABLE IF NOT EXISTS symbol_fts USING fts5(
    symbol_id UNINDEXED,
    text,
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TABLE IF NOT EXISTS settings (
    key             TEXT PRIMARY KEY,
    value           TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS import_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    pack_id         TEXT,
    action          TEXT NOT NULL,
    detail          TEXT,
    warnings        TEXT,
    created_at      TEXT NOT NULL
);
"#,
}, Migration {
    version: 2,
    name: "unit_state_and_modifiers",
    sql: r#"
-- Stato per singola unità CAA: oltre alla scelta bloccata servono visibilità e
-- modificatori grafici (plurale, negazione, tempi). Sono proprietà dell'unità,
-- indipendenti dal testo: cambiare la parola non deve perdere il blocco.
ALTER TABLE document_overrides ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0;
ALTER TABLE document_overrides ADD COLUMN modifiers TEXT NOT NULL DEFAULT '{}';
"#,
}, Migration {
    version: 3,
    name: "pictogram_collections_online_prompt",
    sql: r#"
-- Collezioni personali di pittogrammi importati dall'utente.
CREATE TABLE IF NOT EXISTS pictogram_collections (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    description     TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_collections_name ON pictogram_collections(name);

ALTER TABLE symbols ADD COLUMN collection_id TEXT REFERENCES pictogram_collections(id) ON DELETE SET NULL;
ALTER TABLE symbols ADD COLUMN relative_path TEXT;
ALTER TABLE symbols ADD COLUMN original_name TEXT;
ALTER TABLE symbols ADD COLUMN category TEXT;
CREATE INDEX IF NOT EXISTS idx_symbols_collection ON symbols(collection_id);

-- Sinonimi e parole chiave dei pittogrammi personali: entrano nell'indice
-- semantico condiviso senza duplicare la tabella FTS.
CREATE TABLE IF NOT EXISTS symbol_keywords (
    symbol_id       TEXT NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    term            TEXT NOT NULL,
    PRIMARY KEY(symbol_id, term)
);

-- Cache delle risposte dei provider online (una voce per provider e query).
CREATE TABLE IF NOT EXISTS online_cache (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    provider        TEXT NOT NULL,
    query           TEXT NOT NULL,
    payload         TEXT NOT NULL,
    fetched_at      TEXT NOT NULL,
    UNIQUE(provider, query)
);

-- Cronologia delle modifiche richieste in linguaggio naturale: le operazioni
-- validate restano tracciabili e annullabili.
CREATE TABLE IF NOT EXISTS document_operations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    document_key    TEXT NOT NULL,
    prompt          TEXT NOT NULL,
    operations      TEXT NOT NULL,
    applied         INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL
);
"#,
}, Migration {
    version: 4,
    name: "profiles_and_boards",
    sql: r#"
-- Profili locali: nessun dato sanitario, solo preferenze operative.
CREATE TABLE IF NOT EXISTS profiles (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    avatar          TEXT NOT NULL DEFAULT '',
    age_theme       TEXT NOT NULL DEFAULT 'primaria',
    editor_level    INTEGER NOT NULL DEFAULT 2,
    features        TEXT NOT NULL DEFAULT '[]',
    voice           TEXT NOT NULL DEFAULT '{}',
    accessibility   TEXT NOT NULL DEFAULT '{}',
    navigation      TEXT NOT NULL DEFAULT '{}',
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS profile_boards (
    profile_id      TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    board_id        TEXT NOT NULL,
    position        INTEGER NOT NULL DEFAULT 0,
    enabled         INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY(profile_id, board_id)
);
"#,
}];
