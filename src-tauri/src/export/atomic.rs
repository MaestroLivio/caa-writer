//! Scrittura atomica di file: prima su file temporaneo, poi rename.
//! Evita documenti troncati se l'applicazione viene chiusa durante il salvataggio.

use std::path::Path;

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("cartella non creata: {e}"))?;
        }
    }
    let tmp = path.with_extension("writing.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("scrittura non riuscita: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("salvataggio non completato: {e}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_file_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/out.txt");
        write_atomic(&path, b"ciao").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "ciao");
        assert!(!path.with_extension("writing.tmp").exists());
    }
}
