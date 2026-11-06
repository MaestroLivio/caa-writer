//! Installazione dei modelli: download verificato e importazione locale.
//!
//! Perché non una libreria HTTP: il download è un'azione facoltativa e
//! esplicita dell'utente, non una dipendenza del funzionamento offline.
//! Si usa `curl` di sistema (presente su macOS e su Windows 10/11, dove
//! gestisce anche i certificati TLS del sistema) e si evita di aggiungere
//! un client HTTPS con il suo albero di dipendenze al pacchetto.
//!
//! Regole di onestà:
//! - si scarica solo un modello il cui URL **e** `sha256` sono nel catalogo;
//! - l'impronta viene ricalcolata sul file scaricato: se non coincide, il
//!   file viene eliminato e l'errore è mostrato;
//! - un download interrotto lascia un `.part` ripreso alla successiva
//!   richiesta, non un file spacciato per completo;
//! - l'importazione manuale accetta solo file che iniziano con la firma
//!   `GGUF`: nessun file arbitrario viene copiato nella cartella modelli.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::catalog::ModelEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InstallPhase {
    #[default]
    Idle,
    Downloading,
    Verifying,
    Ready,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstallStatus {
    pub model_id: String,
    pub phase: InstallPhase,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: f32,
    pub message: String,
}

impl InstallStatus {
    pub fn busy(&self) -> bool {
        matches!(self.phase, InstallPhase::Downloading | InstallPhase::Verifying)
    }
}

struct Inner {
    models_dir: PathBuf,
    state: Mutex<InstallStatus>,
    cancel: AtomicBool,
    busy: AtomicBool,
}

/// Installatore condiviso: un solo download alla volta, stato interrogabile
/// dal frontend mentre il thread scarica.
#[derive(Clone)]
pub struct ModelInstaller {
    inner: Arc<Inner>,
}

impl ModelInstaller {
    pub fn new(models_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&models_dir);
        Self {
            inner: Arc::new(Inner {
                models_dir,
                state: Mutex::new(InstallStatus::default()),
                cancel: AtomicBool::new(false),
                busy: AtomicBool::new(false),
            }),
        }
    }

    pub fn models_dir(&self) -> PathBuf {
        self.inner.models_dir.clone()
    }

    pub fn status(&self) -> InstallStatus {
        self.inner
            .state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default()
    }

    fn set_state(&self, update: impl FnOnce(&mut InstallStatus)) {
        if let Ok(mut state) = self.inner.state.lock() {
            update(&mut state);
        }
    }

    pub fn cancel(&self) {
        self.inner.cancel.store(true, Ordering::SeqCst);
    }

    /// Avvia il download in background e ritorna subito. Il frontend
    /// interroga `status()` per la percentuale.
    pub fn start(&self, entry: &ModelEntry) -> Result<(), String> {
        if entry.bundled {
            return Err("questo modello è già incluso nell'applicazione".to_string());
        }
        let url = entry
            .download_url
            .clone()
            .ok_or_else(|| "modello senza URL di download verificato".to_string())?;
        let expected = entry
            .sha256
            .clone()
            .ok_or_else(|| "modello senza impronta sha256 verificata".to_string())?;
        let file_name = safe_file_name(&entry.file_name)?;
        if self.inner.busy.swap(true, Ordering::SeqCst) {
            return Err("un download è già in corso: attendi o annulla quello attivo".to_string());
        }
        self.inner.cancel.store(false, Ordering::SeqCst);
        let total = entry.size_bytes;
        self.set_state(|state| {
            *state = InstallStatus {
                model_id: entry.id.clone(),
                phase: InstallPhase::Downloading,
                downloaded_bytes: 0,
                total_bytes: total,
                percent: 0.0,
                message: "Download avviato.".to_string(),
            };
        });

        let installer = self.clone();
        let entry = entry.clone();
        std::thread::Builder::new()
            .name("caa-model-download".to_string())
            .spawn(move || {
                let outcome = installer.download_and_verify(&entry, &url, &expected, &file_name);
                installer.inner.busy.store(false, Ordering::SeqCst);
                match outcome {
                    Ok(()) => installer.set_state(|state| {
                        state.phase = InstallPhase::Ready;
                        state.percent = 100.0;
                        state.downloaded_bytes = entry.size_bytes;
                        state.message = "Download completato e impronta verificata.".to_string();
                    }),
                    Err(DownloadError::Cancelled) => installer.set_state(|state| {
                        state.phase = InstallPhase::Cancelled;
                        state.message =
                            "Download annullato: il file parziale resta e riprende dalla stessa posizione."
                                .to_string();
                    }),
                    Err(DownloadError::Other(message)) => installer.set_state(|state| {
                        state.phase = InstallPhase::Error;
                        state.message = message;
                    }),
                }
            })
            .map_err(|e| {
                self.inner.busy.store(false, Ordering::SeqCst);
                format!("impossibile avviare il download: {e}")
            })?;
        Ok(())
    }

    fn path_for(&self, file_name: &str) -> PathBuf {
        self.inner.models_dir.join(file_name)
    }

    /// Vero se il modello è già presente sul disco con il file atteso.
    pub fn installed(&self, entry: &ModelEntry) -> bool {
        self.path_for(&entry.file_name).is_file()
    }

    fn download_and_verify(
        &self,
        entry: &ModelEntry,
        url: &str,
        expected: &str,
        file_name: &str,
    ) -> Result<(), DownloadError> {
        let target = self.path_for(file_name);
        let part = self.path_for(&format!("{file_name}.part"));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| DownloadError::Other(format!("cartella modelli: {e}")))?;
        }

        // File già completo (per esempio dopo un riavvio): non si riscarica.
        let already_complete = fs::metadata(&part)
            .map(|meta| meta.len() == entry.size_bytes)
            .unwrap_or(false);
        if !already_complete {
            let mut command = crate::process::command("curl");
            command
                .args([
                    "-L",
                    "--fail",
                    "--silent",
                    "--show-error",
                    "--retry",
                    "2",
                    "--retry-delay",
                    "1",
                    "--connect-timeout",
                    "20",
                    "-C",
                    "-",
                    "-o",
                ])
                .arg(&part)
                .arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped());
            let mut child = command.spawn().map_err(|e| {
                DownloadError::Other(format!(
                    "curl non disponibile ({e}). Puoi scaricare il file con un browser e usare \
                     «Importa un modello GGUF»."
                ))
            })?;
            loop {
                if self.inner.cancel.load(Ordering::SeqCst) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(DownloadError::Cancelled);
                }
                match child.try_wait() {
                    Ok(Some(status)) => {
                        if !status.success() {
                            let mut message = String::new();
                            if let Some(mut stderr) = child.stderr.take() {
                                let mut buffer = String::new();
                                let _ = stderr.read_to_string(&mut buffer);
                                message = buffer.trim().to_string();
                            }
                            return Err(DownloadError::Other(format!(
                                "download non riuscito (curl {}). {} Il file parziale resta per riprovare.",
                                status.code().map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
                                message
                            )));
                        }
                        break;
                    }
                    Ok(None) => {
                        self.report_progress(&part, entry.size_bytes);
                        std::thread::sleep(Duration::from_millis(250));
                    }
                    Err(e) => {
                        let _ = child.kill();
                        return Err(DownloadError::Other(format!("download interrotto: {e}")));
                    }
                }
            }
        }

        self.report_progress(&part, entry.size_bytes);
        self.set_state(|state| {
            state.phase = InstallPhase::Verifying;
            state.message = "Verifica dell'impronta sha256…".to_string();
        });
        let actual = sha256_file(&part)?;
        if !actual.eq_ignore_ascii_case(expected) {
            // Un file con impronta diversa non è riutilizzabile: si elimina.
            let _ = fs::remove_file(&part);
            return Err(DownloadError::Other(format!(
                "l'impronta del file scaricato non coincide (attesa {expected}, ottenuta {actual}). \
                 Il file è stato eliminato: il download potrebbe essere stato alterato o incompleto."
            )));
        }
        if target.exists() {
            fs::remove_file(&target)
                .map_err(|e| DownloadError::Other(format!("sostituzione del modello: {e}")))?;
        }
        fs::rename(&part, &target)
            .map_err(|e| DownloadError::Other(format!("salvataggio del modello: {e}")))?;
        Ok(())
    }

    fn report_progress(&self, part: &Path, total: u64) {
        let downloaded = fs::metadata(part).map(|meta| meta.len()).unwrap_or(0);
        self.set_state(|state| {
            state.downloaded_bytes = downloaded;
            state.total_bytes = total;
            state.percent = if total == 0 {
                0.0
            } else {
                ((downloaded as f64 / total as f64) * 100.0).min(100.0) as f32
            };
        });
    }

    /// Elimina un modello scaricato. Il modello incluso non è rimovibile.
    pub fn remove(&self, entry: &ModelEntry) -> Result<(), String> {
        if entry.bundled {
            return Err("il modello incluso nell'applicazione non può essere eliminato".to_string());
        }
        if self.inner.busy.load(Ordering::SeqCst) {
            return Err("attendi la fine del download prima di eliminare un modello".to_string());
        }
        let target = self.path_for(&entry.file_name);
        if target.exists() {
            fs::remove_file(&target)
                .map_err(|e| format!("eliminazione di {}: {e}", target.display()))?;
        }
        let _ = fs::remove_file(self.path_for(&format!("{}.part", entry.file_name)));
        self.set_state(|state| {
            if state.model_id == entry.id {
                *state = InstallStatus::default();
            }
        });
        Ok(())
    }

    /// Importa un file GGUF scelto dall'utente. Non viene accettato nulla che
    /// non inizi con la firma `GGUF`.
    pub fn import(&self, source: &Path) -> Result<PathBuf, String> {
        if !source.is_file() {
            return Err(format!("file non trovato: {}", source.display()));
        }
        let mut file =
            File::open(source).map_err(|e| format!("apertura di {}: {e}", source.display()))?;
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)
            .map_err(|_| "il file è troppo corto per essere un modello GGUF".to_string())?;
        if &magic != b"GGUF" {
            return Err(
                "il file non è un modello GGUF: la firma iniziale non è «GGUF»".to_string(),
            );
        }
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "nome del file non valido".to_string())?;
        let safe = safe_file_name(file_name)?;
        let target = self.path_for(&safe);
        if source.canonicalize().ok() == target.canonicalize().ok() {
            return Ok(target);
        }
        if target.exists() {
            // Stesso file già importato: nessuna seconda copia.
            return Ok(target);
        }
        let temp = self.path_for(&format!("{safe}.import"));
        fs::copy(source, &temp)
            .map_err(|e| format!("copia del modello in {}: {e}", temp.display()))?;
        fs::rename(&temp, &target)
            .map_err(|e| format!("salvataggio del modello importato: {e}"))?;
        Ok(target)
    }
}

enum DownloadError {
    Cancelled,
    Other(String),
}

fn safe_file_name(name: &str) -> Result<String, String> {
    let candidate = Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    if candidate.is_empty()
        || candidate == "."
        || candidate == ".."
        || candidate.contains('\\')
        || candidate.contains('/')
    {
        return Err(format!("nome di file non valido: {name}"));
    }
    let lower = candidate.to_ascii_lowercase();
    if !lower.ends_with(".gguf") {
        return Err("il modello deve avere estensione .gguf".to_string());
    }
    Ok(candidate)
}

fn sha256_file(path: &Path) -> Result<String, DownloadError> {
    let mut file =
        File::open(path).map_err(|e| DownloadError::Other(format!("lettura del modello: {e}")))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| DownloadError::Other(format!("lettura del modello: {e}")))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_file_name_keeps_only_the_basename() {
        // Il percorso non può uscire dalla cartella modelli: si prende solo il
        // nome del file. Estensioni diverse da .gguf vengono rifiutate.
        assert_eq!(safe_file_name("../model.gguf").unwrap(), "model.gguf");
        assert_eq!(safe_file_name("cartella/model.gguf").unwrap(), "model.gguf");
        assert!(safe_file_name("model.bin").is_err());
        assert!(safe_file_name("").is_err());
        assert_eq!(safe_file_name("qwen.gguf").unwrap(), "qwen.gguf");
    }

    #[test]
    fn status_starts_idle() {
        let installer = ModelInstaller::new(std::env::temp_dir().join("caa-test-models"));
        let status = installer.status();
        assert_eq!(status.phase, InstallPhase::Idle);
        assert!(!status.busy());
    }

    #[test]
    fn import_rejects_non_gguf_files() {
        let dir = std::env::temp_dir().join(format!("caa-import-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let bogus = dir.join("finto.gguf");
        fs::write(&bogus, b"non sono un modello").unwrap();
        let installer = ModelInstaller::new(dir.join("models"));
        let error = installer.import(&bogus).unwrap_err();
        assert!(error.contains("GGUF"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_copies_a_valid_header() {
        let dir = std::env::temp_dir().join(format!("caa-import-ok-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let source = dir.join("modello.gguf");
        let mut bytes = b"GGUF".to_vec();
        bytes.extend_from_slice(&[0u8; 64]);
        fs::write(&source, &bytes).unwrap();
        let installer = ModelInstaller::new(dir.join("models"));
        let copied = installer.import(&source).unwrap();
        assert!(copied.exists());
        assert_eq!(fs::read(&copied).unwrap(), bytes);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sha256_matches_known_value() {
        let dir = std::env::temp_dir().join(format!("caa-sha-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("prova.bin");
        fs::write(&file, b"abc").unwrap();
        assert_eq!(
            sha256_file(&file).ok().unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
