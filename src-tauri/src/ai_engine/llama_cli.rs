//! Ponte con llama.cpp via processo locale.
//!
//! Perché un processo e non un binding FFI:
//! - non aggiunge una toolchain C++ al build (Windows e macOS restano semplici);
//! - consente di sostituire versione/backend di llama.cpp senza ricompilare;
//! - il processo si può terminare a timeout, cosa essenziale su 4 GB di RAM.
//!
//! Limite dichiarato: serve installare llama.cpp e scaricare un modello GGUF.
//! Senza di essi il software funziona con la sola selezione deterministica.

use std::io::Read;
use std::process::Stdio;
use std::time::{Duration, Instant};

use crate::model::AiConfig;

/// Esegue una completion one-shot con temperatura 0 e contesto ridotto.
pub fn run_completion(config: &AiConfig, prompt: &str) -> Result<String, String> {
    if !crate::ai_engine::is_available(config) {
        return Err(
            "motore IA non configurato: indica il percorso di llama.cpp e del modello GGUF nelle impostazioni"
                .to_string(),
        );
    }

    let out_file = std::env::temp_dir().join(format!(
        "caa-writer-ai-{}-{}.txt",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    ));

    let stdout = std::fs::File::create(&out_file).map_err(|e| format!("file temporaneo: {e}"))?;

    let mut command = crate::process::command(&config.binary_path);
    command
        .arg("-m")
        .arg(&config.model_path)
        .arg("-p")
        .arg(prompt)
        .arg("-n")
        .arg("64")
        .arg("--temp")
        .arg("0")
        .arg("-c")
        .arg(config.context_size.to_string())
        .arg("-t")
        .arg(config.threads.to_string())
        .arg("--no-display-prompt")
        // `-st` (single turn): la build inclusa non accetta più `-no-cnv`.
        .arg("--single-turn")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::null());

    // Backend: su Apple Silicon si prova Metal, ma la CPU resta il default.
    let ngpu = match config.backend.as_str() {
        "metal" => Some("999"),
        "cpu" => Some("0"),
        _ => None,
    };
    if let Some(value) = ngpu {
        command.arg("-ngl").arg(value);
    }
    for extra in &config.extra_args {
        command.arg(extra);
    }

    let mut child = command
        .spawn()
        .map_err(|e| format!("impossibile avviare llama.cpp ({}): {e}", config.binary_path))?;

    let deadline = Duration::from_millis(config.timeout_ms.clamp(1000, 60_000));
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if started.elapsed() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(&out_file);
                    return Err(format!(
                        "il modello non ha risposto entro {} ms: richiesta annullata",
                        deadline.as_millis()
                    ));
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(e) => {
                let _ = std::fs::remove_file(&out_file);
                return Err(format!("errore nel processo del modello: {e}"));
            }
        }
    }

    let mut content = String::new();
    std::fs::File::open(&out_file)
        .and_then(|mut f| f.read_to_string(&mut content).map(|_| ()))
        .map_err(|e| format!("lettura risposta del modello: {e}"))?;
    let _ = std::fs::remove_file(&out_file);

    if content.trim().is_empty() {
        return Err("il modello non ha prodotto output".to_string());
    }
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_configuration_reports_clear_error() {
        let config = AiConfig {
            enabled: true,
            binary_path: "/nonexistent/llama-cli".into(),
            model_path: "/nonexistent/model.gguf".into(),
            ..Default::default()
        };
        let err = run_completion(&config, "test").unwrap_err();
        assert!(err.contains("non configurato"));
    }
}
