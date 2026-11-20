//! Cache in memoria delle decisioni.
//!
//! La chiave include modello, revisione, versione delle ipotesi e tassonomia:
//! un aggiornamento invalida naturalmente i risultati precedenti. Non si
//! scrivono testi personali su disco.

use std::collections::{HashMap, VecDeque};

use sha2::{Digest, Sha256};

use crate::decision::hypothesis::HYPOTHESIS_VERSION;
use crate::decision::types::{DecisionRequest, DecisionResponse};

pub const DEFAULT_CAPACITY: usize = 256;

#[derive(Debug)]
pub struct DecisionCache {
    map: HashMap<String, DecisionResponse>,
    order: VecDeque<String>,
    capacity: usize,
    hits: u64,
    misses: u64,
}

impl Default for DecisionCache {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

impl DecisionCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            capacity: capacity.max(1),
            hits: 0,
            misses: 0,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<DecisionResponse> {
        let Some(mut value) = self.map.get(key).cloned() else {
            self.misses += 1;
            return None;
        };
        self.hits += 1;
        value.metadata.cached = true;
        // Aggiorna l'ordine LRU.
        if let Some(position) = self.order.iter().position(|stored| stored == key) {
            if let Some(item) = self.order.remove(position) {
                self.order.push_back(item);
            }
        }
        Some(value)
    }

    pub fn put(&mut self, key: String, mut value: DecisionResponse) {
        value.metadata.cached = false;
        if self.map.contains_key(&key) {
            if let Some(position) = self.order.iter().position(|stored| stored == &key) {
                self.order.remove(position);
            }
        }
        while self.map.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            } else {
                break;
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(key, value);
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn stats(&self) -> (u64, u64) {
        (self.hits, self.misses)
    }
}

/// Chiave canonica: testo normalizzato, target, candidati ordinati per id,
/// opzioni e versioni del modello e delle ipotesi.
pub fn cache_key(
    request: &DecisionRequest,
    model_id: &str,
    model_revision: &str,
    taxonomy_version: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(request.task.as_str().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(crate::decision::normalize::normalize(&request.input.text).as_bytes());
    hasher.update(b"\x1f");
    if let Some(target) = &request.input.target {
        hasher.update(crate::decision::normalize::normalize(&target.text).as_bytes());
        hasher.update(format!("{}:{}", target.start, target.end).as_bytes());
    }
    hasher.update(b"\x1f");
    hasher.update(request.language.as_bytes());
    hasher.update(b"\x1f");
    let multi_label = request
        .options
        .multi_label
        .unwrap_or_else(|| request.task.default_multilabel());
    hasher.update([multi_label as u8]);
    if let Some(mode) = request.options.mode {
        hasher.update(mode.as_str().as_bytes());
    }
    if let Some(value) = request.options.apply_threshold {
        hasher.update(value.to_bits().to_le_bytes());
    }
    if let Some(value) = request.options.suggest_threshold {
        hasher.update(value.to_bits().to_le_bytes());
    }
    hasher.update(b"\x1f");
    let mut candidates: Vec<&crate::decision::types::Candidate> =
        request.candidates.iter().collect();
    candidates.sort_by(|a, b| a.id.cmp(&b.id));
    for candidate in candidates {
        hasher.update(candidate.id.as_bytes());
        hasher.update(b"\x1e");
        hasher.update(candidate.label.as_bytes());
        hasher.update(b"\x1e");
        if let Some(hypothesis) = &candidate.hypothesis {
            hasher.update(hypothesis.as_bytes());
        }
        hasher.update([candidate.locked as u8, candidate.blocked as u8]);
        hasher.update(b"\x1d");
    }
    hasher.update(b"\x1f");
    hasher.update(model_id.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(model_revision.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(HYPOTHESIS_VERSION.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(taxonomy_version.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::types::{Candidate, DecisionInput, DecisionTask};
    use serde_json::json;

    fn request(text: &str) -> DecisionRequest {
        DecisionRequest {
            request_id: None,
            task: DecisionTask::PictogramDisambiguation,
            language: "it".to_string(),
            input: DecisionInput {
                text: text.to_string(),
                target: None,
            },
            candidates: vec![Candidate {
                id: "fruit".to_string(),
                label: "frutto".to_string(),
                hypothesis: None,
                locked: false,
                blocked: false,
                metadata: json!({}),
            }],
            context: Default::default(),
            options: Default::default(),
        }
    }

    #[test]
    fn key_is_stable_and_sensitive_to_text() {
        let a = cache_key(&request("Ho mangiato una pesca."), "m", "r1", "tax1");
        let b = cache_key(&request("Ho mangiato una pesca."), "m", "r1", "tax1");
        let c = cache_key(&request("Vado a pesca."), "m", "r1", "tax1");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn key_changes_with_model_and_versions() {
        let base = request("testo");
        assert_ne!(
            cache_key(&base, "m1", "r1", "tax1"),
            cache_key(&base, "m2", "r1", "tax1")
        );
        assert_ne!(
            cache_key(&base, "m1", "r1", "tax1"),
            cache_key(&base, "m1", "r2", "tax1")
        );
        assert_ne!(
            cache_key(&base, "m1", "r1", "tax1"),
            cache_key(&base, "m1", "r1", "tax2")
        );
    }

    #[test]
    fn lru_evicts_oldest() {
        let mut cache = DecisionCache::new(2);
        let response = crate::decision::types::DecisionResponse::failure(
            "r",
            crate::decision::types::RequestStatus::Failed,
            "x",
            Default::default(),
        );
        cache.put("a".to_string(), response.clone());
        cache.put("b".to_string(), response.clone());
        assert!(cache.get("a").is_some());
        cache.put("c".to_string(), response);
        assert!(cache.get("b").is_none());
        assert!(cache.get("a").is_some());
        assert!(cache.get("c").is_some());
    }
}
