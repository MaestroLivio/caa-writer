//! Misure di prestazione del motore sul computer di sviluppo.
//!
//! ATTENZIONE: queste misure NON sostituiscono la verifica richiesta su
//! Windows 10/11 con 4 GB di RAM e 2 core. Servono a individuare regressioni
//! evidenti (tempi per paragrafo, effetto della cache) e a documentare numeri
//! realmente osservati sulla macchina di sviluppo.

use std::path::PathBuf;
use std::time::Instant;

use caa_writer_lib::engine::Engine;
use caa_writer_lib::model::{CaaMode, Intensity, SymbolizeOptions};
use caa_writer_lib::paths::AppPaths;

fn resources_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources")
}

fn build_engine() -> Engine {
    let dir = tempfile::TempDir::new().expect("cartella temporanea");
    let paths = AppPaths::new(dir.path().to_path_buf(), resources_dir());
    let engine = Engine::new(paths).expect("motore inizializzabile");
    std::mem::forget(dir);
    engine
}

fn options(id: &str) -> SymbolizeOptions {
    SymbolizeOptions {
        intensity: Intensity::Media,
        mode: CaaMode::Sopra,
        document_key: "bench".to_string(),
        commercial_only: false,
        pack_id: None,
        paragraph_id: id.to_string(),
        symbolize_articles: false,
        symbolize_prepositions: false,
        symbolize_conjunctions: false,
    }
}

fn paragraphs(count: usize) -> Vec<(String, String)> {
    let templates = [
        "La maestra scrive alla lavagna e i bambini ascoltano.",
        "Il bambino mangia una mela nel cortile della scuola.",
        "La mamma prepara la pasta per il pranzo della famiglia.",
        "Il cane corre nel parco con la palla rossa.",
        "Oggi il tempo è brutto e piove sulla città.",
        "Il nonno va a pesca al fiume la domenica mattina.",
        "La bambina disegna un fiore con la matita rossa.",
        "Domani andiamo a scuola con lo zaino pesante.",
        "Il gatto dorme sul cuscino del letto.",
        "Il medico visita il bambino che ha la febbre.",
    ];
    (0..count)
        .map(|index| {
            let text = format!("{} Paragrafo numero {}.", templates[index % templates.len()], index);
            (format!("p{index}"), text)
        })
        .collect()
}

#[test]
fn paragraph_latency_and_cache() {
    let engine = build_engine();
    let corpus = paragraphs(50);

    // Primo giro: nessuna cache, ogni paragrafo è nuovo.
    let cold_start = Instant::now();
    let mut total_first_ms = 0u128;
    for (id, text) in &corpus {
        let started = Instant::now();
        let result = engine.symbolize(text, &options(id)).expect("simbolizzazione");
        total_first_ms += started.elapsed().as_millis();
        assert!(!result.tokens.is_empty());
    }
    let cold_total = cold_start.elapsed();

    // Secondo giro: stessi paragrafi, cache calda.
    let warm_start = Instant::now();
    for (id, text) in &corpus {
        engine.symbolize(text, &options(id)).expect("simbolizzazione");
    }
    let warm_total = warm_start.elapsed();

    println!("\n=== Prestazioni motore (macchina di sviluppo) ===");
    println!(
        "50 paragrafi senza cache: {} ms totali ({:.2} ms/paragrafo)",
        cold_total.as_millis(),
        cold_total.as_millis() as f64 / corpus.len() as f64
    );
    println!(
        "50 paragrafi con cache:  {} ms totali ({:.2} ms/paragrafo)",
        warm_total.as_millis(),
        warm_total.as_millis() as f64 / corpus.len() as f64
    );
    println!("somma dei tempi misurati dal motore: {total_first_ms} ms");
    println!("piattaforma: {}", std::env::consts::OS);

    assert!(
        warm_total < cold_total,
        "la cache dei paragrafi non produce alcun vantaggio misurabile"
    );

    // Documento lungo simulato: 50 pagine ~ 1500 paragrafi.
    let long_document = paragraphs(1500);
    let long_start = Instant::now();
    for (id, text) in &long_document {
        engine.symbolize(text, &options(id)).expect("simbolizzazione");
    }
    let long_total = long_start.elapsed();
    println!(
        "1500 paragrafi (documento lungo simulato): {} ms ({:.2} ms/paragrafo)",
        long_total.as_millis(),
        long_total.as_millis() as f64 / long_document.len() as f64
    );
    assert!(long_total.as_secs() < 120, "documento lungo troppo lento");
}
