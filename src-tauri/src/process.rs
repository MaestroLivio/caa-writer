//! Avvio uniforme dei processi esterni.
//!
//! Su Windows ogni processo creato da un'applicazione GUI può aprire una
//! finestra console (llama.cpp, PowerShell, curl, Piper). La finestra compare
//! *prima* che il processo possa essere nascosto, quindi non basta agire dopo
//! l'avvio: il flag `CREATE_NO_WINDOW` va passato direttamente alla creazione.
//!
//! Questo modulo è l'unico punto in cui viene costruito un `Command` per i
//! processi esterni. Non usa `DETACHED_PROCESS`: i processi restano figli
//! dell'applicazione, così possiamo controllarne stdin/stdout e terminazione
//! (nessun processo orfano, nessuna perdita del ciclo di vita).

use std::ffi::OsStr;
use std::process::Command;

/// `CREATE_NO_WINDOW` di Win32: il processo parte senza console visibile.
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// `SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX` di Win32.
///
/// Un processo figlio a cui manca una DLL (per esempio `piper.exe` senza
/// `piper_phonemize.dll`) fa comparire una finestra di sistema «L'esecuzione
/// del codice non può proseguire…», che l'utente deve chiudere a mano. Con
/// questi flag l'errore resta un errore di avvio: il fallback alla voce di
/// sistema funziona e la finestra non compare. Il flag è ereditato dai figli,
/// quindi va impostato prima di creare il processo.
#[cfg(windows)]
const SYSTEM_ERROR_MODE: u32 = 0x0001 | 0x8000;

/// Spegne le finestre di errore del sistema per questo processo e per i figli.
#[cfg(windows)]
pub fn suppress_system_error_dialogs() {
    extern "system" {
        fn SetErrorMode(mode: u32) -> u32;
    }
    // Sicuro: `SetErrorMode` agisce solo sul processo corrente ed è idempotente.
    unsafe {
        SetErrorMode(SYSTEM_ERROR_MODE);
    }
}

/// Crea un comando già configurato per non mostrare finestre console.
pub fn command<S: AsRef<OsStr>>(program: S) -> Command {
    let mut command = Command::new(program);
    configure(&mut command);
    command
}

/// Applica i flag di piattaforma a un comando esistente.
pub fn configure(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Prima della creazione: il figlio eredita la modalità d'errore, così
        // una DLL mancante non apre una finestra di dialogo.
        suppress_system_error_dialogs();
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_is_built_for_the_current_platform() {
        let command = command("echo");
        let debug = format!("{command:?}");
        assert!(debug.contains("echo"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_flag_is_the_documented_value() {
        assert_eq!(CREATE_NO_WINDOW, 0x0800_0000);
    }

    #[cfg(windows)]
    #[test]
    fn system_error_mode_masks_the_loader_dialogs() {
        assert_eq!(SYSTEM_ERROR_MODE & 0x0001, 0x0001, "SEM_FAILCRITICALERRORS");
        assert_eq!(SYSTEM_ERROR_MODE & 0x8000, 0x8000, "SEM_NOOPENFILEERRORBOX");
        // Idempotente: due chiamate non cambiano il comportamento.
        suppress_system_error_dialogs();
        suppress_system_error_dialogs();
    }
}
