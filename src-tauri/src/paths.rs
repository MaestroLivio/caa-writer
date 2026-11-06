//! Percorsi dell'applicazione: dati utente, risorse, asset, modelli.
//!
//! Regola: i dati dell'utente non finiscono mai nella cartella dell'app;
//! le risorse in sola lettura vivono accanto al binario (bundle Tauri).

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub assets_dir: PathBuf,
    pub resources_dir: PathBuf,
    pub models_dir: PathBuf,
    pub documents_cache_dir: PathBuf,
}

impl AppPaths {
    pub fn new(data_dir: PathBuf, resources_dir: PathBuf) -> Self {
        let assets_dir = data_dir.join("assets");
        let models_dir = data_dir.join("models");
        let documents_cache_dir = data_dir.join("documents");
        for dir in [&data_dir, &assets_dir, &models_dir, &documents_cache_dir] {
            let _ = fs::create_dir_all(dir);
        }
        Self {
            db_path: data_dir.join("caa-writer.sqlite3"),
            data_dir,
            assets_dir,
            models_dir,
            documents_cache_dir,
            resources_dir,
        }
    }

    pub fn dictionary_path(&self) -> PathBuf {
        self.resources_dir.join("dictionaries").join("it-core.json")
    }

    pub fn core_pack_dir(&self) -> PathBuf {
        self.resources_dir.join("symbol-packs").join("core")
    }

    /// Cartella che contiene tutti i pacchetti simboli (`core`, `arasaac`, ...).
    pub fn packs_dir(&self) -> PathBuf {
        self.resources_dir.join("symbol-packs")
    }

    /// Runtime e modello del motore neurale incluso nell'applicazione.
    pub fn ai_dir(&self) -> PathBuf {
        self.resources_dir.join("ai")
    }

    pub fn boards_dir(&self) -> PathBuf {
        self.resources_dir.join("boards")
    }

    pub fn templates_dir(&self) -> PathBuf {
        self.resources_dir.join("templates")
    }

    /// Cerca le risorse in più posizioni: bundle Tauri, accanto all'eseguibile,
    /// e (durante lo sviluppo/test) la cartella `resources/` del repository.
    pub fn resolve_resources(explicit: Option<&Path>, executable_dir: Option<&Path>) -> PathBuf {
        // Tauri 2 su macOS mette le risorse di `bundle.resources` in
        // `Contents/Resources/_up_/resources`: la cartella `resource_dir`
        // dichiarata da Tauri non le contiene direttamente. Senza questa
        // variante l'app confezionata si chiude all'avvio perché non trova
        // `dictionaries/it-core.json` (verificato sul bundle macOS).
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(p) = explicit {
            candidates.push(p.to_path_buf());
            candidates.push(p.join("resources"));
            candidates.push(p.join("_up_").join("resources"));
            candidates.push(p.join("_up_"));
        }
        if let Some(d) = executable_dir {
            candidates.push(d.join("resources"));
            candidates.push(d.join("_up_").join("resources"));
            candidates.push(d.join("_up_"));
            if let Some(contents) = d.parent() {
                candidates.push(contents.join("Resources"));
                candidates.push(contents.join("Resources").join("_up_").join("resources"));
            }
        }
        for path in &candidates {
            if path.join("dictionaries").join("it-core.json").exists() {
                return path.clone();
            }
        }

        if let Ok(cwd) = std::env::current_dir() {
            if let Some(found) = Self::find_upwards(&cwd) {
                return found;
            }
        }
        for path in candidates {
            if let Some(found) = Self::find_upwards(&path) {
                return found;
            }
        }
        PathBuf::from("resources")
    }

    fn find_upwards(start: &Path) -> Option<PathBuf> {
        let mut current = Some(start);
        let mut depth = 0;
        while let Some(dir) = current {
            let candidate = dir.join("resources");
            if candidate.join("dictionaries").join("it-core.json").exists() {
                return Some(candidate);
            }
            if depth >= 6 {
                break;
            }
            depth += 1;
            current = dir.parent();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_dictionary(root: &Path) {
        let dir = root.join("dictionaries");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("it-core.json"), "{}").unwrap();
    }

    #[test]
    fn resolves_development_resources_directory() {
        let dir = tempfile::tempdir().unwrap();
        let resources = dir.path().join("resources");
        with_dictionary(&resources);
        let resolved = AppPaths::resolve_resources(Some(&resources), None);
        assert_eq!(resolved, resources);
    }

    #[test]
    fn resolves_tauri_macos_up_layout() {
        // Layout reale del bundle: Contents/Resources/_up_/resources.
        let dir = tempfile::tempdir().unwrap();
        let resources = dir.path().join("_up_").join("resources");
        with_dictionary(&resources);
        let resolved = AppPaths::resolve_resources(Some(dir.path()), None);
        assert_eq!(resolved, resources);
    }

    #[test]
    fn resolves_bundle_from_executable_directory() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("Contents").join("MacOS");
        std::fs::create_dir_all(&executable).unwrap();
        let resources = dir
            .path()
            .join("Contents")
            .join("Resources")
            .join("_up_")
            .join("resources");
        with_dictionary(&resources);
        let resolved = AppPaths::resolve_resources(None, Some(&executable));
        assert_eq!(resolved, resources);
    }
}
