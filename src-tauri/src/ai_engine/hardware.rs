//! Rilevamento delle risorse della macchina e scelta del profilo.
//!
//! Il profilo non dipende solo dalla RAM totale: contano anche la memoria
//! disponibile, il numero di core e la piattaforma. I valori sono misurati al
//! momento, non dedotti dal modello di computer.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub os: String,
    pub arch: String,
    pub cpu_cores: usize,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    /// Accelerazione disponibile e utilizzabile (Metal su macOS arm64).
    pub accelerated: bool,
    /// `essenziale` | `standard` | `avanzato`
    pub profile: String,
    pub max_context: u32,
    pub max_threads: u32,
    /// Quante inferenze possono essere eseguite contemporaneamente.
    pub concurrent_requests: u32,
    pub note: String,
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = crate::process::command(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn total_ram_mb() -> u64 {
    if cfg!(target_os = "macos") {
        if let Some(raw) = command_output("sysctl", &["-n", "hw.memsize"]) {
            if let Ok(bytes) = raw.parse::<u64>() {
                return bytes / 1024 / 1024;
            }
        }
    }
    if cfg!(target_os = "windows") {
        if let Some(raw) = command_output(
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                "(Get-CimInstance Win32_OperatingSystem).TotalVisibleMemorySize",
            ],
        ) {
            if let Ok(kb) = raw.trim().parse::<u64>() {
                return kb / 1024;
            }
        }
    }
    if let Ok(raw) = std::fs::read_to_string("/proc/meminfo") {
        for line in raw.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                if let Some(kb) = rest.trim().split_whitespace().next() {
                    if let Ok(kb) = kb.parse::<u64>() {
                        return kb / 1024;
                    }
                }
            }
        }
    }
    0
}

fn available_ram_mb() -> u64 {
    if cfg!(target_os = "macos") {
        // vm_stat riporta pagine libere e inattive: sono la memoria riusabile.
        if let Some(raw) = command_output("vm_stat", &[]) {
            let page_size = raw
                .lines()
                .find_map(|line| line.split("page size of").nth(1))
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(4096);
            let mut pages = 0u64;
            for line in raw.lines() {
                let lower = line.to_lowercase();
                if lower.starts_with("pages free")
                    || lower.starts_with("pages inactive")
                    || lower.starts_with("pages speculative")
                {
                    if let Some(value) = line.split(':').nth(1) {
                        if let Ok(count) = value.trim().trim_end_matches('.').parse::<u64>() {
                            pages += count;
                        }
                    }
                }
            }
            if pages > 0 {
                return pages * page_size / 1024 / 1024;
            }
        }
    }
    if cfg!(target_os = "windows") {
        if let Some(raw) = command_output(
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                "(Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory",
            ],
        ) {
            if let Ok(kb) = raw.trim().parse::<u64>() {
                return kb / 1024;
            }
        }
    }
    if let Ok(raw) = std::fs::read_to_string("/proc/meminfo") {
        for line in raw.lines() {
            if let Some(rest) = line.strip_prefix("MemAvailable:") {
                if let Some(kb) = rest.trim().split_whitespace().next() {
                    if let Ok(kb) = kb.parse::<u64>() {
                        return kb / 1024;
                    }
                }
            }
        }
    }
    0
}

pub fn detect() -> HardwareProfile {
    let total = total_ram_mb();
    let available = available_ram_mb();
    let cores = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(2);
    let arch = std::env::consts::ARCH.to_string();
    let os = std::env::consts::OS.to_string();
    let accelerated = os == "macos" && arch == "aarch64";

    // Il profilo considera la memoria realmente disponibile: un computer con
    // 8 GB ma con 2 GB liberi non può caricare un modello da 3 miliardi di
    // parametri senza rallentare il sistema.
    let usable = if available > 0 {
        available.min(total)
    } else {
        total
    };
    let (profile, max_context, concurrent) = if total == 0 || usable < 3000 {
        ("essenziale", 1024u32, 1u32)
    } else if usable < 6000 {
        ("standard", 2048, 1)
    } else {
        ("avanzato", 4096, 1)
    };

    let max_threads = (cores.saturating_sub(1).max(1)).min(8) as u32;
    let note = match profile {
        "essenziale" => {
            "Profilo essenziale: contesto ridotto e una sola inferenza alla volta."
        }
        "standard" => "Profilo standard: contesto medio, una inferenza alla volta.",
        _ => "Profilo avanzato: contesto ampio, una inferenza alla volta.",
    };

    HardwareProfile {
        os,
        arch,
        cpu_cores: cores,
        total_ram_mb: total,
        available_ram_mb: available,
        accelerated,
        profile: profile.to_string(),
        max_context,
        max_threads,
        concurrent_requests: concurrent,
        note: note.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_reports_plausible_values() {
        let profile = detect();
        assert!(profile.cpu_cores >= 1);
        assert!(!profile.profile.is_empty());
        assert!(profile.max_context >= 512);
        assert!(profile.concurrent_requests >= 1);
        // La RAM può non essere leggibile su ambienti insoliti: in quel caso
        // resta 0 e il profilo deve comunque essere quello essenziale.
        if profile.total_ram_mb == 0 {
            assert_eq!(profile.profile, "essenziale");
        }
    }
}
