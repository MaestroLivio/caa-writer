//! Neural Voice Engine: Piper (VITS/ONNX, CPU) invocato come processo locale.
//!
//! Perché un processo: il runtime Piper include ONNX Runtime ed espeak-ng già
//! compilati, quindi non aggiungiamo dipendenze native al binario Tauri.
//! Un processo per frase non è un processo per pittogramma: l'audio prodotto
//! viene messo in cache su disco e riusato.
//!
//! Se il motore o il modello non sono installati si usa la voce di sistema:
//! il fallback è dichiarato da `status()` e non viene spacciato per neurale.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub struct Voice {
    pub name: &'static str,
    pub license: &'static str,
}

/// Voce italiana: `it_IT-serena-medium`, 22,05 kHz, CC BY 4.0.
pub const ITALIAN_VOICE: Voice = Voice {
    name: "it_IT-serena-medium",
    license: "CC BY 4.0 — derivata da committa/serena-synthetic-it-27h",
};

/// Voce inglese britannica: `en_GB-cori-high`, 22,05 kHz.
pub const ENGLISH_VOICE: Voice = Voice {
    name: "en_GB-cori-high",
    license: "vedi MODEL_CARD del repertorio rhasspy/piper-voices",
};

/// Tutto ciò che inizia per `en` usa la voce britannica.
pub fn voice_for(language: Option<&str>) -> Voice {
    let code = language.unwrap_or("it").to_ascii_lowercase();
    if code.starts_with("en") {
        ENGLISH_VOICE
    } else {
        ITALIAN_VOICE
    }
}

#[derive(Debug, Clone)]
pub struct VoicePaths {
    pub engine: PathBuf,
    pub model: PathBuf,
    pub config: PathBuf,
    pub cache_dir: PathBuf,
    pub voice: Voice,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceStatus {
    pub installed: bool,
    pub voice: Option<String>,
    pub license: Option<String>,
    pub engine: Option<String>,
    pub model: Option<String>,
    pub cache_files: usize,
    /// `piper` quando il motore neurale è utilizzabile, `system` altrimenti.
    pub backend: String,
    pub notes: Vec<String>,
    /// Voci effettivamente presenti su disco, in qualunque lingua.
    pub voices: Vec<InstalledVoice>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledVoice {
    pub name: String,
    pub license: String,
    pub model: String,
}

/// Cerca motore e modello nelle posizioni note. Ordine: variabili d'ambiente,
/// runtime incluso nelle risorse, ambiente di sviluppo del repository.
pub fn resolve(
    resources_dir: &Path,
    data_dir: &Path,
    language: Option<&str>,
) -> Option<VoicePaths> {
    let voice = voice_for(language);
    let engine = std::env::var("CAA_PIPER")
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| {
            let path = resources_dir.join("voice").join("bin").join("piper");
            path.is_file().then_some(path)
        })
        .or_else(|| {
            let path = resources_dir
                .parent()?
                .join(".voice-venv")
                .join("bin")
                .join("piper");
            path.is_file().then_some(path)
        })?;

    let base = std::env::var("CAA_VOICE_MODEL")
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .unwrap_or_else(|| resources_dir.join("voice").join(format!("{}.onnx", voice.name)));
    if !base.is_file() {
        return None;
    }
    let config = PathBuf::from(format!("{}.json", base.display()));
    if !config.is_file() {
        return None;
    }

    Some(VoicePaths {
        engine,
        model: base,
        config,
        cache_dir: data_dir.join("voice-cache"),
        voice,
    })
}

fn count_cache(cache_dir: &Path) -> usize {
    fs::read_dir(cache_dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("wav"))
                .count()
        })
        .unwrap_or(0)
}

pub fn status(resources_dir: &Path, data_dir: &Path, language: Option<&str>) -> VoiceStatus {
    let selected = voice_for(language);
    let mut voices = Vec::new();
    for voice in [ITALIAN_VOICE, ENGLISH_VOICE] {
        let model = resources_dir
            .join("voice")
            .join(format!("{}.onnx", voice.name));
        if model.is_file() {
            voices.push(InstalledVoice {
                name: voice.name.to_string(),
                license: voice.license.to_string(),
                model: model.to_string_lossy().to_string(),
            });
        }
    }

    match resolve(resources_dir, data_dir, language) {
        Some(paths) => VoiceStatus {
            installed: true,
            voice: Some(selected.name.to_string()),
            license: Some(selected.license.to_string()),
            engine: Some(paths.engine.to_string_lossy().to_string()),
            model: Some(paths.model.to_string_lossy().to_string()),
            cache_files: count_cache(&paths.cache_dir),
            backend: "piper".to_string(),
            notes: vec!["Voce neurale locale: nessuna rete durante la lettura.".to_string()],
            voices,
        },
        None => VoiceStatus {
            installed: false,
            voice: None,
            license: None,
            engine: None,
            model: None,
            cache_files: 0,
            backend: "system".to_string(),
            notes: vec![
                format!(
                    "Voce neurale per «{}» non installata: uso la voce di sistema.",
                    selected.name
                ),
                "Esegui scripts/setup_voice.sh per installare le voci.".to_string(),
            ],
            voices,
        },
    }
}

fn cache_key(text: &str, model: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    model.to_string_lossy().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn last_error_line(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("errore sconosciuto")
        .trim()
        .to_string()
}

/// Genera (o riusa dalla cache) il WAV della frase, già normalizzata per la
/// pronuncia italiana. Il testo originale non viene modificato.
pub fn synthesize(text: &str, paths: &VoicePaths) -> Result<PathBuf, String> {
    let spoken = crate::tts_normalize::normalize(text);
    if spoken.trim().is_empty() {
        return Err("niente da pronunciare".to_string());
    }
    fs::create_dir_all(&paths.cache_dir)
        .map_err(|e| format!("cache vocale non creabile: {e}"))?;
    let output = paths
        .cache_dir
        .join(format!("{}.wav", cache_key(&spoken, &paths.model)));
    let valid = output
        .metadata()
        .map(|meta| meta.len() > 44)
        .unwrap_or(false);
    if valid {
        return Ok(output);
    }

    let mut child = Command::new(&paths.engine)
        .arg("-m")
        .arg(&paths.model)
        .arg("-c")
        .arg(&paths.config)
        .arg("-f")
        .arg(&output)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Piper non avviabile: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(spoken.as_bytes())
            .map_err(|e| format!("testo non inviato a Piper: {e}"))?;
    }
    let result = child
        .wait_with_output()
        .map_err(|e| format!("Piper interrotto: {e}"))?;

    let produced = output.metadata().map(|meta| meta.len()).unwrap_or(0);
    if !result.status.success() || produced <= 44 {
        let _ = fs::remove_file(&output);
        return Err(format!(
            "Piper non ha prodotto audio: {}",
            last_error_line(&result.stderr)
        ));
    }
    Ok(output)
}

/// Riproduce un WAV senza bloccare la UI.
pub fn play(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("afplay")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("riproduzione audio non disponibile: {e}"))
    }
    #[cfg(target_os = "windows")]
    {
        let script = format!(
            "$p = New-Object Media.SoundPlayer '{}'; $p.PlaySync()",
            path.to_string_lossy().replace('\'', "''")
        );
        let status = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(script)
            .status()
            .map_err(|e| format!("riproduzione audio non disponibile: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("la riproduzione audio di Windows ha restituito un errore".to_string())
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = path;
        Err("riproduzione audio non supportata su questo sistema".to_string())
    }
}

/// Sintetizza e riproduce. Restituisce il percorso del WAV in cache.
pub fn speak(text: &str, paths: &VoicePaths) -> Result<PathBuf, String> {
    let wav = synthesize(text, paths)?;
    play(&wav)?;
    Ok(wav)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn fake_engine(dir: &Path, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("fake-piper.sh");
        fs::write(&path, body).unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }

    #[cfg(unix)]
    fn fixture(body: &str) -> (tempfile::TempDir, VoicePaths) {
        let dir = tempfile::tempdir().unwrap();
        let engine = fake_engine(dir.path(), body);
        let model = dir.path().join("voice.onnx");
        fs::write(&model, b"model").unwrap();
        let config = dir.path().join("voice.onnx.json");
        fs::write(&config, b"{}").unwrap();
        let paths = VoicePaths {
            engine,
            model,
            config,
            cache_dir: dir.path().join("cache"),
            voice: ITALIAN_VOICE,
        };
        (dir, paths)
    }

    const WORKING_ENGINE: &str = "#!/bin/sh\n\
        echo x >> \"$0.count\"\n\
        out=\"\"\n\
        while [ \"$#\" -gt 0 ]; do\n\
          case \"$1\" in -f) out=\"$2\"; shift 2;; *) shift 2;; esac\n\
        done\n\
        cat > /dev/null\n\
        dd if=/dev/zero bs=1 count=200 2>/dev/null > \"$out\"\n";

    #[cfg(unix)]
    #[test]
    fn second_request_is_served_from_cache() {
        let (dir, paths) = fixture(WORKING_ENGINE);
        let first = synthesize("Oggi sono le 8:30.", &paths).unwrap();
        assert!(first.is_file());
        assert!(fs::metadata(&first).unwrap().len() > 44);
        let second = synthesize("Oggi sono le 8:30.", &paths).unwrap();
        assert_eq!(first, second);
        let count = fs::read_to_string(dir.path().join("fake-piper.sh.count")).unwrap();
        assert_eq!(count.lines().count(), 1, "il motore è stato richiamato");
    }

    #[cfg(unix)]
    #[test]
    fn engine_failure_reports_stderr_and_leaves_no_wav() {
        let (_dir, paths) = fixture("#!/bin/sh\ncat > /dev/null\n echo 'voce rotta' >&2\nexit 3\n");
        let error = synthesize("Ciao", &paths).unwrap_err();
        assert!(error.contains("voce rotta"), "{error}");
        assert_eq!(count_cache(&paths.cache_dir), 0);
    }

    #[cfg(unix)]
    #[test]
    fn different_text_produces_a_different_cache_entry() {
        let (_dir, paths) = fixture(WORKING_ENGINE);
        let first = synthesize("Ciao", &paths).unwrap();
        let second = synthesize("Ciao a tutti", &paths).unwrap();
        assert_ne!(first, second);
        assert_eq!(count_cache(&paths.cache_dir), 2);
    }

    #[test]
    fn missing_engine_means_system_backend() {
        let dir = tempfile::tempdir().unwrap();
        let resources = dir.path().join("resources");
        fs::create_dir_all(resources.join("voice")).unwrap();
        let status = status(&resources, dir.path(), Some("it"));
        assert!(!status.installed);
        assert_eq!(status.backend, "system");
        assert!(status.notes.iter().any(|note| note.contains("voce di sistema")));
        assert!(status.voices.is_empty());
    }

    fn resources_with(language_models: &[Voice]) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let resources = dir.path().join("resources");
        let voice_dir = resources.join("voice");
        fs::create_dir_all(voice_dir.join("bin")).unwrap();
        fs::write(voice_dir.join("bin").join("piper"), b"#!/bin/sh\n").unwrap();
        for voice in language_models {
            fs::write(voice_dir.join(format!("{}.onnx", voice.name)), b"model").unwrap();
            fs::write(
                voice_dir.join(format!("{}.onnx.json", voice.name)),
                b"{}",
            )
            .unwrap();
        }
        (dir, resources)
    }

    #[test]
    fn language_selects_the_matching_voice() {
        let (dir, resources) = resources_with(&[ITALIAN_VOICE, ENGLISH_VOICE]);
        let italian = resolve(&resources, dir.path(), Some("it")).unwrap();
        assert_eq!(italian.voice.name, "it_IT-serena-medium");
        let english = resolve(&resources, dir.path(), Some("en-GB")).unwrap();
        assert_eq!(english.voice.name, "en_GB-cori-high");
        // L'inglese usa comunque la propria voce, non quella italiana.
        assert_ne!(italian.model, english.model);
    }

    #[test]
    fn missing_english_voice_does_not_speak_italian_phonemes() {
        let (dir, resources) = resources_with(&[ITALIAN_VOICE]);
        assert!(resolve(&resources, dir.path(), Some("en-GB")).is_none());
        let status = status(&resources, dir.path(), Some("en-GB"));
        assert!(!status.installed);
        assert_eq!(status.backend, "system");
        assert_eq!(status.voices.len(), 1);
        assert_eq!(status.voices[0].name, "it_IT-serena-medium");
    }

    #[test]
    fn cache_key_changes_with_model_and_text() {
        let a = cache_key("Ciao", Path::new("/m/a.onnx"));
        let b = cache_key("Ciao", Path::new("/m/b.onnx"));
        let c = cache_key("Ciao!", Path::new("/m/a.onnx"));
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_eq!(a, cache_key("Ciao", Path::new("/m/a.onnx")));
    }
}
