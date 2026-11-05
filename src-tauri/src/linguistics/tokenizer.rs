//! Tokenizzazione dell'italiano.
//!
//! Il tokenizzatore mantiene gli offset in caratteri (non byte) perché il
//! frontend lavora con stringhe JavaScript. Punteggiatura, apostrofi e
//! accenti non devono rompere l'allineamento tra testo e pittogrammi.

#[derive(Debug, Clone, PartialEq)]
pub struct RawToken {
    pub text: String,
    /// Offsets in caratteri (code point), relativi al paragrafo.
    pub start: usize,
    pub end: usize,
    pub kind: TokenKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Word,
    Number,
    Punctuation,
}

impl RawToken {
    pub fn is_word(&self) -> bool {
        self.kind == TokenKind::Word
    }
    pub fn is_punct(&self) -> bool {
        self.kind == TokenKind::Punctuation
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '\u{2019}'
}

fn is_apostrophe(c: char) -> bool {
    c == '\'' || c == '\u{2019}' || c == '\u{02BC}'
}

fn is_punct(c: char) -> bool {
    matches!(
        c,
        '.' | ',' | '!' | '?' | ';' | ':' | '«' | '»' | '"' | '(' | ')' | '[' | ']' | '–' | '—'
            | '…' | '/' | '\\' | '*' | '=' | '+' | '%' | '§' | '&' | '<' | '>' | '|' | '_' | '@'
    )
}

/// Divide il testo in token conservando posizione e tipo.
pub fn tokenize(text: &str) -> Vec<RawToken> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens: Vec<RawToken> = Vec::new();
    let mut idx = 0usize;
    let mut buffer = String::new();
    let mut buffer_start = 0usize;
    let mut buffer_kind = TokenKind::Word;

    let flush = |tokens: &mut Vec<RawToken>,
                 buffer: &mut String,
                 start: &mut usize,
                 end: usize,
                 kind: TokenKind| {
        if buffer.is_empty() {
            return;
        }
        tokens.push(RawToken {
            text: std::mem::take(buffer),
            start: *start,
            end,
            kind,
        });
    };

    while idx < chars.len() {
        let c = chars[idx];
        if c.is_whitespace() {
            flush(&mut tokens, &mut buffer, &mut buffer_start, idx, buffer_kind);
            idx += 1;
            continue;
        }
        if is_apostrophe(c) {
            // "l'acqua" -> "l'" + "acqua"; "dell'acqua" -> "dell'" + "acqua".
            if !buffer.is_empty() {
                buffer.push('\'');
                flush(&mut tokens, &mut buffer, &mut buffer_start, idx + 1, buffer_kind);
                idx += 1;
            } else {
                buffer_start = idx;
                buffer_kind = TokenKind::Punctuation;
                buffer.push('\'');
                flush(&mut tokens, &mut buffer, &mut buffer_start, idx + 1, buffer_kind);
                idx += 1;
            }
            continue;
        }
        if is_word_char(c) {
            if buffer.is_empty() {
                buffer_start = idx;
                buffer_kind = if c.is_numeric() {
                    TokenKind::Number
                } else {
                    TokenKind::Word
                };
            }
            if c.is_numeric() {
                buffer.push(c);
                idx += 1;
                continue;
            }
            buffer.push(c);
            idx += 1;
            continue;
        }
        if is_punct(c) {
            flush(&mut tokens, &mut buffer, &mut buffer_start, idx, buffer_kind);
            tokens.push(RawToken {
                text: c.to_string(),
                start: idx,
                end: idx + 1,
                kind: TokenKind::Punctuation,
            });
            idx += 1;
            continue;
        }
        // Simbolo sconosciuto: trattato come punteggiatura isolata.
        flush(&mut tokens, &mut buffer, &mut buffer_start, idx, buffer_kind);
        idx += 1;
    }
    flush(&mut tokens, &mut buffer, &mut buffer_start, chars.len(), buffer_kind);
    tokens
}

/// Divide il testo in frasi (per l'analisi contestuale e i limiti del prompt).
pub fn split_sentences(tokens: &[RawToken]) -> Vec<Vec<usize>> {
    let mut sentences = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        if token.is_punct() && matches!(token.text.as_str(), "." | "!" | "?" | "…") {
            current.push(i);
            sentences.push(std::mem::take(&mut current));
            continue;
        }
        current.push(i);
    }
    if !current.is_empty() {
        sentences.push(current);
    }
    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(tokens: &[RawToken]) -> Vec<&str> {
        tokens.iter().map(|t| t.text.as_str()).collect()
    }

    #[test]
    fn splits_apostrophes_italian() {
        let t = tokenize("L'acqua dell'orto.");
        assert_eq!(texts(&t), vec!["L'", "acqua", "dell'", "orto", "."]);
    }

    #[test]
    fn keeps_offsets_in_characters() {
        let t = tokenize("Città bella");
        assert_eq!(t[0].start, 0);
        assert_eq!(t[0].end, 5);
        // "Città" ha 5 caratteri ma 6 byte: gli offset devono restare 5.
        assert_eq!(t[1].start, 6);
    }

    #[test]
    fn punctuation_is_its_own_token() {
        let t = tokenize("Ciao, mamma!");
        assert_eq!(texts(&t), vec!["Ciao", ",", "mamma", "!"]);
    }

    #[test]
    fn numbers_are_tokens() {
        let t = tokenize("Ho 3 mele");
        assert_eq!(t[1].kind, TokenKind::Number);
    }
}
