//! Lessico italiano caricato da JSON (risorsa piccola, sempre in memoria).
//!
//! Il lessico è la fonte autorevole per lemma → concetti. L'LLM non lo
//! sostituisce: può solo riordinare i candidati già presenti.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::normalize::normalize_word;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cues {
    #[serde(default)]
    pub prefer: Vec<String>,
    #[serde(default)]
    pub avoid: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptDef {
    pub id: String,
    pub label: String,
    pub meaning: String,
    pub category: String,
    #[serde(default)]
    pub broader: Option<String>,
    #[serde(default)]
    pub pos: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    /// Identificativo di un pittogramma di repertorio da legare al concetto
    /// (per esempio una voce ARASAAC). Il legame viene applicato al seed.
    #[serde(default)]
    pub symbol_ref: Option<String>,
    #[serde(default)]
    pub cues: Cues,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LemmaDef {
    pub lemma: String,
    pub pos: String,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub number: Option<String>,
    #[serde(default)]
    pub concepts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MweDef {
    pub form: String,
    #[serde(default)]
    pub variants: Vec<String>,
    pub concept: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynonymDef {
    pub term: String,
    #[serde(default)]
    pub concepts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationDef {
    pub from: String,
    pub to: String,
    pub kind: String,
    #[serde(default = "default_weight")]
    pub weight: f32,
}

fn default_weight() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexiconFile {
    pub version: u32,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub domains: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub concepts: Vec<ConceptDef>,
    #[serde(default)]
    pub lemmas: Vec<LemmaDef>,
    #[serde(default, rename = "mwe")]
    pub mwe: Vec<MweDef>,
    #[serde(default)]
    pub compositions: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub irregular: HashMap<String, String>,
    #[serde(default)]
    pub function_words: Vec<String>,
    #[serde(default)]
    pub synonyms: Vec<SynonymDef>,
    #[serde(default)]
    pub relations: Vec<RelationDef>,
}

#[derive(Debug, Clone)]
pub struct LemmaEntry {
    pub lemma: String,
    pub pos: String,
    pub gender: Option<String>,
    pub number: Option<String>,
    pub concepts: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Lexicon {
    pub version: u32,
    concepts: HashMap<String, ConceptDef>,
    lemmas: HashMap<String, Vec<LemmaEntry>>,
    irregular: HashMap<String, String>,
    pub function_words: std::collections::HashSet<String>,
    pub domains: HashMap<String, std::collections::HashSet<String>>,
    pub compositions: HashMap<String, Vec<String>>,
    /// forma normalizzata (anche varianti) -> concetto dell'espressione multiword
    pub mwe: HashMap<String, String>,
    /// termine singolo -> concetti (sinonimi e alias)
    pub synonyms: HashMap<String, Vec<String>>,
    /// Relazioni semantiche dichiarate nel lessico (equivalent/related).
    pub relations: Vec<(String, String, String, f32)>,
}

/// Una corrispondenza lemma trovata per una forma superficiale.
#[derive(Debug, Clone)]
pub struct LemmaMatch {
    pub lemma: String,
    pub pos: String,
    pub gender: Option<String>,
    pub number: Option<String>,
    pub concepts: Vec<String>,
    /// true se la forma coincide con il lemma (nessuna flessione inferita)
    pub from_lemma: bool,
}

impl Lexicon {
    pub fn from_file(file: LexiconFile) -> Self {
        let mut lex = Lexicon {
            version: file.version,
            ..Default::default()
        };
        for concept in file.concepts {
            lex.concepts.insert(concept.id.clone(), concept);
        }
        for lemma in file.lemmas {
            let key = normalize_word(&lemma.lemma);
            lex.lemmas.entry(key).or_default().push(LemmaEntry {
                lemma: lemma.lemma,
                pos: lemma.pos,
                gender: lemma.gender,
                number: lemma.number,
                concepts: lemma.concepts,
            });
        }
        for (from, to) in file.irregular {
            lex.irregular.insert(normalize_word(&from), normalize_word(&to));
        }
        for w in file.function_words {
            lex.function_words.insert(normalize_word(&w));
        }
        for (domain, words) in file.domains {
            let set = lex.domains.entry(domain).or_default();
            for w in words {
                set.insert(normalize_word(&w));
            }
        }
        for (concept, parts) in file.compositions {
            lex.compositions.insert(concept, parts);
        }
        for m in file.mwe {
            let concept = m.concept.clone();
            lex.mwe.insert(normalize_word(&m.form), concept.clone());
            for variant in m.variants {
                lex.mwe.insert(normalize_word(&variant), concept.clone());
            }
        }
        for syn in file.synonyms {
            let key = normalize_word(&syn.term);
            let entry = lex.synonyms.entry(key).or_default();
            for c in syn.concepts {
                if !entry.contains(&c) {
                    entry.push(c);
                }
            }
        }
        for rel in file.relations {
            lex.relations
                .push((rel.from, rel.to, rel.kind, rel.weight));
        }
        lex
    }

    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("lessico non leggibile ({}): {e}", path.display()))?;
        let file: LexiconFile =
            serde_json::from_str(&raw).map_err(|e| format!("lessico non valido: {e}"))?;
        Ok(Self::from_file(file))
    }

    pub fn concept(&self, id: &str) -> Option<&ConceptDef> {
        self.concepts.get(id)
    }

    pub fn concepts(&self) -> impl Iterator<Item = &ConceptDef> {
        self.concepts.values()
    }

    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    pub fn lemma_count(&self) -> usize {
        self.lemmas.values().map(|v| v.len()).sum()
    }

    pub fn expression_count(&self) -> usize {
        self.mwe.len()
    }

    /// Cerca le interpretazioni di una forma: lemma diretto, irregolare,
    /// sinonimo o forma flessa (l'inverso della morfologia è in `morphology`).
    pub fn lookup(&self, surface: &str) -> Vec<LemmaMatch> {
        let key = normalize_word(surface);
        let mut out: Vec<LemmaMatch> = Vec::new();

        if let Some(entries) = self.lemmas.get(&key) {
            for e in entries {
                out.push(LemmaMatch {
                    lemma: e.lemma.clone(),
                    pos: e.pos.clone(),
                    gender: e.gender.clone(),
                    number: e.number.clone(),
                    concepts: e.concepts.clone(),
                    from_lemma: true,
                });
            }
        }

        if let Some(base) = self.irregular.get(&key) {
            if let Some(entries) = self.lemmas.get(base) {
                for e in entries {
                    if !out.iter().any(|m| m.lemma == e.lemma && m.pos == e.pos) {
                        out.push(LemmaMatch {
                            lemma: e.lemma.clone(),
                            pos: e.pos.clone(),
                            gender: e.gender.clone(),
                            number: e.number.clone(),
                            concepts: e.concepts.clone(),
                            from_lemma: true,
                        });
                    }
                }
            }
        }

        if out.is_empty() {
            // Cerca un lemma il cui paradigma contiene questa forma.
            for entry in self.lemmas.values().flatten() {
                if entry.lemma.eq_ignore_ascii_case(surface) {
                    continue;
                }
                if super::morphology::generates_form(&entry.lemma, &entry.pos, &key) {
                    out.push(LemmaMatch {
                        lemma: entry.lemma.clone(),
                        pos: entry.pos.clone(),
                        gender: entry.gender.clone(),
                        number: infer_number(&entry.pos, &key),
                        concepts: entry.concepts.clone(),
                        from_lemma: false,
                    });
                }
            }
        }

        if let Some(concepts) = self.synonyms.get(&key) {
            if out.is_empty() {
                out.push(LemmaMatch {
                    lemma: surface.to_lowercase(),
                    pos: "noun".to_string(),
                    gender: None,
                    number: None,
                    concepts: concepts.clone(),
                    from_lemma: true,
                });
            }
        }

        out
    }

    /// Ripiego per forme senza accenti o digitate con accenti diversi.
    pub fn lookup_folded(&self, surface: &str) -> Vec<LemmaMatch> {
        let folded = super::normalize::fold_accents(&normalize_word(surface));
        for key in self.lemmas.keys() {
            if super::normalize::fold_accents(key) == folded {
                return self.lookup(key);
            }
        }
        Vec::new()
    }

    pub fn is_function_word(&self, surface: &str) -> bool {
        self.function_words.contains(&normalize_word(surface))
    }

    /// Domini comunicativi in cui compare la parola (usati per le preferenze
    /// contestuali, es. "banco" in contesto scuola).
    pub fn domains_for(&self, surface: &str) -> Vec<String> {
        let key = normalize_word(surface);
        self.domains
            .iter()
            .filter(|(_, words)| words.contains(&key))
            .map(|(d, _)| d.clone())
            .collect()
    }

    pub fn compositions(&self) -> &HashMap<String, Vec<String>> {
        &self.compositions
    }
}

fn infer_number(pos: &str, form: &str) -> Option<String> {
    if pos != "noun" && pos != "adj" {
        return None;
    }
    let last = form.chars().last()?;
    match last {
        'i' | 'e' => Some("pl".to_string()),
        _ => Some("sg".to_string()),
    }
}

/// Tutti i concetti che hanno un simbolo dichiarato nel lessico e che quindi
/// devono esistere nel pacchetto `core`.
pub fn defined_symbol_concepts(file: &LexiconFile) -> Vec<(String, String)> {
    file.concepts
        .iter()
        .filter_map(|c| c.symbol.as_ref().map(|s| (c.id.clone(), s.clone())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_lexicon() -> Lexicon {
        let json = r#"{
            "version": 1,
            "concepts": [{"id":"a","label":"pesca","meaning":"frutto","category":"alimenti"}],
            "lemmas": [{"lemma":"pesca","pos":"noun","gender":"f","concepts":["a"]}],
            "irregular": {},
            "function_words": ["il"],
            "mwe": []
        }"#;
        let file: LexiconFile = serde_json::from_str(json).unwrap();
        Lexicon::from_file(file)
    }

    #[test]
    fn lookup_finds_lemma() {
        let lex = test_lexicon();
        let matches = lex.lookup("pesca");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].lemma, "pesca");
        assert!(matches[0].from_lemma);
    }

    #[test]
    fn lookup_finds_inflected_noun() {
        let lex = test_lexicon();
        let matches = lex.lookup("pesche");
        assert_eq!(matches.len(), 1, "il plurale deve rimandare al lemma");
        assert_eq!(matches[0].concepts, vec!["a".to_string()]);
    }

    #[test]
    fn function_words_are_recognized() {
        let lex = test_lexicon();
        assert!(lex.is_function_word("il"));
        assert!(!lex.is_function_word("pesca"));
    }
}
