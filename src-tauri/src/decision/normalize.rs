//! Normalizzazione per l'inferenza, senza modificare il testo dell'utente.

use unicode_normalization::UnicodeNormalization;

use crate::linguistics::normalize::{fold_accents, normalize_word};

/// Forma canonica usata per cache, regole e confronti.
pub fn normalize(text: &str) -> String {
    let unified: String = text
        .chars()
        .map(|c| match c {
            '\u{2019}' | '\u{2018}' | '\u{02BC}' | '`' | '´' => '\'',
            '\u{00A0}' => ' ',
            other => other,
        })
        .collect();
    let lowered = unified.nfkc().collect::<String>().to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut last_space = false;
    for ch in lowered.chars() {
        if ch.is_whitespace() {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
        } else {
            out.push(ch);
            last_space = false;
        }
    }
    out.trim().to_string()
}

/// Taglia ai primi `max` caratteri (Unicode), rispettando i confini.
pub fn truncate_chars(text: &str, max: usize) -> (String, bool) {
    if text.chars().count() <= max {
        return (text.to_string(), false);
    }
    (text.chars().take(max).collect(), true)
}

/// Parole normalizzate, con accenti ripiegati per i confronti.
pub fn tokens(text: &str) -> Vec<String> {
    normalize(text)
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .map(|word| fold_accents(&normalize_word(word)))
        .filter(|word| !word.is_empty())
        .collect()
}

/// Estrae la parola indicata dall'intervallo di caratteri.
pub fn target_text(text: &str, start: usize, end: usize) -> Option<String> {
    let slice: String = text
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect();
    if slice.trim().is_empty() {
        None
    } else {
        Some(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_unicode_and_spaces() {
        assert_eq!(normalize("  Città   dell’Acqua  "), "città dell'acqua");
        let decomposed = "citta\u{0300}";
        assert_eq!(normalize(decomposed), "città");
    }

    #[test]
    fn truncates_by_characters_not_bytes() {
        let (text, truncated) = truncate_chars("àèìòù", 3);
        assert_eq!(text, "àèì");
        assert!(truncated);
    }

    #[test]
    fn tokens_fold_accents() {
        let got = tokens("Perché la città?");
        assert_eq!(got, vec!["perche", "la", "citta"]);
    }

    #[test]
    fn target_uses_character_offsets() {
        let text = "Ho mangiato una pesca.";
        assert_eq!(target_text(text, 16, 21).as_deref(), Some("pesca"));
    }
}
