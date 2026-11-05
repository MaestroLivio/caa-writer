//! Normalizzazione del testo italiano.
//!
//! Obiettivo: confronti stabili tra forme digitate in modi diversi
//! (apostrofi tipografici, maiuscole, accenti combinati) senza alterare
//! il testo originale mostrato all'utente.

use unicode_normalization::UnicodeNormalization;

/// Minuscolo + apostrofi uniformati + normalizzazione Unicode NFC.
pub fn normalize_word(word: &str) -> String {
    let unified: String = word
        .chars()
        .map(|c| match c {
            '\u{2019}' | '\u{02BC}' | '`' | '´' => '\'',
            other => other,
        })
        .collect();
    unified.nfc().collect::<String>().to_lowercase()
}

/// Forma senza accenti, usata solo come indice di riserva.
pub fn fold_accents(word: &str) -> String {
    word.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_and_apostrophes() {
        assert_eq!(normalize_word("Dell’Acqua"), "dell'acqua");
    }

    #[test]
    fn folds_accents() {
        assert_eq!(fold_accents("città"), "citta");
        assert_eq!(fold_accents("perché"), "perche");
    }
}
