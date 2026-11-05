//! Prova reale dell'istanza condivisa: avvia `llama-server` con il modello incluso,
//! esegue una richiesta, ripete la stessa richiesta (deve arrivare dalla cache) e
//! misura la latenza. Se le risorse non sono presenti la prova viene dichiarata
//! non eseguita invece di essere simulata.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use caa_writer_lib::ai_engine::service::{LocalModel, Service};
use caa_writer_lib::model::AiConfig;

#[test]
fn shared_server_answers_and_caches() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/ai");
    let server = root.join("llama-server");
    let model = root.join("model.gguf");
    if !server.exists() || !model.exists() {
        eprintln!(
            "risorse assenti ({} / {}): prova live non eseguita",
            server.display(),
            model.display()
        );
        return;
    }

    let config = AiConfig {
        enabled: true,
        binary_path: server.display().to_string(),
        model_path: model.display().to_string(),
        threads: 2,
        context_size: 1024,
        timeout_ms: 30_000,
        backend: "auto".to_string(),
        extra_args: Vec::new(),
    };
    let service = Service::new(
        Arc::new(LocalModel::new(root.clone(), config)),
        Duration::from_secs(60),
    );

    let prompt = "Scegli il significato di «pesca» in: Il nonno va a pesca. Opzioni: \
                  1. pesca — frutto 2. pesca — attività. Rispondi SOLO con il numero.\nRisposta:";

    let started = Instant::now();
    let first = service
        .complete(prompt.to_string(), 8, 10, service.cancel_handle())
        .expect("il modello deve rispondere");
    let first_ms = started.elapsed().as_millis();
    assert!(!first.trim().is_empty(), "risposta vuota dal modello");

    let started = Instant::now();
    let second = service
        .complete(prompt.to_string(), 8, 10, service.cancel_handle())
        .expect("la seconda richiesta deve riuscire");
    let second_ms = started.elapsed().as_millis();

    assert_eq!(first, second, "la cache deve restituire la stessa risposta");
    let metrics = service.metrics();
    assert!(metrics.cache_hits >= 1, "la cache non è stata usata");
    println!(
        "istanza condivisa: prima risposta {first_ms} ms, dalla cache {second_ms} ms, risposta {:?}",
        first.trim()
    );
}
