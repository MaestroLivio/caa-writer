//! Esportazione documenti: `.caadoc`, DOCX, HTML.

pub mod atomic;
pub mod caadoc;
pub mod docx;
pub mod html;
pub mod universal_pdf;

pub use atomic::write_atomic;

/// Escape per contenuti XML/HTML.
pub fn xml_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

/// 1 pollice = 914400 EMU (unità di DrawingML).
pub fn emu(inches: f32) -> i64 {
    (inches * 914_400.0) as i64
}
