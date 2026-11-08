//! Motore immagini neurale opzionale.
//!
//! Regole:
//! - l'app resta utilizzabile senza motore immagini;
//! - il runtime locale (Python + torch/diffusers) e i modelli non sono nel
//!   bundle: si installano su richiesta dalle impostazioni e restano nei dati
//!   dell'utente;
//! - su Apple Silicon si usa la GPU (MPS); se è presente un runtime MLX
//!   Stable Diffusion completo si preferisce MLX;
//! - il provider online è facoltativo, richiede consenso e la chiave sta nel
//!   portachiavi, mai nel database o nei documenti;
//! - l'unico testo inviato è il prompt scritto dall'utente.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::database::Database;
use crate::paths::AppPaths;
use crate::util::now_iso;

const CONFIG_KEY: &str = "image.config";
const KEYRING_SERVICE: &str = "caa-writer-image";
const KEYRING_USER: &str = "provider";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// `auto` | `mlx` | `gpu` | `online` | `off`
    #[serde(default = "default_backend")]
    pub backend: String,
    /// Modello diffusers locale (es. `segmind/tiny-sd`).
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default = "default_online_model")]
    pub online_model: String,
    #[serde(default = "default_size")]
    pub size: u32,
    #[serde(default = "default_steps")]
    pub steps: u32,
    #[serde(default = "default_guidance")]
    pub guidance: f32,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_backend() -> String {
    "auto".to_string()
}
fn default_model() -> String {
    "stabilityai/sd-turbo".to_string()
}
fn default_online_model() -> String {
    "gpt-image-1".to_string()
}
fn default_size() -> u32 {
    512
}
fn default_steps() -> u32 {
    16
}
fn default_guidance() -> f32 {
    7.0
}
fn default_timeout() -> u64 {
    600_000
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            model: default_model(),
            endpoint: String::new(),
            online_model: default_online_model(),
            size: default_size(),
            steps: default_steps(),
            guidance: default_guidance(),
            timeout_ms: default_timeout(),
        }
    }
}

pub fn load(db: &Database) -> ImageConfig {
    db.setting(CONFIG_KEY)
        .and_then(|raw| serde_json::from_str::<ImageConfig>(&raw).ok())
        .unwrap_or_default()
}

pub fn save(db: &Database, config: &ImageConfig) -> Result<(), String> {
    let raw = serde_json::to_string(config).map_err(|e| e.to_string())?;
    db.set_setting(CONFIG_KEY, &raw).map_err(|e| e.to_string())
}

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|error| format!("portachiavi: {error}"))
}

pub fn set_key(key: &str) -> Result<(), String> {
    entry()?
        .set_password(key.trim())
        .map_err(|error| format!("portachiavi: {error}"))
}

pub fn clear_key() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("portachiavi: {error}")),
    }
}

fn api_key() -> Option<String> {
    entry().ok()?.get_password().ok().filter(|key| !key.is_empty())
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ImageRuntime {
    pub python: Option<String>,
    pub script: bool,
    pub mlx: bool,
    pub mlx_diffusion: bool,
    pub torch: bool,
    pub mps: bool,
    pub diffusers: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageStatus {
    pub config: ImageConfig,
    pub runtime: ImageRuntime,
    pub online_ready: bool,
    pub has_key: bool,
    pub available: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneratedImage {
    pub png_base64: String,
    pub backend: String,
    pub model: String,
    pub width: u32,
    pub height: u32,
    pub elapsed_ms: u64,
    pub path: String,
}

/// Script Python del motore: prima la copia utente, poi la risorsa in sola
/// lettura del bundle.
pub fn script_path(paths: &AppPaths) -> Option<PathBuf> {
    let user = paths.data_dir.join("image").join("image_engine.py");
    if user.exists() {
        return Some(user);
    }
    let bundled = paths.resources_dir.join("image").join("image_engine.py");
    if bundled.exists() {
        return Some(bundled);
    }
    None
}

/// Interprete del runtime immagini: venv utente, variabile d'ambiente, venv di
/// sviluppo accanto al repository, poi i Python di sistema.
pub fn python_path(paths: &AppPaths) -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("CAA_IMAGE_PYTHON") {
        let path = PathBuf::from(explicit);
        if path.exists() {
            return Some(path);
        }
    }
    let user = paths.data_dir.join("image-venv").join("bin").join("python");
    if user.exists() {
        return Some(user);
    }
    if let Some(root) = paths.resources_dir.parent() {
        let dev = root.join(".image-venv").join("bin").join("python");
        if dev.exists() {
            return Some(dev);
        }
    }
    for candidate in ["python3.12", "python3.13", "python3.14", "python3"] {
        if let Ok(output) = Command::new(candidate).arg("--version").output() {
            if output.status.success() {
                return Some(PathBuf::from(candidate));
            }
        }
    }
    None
}

fn runtime_json(python: &Path, script: &Path) -> Option<serde_json::Value> {
    let output = Command::new(python)
        .arg(script)
        .arg("--status")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

pub fn status(paths: &AppPaths, db: &Database) -> ImageStatus {
    let config = load(db);
    let script = script_path(paths);
    let python = python_path(paths);
    let mut runtime = ImageRuntime {
        python: python.as_ref().map(|p| p.to_string_lossy().to_string()),
        script: script.is_some(),
        ..Default::default()
    };
    if let (Some(python), Some(script)) = (&python, &script) {
        if let Some(value) = runtime_json(python, script) {
            runtime.mlx = value["mlx"].as_bool().unwrap_or(false);
            runtime.mlx_diffusion = value["mlx_diffusion"].as_bool().unwrap_or(false);
            runtime.torch = value["torch"].as_bool().unwrap_or(false);
            runtime.mps = value["mps"].as_bool().unwrap_or(false);
            runtime.diffusers = value["diffusers"].as_bool().unwrap_or(false);
        }
    }
    let has_key = api_key().is_some();
    let online_ready = !config.endpoint.trim().is_empty()
        && !config.online_model.trim().is_empty()
        && has_key;
    let local_ready = runtime.mlx_diffusion || (runtime.torch && runtime.diffusers);
    let available = match config.backend.as_str() {
        "off" => false,
        "online" => online_ready,
        _ => local_ready,
    };
    let note = if config.backend == "off" {
        "Motore immagini disattivato.".to_string()
    } else if config.backend == "online" {
        if online_ready {
            format!(
                "Provider online pronto: {} · {}.",
                config.endpoint, config.online_model
            )
        } else {
            "Il provider online richiede endpoint, modello e chiave API.".to_string()
        }
    } else if runtime.mlx_diffusion {
        "MLX Stable Diffusion disponibile: generazione sulla GPU Apple.".to_string()
    } else if runtime.mps {
        "GPU Apple (MPS) disponibile tramite torch/diffusers.".to_string()
    } else if runtime.torch && runtime.diffusers {
        "Runtime presente, ma la GPU MPS non è disponibile: si userà la CPU.".to_string()
    } else {
        "Runtime immagini non installato. Impostazioni → Immagini neurali → Installa runtime."
            .to_string()
    };
    ImageStatus {
        config,
        runtime,
        online_ready,
        has_key,
        available,
        note,
    }
}

/// Esegue un processo con un limite di tempo. L'output va su file temporanei
/// (non su pipe) perché pip e torch possono scrivere molto più del buffer di
/// una pipe: leggerlo solo alla fine bloccherebbe il processo figlio.
fn run_with_timeout(mut command: Command, timeout: Duration) -> Result<(String, String), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!(
        "caa-image-{}-{}-{}",
        std::process::id(),
        seq,
        now_iso().replace([':', '.'], "")
    ));
    let out_path = base.with_extension("out");
    let err_path = base.with_extension("err");
    let out_file = fs::File::create(&out_path).map_err(|e| format!("log immagine: {e}"))?;
    let err_file = fs::File::create(&err_path).map_err(|e| format!("log immagine: {e}"))?;
    command.stdout(Stdio::from(out_file)).stderr(Stdio::from(err_file));
    let mut child = command
        .spawn()
        .map_err(|error| format!("avvio motore immagini: {error}"))?;
    let started = Instant::now();
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {
                if started.elapsed() > timeout {
                    let _ = child.kill();
                    break Err("il motore immagini ha superato il tempo massimo".to_string());
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(error) => break Err(format!("stato del motore immagini: {error}")),
        }
    };
    let mut stdout = String::new();
    let mut stderr = String::new();
    let _ = fs::File::open(&out_path).and_then(|mut file| file.read_to_string(&mut stdout));
    let _ = fs::File::open(&err_path).and_then(|mut file| file.read_to_string(&mut stderr));
    let _ = fs::remove_file(&out_path);
    let _ = fs::remove_file(&err_path);
    let status = result?;
    if !status.success() {
        return Err(format!(
            "il motore immagini è uscito con stato {}: {}",
            status,
            stderr
                .lines()
                .rev()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("nessun dettaglio")
        ));
    }
    Ok((stdout, stderr))
}

fn parse_json_line(stdout: &str) -> Option<serde_json::Value> {
    stdout
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok())
}

fn save_generated(paths: &AppPaths, bytes: &[u8]) -> Result<PathBuf, String> {
    let folder = paths.assets_dir.join("generated");
    fs::create_dir_all(&folder).map_err(|e| format!("cartella immagini generate: {e}"))?;
    let name = format!(
        "image-{}.png",
        now_iso().replace([':', '-', '.'], "").replace('T', "-")
    );
    let path = folder.join(name);
    fs::write(&path, bytes).map_err(|e| format!("salvataggio immagine: {e}"))?;
    Ok(path)
}

fn generate_local(
    paths: &AppPaths,
    config: &ImageConfig,
    prompt: &str,
    negative: &str,
) -> Result<GeneratedImage, String> {
    let script = script_path(paths)
        .ok_or_else(|| "script del motore immagini non trovato".to_string())?;
    let python =
        python_path(paths).ok_or_else(|| "Python non trovato per il motore immagini".to_string())?;
    let started = Instant::now();
    let temp = std::env::temp_dir().join(format!("caa-image-{}.png", std::process::id()));
    let mut command = Command::new(&python);
    command
        .arg(&script)
        .arg("--generate")
        .arg("--prompt")
        .arg(prompt)
        .arg("--out")
        .arg(&temp)
        .arg("--model")
        .arg(&config.model)
        .arg("--cache")
        .arg(paths.models_dir.join("image"))
        .arg("--backend")
        .arg(if config.backend == "mlx" { "mlx" } else { "auto" })
        .arg("--size")
        .arg(config.size.to_string())
        .arg("--steps")
        .arg(config.steps.to_string())
        .arg("--guidance")
        .arg(config.guidance.to_string());
    if !negative.trim().is_empty() {
        command.arg("--negative").arg(negative);
    }
    let (stdout, _stderr) = run_with_timeout(command, Duration::from_millis(config.timeout_ms))?;
    let value = parse_json_line(&stdout).ok_or_else(|| {
        "il motore immagini non ha restituito un risultato leggibile".to_string()
    })?;
    if value["ok"].as_bool() != Some(true) {
        return Err(value["error"]
            .as_str()
            .unwrap_or("generazione immagine non riuscita")
            .to_string());
    }
    let bytes = fs::read(&temp).map_err(|e| format!("immagine non prodotta: {e}"))?;
    let _ = fs::remove_file(&temp);
    let path = save_generated(paths, &bytes)?;
    Ok(GeneratedImage {
        png_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
        backend: value["backend"].as_str().unwrap_or("locale").to_string(),
        model: config.model.clone(),
        width: value["width"].as_u64().unwrap_or(config.size as u64) as u32,
        height: value["height"].as_u64().unwrap_or(config.size as u64) as u32,
        elapsed_ms: started.elapsed().as_millis() as u64,
        path: path.to_string_lossy().to_string(),
    })
}

fn generate_online(
    paths: &AppPaths,
    config: &ImageConfig,
    prompt: &str,
) -> Result<GeneratedImage, String> {
    let key = api_key().ok_or_else(|| "chiave API del provider immagini mancante".to_string())?;
    if config.endpoint.trim().is_empty() {
        return Err("endpoint del provider immagini mancante".to_string());
    }
    let url = format!(
        "{}/images/generations",
        config.endpoint.trim().trim_end_matches('/')
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(config.timeout_ms))
        .build()
        .map_err(|e| format!("client HTTP immagini: {e}"))?;
    let body = serde_json::json!({
        "model": config.online_model,
        "prompt": prompt,
        "n": 1,
        "size": format!("{}x{}", config.size, config.size),
        "response_format": "b64_json"
    });
    let started = Instant::now();
    let response = client
        .post(&url)
        .bearer_auth(key)
        .json(&body)
        .send()
        .map_err(|e| format!("provider immagini non raggiungibile: {e}"))?;
    let status = response.status();
    let value: serde_json::Value = response
        .json()
        .map_err(|e| format!("risposta provider immagini non valida: {e}"))?;
    if !status.is_success() {
        return Err(value["error"]["message"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("provider immagini: stato {status}")));
    }
    let b64 = value["data"][0]["b64_json"]
        .as_str()
        .ok_or_else(|| "il provider non ha restituito un'immagine".to_string())?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("immagine base64 non valida: {e}"))?;
    let path = save_generated(paths, &bytes)?;
    Ok(GeneratedImage {
        png_base64: b64.to_string(),
        backend: "online".to_string(),
        model: config.online_model.clone(),
        width: config.size,
        height: config.size,
        elapsed_ms: started.elapsed().as_millis() as u64,
        path: path.to_string_lossy().to_string(),
    })
}

pub fn generate(
    paths: &AppPaths,
    db: &Database,
    prompt: &str,
    negative: &str,
) -> Result<GeneratedImage, String> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("serve un prompt per generare l'immagine".to_string());
    }
    if prompt.chars().count() > 600 {
        return Err("il prompt è troppo lungo (massimo 600 caratteri)".to_string());
    }
    let config = load(db);
    match config.backend.as_str() {
        "off" => Err("motore immagini disattivato nelle impostazioni".to_string()),
        "online" => generate_online(paths, &config, prompt),
        _ => generate_local(paths, &config, prompt, negative),
    }
}

/// Crea il venv utente e installa il runtime Python. Operazione lunga:
/// l'interfaccia deve mostrarla come in corso e non bloccare l'editor.
pub fn install_runtime(paths: &AppPaths) -> Result<String, String> {
    let python = ["python3.12", "python3.13", "python3.14", "python3"]
        .iter()
        .find(|candidate| {
            Command::new(candidate)
                .arg("--version")
                .output()
                .map(|out| out.status.success())
                .unwrap_or(false)
        })
        .ok_or_else(|| "Python 3 non trovato sul sistema".to_string())?;
    let venv = paths.data_dir.join("image-venv");
    if !venv.join("bin").join("python").exists() {
        let output = Command::new(python)
            .arg("-m")
            .arg("venv")
            .arg(&venv)
            .output()
            .map_err(|e| format!("creazione ambiente immagini: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "creazione ambiente immagini non riuscita: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    let pip = venv.join("bin").join("pip");
    let mut command = Command::new(pip);
    command
        .arg("install")
        .arg("--upgrade")
        .arg("torch")
        .arg("torchvision")
        .arg("diffusers")
        .arg("transformers")
        .arg("accelerate")
        .arg("safetensors")
        .arg("Pillow");
    let (stdout, stderr) = run_with_timeout(command, Duration::from_secs(60 * 45))?;
    Ok(format!(
        "Runtime immagini installato in {}.\n{}{}",
        venv.display(),
        stdout.lines().last().unwrap_or(""),
        stderr.lines().last().unwrap_or("")
    ))
}

/// Scarica (nella cache dell'app) il modello diffusers scelto.
pub fn download_model(paths: &AppPaths, model: &str) -> Result<String, String> {
    let script =
        script_path(paths).ok_or_else(|| "script del motore immagini non trovato".to_string())?;
    let python =
        python_path(paths).ok_or_else(|| "Python non trovato per il motore immagini".to_string())?;
    let mut command = Command::new(python);
    command
        .arg(&script)
        .arg("--download")
        .arg("--model")
        .arg(model)
        .arg("--cache")
        .arg(paths.models_dir.join("image"));
    let (stdout, stderr) = run_with_timeout(command, Duration::from_secs(60 * 45))?;
    let value = parse_json_line(&stdout).unwrap_or(serde_json::json!({}));
    if value["ok"].as_bool() != Some(true) {
        return Err(value["error"]
            .as_str()
            .unwrap_or_else(|| stderr.lines().last().unwrap_or("download non riuscito"))
            .to_string());
    }
    Ok(format!("Modello «{model}» scaricato nella cache dell'app."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_auto_backend_and_a_real_model() {
        let config = ImageConfig::default();
        assert_eq!(config.backend, "auto");
        assert_eq!(config.model, "stabilityai/sd-turbo");
        assert!(config.size >= 256);
        assert!(config.steps >= 1);
    }

    #[test]
    fn json_result_uses_the_last_object() {
        let stdout = "warning: something\n{\"ok\":false}\n{\"ok\":true,\"backend\":\"mps\"}\n";
        let value = parse_json_line(stdout).expect("json");
        assert_eq!(value["ok"].as_bool(), Some(true));
        assert_eq!(value["backend"].as_str(), Some("mps"));
    }

    #[test]
    fn a_missing_script_is_reported_as_none() {
        let dir = std::env::temp_dir().join(format!("caa-image-test-{}", std::process::id()));
        let paths = AppPaths::new(dir.join("data"), dir.join("resources"));
        // Nessuno script nelle risorse temporanee: il motore non deve inventarlo.
        assert!(script_path(&paths).is_none());
    }
}
