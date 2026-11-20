//! Registro diagnostico delle decisioni.
//!
//! Conserva solo metadati tecnici (task, stato, latenza, metodo). Non salva il
//! testo dell'utente, i nomi dei pittogrammi né identificativi personali.

use std::collections::VecDeque;
use std::sync::Mutex;

const DEFAULT_CAPACITY: usize = 200;

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub request_id: String,
    pub task: String,
    pub status: String,
    pub method: String,
    pub latency_ms: u64,
    pub candidates: usize,
    pub cached: bool,
}

#[derive(Debug)]
pub struct DecisionAudit {
    entries: Mutex<VecDeque<AuditEntry>>,
    capacity: usize,
}

impl Default for DecisionAudit {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

impl DecisionAudit {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(VecDeque::new()),
            capacity: capacity.max(1),
        }
    }

    pub fn record(&self, entry: AuditEntry) {
        if let Ok(mut entries) = self.entries.lock() {
            while entries.len() >= self.capacity {
                entries.pop_front();
            }
            entries.push_back(entry);
        }
    }

    pub fn snapshot(&self) -> Vec<AuditEntry> {
        self.entries
            .lock()
            .map(|entries| entries.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }

    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .map(|entries| entries.len())
            .unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str) -> AuditEntry {
        AuditEntry {
            request_id: id.to_string(),
            task: "pictogram_disambiguation".to_string(),
            status: "completed".to_string(),
            method: "neural".to_string(),
            latency_ms: 12,
            candidates: 2,
            cached: false,
        }
    }

    #[test]
    fn keeps_only_most_recent_entries() {
        let audit = DecisionAudit::new(2);
        audit.record(entry("a"));
        audit.record(entry("b"));
        audit.record(entry("c"));
        let snapshot = audit.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].request_id, "b");
        assert_eq!(snapshot[1].request_id, "c");
    }
}
