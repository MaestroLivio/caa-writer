//! Ricerca e selezione dei pittogrammi.
//!
//! Livello 2: ricerca lessicale rapida nel database.
//! Livello 4: punteggio dei candidati (contesto, preferenze, specificità).
//! Livello 7: sostituzione intelligente quando il simbolo esatto manca.
//!
//! Regola non negoziabile: se nessuna rappresentazione è adeguata, il token
//! resta senza simbolo. Non si assegnano mai simboli fuorvianti.

use std::collections::HashSet;

use crate::database::{Database, DbResult, SymbolFilters, SymbolRecord};
use crate::linguistics::lexicon::{ConceptDef, Lexicon};
use crate::linguistics::normalize::normalize_word;
use crate::linguistics::tokenizer::tokenize;
use crate::model::{Strategy, SymbolPart};

/// Insieme delle parole normalizzate di una frase: contesto per le cue.
pub fn sentence_words(sentence: &str) -> HashSet<String> {
    tokenize(sentence)
        .into_iter()
        .filter(|t| t.is_word())
        .map(|t| normalize_word(&t.text))
        .collect()
}

pub fn sentence_normalized(sentence: &str) -> String {
    sentence.to_lowercase()
}

/// Punteggio contestuale deterministico.
///
/// - ogni indizio di contesto a favore vale +0.35 (massimo +1.4);
/// - ogni indizio contrario vale -1.3 (penalità forte: gli omonimi sono
///   il rischio principale di un pittogramma fuorviante);
/// - `prior` deriva dall'ordine nel lessico (primo significato = più comune).
pub fn score_concept(
    concept: &ConceptDef,
    words: &HashSet<String>,
    sentence_norm: &str,
    prior: f32,
    frequency_bonus: f32,
) -> f32 {
    let mut score = prior + frequency_bonus;
    let mut prefer_hits = 0.0f32;
    for cue in &concept.cues.prefer {
        let cue_norm = normalize_word(cue);
        if words.contains(&cue_norm) || (cue_norm.contains(' ') && sentence_norm.contains(&cue_norm)) {
            prefer_hits += 1.0;
        }
    }
    score += (prefer_hits * 0.35).min(1.4);

    let mut avoid_hits = 0.0f32;
    for cue in &concept.cues.avoid {
        let cue_norm = normalize_word(cue);
        if words.contains(&cue_norm) || (cue_norm.contains(' ') && sentence_norm.contains(&cue_norm)) {
            avoid_hits += 1.0;
        }
    }
    score -= avoid_hits * 1.3;

    // Il significato più specifico (con cues dichiarate) vince i pareggi:
    // evita che un concetto generico scavalchi un omonimo ben documentato.
    score + (concept.cues.prefer.len() as f32 * 0.001)
}

#[derive(Debug, Clone)]
pub struct ScoredConcept {
    pub concept_id: String,
    pub score: f32,
}

/// Ordina i concetti candidati per il contesto dato.
pub fn rank_concepts(
    concept_ids: &[String],
    lexicon: &Lexicon,
    sentence: &str,
    frequency: &dyn Fn(&str) -> f32,
) -> Vec<ScoredConcept> {
    let words = sentence_words(sentence);
    let norm = sentence_normalized(sentence);
    let mut scored: Vec<ScoredConcept> = concept_ids
        .iter()
        .enumerate()
        .filter_map(|(idx, id)| {
            let concept = lexicon.concept(id)?;
            let prior = 1.0 - (idx as f32 * 0.05);
            let score = score_concept(concept, &words, &norm, prior, frequency(id));
            Some(ScoredConcept {
                concept_id: id.clone(),
                score,
            })
        })
        .collect();
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.concept_id.cmp(&b.concept_id))
    });
    scored
}

#[derive(Debug, Clone)]
pub struct Resolved {
    pub concept_id: Option<String>,
    pub symbol: Option<SymbolRecord>,
    pub strategy: Strategy,
    pub parts: Vec<SymbolPart>,
    pub note: Option<String>,
}

impl Resolved {
    fn empty() -> Self {
        Self {
            concept_id: None,
            symbol: None,
            strategy: Strategy::None,
            parts: Vec::new(),
            note: None,
        }
    }
}

fn first_symbol(
    db: &Database,
    filters: &SymbolFilters,
    concept_id: &str,
) -> DbResult<Option<SymbolRecord>> {
    Ok(db.symbols_for_concept(concept_id, filters)?.into_iter().next())
}

/// Gerarchia delle sostituzioni intelligenti (Strategie A-F del capitolato).
pub fn resolve(
    db: &Database,
    lexicon: &Lexicon,
    filters: &SymbolFilters,
    concept_id: &str,
) -> DbResult<Resolved> {
    if concept_id == "concept.unknown" {
        return Ok(Resolved {
            concept_id: Some(concept_id.to_string()),
            symbol: first_symbol(db, filters, concept_id)?,
            strategy: Strategy::None,
            parts: Vec::new(),
            note: Some("Nessun pittogramma specifico disponibile.".to_string()),
        });
    }

    // Corrispondenza esatta.
    if let Some(symbol) = first_symbol(db, filters, concept_id)? {
        return Ok(Resolved {
            concept_id: Some(concept_id.to_string()),
            symbol: Some(symbol),
            strategy: Strategy::Exact,
            parts: Vec::new(),
            note: None,
        });
    }

    // Strategia A — sinonimo equivalente.
    for equivalent in db.equivalent_concepts(concept_id, 4)? {
        if let Some(symbol) = first_symbol(db, filters, &equivalent)? {
            return Ok(Resolved {
                concept_id: Some(equivalent.clone()),
                symbol: Some(symbol),
                strategy: Strategy::Synonym,
                parts: Vec::new(),
                note: Some(format!("Usato il sinonimo equivalente: {equivalent}")),
            });
        }
    }

    // Strategia C — concetto più generale (massimo due livelli).
    let mut current = concept_id.to_string();
    for hop in 1..=2 {
        let Some(broader) = db.broader_of(&current)? else {
            break;
        };
        if let Some(symbol) = first_symbol(db, filters, &broader)? {
            let label = db
                .concept(&broader)?
                .map(|c| c.label)
                .unwrap_or_else(|| broader.clone());
            return Ok(Resolved {
                concept_id: Some(broader),
                symbol: Some(symbol),
                strategy: Strategy::Broader,
                parts: Vec::new(),
                note: Some(format!(
                    "Nessun pittogramma esatto: mostrato il concetto più generale «{label}» ({hop} livello)."
                )),
            });
        }
        current = broader;
    }

    // Strategia D — composizione di pittogrammi.
    if let Some(parts_concepts) = lexicon.compositions().get(concept_id) {
        let mut parts = Vec::new();
        let mut complete = true;
        for part_concept in parts_concepts {
            match first_symbol(db, filters, part_concept)? {
                Some(symbol) => parts.push(SymbolPart {
                    concept_id: part_concept.clone(),
                    label: db
                        .concept(part_concept)?
                        .map(|c| c.label)
                        .unwrap_or_else(|| part_concept.clone()),
                    symbol_id: Some(symbol.id),
                    svg: symbol.svg,
                    image: symbol.image,
                }),
                None => {
                    complete = false;
                    break;
                }
            }
        }
        if complete && parts.len() >= 2 {
            return Ok(Resolved {
                concept_id: Some(concept_id.to_string()),
                symbol: None,
                strategy: Strategy::Composition,
                parts,
                note: Some("Concetto rappresentato con due pittogrammi.".to_string()),
            });
        }
    }

    // Strategia F — concetto semanticamente vicino (dichiarato come tale).
    for related in db.related_concepts(concept_id, 3)? {
        if let Some(symbol) = first_symbol(db, filters, &related)? {
            let label = db
                .concept(&related)?
                .map(|c| c.label)
                .unwrap_or_else(|| related.clone());
            return Ok(Resolved {
                concept_id: Some(related),
                symbol: Some(symbol),
                strategy: Strategy::Related,
                parts: Vec::new(),
                note: Some(format!("Simbolo vicino, non equivalente: «{label}»")),
            });
        }
    }

    // Strategia G — nessuna rappresentazione adeguata.
    Ok(Resolved {
        concept_id: Some(concept_id.to_string()),
        ..Resolved::empty()
    })
}

/// Alternative suggerite per un testo: usate dal pannello quando non c'è
/// una selezione, e dal Visual Resource Studio.
pub fn suggest_alternatives(
    result: &crate::model::ParagraphResult,
    limit: usize,
) -> Vec<crate::model::Alternative> {
    use crate::model::Alternative;
    let mut out: Vec<Alternative> = Vec::new();
    for token in &result.tokens {
        for alternative in &token.alternatives {
            if out.len() >= limit {
                return out;
            }
            if out.iter().any(|a| a.symbol_id == alternative.symbol_id) {
                continue;
            }
            out.push(alternative.clone());
        }
    }
    out
}

/// Cerca un simbolo personalizzato il cui testo somigli alla parola scritta.
pub fn lookup_custom(
    db: &Database,
    filters: &SymbolFilters,
    surface: &str,
    limit: u32,
) -> DbResult<Vec<SymbolRecord>> {
    let page = db.search_symbols(surface, limit, 0, filters)?;
    Ok(page
        .items
        .iter()
        .filter_map(|alt| db.symbol_by_id(&alt.symbol_id).ok().flatten())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linguistics::lexicon::LexiconFile;

    fn lexicon() -> Lexicon {
        let json = r#"{
          "version": 1,
          "concepts": [
            {"id":"fruit.peach","label":"pesca","meaning":"frutto","category":"alimenti",
             "cues":{"prefer":["mangiare","frutto"],"avoid":["andare","fiume"]}},
            {"id":"activity.fishing","label":"pesca","meaning":"attivita","category":"attivita",
             "cues":{"prefer":["andare","fiume"],"avoid":["mangiare","frutto"]}}
          ],
          "lemmas": []
        }"#;
        let file: LexiconFile = serde_json::from_str(json).unwrap();
        Lexicon::from_file(file)
    }

    #[test]
    fn context_disambiguates_peach() {
        let lex = lexicon();
        let ranked = rank_concepts(
            &["fruit.peach".into(), "activity.fishing".into()],
            &lex,
            "Luca mangia una pesca.",
            &|_| 0.0,
        );
        assert_eq!(ranked[0].concept_id, "fruit.peach");
    }

    #[test]
    fn context_disambiguates_fishing() {
        let lex = lexicon();
        let ranked = rank_concepts(
            &["fruit.peach".into(), "activity.fishing".into()],
            &lex,
            "Il nonno va a pesca al fiume.",
            &|_| 0.0,
        );
        assert_eq!(ranked[0].concept_id, "activity.fishing");
    }

    #[test]
    fn avoid_cues_penalize_strongly() {
        let lex = lexicon();
        let concept = lex.concept("fruit.peach").unwrap();
        let words = sentence_words("Il nonno va a pesca al fiume");
        let score = score_concept(concept, &words, "il nonno va a pesca al fiume", 1.0, 0.0);
        assert!(score < 0.0, "score={score}");
    }
}
