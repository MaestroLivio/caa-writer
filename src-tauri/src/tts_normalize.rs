//! Normalizzazione deterministica del testo italiano destinato alla sintesi
//! vocale. Il testo del documento non viene mai modificato: questa funzione
//! produce solo la forma da pronunciare.
//!
//! Regole coperte senza modello: numeri interi e decimali, orari, date, percentuali,
//! importi in euro, abbreviazioni comuni. Ciò che non è riconosciuto resta com'è.

const UNITS: [&str; 20] = [
    "zero",
    "uno",
    "due",
    "tre",
    "quattro",
    "cinque",
    "sei",
    "sette",
    "otto",
    "nove",
    "dieci",
    "undici",
    "dodici",
    "tredici",
    "quattordici",
    "quindici",
    "sedici",
    "diciassette",
    "diciotto",
    "diciannove",
];

const TENS: [&str; 10] = [
    "", "dieci", "venti", "trenta", "quaranta", "cinquanta", "sessanta", "settanta", "ottanta",
    "novanta",
];

const MONTHS: [&str; 12] = [
    "gennaio",
    "febbraio",
    "marzo",
    "aprile",
    "maggio",
    "giugno",
    "luglio",
    "agosto",
    "settembre",
    "ottobre",
    "novembre",
    "dicembre",
];

fn below_100(n: u64) -> String {
    if n < 20 {
        return UNITS[n as usize].to_string();
    }
    let tens = TENS[(n / 10) as usize];
    let unit = n % 10;
    match unit {
        0 => tens.to_string(),
        1 | 8 => {
            // ventuno, ventotto: la vocale finale della decina cade.
            let stem = &tens[..tens.len() - 1];
            format!("{stem}{}", UNITS[unit as usize])
        }
        3 => format!("{tens}tré"),
        _ => format!("{tens}{}", UNITS[unit as usize]),
    }
}

fn below_1000(n: u64) -> String {
    let hundreds = n / 100;
    let rest = n % 100;
    let mut out = String::new();
    if hundreds > 0 {
        if hundreds > 1 {
            out.push_str(&below_100(hundreds));
        }
        out.push_str("cento");
    }
    if rest > 0 {
        out.push_str(&below_100(rest));
    }
    out
}

/// Numero intero in parole italiane (fino a milioni, senza segno).
pub fn number_words(n: u64) -> String {
    if n == 0 {
        return "zero".to_string();
    }
    if n < 1000 {
        return below_1000(n);
    }
    let thousands = n / 1000;
    let rest = n % 1000;
    let mut out = if thousands == 1 {
        "mille".to_string()
    } else {
        format!("{}mila", below_1000(thousands))
    };
    if rest > 0 {
        out.push_str(&below_1000(rest));
    }
    out
}

fn digits_words(digits: &str) -> String {
    digits
        .chars()
        .filter_map(|c| c.to_digit(10))
        .map(|d| UNITS[d as usize])
        .collect::<Vec<_>>()
        .join(" ")
}

fn time_words(hour: u64, minute: u64) -> String {
    if hour > 23 || minute > 59 {
        return format!("{} {}", number_words(hour), number_words(minute));
    }
    if minute == 0 {
        return number_words(hour);
    }
    let suffix = match minute {
        15 => "e un quarto".to_string(),
        30 => "e trenta".to_string(),
        45 => "e tre quarti".to_string(),
        _ => format!("e {}", number_words(minute)),
    };
    format!("{} {suffix}", number_words(hour))
}

fn date_words(day: u64, month: u64, year: u64) -> Option<String> {
    if !(1..=31).contains(&day) || !(1..=12).contains(&month) {
        return None;
    }
    let month_name = MONTHS[(month - 1) as usize];
    Some(format!(
        "{} {} {}",
        number_words(day),
        month_name,
        number_words(year)
    ))
}

fn read_digits(chars: &[char], mut index: usize) -> (String, usize) {
    let mut value = String::new();
    while index < chars.len() && chars[index].is_ascii_digit() {
        value.push(chars[index]);
        index += 1;
    }
    (value, index)
}

/// Punto di ingresso: testo scritto → testo da pronunciare.
pub fn normalize(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::new();
    let mut i = 0;

    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        let (first, next) = read_digits(&chars, i);

        // Orario: 8:30
        if next + 2 < chars.len() && chars[next] == ':' && chars[next + 1].is_ascii_digit() && chars[next + 2].is_ascii_digit() {
            let hour: u64 = first.parse().unwrap_or(0);
            let minute: u64 = format!("{}{}", chars[next + 1], chars[next + 2])
                .parse()
                .unwrap_or(0);
            out.push_str(&time_words(hour, minute));
            i = next + 3;
            continue;
        }

        // Data: 05/11/2026 oppure 5/11/26
        if next < chars.len() && chars[next] == '/' {
            let (month_raw, after_month) = read_digits(&chars, next + 1);
            if !month_raw.is_empty()
                && after_month < chars.len()
                && chars[after_month] == '/'
            {
                let (year_raw, after_year) = read_digits(&chars, after_month + 1);
                if !year_raw.is_empty() {
                    let year: u64 = if year_raw.len() == 2 {
                        2000 + year_raw.parse::<u64>().unwrap_or(0)
                    } else {
                        year_raw.parse().unwrap_or(0)
                    };
                    match date_words(
                        first.parse().unwrap_or(0),
                        month_raw.parse().unwrap_or(0),
                        year,
                    ) {
                        Some(words) => out.push_str(&words),
                        // Data non valida: si lascia scritta com'è, senza inventare.
                        None => {
                            out.push_str(&first);
                            out.push('/');
                            out.push_str(&month_raw);
                            out.push('/');
                            out.push_str(&year_raw);
                        }
                    }
                    i = after_year;
                    continue;
                }
            }
        }

        // Importo: 12 € / 12,50 €
        {
            let mut probe = next;
            while probe < chars.len() && chars[probe] == ' ' {
                probe += 1;
            }
            let mut cents = String::new();
            let mut after_amount = next;
            if after_amount < chars.len() && chars[after_amount] == ',' {
                let (raw, after_cents) = read_digits(&chars, after_amount + 1);
                if !raw.is_empty() {
                    cents = raw;
                    after_amount = after_cents;
                }
            }
            let mut currency_probe = after_amount;
            while currency_probe < chars.len() && chars[currency_probe] == ' ' {
                currency_probe += 1;
            }
            if currency_probe < chars.len() && chars[currency_probe] == '€' {
                out.push_str(&number_words(first.parse().unwrap_or(0)));
                out.push_str(" euro");
                if !cents.is_empty() {
                    out.push_str(" e ");
                    out.push_str(&number_words(cents.parse().unwrap_or(0)));
                }
                i = currency_probe + 1;
                continue;
            }
        }

        // Percentuale: 50%
        if next < chars.len() && chars[next] == '%' {
            out.push_str(&number_words(first.parse().unwrap_or(0)));
            out.push_str(" per cento");
            i = next + 1;
            continue;
        }

        // Decimale con virgola: 3,5
        if next + 1 < chars.len() && chars[next] == ',' && chars[next + 1].is_ascii_digit() {
            let (raw, after) = read_digits(&chars, next + 1);
            out.push_str(&number_words(first.parse().unwrap_or(0)));
            out.push_str(" virgola ");
            out.push_str(&digits_words(&raw));
            i = after;
            continue;
        }

        out.push_str(&number_words(first.parse().unwrap_or(0)));
        i = next;
    }

    expand_abbreviations(&out)
}

fn expand_abbreviations(text: &str) -> String {
    let mut out = String::new();
    let mut token = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '.' || ch == '/' || ch == '&' || ch == '€' {
            token.push(ch);
        } else {
            out.push_str(&replace_token(&token));
            token.clear();
            out.push(ch);
        }
    }
    out.push_str(&replace_token(&token));
    out
}

fn replace_token(token: &str) -> String {
    let lower = token.to_lowercase();
    let replacement = match lower.as_str() {
        "ecc." | "ecc" => "eccetera",
        "sig." => "signor",
        "sig.ra" | "sig.ra." => "signora",
        "dott." => "dottor",
        "dott.ssa" => "dottoressa",
        "prof." => "professor",
        "km/h" => "chilometri all'ora",
        "km" => "chilometri",
        "cm" => "centimetri",
        "mm" => "millimetri",
        "min" => "minuti",
        "h" => "ore",
        "n." => "numero",
        "art." => "articolo",
        "tel." => "telefono",
        "€" => "euro",
        "&" => "e",
        _ => return token.to_string(),
    };
    replacement.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_numbers_up_to_thousands() {
        assert_eq!(number_words(0), "zero");
        assert_eq!(number_words(21), "ventuno");
        assert_eq!(number_words(23), "ventitré");
        assert_eq!(number_words(28), "ventotto");
        assert_eq!(number_words(100), "cento");
        assert_eq!(number_words(101), "centouno");
        assert_eq!(number_words(1000), "mille");
        assert_eq!(number_words(1500), "millecinquecento");
        assert_eq!(number_words(2026), "duemilaventisei");
    }

    #[test]
    fn time_is_read_as_words() {
        assert_eq!(
            normalize("Vado a scuola alle 8:30."),
            "Vado a scuola alle otto e trenta."
        );
        assert_eq!(normalize("Ci vediamo alle 7:05."), "Ci vediamo alle sette e cinque.");
        assert_eq!(normalize("Alle 18:00."), "Alle diciotto.");
        assert_eq!(normalize("Alle 10:15."), "Alle dieci e un quarto.");
        assert_eq!(normalize("Alle 10:45."), "Alle dieci e tre quarti.");
    }

    #[test]
    fn dates_and_percentages_are_expanded() {
        assert_eq!(
            normalize("Il 05/11/2026 è mercoledì."),
            "Il cinque novembre duemilaventisei è mercoledì."
        );
        assert_eq!(normalize("Ha preso il 50% dei voti."), "Ha preso il cinquanta per cento dei voti.");
    }

    #[test]
    fn currency_and_decimals_are_read_in_italian() {
        assert_eq!(normalize("Costa 12 €."), "Costa dodici euro.");
        assert_eq!(normalize("Costa 12,50 €."), "Costa dodici euro e cinquanta.");
        assert_eq!(normalize("Pesa 3,5 kg."), "Pesa tre virgola cinque kg.");
    }

    #[test]
    fn abbreviations_use_word_boundaries() {
        assert_eq!(normalize("ecc. e km"), "eccetera e chilometri");
        assert_eq!(normalize("Vado al mare, ecc."), "Vado al mare, eccetera");
        assert_eq!(normalize("Parliamo di h"), "Parliamo di ore");
        // "casa" non deve diventare "casore".
        assert_eq!(normalize("La casa è grande."), "La casa è grande.");
    }

    #[test]
    fn punctuation_and_accents_are_preserved() {
        assert_eq!(normalize("Perché? Sì!"), "Perché? Sì!");
    }

    #[test]
    fn invalid_dates_are_left_alone() {
        // 99/99/2026 non è una data valida: nessuna conversione inventata.
        assert_eq!(normalize("99/99/2026"), "99/99/2026");
    }

    #[test]
    fn empty_and_text_only_inputs_are_stable() {
        assert_eq!(normalize(""), "");
        assert_eq!(normalize("Ciao mondo"), "Ciao mondo");
    }
}
