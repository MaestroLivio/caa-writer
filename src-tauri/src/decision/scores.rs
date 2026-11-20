//! Elaborazione dei punteggi e ordinamento dei candidati.

use crate::decision::types::RankedCandidate;

/// Softmax numericamente stabile.
pub fn softmax(values: &[f32]) -> Vec<f32> {
    if values.is_empty() {
        return Vec::new();
    }
    let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exp: Vec<f32> = values.iter().map(|value| (value - max).exp()).collect();
    let sum: f32 = exp.iter().sum();
    if sum <= 0.0 || !sum.is_finite() {
        return vec![1.0 / values.len() as f32; values.len()];
    }
    exp.into_iter().map(|value| value / sum).collect()
}

#[derive(Debug, Clone)]
pub struct ScoredCandidate {
    pub id: String,
    pub label: String,
    pub score: f32,
    pub kind: &'static str,
    pub locked: bool,
}

/// Ordina i candidati già valutati dal modello o dalle regole.
///
/// - `multilabel = false`: normalizza i punteggi con una softmax fra i
///   candidati (semantica della pipeline zero-shot a etichetta singola).
/// - `multilabel = true`: i punteggi restano indipendenti, come richiesto
///   dalla classificazione a più etichette.
pub fn rank(mut scored: Vec<ScoredCandidate>, multilabel: bool) -> Vec<RankedCandidate> {
    if scored.is_empty() {
        return Vec::new();
    }
    if !multilabel {
        let values: Vec<f32> = scored.iter().map(|item| item.score).collect();
        let normalized = softmax(&values);
        for (item, value) in scored.iter_mut().zip(normalized.into_iter()) {
            item.score = value;
        }
    }
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    scored
        .into_iter()
        .enumerate()
        .map(|(index, item)| RankedCandidate {
            id: item.id,
            label: item.label,
            score: round4(item.score),
            rank: index + 1,
            score_kind: item.kind.to_string(),
            locked: item.locked,
        })
        .collect()
}

fn round4(value: f32) -> f32 {
    (value * 10_000.0).round() / 10_000.0
}

/// Distacco fra il primo e il secondo candidato: usato dalla politica di
/// confidenza per distinguere un esito netto da un'ambiguità residua.
pub fn margin(ranked: &[RankedCandidate]) -> f32 {
    match (ranked.first(), ranked.get(1)) {
        (Some(first), Some(second)) => first.score - second.score,
        (Some(first), None) => first.score,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, score: f32) -> ScoredCandidate {
        ScoredCandidate {
            id: id.to_string(),
            label: id.to_string(),
            score,
            kind: "entailment",
            locked: false,
        }
    }

    #[test]
    fn softmax_sums_to_one() {
        let values = softmax(&[2.0, 1.0, 0.1]);
        let sum: f32 = values.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
        assert!(values[0] > values[1] && values[1] > values[2]);
    }

    #[test]
    fn single_label_normalizes_across_candidates() {
        let ranked = rank(vec![item("a", 0.7), item("b", 0.2)], false);
        assert_eq!(ranked[0].id, "a");
        assert!((ranked[0].score + ranked[1].score - 1.0).abs() < 1e-4);
    }

    #[test]
    fn multilabel_keeps_independent_scores() {
        let ranked = rank(vec![item("a", 0.7), item("b", 0.65)], true);
        assert_eq!(ranked[0].score, 0.7);
        assert_eq!(ranked[1].score, 0.65);
    }

    #[test]
    fn margin_reports_gap() {
        let ranked = rank(vec![item("a", 3.0), item("b", 0.0)], false);
        assert!((margin(&ranked) - (0.953 - 0.047)).abs() < 0.05);
    }

    #[test]
    fn empty_input_is_safe() {
        assert!(softmax(&[]).is_empty());
        assert!(rank(Vec::new(), false).is_empty());
    }
}
