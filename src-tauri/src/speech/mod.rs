//! Sintesi vocale offline con le voci di sistema.
//!
//! - macOS: comando `say` (voci di sistema, nessuna rete).
//! - Windows: System.Speech via PowerShell (voci SAPI installate), avviato
//!   senza finestra console.
//! - Altri sistemi: `spd-say` se disponibile.
//!
//! La lettura usa sempre il testo originale, non la descrizione del simbolo.
//! Il processo di lettura viene ricordato: una nuova richiesta interrompe la
//! precedente e il comando «Ferma voce» può terminarla. Nessun processo resta
//! orfano dopo la chiusura dell'applicazione.

use std::process::Child;
use std::sync::{Mutex, OnceLock};

use crate::model::VoiceInfo;

fn speaker_slot() -> &'static Mutex<Option<Child>> {
    static SLOT: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Registra un processo audio e ferma quello precedente. La riproduzione è
/// volutamente non bloccante: la UI mostra subito il feedback.
pub fn track(child: Child) {
    if let Ok(mut slot) = speaker_slot().lock() {
        if let Some(mut previous) = slot.take() {
            let _ = previous.kill();
            let _ = previous.wait();
        }
        *slot = Some(child);
    }
}

/// Interrompe la lettura in corso, se c'è.
pub fn stop() {
    if let Ok(mut slot) = speaker_slot().lock() {
        if let Some(mut child) = slot.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn is_speaking() -> bool {
    let Ok(mut slot) = speaker_slot().lock() else {
        return false;
    };
    let Some(child) = slot.as_mut() else {
        return false;
    };
    match child.try_wait() {
        Ok(Some(_)) => {
            // Il processo è terminato: lo slot va liberato, altrimenti la UI
            // mostrerebbe «Lettura in corso» per sempre e il motore neurale
            // resterebbe bloccato dalla priorità della voce.
            *slot = None;
            false
        }
        Ok(None) => true,
        Err(_) => {
            *slot = None;
            false
        }
    }
}

pub fn list_voices() -> Vec<VoiceInfo> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = crate::process::command("say").arg("-v").arg("?").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut voices = Vec::new();
            for line in text.lines() {
                // Formato: "Alice               it_IT    # Ciao!"
                let head = line.split('#').next().unwrap_or("").trim();
                let mut parts = head.split_whitespace();
                if let (Some(name), Some(lang)) = (parts.next(), parts.next()) {
                    let language = lang.split('_').next().unwrap_or(lang).to_string();
                    voices.push(VoiceInfo {
                        id: name.to_string(),
                        name: name.to_string(),
                        language,
                    });
                }
            }
            return voices;
        }
        Vec::new()
    }
    #[cfg(target_os = "windows")]
    {
        let script = "Add-Type -AssemblyName System.Speech; \
            $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
            $s.GetInstalledVoices() | ForEach-Object { \
                $i = $_.VoiceInfo; Write-Output ($i.Name + '|' + $i.Culture.Name) }";
        if let Ok(output) = crate::process::command("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(script)
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            return text
                .lines()
                .filter_map(|line| {
                    let mut parts = line.trim().split('|');
                    let name = parts.next()?.to_string();
                    let culture = parts.next().unwrap_or("it-IT");
                    let language = culture.split('-').next().unwrap_or("it").to_string();
                    Some(VoiceInfo {
                        id: name.clone(),
                        name,
                        language,
                    })
                })
                .collect();
        }
        Vec::new()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Vec::new()
    }
}

pub fn speak(text: &str, voice: Option<&str>, rate: Option<i32>) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }
    // Numeri, orari, date e abbreviazioni vengono letti in parole; il testo del
    // documento resta invariato perché `text` non viene riscritto.
    let spoken = crate::tts_normalize::normalize(text);
    stop();

    #[cfg(target_os = "macos")]
    {
        let mut command = crate::process::command("say");
        if let Some(voice) = voice.filter(|v| !v.is_empty()) {
            command.arg("-v").arg(voice);
        }
        let words_per_minute = rate.unwrap_or(180).clamp(80, 400);
        command.arg("-r").arg(words_per_minute.to_string());
        command.arg(&spoken);
        let child = command
            .spawn()
            .map_err(|e| format!("sintesi vocale non disponibile: {e}"))?;
        track(child);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let sapi_rate = rate.unwrap_or(0).clamp(-10, 10);
        let script = format!(
            "Add-Type -AssemblyName System.Speech; \
             $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
             $s.Rate = {sapi_rate}; \
             if ($env:CAA_VOICE -ne '') {{ try {{ $s.SelectVoice($env:CAA_VOICE) }} catch {{}} }}; \
             $s.Speak($env:CAA_TEXT)"
        );
        let child = crate::process::command("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-WindowStyle")
            .arg("Hidden")
            .arg("-Command")
            .arg(script)
            .env("CAA_TEXT", &spoken)
            .env("CAA_VOICE", voice.unwrap_or(""))
            .spawn()
            .map_err(|e| format!("sintesi vocale non disponibile: {e}"))?;
        track(child);
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let child = crate::process::command("spd-say")
            .arg("-w")
            .arg(&spoken)
            .spawn()
            .map_err(|e| format!("sintesi vocale non disponibile: {e}"))?;
        track(child);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_is_a_noop() {
        assert!(speak("   ", None, None).is_ok());
    }
}
