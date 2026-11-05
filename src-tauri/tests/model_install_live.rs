//! Prova reale del download verificato di un modello dal catalogo.
//!
//! Non fa parte dell'esecuzione ordinaria: scarica circa 1,1 GB da Hugging
//! Face. Si esegue solo su richiesta:
//!
//! ```text
//! cargo test --test model_install_live -- --ignored --nocapture
//! ```
//!
//! Verifica l'intera catena usata dall'interfaccia: curl di sistema, ripresa
//! del file parziale, ricalcolo dell'impronta sha256 e collocazione del file
//! nella cartella modelli.

use std::time::{Duration, Instant};

use caa_writer_lib::ai_engine::catalog;
use caa_writer_lib::ai_engine::installer::{InstallPhase, ModelInstaller};
use sha2::{Digest, Sha256};

#[test]
#[ignore = "scarica il modello da Hugging Face: eseguire solo su richiesta"]
fn downloads_and_verifies_qwen_1_5b() {
    let entry = catalog::find("qwen2.5-1.5b-instruct-q4_k_m").expect("modello nel catalogo");
    assert!(entry.installable(), "il modello deve essere scaricabile");
    let expected = entry.sha256.clone().expect("impronta pubblicata");

    let dir = tempfile::tempdir().expect("cartella temporanea");
    let installer = ModelInstaller::new(dir.path().join("models"));
    let started = Instant::now();
    installer.start(&entry).expect("download avviato");

    let mut last_percent = -1i32;
    loop {
        std::thread::sleep(Duration::from_millis(500));
        let status = installer.status();
        let percent = status.percent as i32;
        if percent != last_percent && percent % 10 == 0 {
            println!(
                "{}% ({} / {} MB) · {}",
                percent,
                status.downloaded_bytes / 1_048_576,
                status.total_bytes / 1_048_576,
                status.message
            );
            last_percent = percent;
        }
        match status.phase {
            InstallPhase::Downloading | InstallPhase::Verifying => {
                assert!(
                    started.elapsed() < Duration::from_secs(45 * 60),
                    "download oltre il limite di 45 minuti"
                );
                continue;
            }
            InstallPhase::Ready => break,
            other => panic!("download terminato in stato {other:?}: {}", status.message),
        }
    }

    let path = dir.path().join("models").join(&entry.file_name);
    let metadata = std::fs::metadata(&path).expect("file scaricato");
    assert_eq!(metadata.len(), entry.size_bytes, "dimensione del file");

    let mut file = std::fs::File::open(&path).expect("lettura file");
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    use std::io::Read;
    loop {
        let read = file.read(&mut buffer).expect("lettura");
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = format!("{:x}", hasher.finalize());
    assert_eq!(actual, expected, "le due impronte devono coincidere");
    assert!(
        !dir.path().join("models").join(format!("{}.part", entry.file_name)).exists(),
        "il file parziale deve essere stato rinominato"
    );
    println!(
        "Modello verificato in {:.1} s: {} ({} MB)",
        started.elapsed().as_secs_f32(),
        entry.file_name,
        metadata.len() / 1_048_576
    );
}
