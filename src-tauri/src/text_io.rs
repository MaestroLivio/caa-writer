//! Lettura di testo tollerante alla codifica.
//!
//! I file dell'app sono UTF-8, ma i documenti importati e le copie create da
//! altri programmi su Windows possono essere in Windows-1252 o UTF-16. In quei
//! casi `std::fs::read_to_string` fallisce con «stream did not contain valid
//! UTF-8» e interrompe l'apertura. Questa lettura riconosce i BOM, converte
//! UTF-16 e, per gli altri byte, usa una decodifica con sostituzione: il testo
//! resta leggibile invece di bloccare l'app.

use std::path::Path;

/// Legge un file di testo senza mai fallire per la codifica. Gli errori di I/O
/// restano errori reali.
pub fn read_lossy(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(decode_lossy(&bytes))
}

/// Converte byte arbitrari in una stringa UTF-8 valida.
pub fn decode_lossy(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16(rest, true);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16(rest, false);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .collect();
    String::from_utf16_lossy(&units)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_utf8_is_preserved() {
        assert_eq!(decode_lossy("caffè".as_bytes()), "caffè");
    }

    #[test]
    fn utf8_bom_is_stripped() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("città".as_bytes());
        assert_eq!(decode_lossy(&bytes), "città");
    }

    #[test]
    fn utf16_le_is_decoded() {
        let text = "Perché";
        let mut bytes = vec![0xFF, 0xFE];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(decode_lossy(&bytes), text);
    }

    #[test]
    fn invalid_bytes_do_not_fail() {
        let decoded = decode_lossy(&[b'c', b'a', 0xFF, b'f', b'f', 0xE8]);
        assert!(decoded.starts_with("ca"));
        assert!(decoded.contains('f'));
    }
}
