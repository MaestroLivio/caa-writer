//! Avvio condiviso di CLI e server MCP.
//!
//! Un solo punto decide dove stanno dati e risorse, così i due programmi non
//! divergono. Gli stessi percorsi vengono usati dai job in background.

use std::path::{Path, PathBuf};

use crate::engine::Engine;
use crate::paths::AppPaths;

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn default_data_dir() -> PathBuf {
    if let Ok(value) = std::env::var("CAA_WRITER_DATA_DIR") {
        return PathBuf::from(value);
    }
    #[cfg(target_os = "macos")]
    {
        home_dir().join("Library/Application Support/org.caawriter.desktop")
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home_dir)
            .join("org.caawriter.desktop")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join(".local/share"))
            .join("org.caawriter.desktop")
    }
}

pub fn resolve_resources(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        if path.join("dictionaries/it-core.json").exists() {
            return Ok(path);
        }
        return Err(format!(
            "risorse non valide: {} non contiene dictionaries/it-core.json",
            path.display()
        ));
    }
    if let Ok(value) = std::env::var("CAA_WRITER_RESOURCES") {
        let path = PathBuf::from(value);
        if path.join("dictionaries/it-core.json").exists() {
            return Ok(path);
        }
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| cwd.clone());
    let mut candidates = vec![
        cwd.join("resources"),
        cwd.join("../resources"),
        cwd.join("../../resources"),
        exe_dir.join("resources"),
        exe_dir.join("../resources"),
        exe_dir.join("../../../resources"),
        exe_dir.join("../../../../resources"),
    ];
    for candidate in candidates.drain(..) {
        if candidate.join("dictionaries/it-core.json").exists() {
            return Ok(candidate);
        }
    }
    Err("cartella risorse non trovata: usa --resources o CAA_WRITER_RESOURCES".to_string())
}

pub fn build_engine(
    data_dir: Option<PathBuf>,
    resources: Option<PathBuf>,
) -> Result<Engine, String> {
    let resources = resolve_resources(resources)?;
    let data_dir = data_dir.unwrap_or_else(default_data_dir);
    let paths = AppPaths::new(data_dir, resources);
    Engine::new(paths)
}
