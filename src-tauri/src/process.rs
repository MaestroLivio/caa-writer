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
}
