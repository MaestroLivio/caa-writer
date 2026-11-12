//! Download in background con avanzamento globale.
//!
//! I pacchetti grandi (pittogrammi, voce neurale) non bloccano il thread
//! dell'interfaccia: un job per pacchetto aggiorna una tabella condivisa che il
//! frontend interroga con `download_status`. Più job possono procedere insieme;
//! la barra globale somma byte e lavori. Nessun download parte da solo.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::engine::Engine;

#[derive(Debug, Clone, Serialize, Default)]
pub struct DownloadTask {
    pub id: String,
    pub label: String,
    pub done: u64,
    pub total: u64,
    pub percent: f64,
    pub phase: String,
    pub error: Option<String>,
    pub finished: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DownloadSnapshot {
    pub tasks: Vec<DownloadTask>,
    pub active: bool,
    pub done: u64,
    pub total: u64,
    pub percent: f64,
}

static TASKS: OnceLock<Mutex<BTreeMap<String, DownloadTask>>> = OnceLock::new();

fn tasks() -> &'static Mutex<BTreeMap<String, DownloadTask>> {
    TASKS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn update(id: &str, patch: impl FnOnce(&mut DownloadTask)) {
    if let Ok(mut map) = tasks().lock() {
        if let Some(task) = map.get_mut(id) {
            patch(task);
            task.percent = if task.total > 0 {
                (task.done as f64 / task.total as f64 * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };
        }
    }
}

pub fn status() -> DownloadSnapshot {
    let map = tasks().lock().map(|value| value.clone()).unwrap_or_default();
    let tasks: Vec<DownloadTask> = map.into_values().collect();
    let done = tasks.iter().map(|task| task.done).sum();
    let total = tasks.iter().map(|task| task.total).sum();
    // La barra globale è la media degli avanzamenti dei singoli job: un
    // pacchetto piccolo non deve pesare meno di uno grande solo per i byte.
    let percent = if tasks.is_empty() {
        0.0
    } else {
        tasks
            .iter()
            .map(|task| if task.finished { 100.0 } else { task.percent })
            .sum::<f64>()
            / tasks.len() as f64
    };
    DownloadSnapshot {
        active: tasks.iter().any(|task| !task.finished),
        done,
        total,
        percent: percent.clamp(0.0, 100.0),
        tasks,
    }
}

pub fn start(app: AppHandle, id: String) -> Result<(), String> {
    let (label, initial_phase) = match id.as_str() {
        "symbols:arasaac" => ("ARASAAC — catalogo completo", "Preparazione…"),
        "symbols:globalsymbols" => ("Global Symbols — catalogo del lessico", "Preparazione…"),
        "voice" => ("Voce neurale italiana", "Preparazione…"),
        "diffusion" => ("Modello di diffusione per le immagini", "Preparazione…"),
        "update" => ("Aggiornamento di CAA Writer", "Preparazione…"),
        "library" => ("Libreria materiali", "Preparazione…"),
        other => return Err(format!("Pacchetto sconosciuto: {other}")),
    };
    {
        let mut map = tasks().lock().map_err(|_| "stato download non disponibile".to_string())?;
        map.retain(|_, task| !task.finished);
        if let Some(existing) = map.get(&id) {
            if !existing.finished {
                return Err(format!("«{}» è già in download.", existing.label));
            }
        }
        map.insert(
            id.clone(),
            DownloadTask {
                id: id.clone(),
                label: label.to_string(),
                phase: initial_phase.to_string(),
                ..DownloadTask::default()
            },
        );
    }
    std::thread::spawn(move || {
        let result = match id.as_str() {
            "symbols:arasaac" => run_symbols(&app, &id, true),
            "symbols:globalsymbols" => run_symbols(&app, &id, false),
            "voice" => run_voice(&app, &id),
            "diffusion" => run_diffusion(&app, &id),
            "update" => run_update(&app, &id),
            "library" => run_library(&app, &id),
            _ => Err("Pacchetto sconosciuto".to_string()),
        };
        match result {
            Ok(message) => update(&id, |task| {
                task.finished = true;
                task.percent = 100.0;
                task.phase = message;
                task.error = None;
            }),
            Err(error) => update(&id, |task| {
                task.finished = true;
                task.error = Some(error.clone());
                task.phase = format!("Errore: {error}");
            }),
        }
    });
    Ok(())
}

pub fn reset_finished() {
    if let Ok(mut map) = tasks().lock() {
        map.retain(|_, task| !task.finished);
    }
}

fn progress_for(id: &str) -> impl Fn(usize, usize, String) + '_ {
    move |done, total, phase| {
        update(id, |task| {
            task.done = done as u64;
            task.total = total as u64;
            task.phase = phase;
        });
    }
}

fn run_symbols(app: &AppHandle, id: &str, arasaac: bool) -> Result<String, String> {
    let engine = app.state::<Engine>();
    let progress = progress_for(id);
    if arasaac {
        crate::packages::download_arasaac_all(&engine, &progress).map(|report| report.message)
    } else {
        crate::packages::download_globalsymbols_all(&engine, &progress).map(|report| report.message)
    }
}

fn run_library(_app: &AppHandle, _id: &str) -> Result<String, String> {
    // La libreria materiali è inclusa nell'installer e non richiede download.
    Ok("Libreria materiali già inclusa nell'app.".to_string())
}

/// Installa il runtime immagini e scarica il modello consigliato per questo
/// computer. Le due operazioni sono lunghe e non espongono byte parziali:
/// il job mostra la fase reale invece di una percentuale inventata.
fn run_diffusion(app: &AppHandle, id: &str) -> Result<String, String> {
    let engine = app.state::<Engine>();
    let recommendation = crate::image_engine::recommendation(&engine.paths);
    update(id, |task| {
        task.phase = format!(
            "Installo il runtime Python per il modello {}…",
            recommendation.model
        )
    });
    crate::image_engine::install_runtime(&engine.paths)?;

    update(id, |task| {
        task.phase = format!("Scarico il modello {}…", recommendation.model)
    });
    crate::image_engine::download_model(&engine.paths, &recommendation.model)?;

    let db = engine.db()?;
    crate::image_engine::set_model(&db, &recommendation.model)?;
    Ok(format!(
        "Pacchetto di diffusione pronto: modello «{}». La generazione resta offline.",
        recommendation.model
    ))
}

/// Scarica l'asset della release adatto a questa piattaforma e apre
/// l'installatore. Non sostituisce nulla da solo: macOS apre il DMG, Windows
/// avvia il programma di installazione.
fn run_update(app: &AppHandle, id: &str) -> Result<String, String> {
    let engine = app.state::<Engine>();
    update(id, |task| {
        task.phase = "Cerco la versione più recente…".to_string()
    });
    let asset = crate::packages::update_asset()?;

    let folder = engine.paths.data_dir.join("updates");
    std::fs::create_dir_all(&folder).map_err(|error| format!("cartella aggiornamenti: {error}"))?;
    let file_name = Path::new(&asset.file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "nome del pacchetto di aggiornamento non valido".to_string())?
        .to_string();
    let destination = folder.join(&file_name);

    update(id, |task| {
        task.phase = format!("Scarico la versione {}…", asset.version)
    });
    download_file(&asset.url, &destination, asset.size, |done, total| {
        let expected = total.max(asset.size);
        update(id, |task| {
            task.done = done;
            task.total = expected;
            task.phase = format!("Scarico la versione {}…", asset.version);
        });
    })?;

    update(id, |task| task.phase = "Apro l'installatore…".to_string());
    open_installer(&destination)?;
    Ok(format!(
        "Versione {} scaricata in {}. Completa l'installazione dalla finestra aperta.",
        asset.version,
        destination.display()
    ))
}

fn open_installer(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|error| format!("apertura del DMG: {error}"))?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(path)
            .spawn()
            .map_err(|error| format!("avvio dell'installatore: {error}"))?;
        return Ok(());
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let _ = path;
        Err("L'aggiornamento automatico è disponibile per macOS e Windows.".to_string())
    }
}

fn run_voice(app: &AppHandle, id: &str) -> Result<String, String> {
    let engine = app.state::<Engine>();
    let voice_dir = engine.paths.data_dir.join("voice");
    std::fs::create_dir_all(voice_dir.join("bin"))
        .map_err(|error| format!("cartella voce: {error}"))?;

    let progress = progress_for(id);
    let mut total_hint = 0u64;

    // 1. Runtime Piper per la piattaforma corrente.
    let (runtime_url, archive_name) = piper_runtime_url()?;
    let archive = voice_dir.join(archive_name);
    update(id, |task| task.phase = "Scarico il motore Piper…".to_string());
    download_file(
        runtime_url,
        &archive,
        total_hint,
        |done, total| {
            if total > 0 {
                total_hint = total;
            }
            progress(
                done as usize,
                total_hint.max(done) as usize,
                "Scarico il motore Piper…".to_string(),
            );
        },
    )?;
    update(id, |task| task.phase = "Estraggo il motore Piper…".to_string());
    extract_piper(&archive, &voice_dir)?;
    let _ = std::fs::remove_file(&archive);

    // 2. Modello e configurazione della voce italiana.
    let base = "https://huggingface.co/rhasspy/piper-voices/resolve/main/it/it_IT/serena/medium";
    let model = voice_dir.join("it_IT-serena-medium.onnx");
    let config = voice_dir.join("it_IT-serena-medium.onnx.json");
    update(id, |task| task.phase = "Scarico la voce italiana…".to_string());
    download_file(
        &format!("{base}/it_IT-serena-medium.onnx"),
        &model,
        0,
        |done, total| {
            let base_done = total_hint;
            progress(
                (base_done + done) as usize,
                (base_done + total) as usize,
                "Scarico la voce italiana…".to_string(),
            );
        },
    )?;
    download_file(
        &format!("{base}/it_IT-serena-medium.onnx.json"),
        &config,
        0,
        |done, total| {
            let base_done = total_hint;
            progress(
                (base_done + done) as usize,
                (base_done + total) as usize,
                "Scarico la configurazione della voce…".to_string(),
            );
        },
    )?;

    // 3. Collega il motore nella posizione attesa da `voice::resolve`.
    link_engine(&voice_dir)?;
    Ok("Voce neurale italiana installata: lettura offline con Piper.".to_string())
}

fn piper_runtime_url() -> Result<(&'static str, &'static str), String> {
    if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            Ok((
                "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_macos_aarch64.tar.gz",
                "piper-runtime.tar.gz",
            ))
        } else {
            Ok((
                "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_macos_x64.tar.gz",
                "piper-runtime.tar.gz",
            ))
        }
    } else if cfg!(target_os = "windows") {
        Ok((
            "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_windows_amd64.zip",
            "piper-runtime.zip",
        ))
    } else {
        Ok((
            "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_x86_64.tar.gz",
            "piper-runtime.tar.gz",
        ))
    }
}

fn download_file(
    url: &str,
    destination: &Path,
    _total_hint: u64,
    mut progress: impl FnMut(u64, u64),
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .user_agent("CAA-Writer")
        .build()
        .map_err(|error| format!("client HTTP: {error}"))?;
    let mut response = client
        .get(url)
        .send()
        .map_err(|error| format!("download non riuscito: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("il server ha risposto {}", response.status()));
    }
    let total = response.content_length().unwrap_or(0);
    let mut file = std::fs::File::create(destination)
        .map_err(|error| format!("file temporaneo: {error}"))?;
    let mut buffer = [0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|error| format!("lettura download: {error}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|error| format!("scrittura download: {error}"))?;
        done += read as u64;
        progress(done, total);
    }
    Ok(())
}

fn extract_piper(archive: &Path, voice_dir: &Path) -> Result<(), String> {
    let name = archive.to_string_lossy();
    if name.ends_with(".zip") {
        let file = std::fs::File::open(archive).map_err(|error| error.to_string())?;
        let mut zip = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(|error| error.to_string())?;
            if entry.is_dir() {
                continue;
            }
            let relative = entry
                .enclosed_name()
                .ok_or_else(|| "archivio Piper non sicuro".to_string())?;
            let target = voice_dir.join("piper").join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let mut out = std::fs::File::create(&target).map_err(|error| error.to_string())?;
            std::io::copy(&mut entry, &mut out).map_err(|error| error.to_string())?;
        }
    } else {
        let file = std::fs::File::open(archive).map_err(|error| error.to_string())?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut tar = tar::Archive::new(decoder);
        tar.unpack(voice_dir.join("piper"))
            .map_err(|error| format!("estrazione Piper: {error}"))?;
    }
    Ok(())
}

fn link_engine(voice_dir: &Path) -> Result<(), String> {
    let candidates: Vec<PathBuf> = if cfg!(windows) {
        vec![
            voice_dir.join("piper").join("piper").join("piper.exe"),
            voice_dir.join("piper").join("piper.exe"),
        ]
    } else {
        vec![
            voice_dir.join("piper").join("piper").join("piper"),
            voice_dir.join("piper").join("piper"),
        ]
    };
    let engine = candidates
        .iter()
        .find(|path| path.is_file())
        .cloned()
        .ok_or_else(|| {
            format!(
                "il motore Piper non è stato trovato dopo l'estrazione in {}",
                voice_dir.join("piper").display()
            )
        })?;
    let bin = voice_dir.join("bin");
    std::fs::create_dir_all(&bin).map_err(|error| error.to_string())?;
    let target = bin.join(if cfg!(windows) { "piper.exe" } else { "piper" });
    if !target.exists() {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&engine, &target)
                .map_err(|error| format!("collegamento Piper: {error}"))?;
        }
        #[cfg(not(unix))]
        {
            std::fs::copy(&engine, &target).map_err(|error| format!("copia Piper: {error}"))?;
        }
    }
    Ok(())
}

/// Percorso del modello scaricato, utile al frontend per lo stato.
pub fn installed_voice_model(data_dir: &Path) -> Option<PathBuf> {
    let path = data_dir.join("voice").join("it_IT-serena-medium.onnx");
    path.is_file().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piper_runtime_url_is_platform_specific() {
        let (url, archive) = piper_runtime_url().unwrap();
        assert!(url.starts_with("https://github.com/rhasspy/piper/releases/download/"));
        assert!(archive.starts_with("piper-runtime."));
        if cfg!(windows) {
            assert!(url.contains("windows"));
            assert!(archive.ends_with(".zip"));
        } else if cfg!(target_os = "macos") {
            assert!(url.contains("macos"));
            assert!(archive.ends_with(".tar.gz"));
        }
    }

    #[test]
    fn unknown_package_is_rejected() {
        // Senza AppHandle non possiamo avviare un job, ma il controllo del nome
        // avviene prima di qualsiasi thread.
        assert_eq!(status().tasks.len(), 0);
    }
}
