//! Morfologia italiana leggera, basata su regole.
//!
//! Scelta architetturale: nessun modello NLP da centinaia di MB. Le forme
//! regolari sono generate (avanti) e riconosciute (indietro) con regole;
//! le irregolarità stanno nel lessico (`irregular`).
//!
//! Limite dichiarato: i verbi irregolari non elencati nel lessico possono
//! essere riconosciuti solo tramite la forma base.

/// Una forma flessa generata.
#[derive(Debug, Clone)]
pub struct Inflection {
    pub form: String,
    pub mood: Option<String>,
    pub tense: Option<String>,
    pub person: Option<String>,
    pub number: Option<String>,
    pub gender: Option<String>,
}

fn is_vowel(c: char) -> bool {
    matches!(
        c,
        'a' | 'e' | 'i' | 'o' | 'u' | 'à' | 'è' | 'é' | 'ì' | 'ò' | 'ù'
    )
}

fn stem_verb(lemma: &str) -> Option<(&str, &'static str)> {
    for ending in ["are", "ere", "ire"] {
        if let Some(stem) = lemma.strip_suffix(ending) {
            if !stem.is_empty() {
                return Some((stem, ending));
            }
        }
    }
    None
}

/// Forme verbali regolari: indicativo presente, imperfetto, futuro,
/// participio passato, gerundio, infinito.
pub fn verb_forms(lemma: &str) -> Vec<Inflection> {
    let Some((stem, ending)) = stem_verb(lemma) else {
        return Vec::new();
    };
    let mut out = vec![Inflection {
        form: lemma.to_string(),
        mood: Some("inf".into()),
        tense: None,
        person: None,
        number: None,
        gender: None,
    }];

    let push = |out: &mut Vec<Inflection>, form: String, tense: &str, person: &str| {
        out.push(Inflection {
            form,
            mood: Some("ind".into()),
            tense: Some(tense.to_string()),
            person: Some(person.to_string()),
            number: Some(if person == "1s" || person == "2s" || person == "3s" {
                "sg".to_string()
            } else {
                "pl".to_string()
            }),
            gender: None,
        });
    };

    // Presente indicativo. Nei verbi in -are la "i" del tema si comporta
    // diversamente: mangiare -> mangio, mangi, mangia, mangiamo, mangiate,
    // mangiano; camminare -> cammino, cammini, cammina, camminiamo...
    let persons = ["1s", "2s", "3s", "1p", "2p", "3p"];
    let present: Vec<String> = match ending {
        "are" => {
            let i_stem = stem.ends_with('i');
            vec![
                format!("{stem}o"),
                if i_stem {
                    stem.to_string()
                } else {
                    format!("{stem}i")
                },
                format!("{stem}a"),
                if i_stem {
                    format!("{stem}amo")
                } else {
                    format!("{stem}iamo")
                },
                format!("{stem}ate"),
                format!("{stem}ano"),
            ]
        }
        "ere" => ["o", "i", "e", "iamo", "ete", "ono"]
            .iter()
            .map(|e| format!("{stem}{e}"))
            .collect(),
        _ => ["o", "i", "e", "iamo", "ite", "ono"]
            .iter()
            .map(|e| format!("{stem}{e}"))
            .collect(),
    };
    for (i, form) in present.iter().enumerate() {
        push(&mut out, form.clone(), "pres", persons[i]);
        // Variante incoativa (finisco, capisci...): tipica di -ire.
        if ending == "ire" && (i == 0 || i == 1 || i == 2 || i == 5) {
            let e_isc = match i {
                0 => "isco",
                1 => "isci",
                2 => "isce",
                _ => "iscono",
            };
            push(&mut out, format!("{stem}{e_isc}"), "pres", persons[i]);
        }
    }

    // Imperfetto.
    let imperfetto = ["avo", "avi", "ava", "avamo", "avate", "avano"];
    let imperfetto_ere = ["evo", "evi", "eva", "evamo", "evate", "evano"];
    let imperfetto_ire = ["ivo", "ivi", "iva", "ivamo", "ivate", "ivano"];
    let table: [&str; 6] = match ending {
        "are" => imperfetto,
        "ere" => imperfetto_ere,
        _ => imperfetto_ire,
    };
    for (i, e) in table.iter().enumerate() {
        push(&mut out, format!("{stem}{e}"), "impf", persons[i]);
    }

    // Futuro: la base cambia con la coniugazione
    // (mangiare -> mangerò, scrivere -> scriverò, dormire -> dormirò).
    let future_base = match ending {
        "are" => format!(
            "{}er",
            stem.trim_end_matches(|c: char| is_vowel(c))
        ),
        "ire" => format!("{stem}ir"),
        _ => format!("{stem}er"),
    };
    for (i, e) in ["ò", "ai", "à", "emo", "ete", "anno"].iter().enumerate() {
        push(&mut out, format!("{future_base}{e}"), "fut", persons[i]);
    }

    // Condizionale presente: assaggiare -> assaggerei, capire -> capirei.
    // Senza queste forme «assaggerei» non veniva ricondotto ad «assaggiare»
    // e la parola restava senza lemma.
    for (i, e) in ["ei", "esti", "ebbe", "emmo", "este", "ebbero"]
        .iter()
        .enumerate()
    {
        out.push(Inflection {
            form: format!("{future_base}{e}"),
            mood: Some("cond".into()),
            tense: Some("cond".into()),
            person: Some(persons[i].to_string()),
            number: Some(
                if persons[i] == "1s" || persons[i] == "2s" || persons[i] == "3s" {
                    "sg".to_string()
                } else {
                    "pl".to_string()
                },
            ),
            gender: None,
        });
    }

    // Participio passato (forme comuni: -ato/-ato, -uto, -ito).
    let (part_sg, part_pl): (String, String) = match ending {
        "are" => ("ato".into(), "ati".into()),
        "ere" => ("uto".into(), "uti".into()),
        _ => ("ito".into(), "iti".into()),
    };
    let participio: [(&str, &str, &str); 4] = match ending {
        "are" => [("ato", "sg", "m"), ("ata", "sg", "f"), ("ati", "pl", "m"), ("ate", "pl", "f")],
        "ere" => [("uto", "sg", "m"), ("uta", "sg", "f"), ("uti", "pl", "m"), ("ute", "pl", "f")],
        _ => [("ito", "sg", "m"), ("ita", "sg", "f"), ("iti", "pl", "m"), ("ite", "pl", "f")],
    };
    for (suffix, number, gender) in participio {
        out.push(Inflection {
            form: format!("{stem}{suffix}"),
            mood: Some("part".into()),
            tense: Some("past".into()),
            person: None,
            number: Some(number.into()),
            gender: Some(gender.into()),
        });
    }
    let gerundio = match ending {
        "are" => "ando",
        _ => "endo",
    };
    out.push(Inflection {
        form: format!("{stem}{gerundio}"),
        mood: Some("ger".into()),
        tense: None,
        person: None,
        number: None,
        gender: None,
    });
    let _ = (part_sg, part_pl);
    out
}

/// Nomi e aggettivi: singolare/plurale e accordo di genere.
pub fn nominal_forms(lemma: &str, pos: &str, gender: Option<&str>) -> Vec<Inflection> {
    let mut out = Vec::new();
    let chars: Vec<char> = lemma.chars().collect();
    let Some(&last) = chars.last() else {
        return out;
    };
    let can_flect = is_vowel(last) && !matches!(last, 'à' | 'è' | 'ì' | 'ò' | 'ù');
    if !can_flect {
        out.push(Inflection {
            form: lemma.to_string(),
            mood: None,
            tense: None,
            person: None,
            number: Some("sg".into()),
            gender: gender.map(|g| g.to_string()),
        });
        return out;
    }
    let stem: String = chars[..chars.len() - 1].iter().collect();
    let mut add = |form: String, number: &str, g: Option<&str>| {
        out.push(Inflection {
            form,
            mood: None,
            tense: None,
            person: None,
            number: Some(number.into()),
            gender: g.map(|x| x.to_string()),
        });
    };

    if pos == "adj" {
        match last {
            'o' => {
                add(format!("{stem}o"), "sg", Some("m"));
                add(format!("{stem}a"), "sg", Some("f"));
                add(format!("{stem}i"), "pl", Some("m"));
                add(format!("{stem}e"), "pl", Some("f"));
            }
            'e' => {
                add(format!("{stem}e"), "sg", None);
                add(format!("{stem}i"), "pl", None);
            }
            'a' => {
                add(format!("{stem}a"), "sg", Some("f"));
                add(format!("{stem}e"), "pl", Some("f"));
            }
            _ => add(lemma.to_string(), "sg", gender),
        }
        return out;
    }

    // Nomi: maschile in -o, femminile in -a, nomi in -e, invariabili in -i.
    match last {
        'o' => {
            add(format!("{stem}o"), "sg", Some("m"));
            add(format!("{stem}i"), "pl", Some("m"));
            if lemma.ends_with("co") || lemma.ends_with("go") {
                add(format!("{stem}hi"), "pl", Some("m"));
            }
        }
        'a' => {
            add(format!("{stem}a"), "sg", Some("f"));
            add(format!("{stem}e"), "pl", Some("f"));
            if lemma.ends_with("ca") || lemma.ends_with("ga") {
                add(format!("{stem}he"), "pl", Some("f"));
            }
        }
        'e' => {
            add(lemma.to_string(), "sg", None);
            add(format!("{stem}i"), "pl", None);
        }
        _ => add(lemma.to_string(), "sg", gender),
    }
    out
}

/// Genera il paradigma completo per un lemma noto.
pub fn generate_forms(lemma: &str, pos: &str, gender: Option<&str>) -> Vec<Inflection> {
    match pos {
        "verb" => verb_forms(lemma),
        "noun" | "adj" => nominal_forms(lemma, pos, gender),
        _ => Vec::new(),
    }
}

/// Riconoscimento inverso: la forma può appartenere al paradigma del lemma?
pub fn generates_form(lemma: &str, pos: &str, form: &str) -> bool {
    if lemma == form {
        return true;
    }
    generate_forms(lemma, pos, None)
        .iter()
        .any(|f| f.form == form)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forms(lemma: &str, pos: &str) -> Vec<String> {
        generate_forms(lemma, pos, None)
            .into_iter()
            .map(|f| f.form)
            .collect()
    }

    #[test]
    fn regular_are_verb() {
        let f = forms("mangiare", "verb");
        for expected in ["mangio", "mangi", "mangia", "mangiamo", "mangiavo", "mangerò", "mangiato"] {
            assert!(
                f.contains(&expected.to_string()),
                "manca {expected}; forme generate: {f:?}"
            );
        }
    }

    #[test]
    fn conditional_forms_are_recognised() {
        let f = forms("assaggiare", "verb");
        for expected in ["assaggerei", "assaggeresti", "assaggerebbe"] {
            assert!(
                f.contains(&expected.to_string()),
                "manca {expected}; forme generate: {f:?}"
            );
        }
        assert!(generates_form("assaggiare", "verb", "assaggerei"));
        assert!(generates_form("capire", "verb", "capirei"));
    }

    #[test]
    fn regular_ere_verb() {
        let f = forms("scrivere", "verb");
        for expected in ["scrivo", "scrive", "scriviamo", "scriveva", "scriverò", "scritto"] {
            if expected == "scritto" {
                continue; // irregolare, gestito dal lessico
            }
            assert!(
                f.contains(&expected.to_string()),
                "manca {expected}; forme generate: {f:?}"
            );
        }
    }

    #[test]
    fn noun_plural() {
        let f = forms("pesca", "noun");
        assert!(f.contains(&"pesche".to_string()));
        let f = forms("casa", "noun");
        assert!(f.contains(&"case".to_string()));
        let f = forms("libro", "noun");
        assert!(f.contains(&"libri".to_string()));
    }

    #[test]
    fn adjective_agreement() {
        let f = forms("bello", "adj");
        assert!(f.contains(&"bella".to_string()));
        assert!(f.contains(&"belli".to_string()));
        assert!(f.contains(&"belle".to_string()));
    }

    #[test]
    fn reverse_lookup() {
        assert!(generates_form("mangiare", "verb", "mangiavano"));
        assert!(!generates_form("mangiare", "verb", "bevo"));
    }
}
