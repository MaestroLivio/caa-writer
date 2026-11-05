//! Sintesi vocale offline con le voci di sistema.
//!
//! - macOS: comando `say` (voci di sistema, nessuna rete).
//! - Windows: System.Speech via PowerShell (voci SAPI installate).
//! - Altri sistemi: `spd-say` se disponibile.
//!
//! La lettura usa sempre il testo originale, non la descrizione del simbolo.

use std::process::Command;

use crate::model::VoiceInfo;

pub fn list_voices() -> Vec<VoiceInfo> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("say").arg("-v").arg("?").output() {
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
        if let Ok(output) = Command::new("powershell")
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

    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("say");
        if let Some(voice) = voice.filter(|v| !v.is_empty()) {
            command.arg("-v").arg(voice);
        }
        let words_per_minute = rate.unwrap_or(180).clamp(80, 400);
        command.arg("-r").arg(words_per_minute.to_string());
        command.arg(&spoken);
        command
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("sintesi vocale non disponibile: {e}"))
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
        let status = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(script)
            .env("CAA_TEXT", &spoken)
            .env("CAA_VOICE", voice.unwrap_or(""))
            .status()
            .map_err(|e| format!("sintesi vocale non disponibile: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("la sintesi vocale di Windows ha restituito un errore".to_string())
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let status = Command::new("spd-say")
            .arg("-w")
            .arg(&spoken)
            .status()
            .map_err(|e| format!("sintesi vocale non disponibile: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("spd-say ha restituito un errore".to_string())
        }
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
