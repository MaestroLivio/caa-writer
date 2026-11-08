//! Gestione degli asset (pittogrammi, immagini importate, miniature).
//!
//! I SVG originali restano vettoriali e vengono letti dal database con cache
//! LRU; i raster hanno un percorso su disco e non vengono caricati
//! automaticamente in memoria.

pub mod svg;

use std::collections::HashMap;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::model::Alternative;

/// Cache LRU semplice, senza dipendenze: evita di rileggere lo stesso SVG
/// decine di volte mentre si ridisegna il documento.
pub struct SvgCache {
    capacity: usize,
    order: Vec<String>,
    items: HashMap<String, Option<String>>,
}

impl SvgCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            order: Vec::new(),
            items: HashMap::new(),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<Option<String>> {
        if let Some(value) = self.items.get(key) {
            let value = value.clone();
            self.touch(key);
            return Some(value);
        }
        None
    }

    pub fn insert(&mut self, key: String, value: Option<String>) {
        if self.items.len() >= self.capacity {
            if let Some(oldest) = self.order.first().cloned() {
                self.order.remove(0);
                self.items.remove(&oldest);
            }
        }
        self.touch(&key);
        self.items.insert(key, value);
    }

    fn touch(&mut self, key: &str) {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
        }
        self.order.push(key.to_string());
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.order.clear();
    }
}

/// Informazioni su un'immagine raster importata.
pub struct ImportedImage {
    pub path: String,
    pub format: String,
    pub bytes: usize,
}

/// Copia un'immagine raster nella cartella asset dell'utente.
/// Non converte e non re-encode: il formato originale viene conservato.
pub fn store_raster(source: &Path, assets_dir: &Path) -> Result<ImportedImage, String> {
    let bytes = std::fs::read(source).map_err(|e| format!("immagine non leggibile: {e}"))?;
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();
    if !matches!(ext.as_str(), "png" | "webp" | "jpg" | "jpeg" | "gif" | "svg") {
        return Err(format!("formato immagine non supportato: {ext}"));
    }
    let hash = Sha256::digest(&bytes);
    let name = format!("{}.{}", hex(&hash), ext);
    let target = assets_dir.join(&name);
    if !target.exists() {
        std::fs::create_dir_all(assets_dir).map_err(|e| e.to_string())?;
        std::fs::write(&target, &bytes).map_err(|e| e.to_string())?;
    }
    Ok(ImportedImage {
        path: target.to_string_lossy().to_string(),
        format: if ext == "jpeg" { "jpg".to_string() } else { ext },
        bytes: bytes.len(),
    })
}

pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Converte una riga di database in un'alternativa per l'interfaccia.
pub fn alternative_from_record(record: &crate::database::SymbolRecord) -> Alternative {
    Alternative {
        symbol_id: record.id.clone(),
        concept_id: record.concept_id.clone(),
        label: record.label.clone(),
        meaning: record.meaning.clone().unwrap_or_default(),
        pack_id: record.pack_id.clone(),
        pack_name: record.pack_name.clone(),
        format: record.format.clone(),
        svg: record.svg.clone(),
        image: record.image.clone(),
        approximate: false,
        neural: false,
        license: record.license.clone(),
        source_url: record.source_url.clone(),
        author: record.author.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lru_evicts_oldest() {
        let mut cache = SvgCache::new(2);
        cache.insert("a".into(), Some("<svg/>".into()));
        cache.insert("b".into(), Some("<svg/>".into()));
        assert!(cache.get("a").is_some());
        cache.insert("c".into(), Some("<svg/>".into()));
        assert!(cache.get("b").is_none(), "b doveva essere evacuato");
        assert!(cache.get("a").is_some());
    }
}
