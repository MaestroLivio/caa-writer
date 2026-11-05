//! Utility comuni.

use std::time::{SystemTime, UNIX_EPOCH};

/// Timestamp ISO-8601 UTC (Rfc3339) usato nelle tabelle.
pub fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    match time::OffsetDateTime::from_unix_timestamp(secs as i64) {
        Ok(dt) => dt
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| format!("{secs}")),
        Err(_) => format!("{secs}"),
    }
}

/// Conteggio caratteri (code point), coerente con il tokenizzatore.
pub fn char_len(text: &str) -> usize {
    text.chars().count()
}

/// Taglia una stringa a `max` caratteri aggiungendo ellissi.
pub fn truncate_chars(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_string();
    }
    let mut out: String = chars.into_iter().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
