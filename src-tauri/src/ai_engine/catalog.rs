//! Catalogo dei modelli e scelta automatica.
//!
//! Regola: nel catalogo entra solo ciò che è verificabile.
//!
//! - `bundled`: il file è dentro il pacchetto dell'applicazione; il suo
//!   `sha256` è stato ricalcolato sul file realmente distribuito.
//! - `verified`: l'URL e il `sha256` sono quelli pubblicati dal repository
//!   ufficiale del modello. Il download viene comunque eseguito in
//!   `installer.rs`, che **ricalcola** l'impronta del file scaricato e
//!   rifiuta il modello se non coincide.
//!
//! Un modello senza URL o senza impronta resta elencato ma non installabile:
//! non si inventano checksum per far comparire un pulsante di download.

use serde::{Deserialize, Serialize};

use super::hardware::HardwareProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub parameters: String,
    pub quantization: String,
    pub format: String,
    /// Nome del file dentro `resources/ai` (incluso) o `models/` (scaricato).
    pub file_name: String,
    pub size_bytes: u64,
    pub minimum_ram_mb: u64,
    pub recommended_ram_mb: u64,
    pub context_length: u32,
    pub supported_backends: Vec<String>,
    pub languages: Vec<String>,
    pub capabilities: Vec<String>,
    pub license: String,
    pub download_url: Option<String>,
    pub sha256: Option<String>,
    /// Vero solo se URL e impronta sono stati verificati da questo progetto.
    pub verified: bool,
    /// Vero se il modello è già dentro il pacchetto dell'applicazione.
    pub bundled: bool,
    /// Qualità stimata, non misurata: il campo esiste per non spacciare
    /// numeri inventati per misure.
    pub estimated_italian_quality: Option<f32>,
    pub notes: String,
    /// Compilati da `Engine::ai_models`: stato reale sul disco dell'utente.
    #[serde(default)]
    pub installed: bool,
    #[serde(default)]
    pub local_path: Option<String>,
}

impl ModelEntry {
    /// Vero se il modello può essere scaricato adesso.
    pub fn installable(&self) -> bool {
        !self.bundled && self.verified && self.download_url.is_some() && self.sha256.is_some()
    }
}

fn qwen(
    id: &str,
    name: &str,
    parameters: &str,
    file_name: &str,
    size_bytes: u64,
    minimum_ram_mb: u64,
    recommended_ram_mb: u64,
    download_url: &str,
    sha256: &str,
    notes: &str,
) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        name: name.to_string(),
        parameters: parameters.to_string(),
        quantization: "Q4_K_M".to_string(),
        format: "GGUF".to_string(),
        file_name: file_name.to_string(),
        size_bytes,
        minimum_ram_mb,
        recommended_ram_mb,
        context_length: 4096,
        supported_backends: vec!["cpu".to_string(), "metal".to_string()],
        languages: vec!["it".to_string(), "en".to_string()],
        capabilities: vec![
            "semantic_disambiguation".to_string(),
            "instruction_following".to_string(),
            "structured_output".to_string(),
            "related_words".to_string(),
        ],
        license: "Apache-2.0".to_string(),
        download_url: Some(download_url.to_string()),
        sha256: Some(sha256.to_string()),
        verified: true,
        bundled: false,
        estimated_italian_quality: None,
        notes: notes.to_string(),
        installed: false,
        local_path: None,
    }
}

/// Catalogo incluso nell'applicazione.
///
/// Le impronte provengono dai metadati LFS dei repository ufficiali
/// (`huggingface.co/api/models/<repo>/tree/main`). Non sono stime: il file
/// scaricato viene rifiutato se l'impronta ricalcolata è diversa.
pub fn catalog() -> Vec<ModelEntry> {
    vec![
        ModelEntry {
            id: "qwen2.5-0.5b-instruct-q4_k_m".to_string(),
            name: "Qwen2.5-0.5B-Instruct".to_string(),
            parameters: "0.5B".to_string(),
            quantization: "Q4_K_M".to_string(),
            format: "GGUF".to_string(),
            file_name: "model.gguf".to_string(),
            size_bytes: 491_400_032,
            minimum_ram_mb: 1500,
            recommended_ram_mb: 3000,
            context_length: 2048,
            supported_backends: vec!["cpu".to_string(), "metal".to_string()],
            languages: vec!["it".to_string(), "en".to_string()],
            capabilities: vec![
                "semantic_disambiguation".to_string(),
                "instruction_following".to_string(),
                "structured_output".to_string(),
                "related_words".to_string(),
            ],
            license: "Apache-2.0".to_string(),
            download_url: None,
            sha256: Some(
                "74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db".to_string(),
            ),
            verified: true,
            bundled: true,
            estimated_italian_quality: None,
            notes: "Incluso nel pacchetto: nessun download necessario. È il modello più leggero \
                    (circa 0,5 GB in RAM): adatto a computer da 4 GB, meno preciso sulla \
                    disambiguazione fine. Qualità non ancora misurata sul corpus di CAA Writer."
                .to_string(),
            installed: true,
            local_path: None,
        },
        qwen(
            "qwen2.5-1.5b-instruct-q4_k_m",
            "Qwen2.5-1.5B-Instruct",
            "1.5B",
            "qwen2.5-1.5b-instruct-q4_k_m.gguf",
            1_117_320_736,
            3000,
            6000,
            "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
            "6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e",
            "Circa 1,1 GB su disco e 1,6 GB in RAM: il compromesso consigliato per computer con \
             6–8 GB. Più affidabile del modello da 0,5 B su frasi italiane complete; la qualità \
             non è ancora misurata sul corpus di CAA Writer. Barra un'etichetta come «affine» \
             anche quando il modello propone parole vicine.",
        ),
        qwen(
            "qwen2.5-3b-instruct-q4_k_m",
            "Qwen2.5-3B-Instruct",
            "3B",
            "qwen2.5-3b-instruct-q4_k_m.gguf",
            2_104_932_768,
            5000,
            8000,
            "https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf",
            "626b4a6678b86442240e33df819e00132d3ba7dddfe1cdc4fbb18e0a9615c62d",
            "Circa 2,1 GB su disco e 2,8 GB in RAM: richiede almeno 8 GB liberi per restare \
             reattivo. Le risposte contestuali sono le più curate del catalogo, ma su una \
             macchina da 4 GB non è selezionabile e l'interfaccia lo dichiara.",
        ),
        qwen(
            "smollm2-1.7b-instruct-q4_k_m",
            "SmolLM2-1.7B-Instruct",
            "1.7B",
            "smollm2-1.7b-instruct-q4_k_m.gguf",
            1_055_609_536,
            3000,
            6000,
            "https://huggingface.co/HuggingFaceTB/SmolLM2-1.7B-Instruct-GGUF/resolve/main/smollm2-1.7b-instruct-q4_k_m.gguf",
            "decd2598bc2c8ed08c19adc3c8fdd461ee19ed5708679d1c54ef54a5a30d4f33",
            "Alternativa compatta e veloce su CPU. Addestrato soprattutto in inglese: in \
             italiano propone parole vicine meno precise, quindi l'interfaccia lo presenta come \
             sperimentale e le sue proposte restano sempre marcate come approssimative.",
        ),
    ]
}

pub fn find(id: &str) -> Option<ModelEntry> {
    catalog().into_iter().find(|entry| entry.id == id)
}

/// Punteggio di adeguatezza (0-100). I pesi sono quelli del capitolato, ma
/// qualità e latenza restano ignote finché non esiste un benchmark: in quel
/// caso si usa la compatibilità con la memoria come criterio dominante.
pub fn score(entry: &ModelEntry, hardware: &HardwareProfile) -> f32 {
    let usable = if hardware.available_ram_mb > 0 {
        hardware.available_ram_mb
    } else {
        hardware.total_ram_mb
    } as f32;
    let required = entry.minimum_ram_mb as f32;
    let recommended = entry.recommended_ram_mb as f32;

    let memory = if usable <= 0.0 {
        50.0
    } else if usable < required {
        0.0
    } else if usable >= recommended {
        100.0
    } else {
        60.0 + 40.0 * ((usable - required) / (recommended - required).max(1.0))
    };

    let structured = if entry.capabilities.iter().any(|c| c == "structured_output") {
        100.0
    } else {
        40.0
    };
    let compatibility = if entry.supported_backends.iter().any(|backend| {
        (backend == "metal" && hardware.accelerated)
            || (backend == "cpu")
            || (backend == "cuda" && !hardware.accelerated)
    }) {
        100.0
    } else {
        0.0
    };
    let installable = if entry.bundled || entry.installable() || entry.installed {
        100.0
    } else {
        40.0
    };

    (memory * 0.40 + structured * 0.20 + compatibility * 0.25 + installable * 0.15).clamp(0.0, 100.0)
}

pub fn recommend(hardware: &HardwareProfile) -> Option<ModelEntry> {
    catalog()
        .into_iter()
        .filter(|entry| entry.bundled || entry.installable())
        .filter(|entry| {
            let usable = if hardware.available_ram_mb > 0 {
                hardware.available_ram_mb
            } else {
                hardware.total_ram_mb
            };
            usable == 0 || usable >= entry.minimum_ram_mb
        })
        .max_by(|a, b| {
            score(a, hardware)
                .partial_cmp(&score(b, hardware))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Il modello più capace che entra nella memoria disponibile. Serve al
/// suggerimento dell'interfaccia («con questa macchina puoi usare …»), non
/// attiva nulla da solo.
pub fn best_fitting(hardware: &HardwareProfile) -> Option<ModelEntry> {
    catalog()
        .into_iter()
        .filter(|entry| entry.bundled || entry.installable())
        .filter(|entry| {
            let usable = if hardware.available_ram_mb > 0 {
                hardware.available_ram_mb
            } else {
                hardware.total_ram_mb
            };
            usable == 0 || usable >= entry.minimum_ram_mb
        })
        .max_by_key(|entry| (entry.parameters_rank(), entry.minimum_ram_mb))
}

impl ModelEntry {
    fn parameters_rank(&self) -> u32 {
        self.parameters
            .trim_end_matches(|c: char| !c.is_ascii_digit())
            .parse::<f32>()
            .map(|value| (value * 100.0) as u32)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hardware(ram_mb: u64) -> HardwareProfile {
        HardwareProfile {
            os: "macos".into(),
            arch: "aarch64".into(),
            cpu_cores: 8,
            total_ram_mb: ram_mb,
            available_ram_mb: ram_mb,
            accelerated: true,
            profile: "standard".into(),
            max_context: 2048,
            max_threads: 4,
            concurrent_requests: 1,
            note: String::new(),
        }
    }

    #[test]
    fn bundled_model_is_always_recommendable() {
        let choice = recommend(&hardware(4096)).expect("modello incluso");
        assert!(choice.bundled);
        assert_eq!(choice.license, "Apache-2.0");
    }

    #[test]
    fn every_downloadable_model_has_url_and_checksum() {
        for entry in catalog().into_iter().filter(|entry| !entry.bundled) {
            assert!(entry.verified, "{} non verificato", entry.id);
            assert!(entry.download_url.is_some(), "{} senza URL", entry.id);
            let sha = entry.sha256.expect("checksum presente");
            assert_eq!(sha.len(), 64, "{} checksum non valido", entry.id);
            assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
            assert!(entry.file_name.ends_with(".gguf"));
        }
    }

    #[test]
    fn catalog_has_several_models_with_distinct_files() {
        let entries = catalog();
        assert!(entries.len() >= 4, "il catalogo deve offrire più di un modello");
        let mut names: Vec<&str> = entries.iter().map(|entry| entry.file_name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), entries.len(), "file duplicati nel catalogo");
    }

    #[test]
    fn low_memory_penalizes_large_model() {
        let small = catalog().into_iter().find(|m| m.bundled).unwrap();
        let large = catalog().into_iter().find(|m| !m.bundled).unwrap();
        assert!(score(&small, &hardware(2048)) > score(&large, &hardware(2048)));
    }

    #[test]
    fn best_fitting_grows_with_available_memory() {
        let modest = best_fitting(&hardware(4096)).unwrap();
        let generous = best_fitting(&hardware(16_384)).unwrap();
        assert!(generous.parameters_rank() >= modest.parameters_rank());
        assert!(
            best_fitting(&hardware(2048)).map(|entry| entry.bundled).unwrap_or(false),
            "con 2 GB resta disponibile il modello incluso"
        );
    }
}
