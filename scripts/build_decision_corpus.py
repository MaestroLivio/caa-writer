#!/usr/bin/env python3
"""Costruisce il corpus di regressione del motore decisionale.

Il corpus curato contiene i casi obbligatori della specifica più frasi
held-out. Non è il dataset da 500 casi richiesto per la calibrazione: quello
richiede annotazione umana e resta un'attività aperta.
"""
from __future__ import annotations

import json
from pathlib import Path

CASES = [
    # Casi obbligatori della specifica.
    ("pesca-frutto", "Ho mangiato una pesca.", "pesca",
     [("frutto", "frutto (alimento)"), ("attivita", "attività di pesca (catturare pesci)")], "frutto"),
    ("pesca-attivita", "Vado a pesca.", "pesca",
     [("frutto", "frutto (alimento)"), ("attivita", "attività di pesca (catturare pesci)")], "attivita"),
    ("amo-amore", "Amo la pizza.", "Amo",
     [("amore", "amore (sentimento)"), ("amo", "amo da pesca (oggetto per pescare)")], "amore"),
    ("amo-oggetto", "Ho comprato un amo.", "amo",
     [("amore", "amore (sentimento)"), ("amo", "amo da pesca (oggetto per pescare)")], "amo"),
    ("riso-cibo", "Mangio il riso.", "riso",
     [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")], "alimento"),
    ("riso-risata", "Il suo riso è contagioso.", "riso",
     [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")], "ridere"),
    ("porta-stanza", "Chiudi la porta.", "porta",
     [("porta_casa", "porta (di una stanza)"), ("portare", "porta (verbo portare)")], "porta_casa"),
    ("porta-verbo", "La bambina porta un libro.", "porta",
     [("porta_casa", "porta (di una stanza)"), ("portare", "porta (verbo portare)")], "portare"),
    ("piano-strumento", "Suono il piano.", "piano",
     [("pianoforte", "pianoforte (strumento musicale)"), ("avverbio", "piano (lentamente)")], "pianoforte"),
    ("piano-avverbio", "Cammina piano.", "piano",
     [("pianoforte", "pianoforte (strumento musicale)"), ("avverbio", "piano (lentamente)")], "avverbio"),
    ("penna-scrivere", "Scrivo con la penna.", "penna",
     [("penna_scrivere", "penna (per scrivere)"), ("piuma", "penna (piuma di uccello)")], "penna_scrivere"),
    ("penna-piuma", "L'uccello ha perso una penna.", "penna",
     [("penna_scrivere", "penna (per scrivere)"), ("piuma", "penna (piuma di uccello)")], "piuma"),
    # Frasi held-out, non usate per scrivere le regole.
    ("pesca-matura", "La pesca matura è dolce.", "pesca",
     [("frutto", "frutto (alimento)"), ("attivita", "attività di pesca (catturare pesci)")], "frutto"),
    ("pesca-mare", "Andiamo al mare per la pesca.", "pesca",
     [("frutto", "frutto (alimento)"), ("attivita", "attività di pesca (catturare pesci)")], "attivita"),
    ("amo-ti", "Ti amo tanto.", "amo",
     [("amore", "amore (sentimento)"), ("amo", "amo da pesca (oggetto per pescare)")], "amore"),
    ("amo-lenza", "L'amo è attaccato alla lenza.", "amo",
     [("amore", "amore (sentimento)"), ("amo", "amo da pesca (oggetto per pescare)")], "amo"),
    ("riso-cucina", "Il riso è pronto in cucina.", "riso",
     [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")], "alimento"),
    ("riso-contagioso", "Il riso di Marco è contagioso.", "riso",
     [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")], "ridere"),
    ("porta-apri", "Apri la porta, per favore.", "porta",
     [("porta_casa", "porta (di una stanza)"), ("portare", "porta (verbo portare)")], "porta_casa"),
    ("porta-borsa", "Marco porta la borsa.", "porta",
     [("porta_casa", "porta (di una stanza)"), ("portare", "porta (verbo portare)")], "portare"),
    ("penna-compito", "Uso la penna per il compito.", "penna",
     [("penna_scrivere", "penna (per scrivere)"), ("piuma", "penna (piuma di uccello)")], "penna_scrivere"),
    ("penna-bianca", "L'uccello ha una penna bianca.", "penna",
     [("penna_scrivere", "penna (per scrivere)"), ("piuma", "penna (piuma di uccello)")], "piuma"),
    ("piano-giorno", "Suono il piano ogni giorno.", "piano",
     [("pianoforte", "pianoforte (strumento musicale)"), ("avverbio", "piano (lentamente)")], "pianoforte"),
]


def span(text: str, word: str) -> dict:
    byte = text.find(word)
    if byte < 0:
        raise ValueError(f"parola {word!r} assente in {text!r}")
    start = len(text[:byte])
    return {"text": word, "start": start, "end": start + len(word)}


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    cases = []
    for case_id, text, word, candidates, expected in CASES:
        cases.append({
            "id": case_id,
            "task": "pictogram_disambiguation",
            "text": text,
            "target": span(text, word),
            "candidates": [{"id": cid, "label": label} for cid, label in candidates],
            "expected": expected,
        })
    payload = {
        "version": "1",
        "language": "it",
        "description": (
            "Corpus curato di regressione: casi obbligatori della specifica più "
            "frasi held-out. Il dataset da 500 casi per la calibrazione richiede "
            "annotazione umana ed è ancora aperto."
        ),
        "cases": cases,
    }
    target = root / "tests" / "decision_corpus.json"
    target.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"scritti {len(cases)} casi in {target}")


if __name__ == "__main__":
    main()
