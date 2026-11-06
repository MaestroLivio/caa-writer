#!/usr/bin/env python3
"""Genera la libreria deterministica dei 300 modelli CAA.

I file prodotti stanno in resources/templates/library/*.json e rispettano lo
schema dei modelli esistenti (id, name, category, description, kind, version,
license, grid, fields, items), con alcuni metadati opzionali aggiuntivi.

Caratteristiche:
  * nessun simbolo inventato: ogni item ha solo etichetta e, quando esiste,
    l'id di un concetto presente in resources/dictionaries/it-core.json;
  * esecuzione idempotente: la cartella library viene riscritta per intero,
    quindi eventuali file orfani vengono rimossi;
  * validazione integrata dei conteggi, degli id, dei nomi e degli item.

Uso:
    python3 scripts/generate_template_library.py
"""

from __future__ import annotations

import json
import math
import re
import sys
from collections import Counter, OrderedDict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONCEPTS_PATH = ROOT / "resources" / "dictionaries" / "it-core.json"
OUT_DIR = ROOT / "resources" / "templates" / "library"

ID_RE = re.compile(r"^lib_[a-z0-9_-]+$")

CATEGORY_ORDER = [
    "Tabelle comunicative",
    "Agende e calendari visivi",
    "Sequenze e routine",
    "Storie sociali",
    "Materiali TEACCH",
    "Materiali ABA e supporti visivi",
    "Materiali didattici",
    "Emozioni e autoregolazione",
    "Autonomia personale",
    "Giochi e attività interattive",
]

EXPECTED = {
    "Tabelle comunicative": 60,
    "Agende e calendari visivi": 35,
    "Sequenze e routine": 35,
    "Storie sociali": 30,
    "Materiali TEACCH": 30,
    "Materiali ABA e supporti visivi": 20,
    "Materiali didattici": 45,
    "Emozioni e autoregolazione": 20,
    "Autonomia personale": 15,
    "Giochi e attività interattive": 10,
}

INTERACTIVE_CATEGORIES = {
    "Tabelle comunicative",
    "Agende e calendari visivi",
    "Giochi e attività interattive",
}


def load_concepts() -> tuple[dict[str, str], set[str]]:
    """Mappa etichetta (minuscola) -> primo id di concetto, in modo stabile."""
    data = json.loads(CONCEPTS_PATH.read_text(encoding="utf-8"))
    by_label: dict[str, str] = {}
    ids: set[str] = set()
    for concept in data["concepts"]:
        cid = concept["id"]
        ids.add(cid)
        label = concept["label"].strip().lower()
        # Il primo concetto dell'elenco vince: l'ordine del dizionario è fisso.
        by_label.setdefault(label, cid)
    return by_label, ids


CONCEPT_BY_LABEL, CONCEPT_IDS = load_concepts()
TEMPLATES: list[OrderedDict] = []


def _items(raw_items: list[str]) -> list[dict]:
    out = []
    for label in raw_items:
        clean = label.strip()
        if not clean:
            raise ValueError("etichetta vuota")
        out.append(
            {
                "label": clean,
                "concept": CONCEPT_BY_LABEL.get(clean.lower()),
            }
        )
    return out


def _default_fields(kind: str, category: str, name: str, cols: int, layout: str | None) -> list[dict]:
    title = {"key": "title", "label": "Titolo", "kind": "text", "default_value": name}
    if kind == "story":
        return [
            title,
            {"key": "first_person", "label": "Racconto in prima persona", "kind": "boolean", "default_value": "false"},
            {"key": "adult_guide", "label": "Nota per l'adulto", "kind": "text", "default_value": "Leggere una frase alla volta e lasciare spazio alle domande."},
        ]
    if category == "Giochi e attività interattive":
        if "Memory" in name:
            rules = "Mescola le carte e scopri le coppie uguali."
        elif "Tombola" in name:
            rules = "Estrai una carta e copri la casella corrispondente."
        elif "Domino" in name:
            rules = "Abbina le carte che mostrano lo stesso simbolo."
        elif "Flashcard" in name:
            rules = "Mostra una carta alla volta e nomina il simbolo."
        else:
            rules = "Scegli un'attività, rispetta il turno e completa il gioco."
        return [
            title,
            {"key": "players", "label": "Numero di giocatori", "kind": "number", "default_value": "2"},
            {"key": "rules", "label": "Regole", "kind": "text", "default_value": rules},
        ]
    if kind == "grid":
        return [
            title,
            {"key": "columns", "label": "Colonne", "kind": "number", "default_value": str(cols)},
            {"key": "caption", "label": "Didascalia sotto il simbolo", "kind": "boolean", "default_value": "true"},
        ]
    if kind == "schedule":
        return [
            title,
            {"key": "time_column", "label": "Colonna orario", "kind": "boolean", "default_value": "true"},
            {"key": "checkboxes", "label": "Caselle da completare", "kind": "boolean", "default_value": "true"},
        ]
    if kind == "sequence":
        return [
            title,
            {"key": "layout", "label": "Disposizione", "kind": "text", "default_value": layout or "passo per passo"},
            {"key": "checkboxes", "label": "Caselle da completare", "kind": "boolean", "default_value": "true"},
        ]
    if kind == "tokens":
        token_count = 5
        if "tre gettoni" in name:
            token_count = 3
        elif "dieci gettoni" in name:
            token_count = 10
        return [
            title,
            {"key": "tokens", "label": "Numero di gettoni", "kind": "number", "default_value": str(token_count)},
            {"key": "reward", "label": "Premio finale", "kind": "text", "default_value": "giocare"},
        ]
    return [
        title,
        {"key": "instructions", "label": "Consegna", "kind": "text", "default_value": "Osserva, scegli e completa."},
        {"key": "answer_lines", "label": "Righe per la risposta", "kind": "boolean", "default_value": "true"},
    ]


def add(
    category: str,
    slug: str,
    name: str,
    kind: str,
    items: list[str],
    purpose: str,
    *,
    cols: int = 1,
    rows: int | None = None,
    layout: str | None = None,
    orientation: str | None = None,
    border: bool = True,
    color_coding: str = "nessuna",
    complexity: str | None = None,
    interactive: bool | None = None,
    fields: list[dict] | None = None,
) -> None:
    if category not in EXPECTED:
        raise ValueError(f"categoria sconosciuta: {category}")
    full_id = f"lib_{slug}"
    if not ID_RE.match(full_id):
        raise ValueError(f"id non valido: {full_id}")
    item_objs = _items(items)
    if rows is None:
        rows = max(1, math.ceil(len(item_objs) / cols))
    if rows * cols < len(item_objs):
        raise ValueError(f"{full_id}: griglia {cols}x{rows} troppo piccola per {len(item_objs)} item")

    labels = [i["label"] for i in item_objs]
    if len(item_objs) > 4:
        label_sample = ", ".join(labels[:4]) + " e altre ancora"
    else:
        label_sample = ", ".join(labels)
    if kind == "story":
        description = (
            f"{purpose.rstrip('.')}. Storia in {len(item_objs)} frasi brevi, "
            "da leggere una alla volta, senza giudizi e senza emozioni attribuite."
        )
    elif category == "Giochi e attività interattive":
        description = (
            f"{purpose.rstrip('.')}. Materiale pronto da stampare e ritagliare, "
            "con istruzioni per il gioco."
        )
    else:
        description = f"{purpose.rstrip('.')}. {len(item_objs)} voci, tra cui {label_sample}."

    if orientation is None:
        orientation = "verticale" if cols <= 2 else "orizzontale"
    if complexity is None:
        cells = cols * rows
        complexity = "semplice" if cells <= 6 else "intermedia" if cells <= 16 else "articolata"
    if interactive is None:
        interactive = category in INTERACTIVE_CATEGORIES or kind == "tokens"

    template = OrderedDict(
        [
            ("id", full_id),
            ("name", name),
            ("category", category),
            ("description", description),
            ("kind", kind),
            ("version", "1.0.0"),
            ("license", "CC BY-NC-SA 4.0"),
            ("complexity", complexity),
            ("pages", 1),
            ("orientation", orientation),
            ("printable", True),
            ("interactive", interactive),
            (
                "grid",
                {
                    "columns": cols,
                    "rows": rows,
                    "caption": True,
                    "symbol": True,
                    "border": border,
                    "color_coding": color_coding,
                },
            ),
            ("fields", fields or _default_fields(kind, category, name, cols, layout)),
            ("items", item_objs),
        ]
    )
    TEMPLATES.append(template)


# ---------------------------------------------------------------------------
# Tabelle comunicative (60)
# ---------------------------------------------------------------------------

CORE36 = [
    "volere", "mangiare", "bere", "bagno", "chiedere aiuto", "giocare",
    "dormire", "casa", "scuola", "felice", "triste", "finire",
    "andare", "correre", "camminare", "leggere", "ascoltare", "parlare",
    "scrivere", "palla", "libro", "matita", "penna", "foglio",
    "forbici", "colla", "zaino", "maestra", "amico", "famiglia",
    "cane", "gatto", "acqua", "pane", "mela", "bicchiere",
]

COMMUNICATION: list[tuple[str, str, str, int, int, list[str]]] = [
    ("com_bisogni_essenziali", "Bisogni essenziali", "Griglia rapida per esprimere i bisogni di base", 2, 3,
     ["mangiare", "bere", "bagno", "dormire", "chiedere aiuto", "giocare"]),
    ("com_bisogni_scuola", "Bisogni a scuola", "Selezione dei bisogni tipici durante la giornata scolastica", 3, 2,
     ["chiedere aiuto", "bagno", "bere", "mangiare", "finire", "intervallo"]),
    ("com_bisogni_casa", "Bisogni a casa", "Griglia per comunicare i bisogni nella routine domestica", 2, 3,
     ["mangiare", "bere", "bagno", "dormire", "giocare", "chiedere aiuto"]),
    ("com_bisogni_urgenti", "Bisogni urgenti", "Celle essenziali per segnalare un bisogno immediato", 4, 2,
     ["chiedere aiuto", "bagno", "bere", "dolore", "medico", "casa", "finire", "non so"]),
    ("com_mangiare_scelte", "Mangiare: scelte", "Griglia per scegliere cibi e azioni del pasto", 3, 3,
     ["mangiare", "pane", "pasta", "mela", "frutta", "gelato", "acqua", "piatto", "finire"]),
    ("com_mensa_pasti", "Mensa: i pasti", "Celle per comunicare durante il pranzo in mensa", 4, 2,
     ["pasta", "pane", "frutta", "acqua", "piatto", "bicchiere", "mangiare", "finire"]),
    ("com_colazione", "Colazione", "Griglia dedicata alla colazione di casa", 3, 2,
     ["fare colazione", "pane", "mela", "succo di mela", "acqua", "finire"]),
    ("com_bere_scelte", "Bere: scelte", "Celle per chiedere da bere e scegliere il contenitore", 2, 3,
     ["bere", "acqua", "succo di mela", "bicchiere", "finire", "chiedere aiuto"]),
    ("com_bere_giornata", "Bere nella giornata", "Griglia per indicare quando bere durante la giornata", 3, 2,
     ["bere", "acqua", "bicchiere", "mattina", "pomeriggio", "sera"]),
    ("com_igiene_mani_denti", "Igiene: mani e denti", "Celle per le routine di igiene quotidiana", 3, 2,
     ["lavarsi le mani", "lavarsi i denti", "bagno", "acqua", "finire", "chiedere aiuto"]),
    ("com_igiene_scuola", "Igiene a scuola", "Griglia per l'igiene nei bagni della scuola", 2, 3,
     ["lavarsi le mani", "bagno", "acqua", "finire", "chiedere aiuto", "asciugamano"]),
    ("com_bagno_scuola", "Bagno a scuola", "Celle per chiedere e usare il bagno a scuola", 2, 3,
     ["bagno", "chiedere aiuto", "lavarsi le mani", "finire", "acqua", "porta"]),
    ("com_bagno_casa", "Bagno a casa", "Griglia per la routine del bagno domestico", 3, 2,
     ["bagno", "lavarsi le mani", "lavarsi i denti", "acqua", "porta", "finire"]),
    ("com_vestirsi_scelte", "Vestirsi: scelte", "Celle per scegliere i capi e completare la vestizione", 3, 3,
     ["maglietta", "pantaloni", "calze", "scarpe", "giacca", "cappello", "vestirsi", "finire", "chiedere aiuto"]),
    ("com_vestirsi_scuola", "Vestirsi per la scuola", "Griglia dei capi e degli accessori per uscire", 4, 2,
     ["maglietta", "pantaloni", "scarpe", "cappello", "giacca", "zaino", "finire", "chiedere aiuto"]),
    ("com_dolore_corpo", "Dolore: parti del corpo", "Celle per indicare dove si sente dolore", 3, 3,
     ["testa", "pancia", "denti", "orecchio", "gola", "schiena", "dolore", "medico", "chiedere aiuto"]),
    ("com_dolore_medico", "Dolore: chiedere il medico", "Griglia essenziale per segnalare dolore e chiedere aiuto", 2, 3,
     ["dolore", "medico", "chiedere aiuto", "casa", "acqua", "finire"]),
    ("com_emozioni_base", "Emozioni di base", "Celle per riconoscere e comunicare sei emozioni", 3, 2,
     ["felice", "triste", "arrabbiato", "paura", "stanco", "calmo"]),
    ("com_emozioni_scuola", "Emozioni a scuola", "Griglia per esprimere come ci si sente in classe", 3, 3,
     ["felice", "triste", "arrabbiato", "paura", "stanco", "chiedere aiuto", "amico", "maestra", "finire"]),
    ("com_famiglia_casa", "Famiglia e casa", "Celle per parlare delle persone di famiglia e degli ambienti di casa", 3, 3,
     ["famiglia", "mamma", "papà", "nonno", "nonna", "fratello", "sorella", "casa", "amico"]),
    ("com_famiglia_attivita", "Attività in famiglia", "Griglia delle cose da fare insieme a casa", 4, 2,
     ["famiglia", "casa", "mangiare", "giocare", "parlare", "leggere", "guardare", "dormire"]),
    ("com_scuola_mattina", "Scuola: la mattina", "Celle per seguire le attività della mattina scolastica", 4, 3,
     ["entrata a scuola", "lavoro in classe", "intervallo", "pranzo", "maestra", "banco", "zaino", "matita", "libro", "finire", "giocare", "casa"]),
    ("com_scuola_materiali", "Scuola: i materiali", "Griglia per chiedere e nominare i materiali scolastici", 4, 3,
     ["matita", "penna", "foglio", "libro", "forbici", "colla", "zaino", "banco", "maestra", "scrivere", "leggere", "finire"]),
    ("com_ricreazione", "Ricreazione", "Celle per scegliere cosa fare durante l'intervallo", 3, 3,
     ["giocare", "palla", "amico", "intervallo", "acqua", "bagno", "mangiare", "finire", "chiedere aiuto"]),
    ("com_mensa", "Mensa scolastica", "Griglia completa per il pasto a scuola", 4, 3,
     ["mangiare", "pasta", "pane", "mela", "frutta", "acqua", "piatto", "bicchiere", "finire", "chiedere aiuto", "bagno", "amico"]),
    ("com_biblioteca", "Biblioteca", "Celle per scegliere, leggere e restituire un libro", 3, 3,
     ["leggere", "libro", "ascoltare", "parlare", "finire", "chiedere aiuto", "maestra", "amico", "casa"]),
    ("com_palestra", "Palestra", "Griglia delle attività e dei bisogni in palestra", 3, 3,
     ["correre", "camminare", "palla", "giocare", "acqua", "finire", "chiedere aiuto", "amico", "bagno"]),
    ("com_piscina", "Piscina", "Celle per comunicare durante l'attività in piscina", 3, 3,
     ["nuotare", "acqua", "giocare", "finire", "chiedere aiuto", "bagno", "asciugamano", "costume", "amico"]),
    ("com_giardino", "Giardino", "Griglia per giocare e esplorare il giardino", 3, 3,
     ["giocare", "palla", "correre", "camminare", "albero", "fiore", "acqua", "finire", "chiedere aiuto"]),
    ("com_casa_stanze", "Casa: le stanze", "Celle per indicare gli ambienti e le azioni di casa", 3, 3,
     ["casa", "cucina", "bagno", "letto", "tavolo", "sedia", "porta", "giocare", "dormire"]),
    ("com_viaggio_preparativi", "Viaggio: preparativi", "Griglia delle cose da preparare prima di partire", 4, 2,
     ["macchina", "autobus", "bicicletta", "zaino", "acqua", "mangiare", "bagno", "finire"]),
    ("com_trasporti", "Trasporti", "Celle per scegliere come spostarsi", 3, 3,
     ["macchina", "autobus", "bicicletta", "monopattino", "camminare", "andare", "casa", "scuola", "finire"]),
    ("com_gioco", "Gioco: scelte", "Griglia per scegliere un gioco e i suoi materiali", 4, 3,
     ["giocare", "palla", "libro", "cane", "gatto", "amico", "musica", "casa", "parco", "finire", "chiedere aiuto", "bere"]),
    ("com_animali", "Animali", "Celle per parlare degli animali e delle loro cure", 4, 3,
     ["cane", "gatto", "uccello", "bassotto", "albero", "acqua", "casa", "giocare", "mangiare", "accarezzare", "finire", "amico"]),
    ("com_sport", "Sport", "Griglia per scegliere un'attività sportiva", 3, 3,
     ["correre", "palla", "bicicletta", "giocare", "acqua", "finire", "chiedere aiuto", "amico", "bagno"]),
    ("com_musica", "Musica", "Celle per scegliere di ascoltare o fare musica", 2, 3,
     ["ascoltare", "parlare", "giocare", "canzone", "musica", "finire"]),
    ("com_tempo_libero", "Tempo libero", "Griglia delle attività preferite nel tempo libero", 4, 3,
     ["giocare", "leggere", "ascoltare", "correre", "camminare", "palla", "libro", "amico", "casa", "parco", "finire", "bere"]),
    ("com_conversazione", "Conversazione", "Celle di base per iniziare e mantenere una conversazione", 3, 3,
     ["parlare", "ascoltare", "amico", "maestra", "felice", "triste", "chiedere aiuto", "finire", "casa"]),
    ("com_domande_chi", "Domande: chi?", "Griglia per formulare e rispondere alla domanda «chi?»", 3, 3,
     ["chi", "maestra", "amico", "famiglia", "cane", "gatto", "bambino", "casa", "scuola"]),
    ("com_domande_dove", "Domande: dove?", "Celle per chiedere e indicare un luogo", 3, 3,
     ["dove", "casa", "scuola", "bagno", "cucina", "parco", "giardino", "biblioteca", "fuori"]),
    ("com_domande_quando", "Domande: quando?", "Griglia per collocare un'azione nel tempo", 2, 3,
     ["quando", "mattina", "adesso", "giorno", "notte", "finire"]),
    ("com_risposte_si_no", "Risposte: sì e no", "Celle minime per rispondere e segnalare incertezza", 2, 3,
     ["sì", "no", "non so", "felice", "triste", "finire"]),
    ("com_risposte_scelte", "Risposte e scelte", "Griglia per rispondere, scegliere e chiedere aiuto", 3, 3,
     ["questo", "quello", "ancora", "finire", "non so", "sì", "no", "chiedere aiuto", "giocare"]),
    ("com_richieste_base", "Richieste di base", "Celle per formulare le richieste più frequenti", 3, 3,
     ["volere", "chiedere aiuto", "bere", "mangiare", "bagno", "giocare", "finire", "ancora", "pausa"]),
    ("com_richieste_scuola", "Richieste a scuola", "Griglia per chiedere materiali e aiuto in classe", 3, 3,
     ["chiedere aiuto", "matita", "penna", "foglio", "libro", "bagno", "bere", "finire", "maestra"]),
    ("com_scelte_cibo", "Scelte di cibo", "Celle per scegliere cosa mangiare e bere", 4, 3,
     ["mela", "pane", "pasta", "frutta", "gelato", "succo di mela", "acqua", "piatto", "mangiare", "bere", "finire", "chiedere aiuto"]),
    ("com_scelte_gioco", "Scelte di gioco", "Griglia per scegliere un gioco o un'attività", 4, 3,
     ["giocare", "palla", "libro", "cane", "gatto", "bicicletta", "parco", "casa", "amico", "musica", "finire", "bere"]),
    ("com_rifiuti", "Rifiuti e pause", "Celle per dire no, chiedere una pausa e fermarsi", 2, 3,
     ["no", "basta", "finire", "no grazie", "pausa", "chiedere aiuto"]),
    ("com_rifiuti_cibo", "Rifiutare un cibo", "Griglia per rifiutare un alimento con calma", 3, 3,
     ["no", "finire", "acqua", "pane", "pasta", "mela", "piatto", "basta", "chiedere aiuto"]),
    ("com_opinioni_emozioni", "Opinioni ed emozioni", "Celle per esprimere un parere personale", 3, 3,
     ["felice", "triste", "arrabbiato", "paura", "stanco", "sì", "no", "non so", "finire"]),
    ("com_opinioni_preferenze", "Preferenze", "Griglia per indicare ciò che piace e ciò che non piace", 4, 2,
     ["felice", "triste", "giocare", "leggere", "mangiare", "bere", "cane", "gatto"]),
    ("com_bisogni_20", "Bisogni e attività: 20 celle", "Tabella media con venti parole ad alta frequenza", 5, 4,
     CORE36[:20]),
    ("com_bisogni_24", "Bisogni e attività: 24 celle", "Tabella ampia con ventiquattro parole ad alta frequenza", 6, 4,
     CORE36[:24]),
    ("com_core_36", "Tabella completa: 36 celle", "Tabella completa per la comunicazione quotidiana", 6, 6, CORE36),
    ("com_core_16", "Tabella essenziale: 16 celle", "Tabella compatta con sedici parole di uso frequente", 4, 4,
     CORE36[:16]),
    ("com_core_12", "Tabella base: 12 celle", "Tabella a dodici celle per iniziare a comunicare", 4, 3,
     CORE36[:12]),
    ("com_core_9", "Tabella ridotta: 9 celle", "Tabella a nove celle per scelte rapide", 3, 3, CORE36[:9]),
    ("com_core_6", "Tabella minima: 6 celle", "Tabella a sei celle per i bisogni più urgenti", 2, 3, CORE36[:6]),
    ("com_emergenza", "Comunicazione di emergenza", "Celle per chiedere aiuto e segnalare un problema", 4, 4,
     ["chiedere aiuto", "bagno", "dolore", "medico", "casa", "mangiare", "bere", "no",
      "finire", "sì", "non so", "paura", "stanco", "acqua", "amico", "maestra"]),
    ("com_quotidiano", "Comunicazione quotidiana: 20 celle", "Griglia mista per la routine di tutti i giorni", 5, 4,
     ["volere", "mangiare", "bere", "bagno", "chiedere aiuto", "giocare", "dormire", "casa",
      "scuola", "felice", "triste", "finire", "andare", "leggere", "ascoltare", "parlare",
      "palla", "libro", "matita", "zaino"]),
]

for slug, name, purpose, cols, rows, items in COMMUNICATION:
    add(
        "Tabelle comunicative", slug, name, "grid", items, purpose,
        cols=cols, rows=rows, color_coding="categorie",
        interactive=True,
    )


# ---------------------------------------------------------------------------
# Agende e calendari visivi (35)
# ---------------------------------------------------------------------------

AGENDAS: list[tuple[str, str, str, list[str], int]] = [
    ("ag_giornata_infanzia", "Giornata alla scuola dell'infanzia", "Agenda della giornata tipo alla scuola dell'infanzia",
     ["8:00 entrata a scuola", "8:30 gioco libero", "9:30 attività in classe", "10:30 merenda",
      "11:30 giardino", "12:30 pranzo", "13:30 riposo", "15:30 uscita"], 1),
    ("ag_giornata_primaria", "Giornata alla scuola primaria", "Agenda della giornata tipo alla scuola primaria",
     ["8:00 entrata a scuola", "8:15 lavoro in classe", "10:30 intervallo", "11:00 lavoro in classe",
      "12:30 pranzo", "14:00 attività in classe", "15:30 uscita", "16:00 casa"], 1),
    ("ag_giornata_secondaria", "Giornata alla scuola secondaria", "Agenda della giornata tipo alla scuola secondaria",
     ["7:30 autobus", "8:00 entrata a scuola", "8:10 lavoro in classe", "10:30 intervallo",
      "12:30 pranzo", "14:00 lavoro in classe", "16:00 uscita", "17:00 casa"], 1),
    ("ag_mattina_casa", "Mattina a casa", "Agenda della mattina prima di uscire",
     ["7:00 sveglia", "7:10 lavarsi le mani", "7:20 vestirsi", "7:40 fare colazione",
      "8:00 zaino", "8:10 uscita"], 1),
    ("ag_pomeriggio_casa", "Pomeriggio a casa", "Agenda del pomeriggio dopo la scuola",
     ["13:30 pranzo", "14:00 riposo", "15:00 compiti", "16:00 merenda", "17:00 giocare", "18:30 cena"], 1),
    ("ag_sera_casa", "Sera a casa", "Agenda della sera prima di dormire",
     ["19:00 cena", "19:30 lavarsi i denti", "19:45 pigiama", "20:00 lettura", "20:30 letto", "21:00 dormire"], 1),
    ("ag_settimana_scolastica", "Settimana scolastica", "Agenda settimanale delle attività di scuola e sport",
     ["lunedì scuola e nuoto", "martedì scuola e musica", "mercoledì scuola e calcio",
      "giovedì scuola e danza", "venerdì scuola e famiglia"], 5),
    ("ag_settimana_completa", "Settimana completa", "Calendario visivo da lunedì a domenica",
     ["lunedì scuola", "martedì palestra", "mercoledì biblioteca", "giovedì piscina",
      "venerdì musica", "sabato parco", "domenica famiglia"], 7),
    ("ag_mese_scolastico", "Mese scolastico", "Agenda del mese con le principali scadenze scolastiche",
     ["settimana 1: laboratorio", "settimana 2: verifica", "settimana 3: gita", "settimana 4: festa"], 4),
    ("ag_calendario_mensile", "Calendario mensile delle attività", "Calendario sintetico con cinque appuntamenti del mese",
     ["1: incontro", "8: visita", "15: compleanno", "22: gita", "29: festa"], 5),
    ("ag_extrascolastiche", "Attività extrascolastiche", "Agenda settimanale dei corsi dopo la scuola",
     ["lunedì nuoto", "martedì musica", "mercoledì calcio", "giovedì inglese",
      "venerdì danza", "sabato cavallo", "domenica riposo"], 7),
    ("ag_vacanze_estate", "Vacanze d'estate", "Agenda delle giornate di vacanza al mare o in campagna",
     ["sveglia tardi", "mare", "gelato", "gioco", "pranzo", "passeggiata", "dormire"], 1),
    ("ag_vacanze_inverno", "Vacanze d'inverno", "Agenda delle giornate di vacanza in inverno",
     ["sveglia", "colazione", "giochi in casa", "pranzo", "passeggiata", "cena"], 1),
    ("ag_vacanze_mare", "Vacanze al mare", "Agenda della giornata in spiaggia",
     ["spiaggia", "bagno", "castelli di sabbia", "pranzo", "gelato", "tramonto"], 1),
    ("ag_vacanze_montagna", "Vacanze in montagna", "Agenda della giornata in montagna",
     ["colazione", "sentiero", "pranzo", "gioco", "merenda", "cena"], 1),
    ("ag_visita_medico", "Visita dal medico", "Agenda della visita medica passo per passo",
     ["attesa", "medico", "visita", "medicina", "casa"], 1),
    ("ag_visita_dentista", "Visita dal dentista", "Agenda della visita dentistica passo per passo",
     ["attesa", "dentista", "controllo", "pulizia", "casa"], 1),
    ("ag_visita_nonni", "Visita ai nonni", "Agenda della visita ai nonni",
     ["macchina", "arrivo", "merenda", "gioco", "casa"], 1),
    ("ag_uscita_parco", "Uscita al parco", "Agenda dell'uscita al parco",
     ["preparativi", "macchina", "parco", "giocare", "casa"], 1),
    ("ag_uscita_biblioteca", "Uscita in biblioteca", "Agenda dell'uscita in biblioteca",
     ["preparativi", "biblioteca", "leggere", "prestito", "casa"], 1),
    ("ag_uscita_museo", "Uscita al museo", "Agenda dell'uscita al museo",
     ["autobus", "biglietto", "visita", "pausa", "casa"], 1),
    ("ag_uscita_supermercato", "Uscita al supermercato", "Agenda della spesa al supermercato",
     ["lista", "supermercato", "carrello", "cassa", "busta", "casa"], 1),
    ("ag_cambio_imprevisto", "Cambio di programma imprevisto", "Agenda per gestire un cambiamento non previsto",
     ["programma cambiato", "ascoltare la maestra", "nuova attività", "pausa", "finire"], 1),
    ("ag_supplente", "Giornata con la supplente", "Agenda per la giornata con un'insegnante supplente",
     ["supplente in classe", "ascoltare", "attività", "finire"], 1),
    ("ag_gita_scolastica", "Gita scolastica", "Agenda oraria della gita scolastica",
     ["7:30 ritrovo", "8:00 autobus", "9:30 arrivo", "10:00 visita",
      "12:30 pranzo", "14:00 laboratorio", "15:30 ritorno", "16:30 scuola"], 1),
    ("ag_weekend_casa", "Weekend a casa", "Agenda del fine settimana a casa",
     ["sabato mattina", "sabato pomeriggio", "sabato sera", "domenica mattina",
      "domenica pomeriggio", "domenica sera"], 1),
    ("ag_compleanno", "Festa di compleanno", "Agenda della festa di compleanno",
     ["inviti", "torta", "candeline", "regali", "giochi", "musica", "saluti"], 1),
    ("ag_festa_scuola", "Festa di scuola", "Agenda della festa scolastica",
     ["preparativi", "addobbi", "spettacolo", "merenda", "giochi", "saluti"], 1),
    ("ag_trasporto_scuola", "Casa-scuola con l'autobus", "Agenda del tragitto da casa a scuola",
     ["uscita", "fermata", "autobus", "scuola", "entrata"], 1),
    ("ag_mensa_settimanale", "Menu della settimana", "Agenda settimanale dei piatti della mensa",
     ["lunedì pasta", "martedì riso", "mercoledì pollo", "giovedì pesce", "venerdì pizza"], 5),
    ("ag_terapia_settimanale", "Appuntamenti settimanali", "Agenda settimanale delle terapie e degli incontri",
     ["lunedì logopedia", "martedì psicomotricità", "mercoledì acquaticità",
      "giovedì logopedia", "venerdì riposo"], 5),
    ("ag_sport_settimanale", "Settimana sportiva", "Agenda settimanale di allenamenti e partite",
     ["lunedì allenamento", "mercoledì partita", "venerdì allenamento", "sabato gara", "domenica riposo"], 5),
    ("ag_piscina_settimanale", "Settimana in piscina", "Agenda settimanale degli allenamenti in piscina",
     ["lunedì nuoto", "mercoledì nuoto", "venerdì nuoto", "sabato piscina libera", "domenica riposo"], 5),
    ("ag_doposcuola", "Doposcuola", "Agenda del pomeriggio al doposcuola",
     ["arrivo", "merenda", "compiti", "gioco", "uscita"], 1),
    ("ag_natale", "Giorni di Natale", "Agenda dei giorni di festa a Natale",
     ["vigilia", "cena", "regali", "messa", "pranzo", "famiglia"], 1),
]

for slug, name, purpose, items, cols in AGENDAS:
    add(
        "Agende e calendari visivi", slug, name, "schedule", items, purpose,
        cols=cols, rows=max(1, math.ceil(len(items) / cols)),
        orientation="orizzontale" if cols > 1 else "verticale",
        interactive=True,
    )


# ---------------------------------------------------------------------------
# Sequenze e routine (35)
# ---------------------------------------------------------------------------

SEQUENCES: list[tuple[str, str, str, list[str], str]] = [
    ("seq_lavarsi_mani_lineare", "Lavarsi le mani: sequenza lineare", "Routine per lavarsi le mani in sei passi", [
        "aprire l'acqua", "bagnare le mani", "mettere il sapone",
        "sfregare le mani", "sciacquare", "asciugare"], "lineare"),
    ("seq_lavarsi_mani_checklist", "Lavarsi le mani: checklist", "Checklist della routine per lavarsi le mani", [
        "aprire l'acqua", "bagnare le mani", "mettere il sapone",
        "sfregare le mani", "sciacquare", "asciugare"], "checklist"),
    ("seq_lavarsi_mani_orizzontale", "Lavarsi le mani: striscia orizzontale", "Striscia orizzontale della routine per lavarsi le mani", [
        "aprire l'acqua", "bagnare le mani", "mettere il sapone",
        "sfregare le mani", "sciacquare", "asciugare"], "orizzontale"),
    ("seq_lavarsi_denti_verticale", "Lavarsi i denti: sequenza verticale", "Routine verticale per lavarsi i denti", [
        "prendere lo spazzolino", "mettere il dentifricio", "spazzolare i denti",
        "sciacquare la bocca", "sciacquare lo spazzolino", "riporre lo spazzolino"], "verticale"),
    ("seq_lavarsi_denti_checklist", "Lavarsi i denti: checklist", "Checklist della routine per lavarsi i denti", [
        "prendere lo spazzolino", "mettere il dentifricio", "spazzolare i denti",
        "sciacquare la bocca", "sciacquare lo spazzolino", "riporre lo spazzolino"], "checklist"),
    ("seq_vestirsi_lineare", "Vestirsi: sequenza lineare", "Routine per vestirsi in otto passi", [
        "mutande", "maglietta", "pantaloni", "calze",
        "scarpe", "giacca", "zaino", "specchio"], "lineare"),
    ("seq_vestirsi_orizzontale", "Vestirsi: striscia orizzontale", "Striscia orizzontale della routine per vestirsi", [
        "mutande", "maglietta", "pantaloni", "calze",
        "scarpe", "giacca", "zaino", "specchio"], "orizzontale"),
    ("seq_zaino_checklist", "Preparare lo zaino: checklist", "Checklist per preparare lo zaino prima di uscire", [
        "astuccio", "quaderni", "libri", "agenda", "borraccia", "merenda", "cappello"], "checklist"),
    ("seq_zaino_verticale", "Preparare lo zaino: sequenza verticale", "Routine verticale per preparare lo zaino", [
        "astuccio", "quaderni", "libri", "agenda", "borraccia", "merenda", "cappello"], "verticale"),
    ("seq_riordinare_lineare", "Riordinare la stanza: sequenza lineare", "Routine per riordinare la stanza in sei passi", [
        "raccogliere i giochi", "mettere i libri", "buttare la carta",
        "spolverare", "mettere a posto", "controllare"], "lineare"),
    ("seq_riordinare_checklist", "Riordinare la stanza: checklist", "Checklist per riordinare la stanza", [
        "raccogliere i giochi", "mettere i libri", "buttare la carta",
        "spolverare", "mettere a posto", "controllare"], "checklist"),
    ("seq_merenda_lineare", "Fare merenda: sequenza lineare", "Routine per la merenda in cinque passi", [
        "lavarsi le mani", "scegliere la merenda", "sedersi", "mangiare", "riordinare"], "lineare"),
    ("seq_merenda_orizzontale", "Fare merenda: striscia orizzontale", "Striscia orizzontale della routine della merenda", [
        "lavarsi le mani", "scegliere la merenda", "sedersi", "mangiare", "riordinare"], "orizzontale"),
    ("seq_apparecchiare_lineare", "Apparecchiare la tavola: sequenza lineare", "Routine per apparecchiare la tavola", [
        "tovaglia", "piatti", "bicchieri", "posate", "tovaglioli", "cibo"], "lineare"),
    ("seq_apparecchiare_checklist", "Apparecchiare la tavola: checklist", "Checklist per apparecchiare la tavola", [
        "tovaglia", "piatti", "bicchieri", "posate", "tovaglioli", "cibo"], "checklist"),
    ("seq_bagno_verticale", "Andare in bagno: sequenza verticale", "Routine verticale per andare in bagno", [
        "entrare", "abbassare i pantaloni", "sedersi", "pulirsi", "tirare l'acqua", "lavarsi le mani"], "verticale"),
    ("seq_bagno_checklist", "Andare in bagno: checklist", "Checklist della routine del bagno", [
        "entrare", "abbassare i pantaloni", "sedersi", "pulirsi", "tirare l'acqua", "lavarsi le mani"], "checklist"),
    ("seq_strada_lineare", "Attraversare la strada: sequenza lineare", "Routine per attraversare la strada in sicurezza", [
        "fermarsi al marciapiede", "guardare a sinistra", "guardare a destra",
        "guardare ancora", "attraversare", "arrivare"], "lineare"),
    ("seq_strada_checklist", "Attraversare la strada: checklist", "Checklist per attraversare la strada in sicurezza", [
        "fermarsi al marciapiede", "guardare a sinistra", "guardare a destra",
        "guardare ancora", "attraversare", "arrivare"], "checklist"),
    ("seq_piscina_lineare", "Andare in piscina: sequenza lineare", "Routine per l'attività in piscina", [
        "preparare la borsa", "spogliatoio", "costume", "doccia",
        "bordo vasca", "nuotare", "asciugarsi"], "lineare"),
    ("seq_piscina_checklist", "Andare in piscina: checklist", "Checklist per l'attività in piscina", [
        "preparare la borsa", "spogliatoio", "costume", "doccia",
        "bordo vasca", "nuotare", "asciugarsi"], "checklist"),
    ("seq_materiali_lineare", "Preparare i materiali scolastici: sequenza lineare", "Routine per preparare i materiali per la lezione", [
        "astuccio", "matita", "penna", "gomma", "righello", "quaderno", "zaino"], "lineare"),
    ("seq_materiali_checklist", "Preparare i materiali scolastici: checklist", "Checklist dei materiali scolastici", [
        "astuccio", "matita", "penna", "gomma", "righello", "quaderno", "zaino"], "checklist"),
    ("seq_lavarsi_viso", "Lavarsi il viso", "Routine per lavarsi il viso", [
        "aprire l'acqua", "bagnare il viso", "mettere il sapone", "sciacquare", "asciugare"], "verticale"),
    ("seq_pettinarsi", "Pettinarsi", "Routine per pettinarsi davanti allo specchio", [
        "prendere il pettine", "guardarsi allo specchio", "pettinare i capelli",
        "mettere a posto il pettine", "controllare"], "lineare"),
    ("seq_fare_il_bagno", "Fare il bagno o la doccia", "Routine per il bagno o la doccia", [
        "preparare l'asciugamano", "entrare nella doccia", "bagnarsi",
        "lavarsi", "sciacquarsi", "asciugarsi"], "verticale"),
    ("seq_prepararsi_dormire", "Prepararsi per dormire", "Routine della sera prima di dormire", [
        "pigiama", "lavarsi i denti", "leggere", "spegnere la luce", "letto", "dormire"], "lineare"),
    ("seq_svegliarsi", "Svegliarsi e uscire", "Routine del mattino dalla sveglia all'uscita", [
        "suonare la sveglia", "alzarsi", "lavarsi il viso", "vestirsi", "colazione", "uscita"], "verticale"),
    ("seq_colazione", "La colazione: checklist", "Routine della colazione a casa", [
        "sedersi", "prendere il pane", "bere", "mangiare", "pulire", "lavarsi le mani"], "checklist"),
    ("seq_pranzo", "Il pranzo: sequenza lineare", "Routine del pranzo a casa o a scuola", [
        "lavarsi le mani", "sedersi", "servirsi", "mangiare", "bere", "riordinare"], "lineare"),
    ("seq_cena", "La cena: sequenza verticale", "Routine della cena in famiglia", [
        "apparecchiare", "sedersi", "servirsi", "mangiare", "riordinare", "lavarsi i denti"], "verticale"),
    ("seq_lavare_piatti", "Lavare i piatti", "Routine per lavare i piatti", [
        "togliere i resti", "insaponare", "sciacquare", "asciugare", "riporre", "lavarsi le mani"], "checklist"),
    ("seq_cameretta", "Mettere in ordine la cameretta", "Routine per riordinare la cameretta", [
        "rifare il letto", "raccogliere i vestiti", "mettere i giochi",
        "spolverare", "mettere a posto", "controllare"], "orizzontale"),
    ("seq_dopo_palestra", "Dopo la palestra", "Routine per cambiarsi dopo la palestra", [
        "asciugarsi", "cambiarsi", "mettere i vestiti nella borsa",
        "pettinarsi", "bere", "uscire"], "lineare"),
    ("seq_rientro_scuola", "Rientro da scuola", "Routine del rientro a casa dopo la scuola", [
        "uscita", "zaino", "fermata", "autobus", "casa", "merenda"], "verticale"),
]

for slug, name, purpose, items, layout in SEQUENCES:
    cols = len(items) if layout == "orizzontale" else (2 if layout == "checklist" and len(items) >= 6 else 1)
    rows = max(1, math.ceil(len(items) / cols))
    add(
        "Sequenze e routine", slug, name, "sequence", items, purpose,
        cols=cols, rows=rows, layout=layout,
        orientation="orizzontale" if layout == "orizzontale" else "verticale",
        interactive=False,
    )


# ---------------------------------------------------------------------------
# Storie sociali (30)
# ---------------------------------------------------------------------------

STORIES: list[tuple[str, str, str, list[str]]] = [
    ("sto_primo_giorno", "Il primo giorno di scuola", "Storia sociale sul primo giorno di scuola", [
        "Il primo giorno di scuola alcune cose sono nuove.",
        "Conosco la maestra e i compagni.",
        "Guardo l'aula e trovo il mio banco.",
        "Se non so dove andare, posso chiedere.",
        "Posso ascoltare le indicazioni della maestra.",
        "A poco a poco la scuola diventa familiare.",
        "Posso raccontare a casa com'è andata.",
    ]),
    ("sto_maestra_parla", "Quando la maestra parla", "Storia sociale sull'ascolto durante la lezione", [
        "La maestra a volte parla a tutta la classe.",
        "Quando parla, posso fermarmi e ascoltare.",
        "Se sto finendo una cosa, posso chiedere un momento.",
        "Posso guardare la maestra per capire cosa dice.",
        "Se non ho capito, posso chiedere di ripetere.",
        "Ascoltare aiuta a sapere cosa fare dopo.",
    ]),
    ("sto_aspettare_turno", "Aspettare il proprio turno", "Storia sociale sull'attesa del turno", [
        "A scuola e nei giochi a volte bisogna aspettare.",
        "Posso guardare chi sta davanti a me.",
        "Posso respirare e contare fino a cinque.",
        "Quando arriva il mio turno, posso giocare.",
        "Aspettare può essere noioso, ed è normale.",
        "Posso chiedere quanto manca.",
    ]),
    ("sto_chiedere_aiuto", "Chiedere aiuto", "Storia sociale sul chiedere aiuto a un adulto", [
        "A volte una cosa è difficile.",
        "Posso dire «mi serve aiuto».",
        "Posso alzare la mano o cercare un adulto.",
        "L'adulto può spiegarmi o farmi vedere.",
        "Chiedere aiuto non è sbagliato.",
        "Dopo posso provare di nuovo.",
    ]),
    ("sto_fare_errore", "Quando faccio un errore", "Storia sociale sull'errore come parte dell'apprendimento", [
        "Tutti possono sbagliare.",
        "Se sbaglio, posso guardare cosa è successo.",
        "Posso cancellare e riprovare.",
        "Posso chiedere una spiegazione.",
        "Un errore mi aiuta a capire meglio.",
        "Posso andare avanti con calma.",
    ]),
    ("sto_perdere_gioco", "Quando perdo un gioco", "Storia sociale sul perdere durante un gioco", [
        "A volte si vince e a volte si perde.",
        "Se perdo, può dispiacermi.",
        "Posso fare un respiro e aspettare.",
        "Posso stringere la mano a chi ha vinto.",
        "Possiamo giocare ancora.",
        "Il gioco resta un gioco.",
    ]),
    ("sto_compagno_nuovo", "Un compagno nuovo", "Storia sociale sull'accogliere un nuovo compagno", [
        "Arriva un compagno nuovo in classe.",
        "Posso dirgli il mio nome.",
        "Posso mostrargli dove mettere lo zaino.",
        "Posso invitarlo a giocare.",
        "Lui può essere timido all'inizio.",
        "Posso dargli tempo.",
        "Se ha bisogno, posso aiutarlo.",
    ]),
    ("sto_rumore_forte", "Un rumore forte", "Storia sociale sui rumori forti e sul cercare un posto tranquillo", [
        "A volte ci sono rumori forti.",
        "Posso coprirmi le orecchie.",
        "Posso respirare lentamente.",
        "Posso cercare un posto più tranquillo.",
        "Posso dire a un adulto che il rumore mi disturba.",
        "Quando il rumore passa, posso tornare.",
    ]),
    ("sto_ricreazione", "La ricreazione", "Storia sociale su come vivere l'intervallo", [
        "Durante la ricreazione posso giocare.",
        "Posso scegliere un gioco con i compagni.",
        "Posso chiedere se posso unirmi.",
        "Se voglio stare da solo, posso dirlo.",
        "Quando suona la campanella, rientro.",
        "Metto a posto il gioco.",
    ]),
    ("sto_bagno_scuola", "Andare in bagno a scuola", "Storia sociale sull'uso del bagno a scuola", [
        "A scuola posso andare in bagno.",
        "Posso chiedere alla maestra.",
        "Cammino senza correre.",
        "Uso il bagno e mi lavo le mani.",
        "Torno in classe.",
        "Se è chiuso, posso aspettare.",
    ]),
    ("sto_pranzo_mensa", "Il pranzo in mensa", "Storia sociale sul pranzo in mensa", [
        "In mensa mangio con i compagni.",
        "Posso scegliere cosa mangiare.",
        "Posso usare forchetta e bicchiere.",
        "Se non mi piace qualcosa, posso dirlo.",
        "Quando finisco, porto il vassoio.",
        "Posso aspettare il mio turno.",
    ]),
    ("sto_cambio_attivita", "Quando cambia l'attività", "Storia sociale sui cambi di attività a scuola", [
        "A scuola le attività cambiano.",
        "La maestra avvisa prima del cambio.",
        "Posso guardare l'agenda visiva.",
        "Metto a posto il materiale.",
        "Prendo quello che serve dopo.",
        "Il cambio può essere veloce o lento.",
    ]),
    ("sto_supplente", "La supplente", "Storia sociale sulla giornata con un'insegnante supplente", [
        "Oggi la maestra non c'è.",
        "In classe c'è una supplente.",
        "Posso ascoltare le sue indicazioni.",
        "Posso dirle il mio nome.",
        "Le attività possono essere diverse.",
        "Se ho bisogno, posso chiedere.",
    ]),
    ("sto_gita", "La gita scolastica", "Storia sociale sulla gita scolastica", [
        "Oggi andiamo in gita.",
        "Prepariamo lo zaino con acqua e merenda.",
        "Saliamo sull'autobus.",
        "Durante il viaggio resto seduto.",
        "Visitiamo il posto.",
        "Alla fine torniamo a scuola.",
        "Posso raccontare cosa ho visto.",
    ]),
    ("sto_dentista", "La visita dal dentista", "Storia sociale sulla visita dal dentista", [
        "Vado dal dentista.",
        "In sala d'attesa posso aspettare.",
        "Il dentista guarda i miei denti.",
        "A volte usa uno specchietto.",
        "Posso aprire la bocca quando me lo chiede.",
        "La visita dura poco.",
        "Dopo posso tornare a casa.",
    ]),
    ("sto_medico", "La visita dal medico", "Storia sociale sulla visita dal medico", [
        "Vado dal medico.",
        "Il medico mi fa alcune domande.",
        "Posso dire cosa sento.",
        "Mi visita con calma.",
        "A volte scrive una medicina.",
        "Dopo posso riposare a casa.",
    ]),
    ("sto_parrucchiere", "Il taglio dei capelli", "Storia sociale sul taglio dei capelli", [
        "Vado dal parrucchiere.",
        "Posso scegliere come voglio i capelli.",
        "Il parrucchiere usa forbici e pettine.",
        "Le forbici fanno un piccolo rumore.",
        "Posso guardarmi allo specchio.",
        "Alla fine vedo il risultato.",
    ]),
    ("sto_temporale", "Arriva un temporale", "Storia sociale sul temporale", [
        "A volte arriva un temporale.",
        "Il cielo diventa scuro.",
        "Si sentono tuoni e si vede la pioggia.",
        "Posso stare in un posto sicuro.",
        "Posso aspettare che passi.",
        "Il temporale finisce.",
    ]),
    ("sto_buio", "Quando è buio", "Storia sociale sulla paura del buio", [
        "A volte la stanza è buia.",
        "Posso accendere la luce.",
        "Posso usare una lampada piccola.",
        "Se ho paura, posso chiamare un adulto.",
        "Il buio non è pericoloso di per sé.",
        "Posso respirare e calmarmi.",
    ]),
    ("sto_dormire_fuori", "Dormire fuori casa", "Storia sociale sul dormire fuori casa", [
        "A volte dormo fuori casa.",
        "Porto il mio pigiama e il mio cuscino.",
        "La stanza può essere diversa.",
        "Posso chiedere dove dormo.",
        "Posso fare la mia solita routine.",
        "Al mattino torno a casa.",
    ]),
    ("sto_compleanno", "Il mio compleanno", "Storia sociale sulla festa di compleanno", [
        "Oggi è il mio compleanno.",
        "Ci sono torta e candeline.",
        "Gli amici cantano.",
        "Posso soffiare le candeline.",
        "Apro i regali con calma.",
        "Ringrazio chi me li ha dati.",
        "La festa finisce.",
    ]),
    ("sto_fila", "Fare la fila", "Storia sociale sul fare la fila a scuola", [
        "A scuola a volte si fa la fila.",
        "Posso mettermi dietro a chi è davanti.",
        "Posso aspettare il mio turno.",
        "Posso tenere le mani lungo i fianchi.",
        "Quando la fila avanza, cammino.",
        "Se mi perdo, posso chiedere.",
    ]),
    ("sto_prestare", "Prestare un giocattolo", "Storia sociale sul prestare un giocattolo", [
        "Un compagno mi chiede un giocattolo.",
        "Posso decidere se prestarlo.",
        "Se dico sì, posso spiegare come si usa.",
        "Se dico no, posso dirlo con calma.",
        "Il giocattolo torna a me dopo.",
        "Posso chiedere di prestarlo a mia volta.",
    ]),
    ("sto_dire_no", "Dire di no", "Storia sociale sul dire di no con calma", [
        "A volte non voglio fare qualcosa.",
        "Posso dire no con calma.",
        "Posso spiegare il motivo.",
        "Posso proporre un'altra cosa.",
        "L'adulto può ascoltarmi.",
        "Dire no aiuta a farsi capire.",
    ]),
    ("sto_ricevere_no", "Quando mi viene detto no", "Storia sociale sul ricevere un no", [
        "A volte mi viene detto no.",
        "Posso sentirmi deluso.",
        "Posso respirare e ascoltare il motivo.",
        "Posso chiedere una spiegazione.",
        "Posso pensare a un'alternativa.",
        "Dopo posso andare avanti.",
    ]),
    ("sto_litigio", "Un litigio con un amico", "Storia sociale su come gestire un litigio", [
        "A volte litigo con un amico.",
        "Possiamo essere entrambi arrabbiati.",
        "Posso allontanarmi un momento.",
        "Posso respirare e pensare.",
        "Posso parlare con calma.",
        "Possiamo chiedere scusa.",
        "Possiamo ricominciare a giocare.",
    ]),
    ("sto_stanchezza", "Quando mi sento stanco", "Storia sociale sulla stanchezza", [
        "A volte mi sento stanco.",
        "Il corpo diventa pesante.",
        "Posso smettere un momento.",
        "Posso bere un po' d'acqua.",
        "Posso chiedere una pausa.",
        "Dopo il riposo sto meglio.",
    ]),
    ("sto_piscina", "La piscina", "Storia sociale sulla routine in piscina", [
        "Vado in piscina.",
        "Mi cambio nello spogliatoio.",
        "Faccio la doccia prima di entrare.",
        "Aspetto il mio turno al bordo vasca.",
        "Nuoto con calma.",
        "Alla fine mi asciugo e mi vesto.",
    ]),
    ("sto_supermercato", "Il supermercato", "Storia sociale sulla spesa al supermercato", [
        "Vado al supermercato.",
        "Prendo il carrello.",
        "Cerco gli articoli della lista.",
        "Posso chiedere aiuto a un commesso.",
        "Alla cassa aspetto il mio turno.",
        "Metto tutto nella busta.",
        "Torno a casa.",
    ]),
    ("sto_autobus", "Il viaggio in autobus", "Storia sociale sul viaggio in autobus", [
        "Per andare a scuola prendo l'autobus.",
        "Aspetto alla fermata.",
        "Controllo il numero dell'autobus.",
        "Salgo e convalido il biglietto.",
        "Resto seduto o mi tengo alla maniglia.",
        "Scendo alla mia fermata.",
        "Cammino fino a scuola.",
    ]),
]

for slug, name, purpose, sentences in STORIES:
    add(
        "Storie sociali", slug, name, "story", sentences, purpose,
        cols=1, rows=len(sentences), border=False, interactive=False,
    )


# ---------------------------------------------------------------------------
# Materiali TEACCH (30)
# ---------------------------------------------------------------------------

TEACCH: list[tuple[str, str, str, list[str], int]] = [
    ("tea_classifica_colori", "Classificare i colori", "Compito di classificazione per colore", [
        "rosso", "blu", "giallo", "verde", "arancione", "viola"], 3),
    ("tea_classifica_forme", "Classificare le forme", "Compito di classificazione per forma", [
        "cerchio", "quadrato", "triangolo", "rettangolo", "stella", "cuore"], 3),
    ("tea_classifica_animali_oggetti", "Animali e oggetti", "Classificazione in due gruppi: animali e oggetti", [
        "cane", "gatto", "tavolo", "sedia", "uccello", "matita", "libro", "palla"], 4),
    ("tea_abbina_ombre", "Abbinare le ombre", "Compito di abbinamento tra figura e ombra", [
        "cane", "gatto", "uccello", "albero", "fiore", "casa"], 3),
    ("tea_abbina_uguali", "Abbinare gli uguali", "Compito di abbinamento di figure identiche", [
        "cane", "gatto", "mela", "gelato", "palla", "libro"], 3),
    ("tea_numero_quantita", "Numero e quantità", "Abbinamento tra numero e quantità corrispondente", [
        "uno", "due", "tre", "quattro", "cinque", "sei"], 3),
    ("tea_sequenza_tre", "Versare e bere: quattro passi", "Ordinare una breve sequenza di azioni per bere", [
        "prendere il bicchiere", "versare l'acqua", "mettere il succo", "bere"], 4),
    ("tea_sequenza_quattro", "Sequenza di quattro azioni", "Ordinare una sequenza di quattro azioni", [
        "prendere il foglio", "piegare", "tagliare", "incollare"], 4),
    ("tea_ordina_piccolo_grande", "Dal più piccolo al più grande", "Ordinamento per dimensione", [
        "piccolo", "medio", "grande", "grandissimo"], 4),
    ("tea_ordina_lunghezza", "Dal più corto al più lungo", "Ordinamento per lunghezza", [
        "corto", "medio", "lungo", "lunghissimo"], 4),
    ("tea_prima_dopo_scuola", "Prima e dopo a scuola", "Sequenza temporale della mattina scolastica", [
        "entrata a scuola", "lavoro in classe", "intervallo", "pranzo", "uscita"], 1),
    ("tea_inizio_fine", "Inizio e fine del lavoro", "Struttura per segnare inizio, svolgimento e fine di un compito", [
        "inizio", "lavoro", "pausa", "controllo", "fine"], 1),
    ("tea_vassoio_lavoro_1", "Vassoio di lavoro: primo compito", "Compito strutturato con vassoio di lavoro", [
        "aprire il vassoio", "prendere il materiale", "svolgere il compito",
        "mettere a posto", "chiudere il vassoio"], 1),
    ("tea_vassoio_lavoro_2", "Vassoio di lavoro: secondo compito", "Secondo compito strutturato con vassoio di lavoro", [
        "scegliere il compito", "lavorare", "controllare", "riporre", "segnare finito"], 1),
    ("tea_categoria_frutta", "Raggruppare la frutta", "Compito di raggruppamento per categoria: frutta", [
        "mela", "banana", "pesca", "uva", "fragola", "arancia"], 3),
    ("tea_categoria_animali", "Raggruppare gli animali", "Compito di raggruppamento per categoria: animali", [
        "cane", "gatto", "uccello", "cavallo", "coniglio", "pesce"], 3),
    ("tea_categoria_vestiti", "Raggruppare i vestiti", "Compito di raggruppamento per categoria: vestiti", [
        "maglietta", "pantaloni", "calze", "scarpe", "cappello", "giacca"], 3),
    ("tea_colori_cestini", "Colori e cestini", "Abbinare ogni colore al cestino corrispondente", [
        "rosso", "blu", "giallo", "verde", "cestino rosso", "cestino blu"], 3),
    ("tea_puzzle", "Completare un puzzle", "Compito strutturato per completare un puzzle", [
        "bordo", "angolo", "centro", "immagine", "finire", "controllare"], 3),
    ("tea_perline", "Infilare le perline", "Compito di motricità fine con le perline", [
        "prendere il filo", "infilare la perlina", "contare", "continuare", "annodare", "finito"], 3),
    ("tea_cubi", "Costruire con i cubi", "Compito strutturato di costruzione con i cubi", [
        "cubo rosso", "cubo blu", "cubo giallo", "torre", "finire", "controllare"], 3),
    ("tea_copiatura", "Copiare un modello", "Compito di copiatura da un modello scritto", [
        "guardare il modello", "scrivere la prima parola", "controllare",
        "scrivere la seconda parola", "rileggere", "finire"], 1),
    ("tea_piegatura", "Piegare un foglio", "Compito strutturato di piegatura della carta", [
        "foglio aperto", "prima piega", "seconda piega", "premere", "aprire", "finito"], 1),
    ("tea_ritaglio", "Ritagliare lungo la linea", "Compito di ritaglio con le forbici", [
        "prendere le forbici", "tagliare lungo la linea", "girare il foglio",
        "continuare", "posare le forbici", "finito"], 1),
    ("tea_incollaggio", "Incollare i pezzi", "Compito strutturato di incollaggio", [
        "mettere la colla", "posizionare il pezzo", "premere", "aspettare", "controllare", "finito"], 1),
    ("tea_presenza", "Tabella delle presenze", "Tabella settimanale per segnare la presenza a scuola", [
        "lunedì", "martedì", "mercoledì", "giovedì", "venerdì"], 5),
    ("tea_routine_mattina", "Routine del mattino TEACCH", "Sequenza strutturata della routine del mattino", [
        "sveglia", "lavarsi", "vestirsi", "colazione", "zaino", "uscita"], 1),
    ("tea_scatola_incarichi", "Scatola degli incarichi", "Compito strutturato con scatola degli incarichi", [
        "aprire la scatola", "leggere l'incarico", "svolgere", "controllare", "riporre", "chiudere"], 1),
    ("tea_grande_piccolo", "Grande e piccolo", "Classificazione dei contrari grande e piccolo", [
        "grande", "piccolo", "alto", "basso", "lungo", "corto"], 3),
    ("tea_dentro_fuori", "Dentro e fuori", "Classificazione dei concetti dentro e fuori", [
        "dentro", "fuori", "sopra", "sotto", "vicino", "lontano"], 3),
]

for slug, name, purpose, items, cols in TEACCH:
    rows = max(1, math.ceil(len(items) / cols))
    add(
        "Materiali TEACCH", slug, name, "worksheet", items, purpose,
        cols=cols, rows=rows, interactive=False,
    )


# ---------------------------------------------------------------------------
# Materiali ABA e supporti visivi (20)
# ---------------------------------------------------------------------------

ABA: list[tuple[str, str, str, str, list[str], int]] = [
    ("aba_scelta_due", "Tabella di scelta: due opzioni", "Supporto visivo per scegliere tra due attività", "grid", [
        "mangiare", "giocare", "bere", "leggere"], 2),
    ("aba_scelta_quattro", "Tabella di scelta: quattro opzioni", "Supporto visivo per scegliere tra quattro attività", "grid", [
        "mangiare", "giocare", "bere", "leggere", "ascoltare", "dormire"], 3),
    ("aba_scelta_attivita", "Tabella di scelta delle attività", "Supporto visivo per scegliere l'attività successiva", "grid", [
        "lavoro in classe", "intervallo", "pranzo", "giocare", "leggere", "casa"], 3),
    ("aba_token_tre", "Token board: tre gettoni", "Tabella di rinforzo con tre gettoni", "tokens", [
        "lavorare", "pausa", "finire", "premio"], 4),
    ("aba_token_cinque", "Token board: cinque gettoni", "Tabella di rinforzo con cinque gettoni", "tokens", [
        "lavorare", "pausa", "finire", "premio", "giocare"], 5),
    ("aba_token_dieci", "Token board: dieci gettoni", "Tabella di rinforzo con dieci gettoni", "tokens", [
        "lavorare", "pausa", "finire", "premio", "giocare", "leggere"], 5),
    ("aba_rinforzo_premi", "Elenco dei rinforzi preferiti", "Elenco visivo dei premi graditi alla persona", "grid", [
        "giocare", "leggere", "palla", "musica", "gelato", "parco"], 3),
    ("aba_rinforzo_scelta", "Scelta del rinforzo", "Tabella per scegliere il premio finale", "grid", [
        "giocare", "leggere", "palla", "gelato", "parco", "casa"], 3),
    ("aba_osservazione_attivita", "Osservazione: attività completate", "Scheda di osservazione delle attività svolte", "worksheet", [
        "iniziata", "svolta", "finita", "con aiuto", "da solo", "da ripetere"], 3),
    ("aba_osservazione_richieste", "Osservazione: richieste", "Scheda di osservazione delle modalità di richiesta", "worksheet", [
        "chiedere aiuto", "chiedere pausa", "chiedere oggetto", "indicare", "parlare", "ripetere"], 3),
    ("aba_osservazione_transizioni", "Osservazione: transizioni", "Scheda di osservazione dei passaggi tra attività", "worksheet", [
        "entrata", "cambio attività", "intervallo", "pranzo", "uscita", "rientro"], 3),
    ("aba_osservazione_attesa", "Osservazione: attesa", "Scheda di osservazione dei tempi di attesa", "worksheet", [
        "aspettare 1 minuto", "aspettare 3 minuti", "aspettare il turno",
        "aspettare in fila", "aspettare seduto", "aspettare in piedi"], 3),
    ("aba_prima_poi", "Prima e poi: quattro passi", "Sequenza visiva prima e poi con quattro passi", "sequence", [
        "lavoro in classe", "pausa", "giocare", "casa"], 1),
    ("aba_prima_dopo_dopo", "Prima, dopo, dopo, infine", "Sequenza visiva con quattro momenti consecutivi", "sequence", [
        "lavoro in classe", "pausa", "giocare", "casa"], 1),
    ("aba_richiesta_oggetto", "Chiedere un oggetto", "Sequenza per chiedere un oggetto desiderato", "sequence", [
        "indicare", "dire il nome", "chiedere", "ricevere", "ringraziare"], 1),
    ("aba_richiesta_pausa", "Chiedere una pausa durante il lavoro", "Sequenza per chiedere una pausa durante il lavoro", "sequence", [
        "sentire la stanchezza", "fare il segno", "chiedere pausa", "riposare", "tornare"], 1),
    ("aba_richiesta_aiuto", "Chiedere aiuto durante un compito", "Sequenza per chiedere aiuto durante un compito", "sequence", [
        "provare", "fermarsi", "chiedere aiuto", "ricevere aiuto", "continuare"], 1),
    ("aba_generalizzazione_casa", "Generalizzazione: a casa", "Elenco di attività da riproporre a casa", "worksheet", [
        "mangiare", "vestirsi", "lavarsi le mani", "riordinare", "dormire"], 1),
    ("aba_generalizzazione_scuola", "Generalizzazione: a scuola", "Elenco di attività da riproporre a scuola", "worksheet", [
        "entrata a scuola", "lavoro in classe", "intervallo", "pranzo", "uscita"], 1),
    ("aba_generalizzazione_cortile", "Generalizzazione: in cortile", "Elenco di attività da riproporre in cortile", "worksheet", [
        "giocare", "palla", "aspettare il turno", "condividere", "rientrare"], 1),
]

for slug, name, purpose, kind, items, cols in ABA:
    rows = max(1, math.ceil(len(items) / cols))
    add(
        "Materiali ABA e supporti visivi", slug, name, kind, items, purpose,
        cols=cols, rows=rows, interactive=(kind == "tokens"),
    )


# ---------------------------------------------------------------------------
# Materiali didattici (45)
# ---------------------------------------------------------------------------

DIDACTIC: list[tuple[str, str, str, list[str], int]] = [
    ("did_alfabeto_maiuscolo", "Alfabeto maiuscolo", "Scheda dell'alfabeto italiano in stampato maiuscolo", [
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "L", "M",
        "N", "O", "P", "Q", "R", "S", "T", "U", "V", "Z"], 7),
    ("did_alfabeto_minuscolo", "Alfabeto minuscolo", "Scheda dell'alfabeto italiano in stampato minuscolo", [
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "l", "m",
        "n", "o", "p", "q", "r", "s", "t", "u", "v", "z"], 7),
    ("did_vocali", "Le vocali", "Scheda di riconoscimento delle cinque vocali", [
        "A", "E", "I", "O", "U"], 5),
    ("did_consonanti", "Le consonanti", "Scheda delle consonanti dell'alfabeto italiano", [
        "B", "C", "D", "F", "G", "H", "L", "M", "N", "P", "Q", "R", "S", "T", "V", "Z"], 4),
    ("did_sillabe_ba", "Sillabe con B", "Scheda di lettura delle sillabe con la consonante B", [
        "BA", "BE", "BI", "BO", "BU"], 5),
    ("did_sillabe_ca", "Sillabe con C", "Scheda di lettura delle sillabe con la consonante C", [
        "CA", "CE", "CI", "CO", "CU"], 5),
    ("did_parole_bisillabe", "Parole bisillabe", "Scheda di lettura di parole bisillabe", [
        "casa", "pane", "mela", "libro", "palla", "cane"], 3),
    ("did_parole_trisillabe", "Parole trisillabe", "Scheda di lettura di parole trisillabe", [
        "matita", "tavolo", "forbici", "albero", "telefono", "bicicletta"], 3),
    ("did_frasi_minime", "Frasi minime", "Scheda di lettura di frasi minime", [
        "il cane corre", "la mela è rossa", "il libro è sul tavolo",
        "la palla è blu", "il gatto dorme", "la maestra legge"], 2),
    ("did_comprensione_chi", "Comprensione: chi?", "Domande di comprensione sul soggetto dell'azione", [
        "Chi corre? Il cane", "Chi legge? La maestra", "Chi dorme? Il gatto",
        "Chi gioca? Il bambino", "Chi cucina? La mamma", "Chi guida? Il papà"], 2),
    ("did_comprensione_dove", "Comprensione: dove?", "Domande di comprensione sul luogo", [
        "Dove dorme il gatto? Sul letto", "Dove mangiamo? In cucina",
        "Dove leggiamo? In biblioteca", "Dove giochiamo? Al parco",
        "Dove studiamo? A scuola", "Dove ci laviamo? In bagno"], 2),
    ("did_comprensione_quando", "Comprensione: quando?", "Domande di comprensione sul tempo", [
        "Quando facciamo colazione? La mattina", "Quando andiamo a dormire? La sera",
        "Quando pranziamo? A mezzogiorno", "Quando andiamo a scuola? Al mattino",
        "Quando vediamo le stelle? La notte", "Quando facciamo merenda? Nel pomeriggio"], 2),
    ("did_categorie_frutta", "Categorie: la frutta", "Scheda di categorizzazione della frutta", [
        "mela", "banana", "pesca", "uva", "fragola", "arancia"], 3),
    ("did_categorie_animali", "Categorie: gli animali", "Scheda di categorizzazione degli animali", [
        "cane", "gatto", "uccello", "cavallo", "coniglio", "pesce"], 3),
    ("did_categorie_trasporti", "Categorie: i trasporti", "Scheda di categorizzazione dei mezzi di trasporto", [
        "macchina", "autobus", "bicicletta", "monopattino", "treno", "aereo"], 3),
    ("did_sinonimi", "Sinonimi", "Scheda di abbinamento tra parole sinonime", [
        "felice = contento", "triste = infelice", "grande = grosso",
        "piccolo = minuscolo", "veloce = rapido", "bello = carino"], 2),
    ("did_contrari", "Contrari", "Scheda di abbinamento tra parole contrarie", [
        "grande / piccolo", "alto / basso", "caldo / freddo",
        "giorno / notte", "aprire / chiudere", "dentro / fuori"], 2),
    ("did_sequenze_mattina", "Sequenze temporali: la mattina", "Scheda per ordinare le azioni della mattina", [
        "sveglia", "colazione", "vestirsi", "uscita", "scuola", "pranzo"], 3),
    ("did_sequenze_stagioni", "Sequenze temporali: le stagioni", "Scheda per ordinare le quattro stagioni", [
        "primavera", "estate", "autunno", "inverno"], 4),
    ("did_numeri_1_10", "Numeri da 1 a 10", "Scheda dei numeri da uno a dieci", [
        "1 uno", "2 due", "3 tre", "4 quattro", "5 cinque",
        "6 sei", "7 sette", "8 otto", "9 nove", "10 dieci"], 5),
    ("did_numeri_11_20", "Numeri da 11 a 20", "Scheda dei numeri da undici a venti", [
        "11 undici", "12 dodici", "13 tredici", "14 quattordici", "15 quindici",
        "16 sedici", "17 diciassette", "18 diciotto", "19 diciannove", "20 venti"], 5),
    ("did_quantita_5", "Quantità fino a 5", "Scheda di associazione tra numero e quantità fino a cinque", [
        "1 mela", "2 mele", "3 mele", "4 mele", "5 mele"], 5),
    ("did_quantita_10", "Quantità fino a 10", "Scheda di associazione tra numero e quantità fino a dieci", [
        "1 mela", "2 mele", "3 mele", "4 mele", "5 mele",
        "6 mele", "7 mele", "8 mele", "9 mele", "10 mele"], 5),
    ("did_addizioni_10", "Addizioni entro 10", "Scheda di addizioni con risultato entro dieci", [
        "2 + 1 = ___", "3 + 2 = ___", "4 + 4 = ___", "5 + 3 = ___", "6 + 2 = ___", "1 + 7 = ___"], 3),
    ("did_sottrazioni_10", "Sottrazioni entro 10", "Scheda di sottrazioni entro dieci", [
        "5 - 2 = ___", "7 - 3 = ___", "9 - 4 = ___", "8 - 2 = ___", "6 - 1 = ___", "10 - 5 = ___"], 3),
    ("did_maggiore_minore", "Maggiore, minore, uguale", "Scheda di confronto tra numeri", [
        "3 ___ 1", "2 ___ 5", "4 ___ 4", "7 ___ 2", "6 ___ 9", "8 ___ 8"], 3),
    ("did_forme_geometriche", "Forme geometriche", "Scheda di riconoscimento delle forme geometriche", [
        "cerchio", "quadrato", "triangolo", "rettangolo", "rombo", "trapezio"], 3),
    ("did_perimetro", "Perimetro delle figure", "Scheda sul numero di lati delle figure piane", [
        "quadrato: 4 lati", "rettangolo: 4 lati", "triangolo: 3 lati",
        "cerchio: 1 bordo", "pentagono: 5 lati", "esagono: 6 lati"], 2),
    ("did_misure_lunghezza", "Misure di lunghezza", "Scheda sulle equivalenze di lunghezza", [
        "1 metro = 100 centimetri", "1 chilometro = 1000 metri",
        "mezzo metro = 50 centimetri", "1 decimetro = 10 centimetri"], 2),
    ("did_misure_peso", "Misure di peso", "Scheda sulle equivalenze di peso", [
        "1 chilogrammo = 1000 grammi", "mezzo chilo = 500 grammi",
        "1 etto = 100 grammi", "1 tonnellata = 1000 chili"], 2),
    ("did_giorni_settimana", "I giorni della settimana", "Scheda per ordinare i giorni della settimana", [
        "lunedì", "martedì", "mercoledì", "giovedì", "venerdì", "sabato", "domenica"], 7),
    ("did_mesi", "I mesi dell'anno", "Scheda per ordinare i dodici mesi", [
        "gennaio", "febbraio", "marzo", "aprile", "maggio", "giugno",
        "luglio", "agosto", "settembre", "ottobre", "novembre", "dicembre"], 4),
    ("did_stagioni", "Le quattro stagioni", "Scheda sulle caratteristiche delle stagioni", [
        "primavera", "estate", "autunno", "inverno"], 4),
    ("did_animali_ambienti", "Animali e ambienti", "Scheda di abbinamento tra animale e ambiente", [
        "cane: casa", "gatto: casa", "uccello: nido",
        "pesce: acqua", "leone: savana", "pinguino: ghiaccio"], 2),
    ("did_parti_pianta", "Le parti della pianta", "Scheda sulle parti della pianta", [
        "radice", "fusto", "foglia", "fiore", "frutto", "seme"], 3),
    ("did_ciclo_pianta", "Il ciclo della pianta", "Scheda sull'ordine di crescita della pianta", [
        "seme", "germoglio", "piantina", "foglie", "fiore", "frutto"], 3),
    ("did_parti_corpo", "Le parti del corpo", "Scheda di riconoscimento delle parti del corpo", [
        "testa", "occhi", "naso", "bocca", "mano", "piede"], 3),
    ("did_cinque_sensi", "I cinque sensi", "Scheda sui cinque sensi", [
        "vista", "udito", "olfatto", "gusto", "tatto"], 5),
    ("did_alimentazione", "Alimentazione equilibrata", "Scheda sui gruppi di alimenti", [
        "frutta", "verdura", "cereali", "proteine", "latticini", "acqua"], 3),
    ("did_civica_regole", "Educazione civica: le regole", "Scheda sulle regole di classe", [
        "ascoltare", "alzare la mano", "rispettare il turno",
        "condividere", "riordinare", "chiedere scusa"], 3),
    ("did_civica_riciclo", "Educazione civica: il riciclo", "Scheda sulla raccolta differenziata", [
        "carta", "plastica", "vetro", "organico", "alluminio", "indifferenziata"], 3),
    ("did_emozioni_volti", "Emozioni e volti", "Scheda di riconoscimento delle espressioni del viso", [
        "felice", "triste", "arrabbiato", "paura", "sorpresa", "calmo"], 3),
    ("did_spazio_sopra_sotto", "Spazio: sopra e sotto", "Scheda sui concetti spaziali di base", [
        "sopra", "sotto", "dentro", "fuori", "vicino", "lontano"], 3),
    ("did_spazio_destra_sinistra", "Spazio: destra e sinistra", "Scheda sull'orientamento nello spazio", [
        "destra", "sinistra", "davanti", "dietro", "in alto", "in basso"], 3),
    ("did_comprensione_sequenza", "Comprensione: prima e poi", "Scheda di comprensione di brevi sequenze", [
        "Prima leggo, poi disegno", "Prima taglio, poi incollo",
        "Prima mi lavo, poi mangio", "Prima mi vesto, poi esco",
        "Prima studio, poi gioco", "Prima ascolto, poi rispondo"], 2),
]

for slug, name, purpose, items, cols in DIDACTIC:
    rows = max(1, math.ceil(len(items) / cols))
    add(
        "Materiali didattici", slug, name, "worksheet", items, purpose,
        cols=cols, rows=rows, interactive=False,
    )


# ---------------------------------------------------------------------------
# Emozioni e autoregolazione (20)
# ---------------------------------------------------------------------------

EMOTIONS: list[tuple[str, str, str, list[str], int]] = [
    ("emo_ruota", "La ruota delle emozioni", "Ruota per riconoscere e nominare sei emozioni", [
        "felice", "triste", "arrabbiato", "paura", "sorpresa", "calmo"], 3),
    ("emo_termometro_calma", "Termometro della calma", "Termometro per misurare l'agitazione e scegliere una strategia", [
        "calmo", "un po' agitato", "agitato", "molto agitato", "pausa", "respiro"], 3),
    ("emo_termometro_rabbia", "Termometro della rabbia", "Termometro per riconoscere la rabbia e fermarsi in tempo", [
        "calmo", "un po' arrabbiato", "arrabbiato", "molto arrabbiato", "pausa", "respiro"], 3),
    ("emo_respirazione", "Respirazione guidata", "Sequenza illustrata per respirare lentamente", [
        "inspira", "trattieni", "espira", "conta fino a 4", "ripeti", "calmo"], 2),
    ("emo_pausa", "Chiedere una pausa", "Sequenza per chiedere e prendere una pausa", [
        "chiedo pausa", "mi allontano", "mi siedo", "respiro", "bevo", "torno"], 2),
    ("emo_contare", "Contare fino a cinque", "Strategia di regolazione attraverso il conteggio", [
        "1", "2", "3", "4", "5", "calmo"], 3),
    ("emo_acqua", "Bere e rinfrescarsi", "Strategia di regolazione con acqua fresca", [
        "bevo acqua", "mi lavo il viso", "respiro", "mi asciugo", "mi siedo", "calmo"], 3),
    ("emo_angolo_calma", "L'angolo della calma", "Cosa trovo nell'angolo della calma", [
        "cuscino", "libro", "cuffie", "acqua", "luce bassa", "timer"], 3),
    ("emo_cosa_mi_aiuta", "Cosa mi aiuta", "Tabella per scegliere cosa aiuta a stare meglio", [
        "respirare", "contare", "bere", "abbraccio", "musica", "silenzio"], 3),
    ("emo_scelta_strategia", "Scegliere una strategia", "Tabella di scelta tra strategie di autoregolazione", [
        "respirare", "contare", "bere", "pausa", "libro", "musica"], 3),
    ("emo_felice_triste", "Felice e triste", "Scheda per distinguere felicità e tristezza", [
        "felice", "triste", "ridere", "piangere", "abbraccio", "parlare"], 3),
    ("emo_arrabbiato", "Quando sono arrabbiato", "Scheda con azioni possibili quando arriva la rabbia", [
        "arrabbiato", "respiro", "pausa", "parlare", "camminare", "calmo"], 3),
    ("emo_paura", "Quando ho paura", "Scheda con azioni possibili quando arriva la paura", [
        "paura", "luce", "adulto", "respiro", "mano", "calmo"], 3),
    ("emo_stanco", "Quando sono stanco", "Scheda con azioni possibili quando arriva la stanchezza", [
        "stanco", "letto", "pausa", "acqua", "silenzio", "dormire"], 3),
    ("emo_sorpresa", "La sorpresa", "Scheda per riconoscere e gestire la sorpresa", [
        "sorpresa", "domanda", "ascoltare", "respiro", "parlare", "calmo"], 3),
    ("emo_calmo", "Stare calmo", "Scheda delle cose che aiutano a stare calmi", [
        "calmo", "respiro", "silenzio", "libro", "musica", "casa"], 3),
    ("emo_intensita", "Quanto è forte?", "Scala di intensità emotiva da uno a cinque", [
        "1 poco", "2 un po'", "3 abbastanza", "4 molto", "5 moltissimo", "chiedo aiuto"], 3),
    ("emo_il_mio_viso", "Il mio viso", "Scheda per osservare le parti del viso nelle emozioni", [
        "occhi", "bocca", "sopracciglia", "guance", "sorriso", "lacrime"], 3),
    ("emo_dopo_litigio", "Dopo un litigio", "Scheda di passi possibili dopo un litigio", [
        "respiro", "allontanarmi", "parlare", "scusarmi", "riposare", "ricominciare"], 3),
    ("emo_aiuto_emotivo", "Chiedere aiuto emotivo", "Scheda per chiedere aiuto quando un'emozione è forte", [
        "sto male", "mi serve aiuto", "chiamo un adulto", "parlo", "abbraccio", "sto meglio"], 3),
]

for slug, name, purpose, items, cols in EMOTIONS:
    rows = max(1, math.ceil(len(items) / cols))
    add(
        "Emozioni e autoregolazione", slug, name, "worksheet", items, purpose,
        cols=cols, rows=rows, interactive=False,
    )


# ---------------------------------------------------------------------------
# Autonomia personale (15)
# ---------------------------------------------------------------------------

AUTONOMY: list[tuple[str, str, str, list[str], str]] = [
    ("aut_vestirsi_mattina", "Vestirsi al mattino", "Routine autonoma per vestirsi al mattino", [
        "mutande", "maglietta", "pantaloni", "calze", "scarpe", "giacca"], "lineare"),
    ("aut_vestirsi_palestra", "Cambiarsi dopo la palestra", "Routine autonoma per cambiarsi dopo la palestra", [
        "asciugarsi", "togliere la maglietta", "mettere la maglietta pulita",
        "cambiare i pantaloni", "mettere le scarpe", "riporre i vestiti"], "lineare"),
    ("aut_igiene_bagno", "Igiene: la routine del bagno", "Routine autonoma per l'igiene in bagno", [
        "entrare", "usare il bagno", "pulirsi", "tirare l'acqua", "lavarsi le mani", "asciugare le mani"], "verticale"),
    ("aut_igiene_mani", "Igiene: lavarsi le mani", "Routine autonoma per lavarsi le mani", [
        "aprire l'acqua", "bagnare le mani", "mettere il sapone", "sciacquare", "asciugare"], "checklist"),
    ("aut_igiene_doccia", "Igiene: fare la doccia", "Routine autonoma per fare la doccia", [
        "preparare l'asciugamano", "entrare", "bagnarsi", "lavarsi", "sciacquarsi", "asciugarsi"], "verticale"),
    ("aut_cucina_merenda", "Cucina: preparare la merenda", "Routine autonoma per preparare la merenda", [
        "lavarsi le mani", "scegliere il cibo", "preparare il piatto", "mangiare", "riordinare"], "lineare"),
    ("aut_cucina_panino", "Cucina: preparare un panino", "Routine autonoma per preparare un panino", [
        "lavarsi le mani", "prendere il pane", "mettere il ripieno", "chiudere il panino", "riordinare"], "lineare"),
    ("aut_cucina_apparecchiare", "Cucina: apparecchiare", "Routine autonoma per apparecchiare la tavola", [
        "tovaglia", "piatti", "bicchieri", "posate", "tovaglioli", "cibo"], "checklist"),
    ("aut_denaro_monete", "Denaro: riconoscere le monete", "Scheda per riconoscere monete e valori", [
        "1 centesimo", "2 centesimi", "5 centesimi", "10 centesimi", "50 centesimi", "1 euro"], "verticale"),
    ("aut_denaro_cassa", "Denaro: pagare alla cassa", "Routine autonoma per pagare alla cassa", [
        "prendere il prodotto", "andare alla cassa", "aspettare il turno",
        "pagare", "prendere lo scontrino", "riporre"], "verticale"),
    ("aut_trasporti_autobus", "Trasporti: prendere l'autobus", "Routine autonoma per prendere l'autobus", [
        "andare alla fermata", "aspettare", "controllare il numero",
        "salire", "convalidare il biglietto", "scendere"], "verticale"),
    ("aut_trasporti_strada", "Trasporti: attraversare la strada", "Routine autonoma per attraversare la strada", [
        "fermarsi", "guardare a sinistra", "guardare a destra", "attraversare", "arrivare"], "checklist"),
    ("aut_casa_camera", "Casa: riordinare la camera", "Routine autonoma per riordinare la camera", [
        "rifare il letto", "raccogliere i vestiti", "mettere i giochi",
        "spolverare", "mettere a posto", "controllare"], "checklist"),
    ("aut_casa_piatti", "Casa: lavare i piatti", "Routine autonoma per lavare i piatti", [
        "togliere i resti", "insaponare", "sciacquare", "asciugare", "riporre", "lavarsi le mani"], "verticale"),
    ("aut_casa_zaino", "Casa: preparare lo zaino", "Routine autonoma per preparare lo zaino", [
        "astuccio", "quaderni", "libri", "agenda", "borraccia", "merenda"], "checklist"),
]

for slug, name, purpose, items, layout in AUTONOMY:
    cols = 2 if layout == "checklist" else 1
    rows = max(1, math.ceil(len(items) / cols))
    add(
        "Autonomia personale", slug, name, "sequence", items, purpose,
        cols=cols, rows=rows, layout=layout, interactive=False,
    )


# ---------------------------------------------------------------------------
# Giochi e attività interattive (10)
# ---------------------------------------------------------------------------

GAMES: list[tuple[str, str, str, str, list[str], int]] = [
    ("gio_memory_animali", "Memory degli animali", "Gioco di memoria con dodici carte di animali", "grid", [
        "cane", "cane", "gatto", "gatto", "uccello", "uccello",
        "cavallo", "cavallo", "coniglio", "coniglio", "pesce", "pesce"], 4),
    ("gio_memory_scuola", "Memory dei materiali scolastici", "Gioco di memoria con dodici carte di materiali scolastici", "grid", [
        "matita", "matita", "libro", "libro", "forbici", "forbici",
        "colla", "colla", "zaino", "zaino", "banco", "banco"], 4),
    ("gio_tombola_colori", "Tombola dei colori", "Tombola con dodici colori da riconoscere", "grid", [
        "rosso", "blu", "giallo", "verde", "arancione", "viola",
        "rosa", "marrone", "nero", "bianco", "grigio", "azzurro"], 4),
    ("gio_tombola_animali", "Tombola degli animali", "Tombola con dodici animali da riconoscere", "grid", [
        "cane", "gatto", "uccello", "cavallo", "coniglio", "pesce",
        "tartaruga", "farfalla", "leone", "elefante", "scimmia", "pinguino"], 4),
    ("gio_domino_azioni", "Domino delle azioni", "Domino con otto azioni da abbinare", "grid", [
        "mangiare", "bere", "dormire", "correre",
        "leggere", "scrivere", "giocare", "parlare"], 4),
    ("gio_domino_animali", "Domino degli animali", "Domino con otto animali da abbinare", "grid", [
        "cane", "gatto", "uccello", "pesce",
        "cavallo", "coniglio", "tartaruga", "farfalla"], 4),
    ("gio_flashcard_parole", "Flashcard delle parole", "Otto flashcard per nominare parole quotidiane", "grid", [
        "casa", "scuola", "cane", "mela", "libro", "palla", "maestra", "amico"], 4),
    ("gio_flashcard_emozioni", "Flashcard delle emozioni", "Sei flashcard per riconoscere le emozioni", "grid", [
        "felice", "triste", "arrabbiato", "paura", "stanco", "calmo"], 3),
    ("gio_scelte", "Gioco delle scelte", "Gioco interattivo per scegliere un'attività divertente", "grid", [
        "mangiare", "giocare", "leggere", "bere", "disegnare", "cantare"], 3),
    ("gio_storie_illustrate", "Storie illustrate: una giornata", "Storia illustrata interattiva con sei scene", "story", [
        "Mi sveglio.", "Faccio colazione.", "Vado a scuola.",
        "Gioco con un amico.", "Torno a casa.", "Dormo."], 1),
]

for slug, name, purpose, kind, items, cols in GAMES:
    rows = max(1, math.ceil(len(items) / cols))
    add(
        "Giochi e attività interattive", slug, name, kind, items, purpose,
        cols=cols, rows=rows, color_coding="categorie", interactive=True,
        border=(kind != "story"),
    )


# ---------------------------------------------------------------------------
# Scrittura e validazione
# ---------------------------------------------------------------------------

def sort_key(t: dict) -> tuple[int, str]:
    return (CATEGORY_ORDER.index(t["category"]), t["id"])


def validate(templates: list[dict]) -> list[str]:
    errors: list[str] = []
    if len(templates) != 300:
        errors.append(f"numero modelli: {len(templates)} (atteso 300)")

    counts = Counter(t["category"] for t in templates)
    for category, expected in EXPECTED.items():
        if counts[category] != expected:
            errors.append(f"{category}: {counts[category]} (atteso {expected})")

    seen_ids: dict[str, str] = {}
    seen_names: dict[str, str] = {}
    for t in templates:
        tid = t["id"]
        if not ID_RE.match(tid):
            errors.append(f"id non valido: {tid}")
        if tid in seen_ids:
            errors.append(f"id duplicato: {tid}")
        seen_ids[tid] = t["name"]
        if t["name"] in seen_names:
            errors.append(f"nome duplicato: {t['name']}")
        seen_names[t["name"]] = tid

        if not t["description"].strip():
            errors.append(f"{tid}: descrizione vuota")
        if t["kind"] not in {"grid", "schedule", "tokens", "sequence", "worksheet", "story"}:
            errors.append(f"{tid}: kind non ammesso {t['kind']}")
        if len(t["items"]) < 4:
            errors.append(f"{tid}: {len(t['items'])} item (minimo 4)")
        if t["category"] == "Tabelle comunicative" and len(t["items"]) < 6:
            errors.append(f"{tid}: tabella comunicativa con {len(t['items'])} item (minimo 6)")
        for item in t["items"]:
            if not item["label"].strip():
                errors.append(f"{tid}: etichetta vuota")
            if item["concept"] is not None and item["concept"] not in CONCEPT_IDS:
                errors.append(f"{tid}: concetto inesistente {item['concept']}")
        if len(t["fields"]) < 2:
            errors.append(f"{tid}: campi insufficienti")
        for field in t["fields"]:
            if not all(k in field for k in ("key", "label", "kind", "default_value")):
                errors.append(f"{tid}: campo incompleto {field}")
        g = t["grid"]
        if g["columns"] * g["rows"] < len(t["items"]):
            errors.append(f"{tid}: griglia troppo piccola")
        if t["orientation"] not in {"verticale", "orizzontale"}:
            errors.append(f"{tid}: orientamento non valido")
        if t["complexity"] not in {"semplice", "intermedia", "articolata"}:
            errors.append(f"{tid}: complessità non valida")

    if counts["Materiali ABA e supporti visivi"]:
        pass
    return errors


def write_all(templates: list[dict]) -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    # Rimuove TUTTI i json, compresi eventuali orfani, poi riscrive il set esatto.
    for old in sorted(OUT_DIR.glob("*.json")):
        old.unlink()
    for t in templates:
        path = OUT_DIR / f"{t['id']}.json"
        text = json.dumps(t, ensure_ascii=False, indent=2) + "\n"
        path.write_text(text, encoding="utf-8")


def main() -> int:
    templates = sorted(TEMPLATES, key=sort_key)
    errors = validate(templates)
    if errors:
        for err in errors:
            print(f"ERRORE: {err}", file=sys.stderr)
        return 1

    write_all(templates)

    counts = Counter(t["category"] for t in templates)
    total_items = sum(len(t["items"]) for t in templates)
    null_items = sum(1 for t in templates for i in t["items"] if i["concept"] is None)
    files = sorted(OUT_DIR.glob("*.json"))
    print(f"Modelli scritti: {len(files)} in {OUT_DIR.relative_to(ROOT)}")
    print(f"Item totali: {total_items}; con concept nullo: {null_items}")
    for category in CATEGORY_ORDER:
        print(f"  {category}: {counts[category]}")
    print("Validazione: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
