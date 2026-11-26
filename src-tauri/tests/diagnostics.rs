//! Diagnostica del primo avvio: stampa avvisi e conteggi reali del database.
//!
//! Serve a capire *perché* un'importazione non ha prodotto simboli, senza
//! indovinare. Non sostituisce i test funzionali.

use std::path::PathBuf;

use caa_writer_lib::engine::Engine;
use caa_writer_lib::model::{CaaMode, Intensity, SymbolizeOptions};
use caa_writer_lib::paths::AppPaths;

#[test]
fn first_start_diagnostics() {
    let dir = tempfile::tempdir().expect("tempdir");
    let resources = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources");
    println!("resources: {}", resources.display());
    println!(
        "manifest: {}",
        resources.join("symbol-packs/core/manifest.json").exists()
    );

    let paths = AppPaths::new(dir.path().to_path_buf(), resources.clone());
    let engine = Engine::new(paths).expect("engine");
    std::mem::forget(dir);

    println!("startup_warnings: {:?}", engine.startup_warnings);
    let stats = engine.db().expect("db").statistics().expect("stats");
    println!(
        "stats: symbols={} concepts={} lemmas={} packs={} expressions={}",
        stats.symbols, stats.concepts, stats.lemmas, stats.packs, stats.expressions
    );

    let opts = SymbolizeOptions {
        intensity: Intensity::Media,
        mode: CaaMode::Sopra,
        document_key: "diag".to_string(),
        commercial_only: false,
        pack_id: None,
        paragraph_id: "p1".to_string(),
        symbolize_articles: false,
        symbolize_prepositions: false,
        symbolize_conjunctions: false,
        manual_units: Vec::new(),
        split_ranges: Vec::new(),
        profile_id: None,
    };
    let result = engine.symbolize("Luca mangia una pesca.", &opts).unwrap();
    for token in &result.tokens {
        println!(
            "  token {:?} concept={:?} symbol={:?} strategy={} alt={}",
            token.surface,
            token.concept_id,
            token.symbol_id,
            token.strategy,
            token.alternatives.len()
        );
    }

    // The bundled Core pack was removed in 0.16.4. External packs may already
    // be available in the local resources tree, so check pack identity rather
    // than requiring the database to contain no symbols.
    assert!(stats.concepts > 0, "lessico iniziale non importato");
    assert!(
        engine
            .db()
            .expect("db")
            .list_packs()
            .expect("packs")
            .iter()
            .all(|pack| pack.id != "core"),
        "il pacchetto Core interno non deve essere importato"
    );
    assert!(
        result
            .tokens
            .iter()
            .any(|token| token.concept_id.as_deref() == Some("fruit.peach")),
        "la diagnostica deve comunque riconoscere il concetto «pesca»"
    );
}
