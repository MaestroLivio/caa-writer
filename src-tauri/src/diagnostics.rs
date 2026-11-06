//! Misure reali di CPU e memoria dei processi dell'applicazione.
//!
//! Non si stimano i consumi: si interroga il sistema operativo al momento
//! della richiesta. La lettura avviene su richiesta esplicita del pannello
//! diagnostico del Creator, non durante la digitazione, quindi non introduce
//! lavoro nel percorso critico.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessSample {
    pub pid: u32,
    /// Memoria residente in MiB.
    pub resident_mb: f64,
    /// Percentuale di CPU istantanea riportata dal sistema.
    pub cpu_percent: f64,
    /// Vero se il processo esiste ed è leggibile.
    pub running: bool,
    /// Spiegazione onesta quando la misura non è disponibile.
    pub note: String,
}

#[cfg(unix)]
fn sample_unix(pid: u32) -> ProcessSample {
    let output = crate::process::command("ps")
        .arg("-o")
        .arg("rss=,%cpu=")
        .arg("-p")
        .arg(pid.to_string())
        .output();
    let Ok(output) = output else {
        return ProcessSample {
            pid,
            note: "ps non disponibile".to_string(),
            ..ProcessSample::default()
        };
    };
    if !output.status.success() {
        return ProcessSample {
            pid,
            note: "processo non attivo".to_string(),
            ..ProcessSample::default()
        };
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut parts = text.split_whitespace();
    let rss_kb = parts.next().and_then(|v| v.parse::<f64>().ok());
    let cpu = parts.next().and_then(|v| v.parse::<f64>().ok());
    match (rss_kb, cpu) {
        (Some(rss), Some(cpu)) => ProcessSample {
            pid,
            resident_mb: (rss / 1024.0 * 10.0).round() / 10.0,
            cpu_percent: (cpu * 10.0).round() / 10.0,
            running: true,
            note: String::new(),
        },
        _ => ProcessSample {
            pid,
            note: "formato di ps non riconosciuto".to_string(),
            ..ProcessSample::default()
        },
    }
}

#[cfg(windows)]
fn sample_windows(pid: u32) -> ProcessSample {
    let script = format!(
        "$p = Get-Process -Id {pid} -ErrorAction SilentlyContinue; \
         if ($p) {{ Write-Output ($p.WorkingSet64.ToString() + '|' + $p.CPU) }} \
         else {{ Write-Output 'missing' }}"
    );
    let output = crate::process::command("powershell")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-Command")
        .arg(script)
        .output();
    let Ok(output) = output else {
        return ProcessSample {
            pid,
            note: "PowerShell non disponibile".to_string(),
            ..ProcessSample::default()
        };
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim();
    if text == "missing" || text.is_empty() {
        return ProcessSample {
            pid,
            note: "processo non attivo".to_string(),
            ..ProcessSample::default()
        };
    }
    let mut parts = text.split('|');
    let bytes = parts.next().and_then(|v| v.parse::<f64>().ok());
    let cpu_seconds = parts.next().and_then(|v| v.parse::<f64>().ok());
    ProcessSample {
        pid,
        resident_mb: bytes
            .map(|value| (value / 1024.0 / 1024.0 * 10.0).round() / 10.0)
            .unwrap_or(0.0),
        // Get-Process espone i secondi CPU cumulati, non una percentuale
        // istantanea: si riporta quel dato senza trasformarlo in una stima.
        cpu_percent: cpu_seconds.map(|value| (value * 10.0).round() / 10.0).unwrap_or(0.0),
        running: true,
        note: "CPU Windows: secondi cumulati del processo (Get-Process)".to_string(),
    }
}

pub fn sample(pid: u32) -> ProcessSample {
    #[cfg(unix)]
    {
        sample_unix(pid)
    }
    #[cfg(windows)]
    {
        sample_windows(pid)
    }
    #[cfg(not(any(unix, windows)))]
    {
        ProcessSample {
            pid,
            note: "misura non disponibile su questa piattaforma".to_string(),
            ..ProcessSample::default()
        }
    }
}

pub fn current_process() -> ProcessSample {
    sample(std::process::id())
}
