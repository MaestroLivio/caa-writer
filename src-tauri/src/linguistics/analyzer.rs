//! Analisi linguistica (Livello 1 della pipeline).
//!
//! Produce, per ogni frase: token, lemmi, parte del discorso stimata,
//! genere/numero, negazione, domini contestuali ed espressioni multiword.
//! Non decide i simboli: quella è responsabilità di `symbol_search`.

use super::lexicon::{LemmaMatch, Lexicon};
use super::normalize::normalize_word;
use super::tokenizer::{split_sentences, tokenize, RawToken, TokenKind};

#[derive(Debug, Clone)]
pub struct TokenAnalysis {
    pub surface: String,
    pub start: usize,
    pub end: usize,
    pub normalized: String,
    pub lemma: Option<String>,
    pub pos: String,
    pub gender: Option<String>,
    pub number: Option<String>,
    pub concepts: Vec<String>,
    /// Parola di contenuto (merita un simbolo in modalità Minima).
    pub key: bool,
    pub is_punct: bool,
    pub is_function: bool,
    pub negated: bool,
    /// Concetto dell'espressione multiword, se il token è un'espressione intera.
    pub mwe_concept: Option<String>,
    pub mwe_parts: Vec<String>,
    /// Quanti token originali copre (1 = token normale).
    pub span: usize,
}

#[derive(Debug, Clone)]
pub struct Sentence {
    pub index: usize,
    pub start: usize,
    pub end: usize,
    /// Testo della frase, usato come contesto per la disambiguazione.
    pub text: String,
    pub tokens: Vec<TokenAnalysis>,
    pub domains: Vec<String>,
}

impl TokenAnalysis {
    pub fn is_word(&self) -> bool {
        !self.is_punct
    }
}

#[derive(Debug, Clone, Default)]
pub struct Analysis {
    pub sentences: Vec<Sentence>,
    pub domains: Vec<String>,
}

pub struct Analyzer<'a> {
    lexicon: &'a Lexicon,
}

impl<'a> Analyzer<'a> {
    pub fn new(lexicon: &'a Lexicon) -> Self {
        Self { lexicon }
    }

    pub fn analyze(&self, text: &str) -> Analysis {
        let raw = tokenize(text);
        let sentence_spans = split_sentences(&raw);
        let mut analysis = Analysis::default();
        let mut domain_counts: std::collections::HashMap<String, usize> = Default::default();

        for (index, indices) in sentence_spans.iter().enumerate() {
            let mut tokens: Vec<TokenAnalysis> = indices
                .iter()
                .map(|&i| self.analyze_token(&raw[i]))
                .collect();

            self.merge_multiword(&mut tokens);
            self.mark_negation(&mut tokens);

            let start = indices.first().map(|&i| raw[i].start).unwrap_or(0);
            let end = indices.last().map(|&i| raw[i].end).unwrap_or(0);
            let mut domains: Vec<String> = Vec::new();
            for token in &tokens {
                if !token.is_word() {
                    continue;
                }
                for domain in self.lexicon.domains_for(&token.normalized) {
                    *domain_counts.entry(domain.clone()).or_default() += 1;
                    if !domains.contains(&domain) {
                        domains.push(domain);
                    }
                }
            }
            let n_chars = text.chars().count();
            let sentence_text: String = text
                .chars()
                .skip(start.min(n_chars))
                .take(end.saturating_sub(start))
                .collect();

            analysis.sentences.push(Sentence {
                index,
                start,
                end,
                text: sentence_text,
                tokens,
                domains,
            });
        }

        let mut domains: Vec<(String, usize)> = domain_counts.into_iter().collect();
        domains.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        analysis.domains = domains.into_iter().map(|(d, _)| d).collect();
        analysis
    }

    fn analyze_token(&self, raw: &RawToken) -> TokenAnalysis {
        let normalized = normalize_word(&raw.text);
        let is_punct = raw.kind == TokenKind::Punctuation;
        let is_number = raw.kind == TokenKind::Number;
        let is_function = self.lexicon.is_function_word(&normalized);

        let (mut lemma, mut pos, mut gender, mut number, mut concepts, mut from_lemma) =
            (None, if is_number { "num" } else { "unk" }.to_string(), None, None, Vec::new(), true);

        if !is_punct && !is_number {
            let mut matches = self.lexicon.lookup(&raw.text);
            if matches.is_empty() {
                // Ripiego: forma senza accenti (perche -> perché).
                matches = self.lexicon.lookup_folded(&normalized);
            }
            if let Some(best) = pick_best(&matches) {
                lemma = Some(best.lemma.clone());
                pos = best.pos.clone();
                gender = best.gender.clone();
                number = best.number.clone();
                concepts = best.concepts.clone();
                from_lemma = best.from_lemma;
            } else if is_function {
                pos = "fun".to_string();
            }
        }

        let _ = from_lemma;
        let key = matches!(
            pos.as_str(),
            "noun" | "verb" | "adj" | "adv" | "num"
        ) && !is_function;

        TokenAnalysis {
            surface: raw.text.clone(),
            start: raw.start,
            end: raw.end,
            normalized,
            lemma,
            pos,
            gender,
            number,
            concepts,
            key,
            is_punct,
            is_function,
            negated: false,
            mwe_concept: None,
            mwe_parts: Vec::new(),
            span: 1,
        }
    }

    /// Riconosce le espressioni multiword con la corrispondenza più lunga,
    /// scansionando le forme normalizzate del testo.
    fn merge_multiword(&self, tokens: &mut Vec<TokenAnalysis>) {
        if self.lexicon.mwe.is_empty() {
            return;
        }
        let words: Vec<(usize, String)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_word())
            .map(|(i, t)| (i, t.normalized.clone()))
            .collect();
        let mut consumed = vec![false; tokens.len()];
        let mut replacements: Vec<(usize, TokenAnalysis)> = Vec::new();

        let mut i = 0;
        while i < words.len() {
            let mut best: Option<(usize, String)> = None; // (numero di parole, concetto)
            for len in (2..=6).rev() {
                if i + len > words.len() {
                    continue;
                }
                // Le parole dell'espressione non devono attraversare la punteggiatura.
                let contiguous = (i..i + len).all(|w| {
                    w == i
                        || words[w].0 == words[w - 1].0 + 1
                });
                if !contiguous {
                    continue;
                }
                let phrase = words[i..i + len]
                    .iter()
                    .map(|(_, s)| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                if let Some(concept) = self.lexicon.mwe.get(&phrase) {
                    best = Some((len, concept.clone()));
                    break;
                }
            }
            if let Some((len, concept)) = best {
                for w in i..i + len {
                    consumed[words[w].0] = true;
                }
                let first_idx = words[i].0;
                let last_idx = words[i + len - 1].0;
                let mut merged = tokens[first_idx].clone();
                merged.surface = tokens[first_idx..=last_idx]
                    .iter()
                    .map(|t| t.surface.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                merged.end = tokens[last_idx].end;
                merged.span = last_idx - first_idx + 1;
                merged.concepts = vec![concept.clone()];
                merged.mwe_concept = Some(concept);
                merged.key = true;
                merged.pos = "mwe".to_string();
                // L'espressione intera è un'unità lessicale: non eredita la
                // funzione grammaticale della prima parola («da sole», «non so»).
                merged.is_function = false;
                merged.normalized = normalize_word(&merged.surface);
                replacements.push((first_idx, merged));
                i += len;
            } else {
                i += 1;
            }
        }

        for (idx, merged) in replacements {
            for (offset, token) in tokens.iter_mut().enumerate() {
                if offset == idx {
                    *token = merged.clone();
                    token.span = merged.span;
                } else if consumed[offset] && offset > idx {
                    token.is_punct = true; // nascosto: sostituito dall'espressione
                    token.span = 0;
                    token.key = false;
                    token.concepts.clear();
                }
            }
        }
    }

    fn mark_negation(&self, tokens: &mut Vec<TokenAnalysis>) {
        let mut negate_next = false;
        for token in tokens.iter_mut() {
            if token.normalized == "non" || token.normalized == "mai" || token.normalized == "senza" {
                negate_next = true;
                continue;
            }
            if negate_next && token.is_word() {
                token.negated = true;
                negate_next = false;
            }
            if token.is_punct {
                negate_next = false;
            }
        }
    }
}

fn pick_best(matches: &[LemmaMatch]) -> Option<&LemmaMatch> {
    matches
        .iter()
        .max_by_key(|m| (m.concepts.len(), m.from_lemma))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linguistics::lexicon::{LexiconFile, Lexicon};

    fn lexicon() -> Lexicon {
        let json = r#"{
            "version": 1,
            "domains": {"scuola": ["banco", "scuola"], "cibo": ["pesca", "mela"]},
            "concepts": [
                {"id":"fruit.peach","label":"pesca","meaning":"frutto","category":"alimenti"},
                {"id":"activity.fishing","label":"pesca","meaning":"attivita","category":"attivita"}
            ],
            "lemmas": [
                {"lemma":"pesca","pos":"noun","gender":"f","concepts":["fruit.peach","activity.fishing"]},
                {"lemma":"mangiare","pos":"verb","concepts":[]},
                {"lemma":"bambino","pos":"noun","gender":"m","concepts":[]}
            ],
            "irregular": {"mangia":"mangiare"},
            "function_words": ["una","il"],
            "mwe": [{"form":"lavarsi i denti","variants":["lava i denti"],"concept":"hygiene.brush_teeth"}]
        }"#;
        let file: LexiconFile = serde_json::from_str(json).unwrap();
        Lexicon::from_file(file)
    }

    #[test]
    fn analysis_marks_function_words() {
        let lex = lexicon();
        let analysis = Analyzer::new(&lex).analyze("Il bambino mangia una pesca.");
        let tokens = &analysis.sentences[0].tokens;
        assert!(tokens[0].is_function);
        assert!(!tokens[1].is_function);
        assert_eq!(tokens[1].lemma.as_deref(), Some("bambino"));
    }

    #[test]
    fn analysis_resolves_inflected_verb() {
        let lex = lexicon();
        let analysis = Analyzer::new(&lex).analyze("Il bambino mangia una pesca.");
        let verb = analysis.sentences[0]
            .tokens
            .iter()
            .find(|t| t.normalized == "mangia")
            .unwrap();
        assert_eq!(verb.lemma.as_deref(), Some("mangiare"));
    }

    #[test]
    fn multiword_expression_becomes_single_token() {
        let lex = lexicon();
        let analysis = Analyzer::new(&lex).analyze("Il bambino lava i denti.");
        let tokens = &analysis.sentences[0].tokens;
        let mwe = tokens.iter().find(|t| t.mwe_concept.is_some()).unwrap();
        assert_eq!(mwe.surface, "lava i denti");
        assert_eq!(mwe.span, 3);
    }

    #[test]
    fn negation_marks_following_key_word() {
        let lex = lexicon();
        let analysis = Analyzer::new(&lex).analyze("Il bambino non mangia la pesca.");
        let tokens = &analysis.sentences[0].tokens;
        let verb = tokens.iter().find(|t| t.normalized == "mangia").unwrap();
        assert!(verb.negated);
    }

    #[test]
    fn domains_are_detected() {
        let lex = lexicon();
        let analysis = Analyzer::new(&lex).analyze("Il banco di scuola.");
        assert!(analysis.domains.contains(&"scuola".to_string()));
    }
}
