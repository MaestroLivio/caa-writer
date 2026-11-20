//! Manifest, verifica e installazione del modello Horizon.
//!
//! I pesi non vivono nel repository: vengono installati in
//! `<data-dir>/models/horizon-multilingual-zeroshot-small`.
//! Revisione, hash e dimensioni sono quelli pubblicati su Hugging Face e
//! verificati durante lo sviluppo.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::paths::AppPaths;

pub const MODEL_ID: &str = "horizon-multilingual-zeroshot-small";
pub const REPOSITORY: &str = "Horizon-Labs/multilingual-zeroshot-small";
pub const REVISION: &str = "17af5f8a728d101ce33a9bffad227b0cd005e95e";
pub const LICENSE: &str = "Apache-2.0";
pub const RUNTIME: &str = "onnxruntime";
pub const RUNTIME_VERSION: &str = "1.20";
pub const TOKENIZER_VERSION: &str = "tokenizers-0.21";
pub const PARAMETERS: u64 = 141_000_000;
pub const QUANTIZED_BYTES: u64 = 268_409_234;

pub struct ModelFile {
    pub path: &'static str,
    pub sha256: &'static str,
    pub bytes: u64,
}

pub const FILES: &[ModelFile] = &[
    ModelFile {
        path: "config.json",
        sha256: "3e5f7eebf8ddee141f092672ca5865b7741e64df974fbc68223c91da6cd74222",
        bytes: 2_053,
    },
    ModelFile {
        path: "tokenizer.json",
        sha256: "aebee76d0312011b0d73b08a255d4d11d25c83e9639324870c7b916ea102e13e",
        bytes: 34_363_287,
    },
    ModelFile {
        path: "tokenizer_config.json",
        sha256: "ea349495764b24570b4560fe2e4a557e5c0a47e6bce782c2409b1d6445cf8633",
        bytes: 46_440,
    },
    ModelFile {
        path: "special_tokens_map.json",
        sha256: "baec30ea10906f16adb8c18af7a34023002c1746542612b8b41c9f09e1351351",
        bytes: 636,
    },
    ModelFile {
        path: "onnx/model_quantized.onnx",
        sha256: "0ac394942a086fb1c02beb0fde0357bf7f3bd739ca07af7fa6c9fa11c9c05512",
        bytes: 268_409_234,
    },
    ModelFile {
        path: "SHA256SUMS",
        sha256: "2b976590b16df67e17b00f8333e37bc931420dae6e4aa440be867b45312d1bfa",
        bytes: 690,
    },
    ModelFile {
        path: "LICENSE",
        sha256: "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30",
        bytes: 11_358,
    },
    ModelFile {
        path: "NOTICE",
        sha256: "ead54ffa29a7bda1a08ac4d006f5bc30693e0044e3197bde8e59131746ea4d9f",
        bytes: 1_565,
    },
];

#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    pub model_id: String,
    pub repository: String,
    pub revision: String,
    pub license: String,
    pub runtime: String,
    pub runtime_version: String,
    pub parameters: u64,
    pub quantized_bytes: u64,
    pub model_dir: String,
    pub installed: bool,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileCheck {
    pub path: String,
    pub expected_sha256: String,
    pub actual_sha256: Option<String>,
    pub ok: bool,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifyReport {
    pub ok: bool,
    pub files: Vec<FileCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallReport {
    pub installed: bool,
    pub model_dir: String,
    pub bytes: u64,
    pub files: Vec<String>,
}

pub fn model_dir(paths: &AppPaths) -> PathBuf {
    paths.models_dir.join(MODEL_ID)
}

pub fn status(paths: &AppPaths) -> ModelStatus {
    let dir = model_dir(paths);
    let missing: Vec<String> = FILES
        .iter()
        .filter(|file| !dir.join(file.path).is_file())
        .map(|file| file.path.to_string())
        .collect();
    ModelStatus {
        model_id: MODEL_ID.to_string(),
        repository: REPOSITORY.to_string(),
        revision: REVISION.to_string(),
        license: LICENSE.to_string(),
        runtime: RUNTIME.to_string(),
        runtime_version: RUNTIME_VERSION.to_string(),
        parameters: PARAMETERS,
        quantized_bytes: QUANTIZED_BYTES,
        model_dir: dir.display().to_string(),
        installed: missing.is_empty(),
        missing,
    }
}

pub fn verify(paths: &AppPaths) -> VerifyReport {
    let dir = model_dir(paths);
    let mut files = Vec::new();
    let mut ok = true;
    for file in FILES {
        let path = dir.join(file.path);
        let actual = sha256_file(&path).ok();
        let file_ok = actual.as_deref() == Some(file.sha256);
        ok &= file_ok;
        files.push(FileCheck {
            path: file.path.to_string(),
            expected_sha256: file.sha256.to_string(),
            actual_sha256: actual,
            ok: file_ok,
            bytes: file.bytes,
        });
    }
    VerifyReport { ok, files }
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Installa i file ufficiali nella cartella modelli dell'utente.
///
/// L'installazione è atomica: i file vengono scritti in una cartella
/// temporanea, verificati con SHA-256 e solo allora spostati nella posizione
/// definitiva. Richiede rete; dopo l'installazione il modello funziona offline.
pub fn install(paths: &AppPaths, force: bool) -> Result<InstallReport, String> {
    let target = model_dir(paths);
    if target.is_dir() && !force {
        let existing = verify(paths);
        if existing.ok {
            return Ok(InstallReport {
                installed: true,
                model_dir: target.display().to_string(),
                bytes: FILES.iter().map(|file| file.bytes).sum(),
                files: FILES.iter().map(|file| file.path.to_string()).collect(),
            });
        }
    }
    std::fs::create_dir_all(&paths.models_dir)
        .map_err(|error| format!("cartella modelli: {error}"))?;
    let temp = paths
        .models_dir
        .join(format!(".{MODEL_ID}.tmp-{}", std::process::id()));
    if temp.exists() {
        std::fs::remove_dir_all(&temp).map_err(|error| format!("pulizia temporanea: {error}"))?;
    }
    std::fs::create_dir_all(temp.join("onnx"))
        .map_err(|error| format!("cartella temporanea: {error}"))?;

    let result = (|| -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(900))
            .user_agent("caa-writer/decision-model-installer")
            .build()
            .map_err(|error| format!("client HTTP: {error}"))?;
        for file in FILES {
            let url = format!(
                "https://huggingface.co/{REPOSITORY}/resolve/{REVISION}/{}",
                file.path
            );
            let destination = temp.join(file.path);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("cartella {}: {error}", parent.display()))?;
            }
            download_verified(&client, &url, &destination, file.sha256, file.bytes)?;
        }
        Ok(())
    })();

    if let Err(error) = result {
        let _ = std::fs::remove_dir_all(&temp);
        return Err(error);
    }

    if target.exists() {
        std::fs::remove_dir_all(&target)
            .map_err(|error| format!("rimozione versione precedente: {error}"))?;
    }
    std::fs::rename(&temp, &target).map_err(|error| format!("installazione atomica: {error}"))?;

    // Manifest copiato accanto ai pesi per diagnostica offline.
    let manifest = serde_json::json!({
        "model_id": MODEL_ID,
        "repository": REPOSITORY,
        "revision": REVISION,
        "runtime": RUNTIME,
        "runtime_version": RUNTIME_VERSION,
        "tokenizer_version": TOKENIZER_VERSION,
        "license": LICENSE,
        "parameters": PARAMETERS,
        "quantized_bytes": QUANTIZED_BYTES,
        "files": FILES.iter().map(|file| serde_json::json!({
            "path": file.path,
            "sha256": file.sha256,
            "bytes": file.bytes,
        })).collect::<Vec<_>>(),
    });
    std::fs::write(
        target.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("manifest: {error}"))?;

    Ok(InstallReport {
        installed: true,
        model_dir: target.display().to_string(),
        bytes: FILES.iter().map(|file| file.bytes).sum(),
        files: FILES.iter().map(|file| file.path.to_string()).collect(),
    })
}

fn download_verified(
    client: &reqwest::blocking::Client,
    url: &str,
    destination: &Path,
    expected_sha256: &str,
    expected_bytes: u64,
) -> Result<(), String> {
    let mut response = client
        .get(url)
        .send()
        .map_err(|error| format!("{url}: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("{url}: HTTP {}", response.status()));
    }
    let mut file =
        File::create(destination).map_err(|error| format!("{}: {error}", destination.display()))?;
    let mut hasher = Sha256::new();
    let mut written = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|error| format!("{url}: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .map_err(|error| format!("{}: {error}", destination.display()))?;
        written += read as u64;
    }
    file.flush()
        .map_err(|error| format!("{}: {error}", destination.display()))?;
    if written != expected_bytes {
        return Err(format!(
            "{}: dimensione {written} diversa da {expected_bytes}",
            destination.display()
        ));
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected_sha256 {
        return Err(format!(
            "{}: SHA-256 {actual} diverso da {expected_sha256}",
            destination.display()
        ));
    }
    Ok(())
}

pub fn remove(paths: &AppPaths) -> Result<bool, String> {
    let dir = model_dir(paths);
    if !dir.exists() {
        return Ok(false);
    }
    std::fs::remove_dir_all(&dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_internally_consistent() {
        assert_eq!(FILES.len(), 8);
        for file in FILES {
            assert_eq!(file.sha256.len(), 64, "{}", file.path);
            assert!(file.bytes > 0, "{}", file.path);
        }
        let total: u64 = FILES.iter().map(|file| file.bytes).sum();
        assert!(total > QUANTIZED_BYTES);
    }

    #[test]
    fn status_reports_missing_files_without_model() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = AppPaths::new(dir.path().to_path_buf(), dir.path().to_path_buf());
        let status = status(&paths);
        assert!(!status.installed);
        assert_eq!(status.missing.len(), FILES.len());
    }

    #[test]
    fn sha256_matches_known_value() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("probe.txt");
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(
            sha256_file(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn embedded_manifest_matches_constants() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("radice repository")
            .join("resources/decision/horizon-multilingual-zeroshot-small.manifest.json");
        let text = std::fs::read_to_string(&path).expect("manifest versionato");
        let manifest: serde_json::Value = serde_json::from_str(&text).expect("JSON valido");
        assert_eq!(manifest["model_id"], MODEL_ID);
        assert_eq!(manifest["repository"], REPOSITORY);
        assert_eq!(manifest["revision"], REVISION);
        assert_eq!(manifest["license"], LICENSE);
        assert_eq!(manifest["runtime_version"], RUNTIME_VERSION);
        let files = manifest["files"].as_array().expect("files");
        assert_eq!(files.len(), FILES.len());
        for expected in FILES {
            let found = files
                .iter()
                .find(|file| file["path"] == expected.path)
                .unwrap_or_else(|| panic!("file mancante nel manifest: {}", expected.path));
            assert_eq!(found["sha256"], expected.sha256, "{}", expected.path);
            assert_eq!(
                found["bytes"].as_u64(),
                Some(expected.bytes),
                "{}",
                expected.path
            );
        }
    }
}
