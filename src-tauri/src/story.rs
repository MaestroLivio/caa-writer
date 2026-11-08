//! Costruzione di storie multipagina con scene vettoriali modificabili.
//!
//! Le scene sono composte da elementi `symbol` indipendenti, non da
//! un'immagine unica: testo, pittogrammi, personaggi e pulsanti restano
//! modificabili nell'editor universale. Questo è il backend offline che resta
//! disponibile anche senza modelli diffusion.

use serde_json::{json, Value};

use crate::database::SymbolFilters;
use crate::engine::Engine;
use crate::universal::UniversalDocument;
use crate::universal_build::{self, BuildOutcome};

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|item| item.as_str()).map(str::to_string)
}

fn number(value: &Value, key: &str) -> Option<usize> {
    value.get(key).and_then(|item| item.as_u64()).map(|v| v as usize)
}

fn words(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|item| item.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn sentence_templates(kind: &str, title: &str, setting: &str, subject: &str) -> Vec<String> {
    match kind {
        "social" => vec![
            format!("Questa storia parla di {subject}."),
            format!("Mi trovo in {setting}."),
            "Ci sono persone che mi conoscono e possono aiutarmi.".to_string(),
            "A volte può succedere qualcosa di inatteso.".to_string(),
            "Posso sentirmi sorpreso, preoccupato o stanco.".to_string(),
            "Posso chiedere aiuto a un adulto che conosco.".to_string(),
            "Posso spiegare che cosa mi serve.".to_string(),
            "Posso provare una strategia e poi controllare come sto.".to_string(),
            "Dopo un momento la situazione può cambiare.".to_string(),
            format!("Questa storia mi aiuta a capire «{title}»."),
        ],
        "routine" => vec![
            format!("Oggi seguo la routine: {title}."),
            "Guardo la sequenza e comincio dal primo passo.".to_string(),
            "Faccio un passo alla volta.".to_string(),
            "Se non so che cosa fare, chiedo aiuto.".to_string(),
            "Controllo di aver finito il passo.".to_string(),
            "Passo al passo successivo.".to_string(),
            "Posso fare una piccola pausa se ne ho bisogno.".to_string(),
            "Quando ho finito, metto a posto il materiale.".to_string(),
            "La routine è finita.".to_string(),
            "La prossima volta posso rifarla.".to_string(),
        ],
        "emotion" => vec![
            format!("Questa storia parla di {subject}."),
            "Le emozioni possono cambiare durante la giornata.".to_string(),
            "Posso sentire il mio corpo: respiro, voce, muscoli.".to_string(),
            "Posso dare un nome a quello che sento.".to_string(),
            "Non devo nascondere quello che sento.".to_string(),
            "Posso chiedere una pausa, un abbraccio o aiuto.".to_string(),
            "Posso respirare e aspettare un momento.".to_string(),
            "Dopo un po' l'emozione può diventare più piccola.".to_string(),
            "Posso parlare con una persona di fiducia.".to_string(),
            "Questa storia mi aiuta a capire le emozioni.".to_string(),
        ],
        "educational" => vec![
            format!("Oggi impariamo qualcosa su {subject}."),
            "Osservo gli elementi della scena.".to_string(),
            "Noto che cosa è uguale e che cosa è diverso.".to_string(),
            "Faccio una domanda.".to_string(),
            "Cerco una risposta insieme a un adulto.".to_string(),
            "Provo a spiegare con parole mie.".to_string(),
            "Controllo se la mia spiegazione funziona.".to_string(),
            "Collego quello che ho imparato alla vita quotidiana.".to_string(),
            "Posso fare un esempio.".to_string(),
            "Ora so qualcosa di nuovo.".to_string(),
        ],
        _ => vec![
            format!("Questa è la storia «{title}»."),
            format!("La storia si svolge in {setting}."),
            "Ci sono personaggi e oggetti importanti.".to_string(),
            "Succede qualcosa di interessante.".to_string(),
            "I personaggi guardano, ascoltano e parlano.".to_string(),
            "A volte arriva un problema.".to_string(),
            "I personaggi provano una soluzione.".to_string(),
            "La soluzione funziona oppure si prova ancora.".to_string(),
            "Alla fine la situazione si risolve.".to_string(),
            "La storia finisce, ma possiamo raccontarla di nuovo.".to_string(),
        ],
    }
}

fn caa_words(sentence: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "il", "lo", "la", "i", "gli", "le", "un", "uno", "una", "di", "a", "da", "in", "con",
        "su", "per", "tra", "fra", "e", "o", "ma", "che", "non", "si", "mi", "ti", "ci", "è",
        "sono", "ha", "ho", "questa", "questo", "questi", "queste", "come", "quando", "perché",
        "perche", "mentre", "dopo", "prima", "anche", "ancora", "sempre", "mai", "molto", "poco",
        "può", "puo", "posso", "siamo", "siete", "essere", "stato", "stata", "trovo", "storia",
        "parla", "parlare", "della", "delle", "dello", "degli", "dalla", "dalle", "nelle", "nella",
        "nel", "nei", "negli", "sul", "sui", "sugli", "alla", "alle", "allo", "agli",
    ];
    sentence
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|ch| ch.is_alphanumeric() || *ch == '\'')
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|word| word.chars().count() >= 3 && !STOP.contains(&word.as_str()))
        .take(5)
        .collect()
}

fn symbol_element(id: &str, label: &str, x: f32, y: f32, w: f32, h: f32, font: f32, opacity: f32, z: i32) -> Value {
    json!({
        "id": id,
        "kind": "symbol",
        "text": "",
        "symbol": label,
        "x_mm": x,
        "y_mm": y,
        "w_mm": w,
        "h_mm": h,
        "z": z,
        "opacity": opacity,
        "style": {"font_family": "Arial", "font_size": font, "align": "center", "valign": "bottom", "padding_mm": 1.0}
    })
}

fn text_element(id: &str, kind: &str, value: &str, x: f32, y: f32, w: f32, h: f32, font: f32, bold: bool, background: &str, z: i32) -> Value {
    json!({
        "id": id,
        "kind": kind,
        "text": value,
        "x_mm": x,
        "y_mm": y,
        "w_mm": w,
        "h_mm": h,
        "z": z,
        "style": {
            "font_family": "Arial",
            "font_size": font,
            "bold": bold,
            "align": "left",
            "valign": "top",
            "padding_mm": 2.0,
            "background": background,
            "border_color": if background == "#ffffff" { "" } else { "#94a3b8" },
            "border_width": if background == "#ffffff" { 0.0 } else { 0.4 },
            "border_radius": 2.5
        }
    })
}

fn nav_element(id: &str, label: &str, x: f32, action: &str, target: Option<&str>) -> Value {
    let mut action_value = json!({"type": action});
    if let Some(target) = target {
        action_value["target"] = json!(target);
    }
    json!({
        "id": id,
        "kind": "button",
        "text": label,
        "x_mm": x,
        "y_mm": 280.0,
        "w_mm": 34.0,
        "h_mm": 8.0,
        "z": 80,
        "action": action_value,
        "spoken_text": label,
        "style": {"font_family": "Arial", "font_size": 3.4, "align": "center", "valign": "middle", "background": "#ecfdf5", "border_color": "#0f766e", "border_width": 0.4, "border_radius": 2.0, "bold": true}
    })
}

/// Genera il documento completo. `input` è la specifica JSON dello Studio.
pub fn build_story(engine: &Engine, input: &Value) -> Result<BuildOutcome, String> {
    let title = text(input, "title").unwrap_or_else(|| "La mia storia".to_string());
    let id = text(input, "id")
        .map(|value| crate::pictogram_import::slugify(&value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("story-{}", crate::pictogram_import::slugify(&title)));
    let kind = text(input, "kind").unwrap_or_else(|| "narrative".to_string());
    let layout = text(input, "layout").unwrap_or_else(|| "image_top".to_string());
    let setting = text(input, "setting").unwrap_or_else(|| "scuola".to_string());
    let subject = text(input, "subject").unwrap_or_else(|| title.clone());
    let characters = {
        let list = words(input, "characters");
        if list.is_empty() {
            vec!["bambino".to_string()]
        } else {
            list
        }
    };
    let pages = number(input, "pages").unwrap_or(10).clamp(10, 20);
    let provided = words(input, "sentences");
    let templates = sentence_templates(&kind, &title, &setting, &subject);
    let sentences: Vec<String> = (0..pages)
        .map(|index| {
            provided
                .get(index)
                .cloned()
                .unwrap_or_else(|| templates[index % templates.len()].clone())
        })
        .collect();

    let mut page_specs = Vec::new();
    // Copertina
    let mut cover = vec![
        text_element("cover-badge", "badge", "STORIA", 10.0, 8.0, 26.0, 7.0, 3.0, true, "#0f766e", 0),
        text_element("cover-title", "title", &title, 10.0, 22.0, 190.0, 26.0, 10.0, true, "#ffffff", 1),
    ];
    cover.push(symbol_element("cover-setting", &setting, 10.0, 54.0, 190.0, 90.0, 4.0, 0.25, 2));
    for (index, character) in characters.iter().take(3).enumerate() {
        cover.push(symbol_element(
            &format!("cover-character-{index}"),
            character,
            22.0 + index as f32 * 62.0,
            64.0,
            54.0,
            72.0,
            3.6,
            1.0,
            3 + index as i32,
        ));
    }
    cover.push(text_element("cover-description", "text_box", &subject, 10.0, 152.0, 190.0, 38.0, 4.6, false, "#ecfdf5", 10));
    cover.push(text_element("cover-note", "note", "Storia modificabile. Le scene sono composte da pittogrammi indipendenti: puoi sostituirli, spostarli o aggiungere immagini.", 10.0, 200.0, 190.0, 28.0, 3.3, false, "#ffffff", 11));
    page_specs.push(json!({"id": "home", "title": "Copertina", "layout": "absolute", "home": true, "elements": cover}));

    for (index, sentence) in sentences.iter().enumerate() {
        let page_number = index + 1;
        let page_id = format!("p{page_number}");
        let next = if page_number < sentences.len() { format!("p{}", page_number + 1) } else { "fine".to_string() };
        let prev = if page_number == 1 { "home".to_string() } else { format!("p{}", page_number - 1) };
        let mut elements = vec![
            text_element(&format!("{page_id}-badge"), "badge", &format!("{}/{}", page_number, sentences.len()), 10.0, 8.0, 24.0, 7.0, 3.0, true, "#0f766e", 0),
            text_element(&format!("{page_id}-title"), "subtitle", &format!("Pagina {page_number}"), 36.0, 8.0, 164.0, 7.0, 3.8, false, "#ffffff", 1),
        ];
        let (strip_y, text_y, scene_y, scene_x, scene_w) = match layout.as_str() {
            "image_left" => (92.0, 30.0, 30.0, 10.0, 88.0),
            "image_right" => (92.0, 30.0, 30.0, 112.0, 88.0),
            "pictograms_top" => (24.0, 78.0, 124.0, 10.0, 190.0),
            _ => (132.0, 178.0, 24.0, 10.0, 190.0),
        };
        if layout == "image_left" || layout == "image_right" {
            let text_x = if layout == "image_left" { 104.0 } else { 10.0 };
            elements.push(text_element(&format!("{page_id}-sentence"), "text_box", sentence, text_x, text_y, 96.0, 60.0, 4.6, false, "#f8fafc", 10));
        } else {
            elements.push(text_element(&format!("{page_id}-sentence"), "text_box", sentence, 10.0, text_y, 190.0, 38.0, 4.8, false, "#f8fafc", 10));
        }
        // Scena vettoriale: sfondo attenuato e personaggi indipendenti.
        elements.push(symbol_element(&format!("{page_id}-setting"), &setting, scene_x, scene_y, scene_w, if layout == "image_left" || layout == "image_right" { 56.0 } else { 100.0 }, 3.0, 0.22, 2));
        let count = characters.len().min(3).max(1);
        for (character_index, character) in characters.iter().take(count).enumerate() {
            let step = scene_w / (count as f32 + 1.0);
            let x = scene_x + step * (character_index as f32 + 1.0) - 24.0;
            let w = (step * 0.9).clamp(34.0, 58.0);
            elements.push(symbol_element(
                &format!("{page_id}-character-{character_index}"),
                character,
                x,
                scene_y + 8.0,
                w,
                if layout == "image_left" || layout == "image_right" { 50.0 } else { 82.0 },
                3.4,
                1.0,
                3 + character_index as i32,
            ));
        }
        // Striscia CAA di parole chiave: si tengono solo le parole che hanno
        // davvero un simbolo nel repertorio, evitando caselle vuote o
        // associazioni ambigue.
        let db = engine.db()?;
        let filters = SymbolFilters::default();
        let mut keywords: Vec<String> = caa_words(sentence)
            .into_iter()
            .filter(|word| {
                db.symbol_by_label(word, &filters)
                    .ok()
                    .flatten()
                    .is_some()
            })
            .take(5)
            .collect();
        if keywords.is_empty() {
            keywords = characters
                .iter()
                .cloned()
                .chain(std::iter::once(setting.clone()))
                .take(4)
                .collect();
        }
        let cells: Vec<Value> = keywords
            .into_iter()
            .map(|word| json!({"text": word, "symbol": word, "spoken_text": word, "action": {"type": "speak"}}))
            .collect();
        if !cells.is_empty() {
            elements.push(json!({
                "id": format!("{page_id}-strip"),
                "kind": "grid",
                "text": "",
                "x_mm": 10.0,
                "y_mm": strip_y,
                "w_mm": 190.0,
                "h_mm": 34.0,
                "z": 20,
                "columns": cells.len(),
                "rows": 1,
                "gap_mm": 2.0,
                "cells": cells,
                "style": {"font_family": "Arial", "font_size": 3.2, "border_color": "#94a3b8", "border_width": 0.3, "border_radius": 2.0, "padding_mm": 1.0}
            }));
        }
        elements.push(nav_element(&format!("{page_id}-prev"), "‹ Indietro", 10.0, "navigate", Some(&prev)));
        elements.push(nav_element(&format!("{page_id}-next"), "Avanti ›", 50.0, "navigate", Some(&next)));
        elements.push(nav_element(&format!("{page_id}-home"), "⌂ Home", 90.0, "home", None));
        page_specs.push(json!({"id": page_id, "title": format!("Pagina {page_number}"), "layout": "absolute", "elements": elements}));
    }
    // Pagina finale
    page_specs.push(json!({
        "id": "fine",
        "title": "Fine",
        "layout": "absolute",
        "elements": [
            text_element("end-title", "title", "Fine", 10.0, 30.0, 190.0, 26.0, 12.0, true, "#ffffff", 0),
            text_element("end-text", "text_box", "La storia è finita. Puoi raccontare che cosa è successo, come si sentono i personaggi e che cosa hai imparato.", 10.0, 68.0, 190.0, 44.0, 4.6, false, "#ecfdf5", 1),
            nav_element("end-prev", "‹ Indietro", 10.0, "navigate", Some("p1")),
            nav_element("end-home", "⌂ Home", 50.0, "home", None)
        ]
    }));

    let spec = json!({
        "id": id,
        "title": title,
        "description": format!("Storia illustrata in CAA · {} pagine · scene vettoriali modificabili", sentences.len()),
        "category": "Storie sociali",
        "source_type": "template",
        "kind": "story",
        "interactive": true,
        "license": "CC BY-NC-SA 4.0",
        "page_size": "A4",
        "orientation": "portrait",
        "pages": page_specs,
        "taxonomy": {
            "category_id": if kind == "social" { "storie-sociali" } else { "storie-caa" },
            "subcategory_id": if kind == "social" { "sociali" } else { "sopra-testo" },
            "audience": "primaria",
            "objective": format!("Comprendere e raccontare: {subject}"),
            "tags": ["storia", "caa", "scene", "vettoriale"],
            "complexity": "intermedio",
            "needs_symbols": true,
            "printable": true,
            "customizable": true
        }
    });
    universal_build::build_from_spec(engine, &spec)
}

pub fn save_story(
    path: &std::path::Path,
    document: &UniversalDocument,
    assets: &[crate::model::DocumentAsset],
) -> Result<crate::model::ExportResult, String> {
    universal_build::save_universal(path, document, assets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caa_words_drop_function_words_and_limit() {
        assert_eq!(caa_words("Il cane corre nel parco con il bambino"), vec!["cane", "corre", "parco", "bambino"]);
        assert_eq!(caa_words("e il la"), Vec::<String>::new());
    }

    #[test]
    fn templates_have_at_least_ten_pages() {
        for kind in ["social", "routine", "emotion", "educational", "narrative"] {
            assert!(sentence_templates(kind, "Titolo", "scuola", "argomento").len() >= 10);
        }
    }
}
