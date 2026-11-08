#!/usr/bin/env python3
"""Genera le specifiche sorgente della biblioteca universale.

Legge il contenuto originale dei 300 modelli e dei 30 sistemi comunicativi e
produce specifiche JSON versionate in `templates/sources/`. Le specifiche non
sono il formato della biblioteca: `caa-cli template build-all` le converte in
documenti nativi `.caadoc` con il motore universale.
"""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TEMPLATES = ROOT / "resources/templates/library"
BOARDS = ROOT / "resources/boards"
SOURCES = ROOT / "templates/sources"
TPL_OUT = SOURCES / "templates"
BOARD_OUT = SOURCES / "communication-systems"

CATEGORY_KIND = {
    "Tabelle comunicative": ("communication_grid", "grid", "append", True),
    "Agende e calendari visivi": ("visual_schedule", "grid", "speak", True),
    "Sequenze e routine": ("visual_sequence", "flow", "speak", False),
    "Storie sociali": ("social_story", "flow", "speak", False),
    "Materiali TEACCH": ("teacch", "grid", "append", False),
    "Materiali ABA e supporti visivi": ("aba", "grid", "speak", False),
    "Materiali didattici": ("teaching", "flow", "speak", False),
    "Emozioni e autoregolazione": ("emotions", "grid", "speak", False),
    "Autonomia personale": ("autonomy", "flow", "speak", False),
    "Giochi e attività interattive": ("activity", "absolute", "speak", True),
}


def slug(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-") or "altro"


def page_id(value: str) -> str:
    return slug(value)[:40] or "pagina"


def split_chunks(items: list, count: int) -> list[list]:
    if count <= 1:
        return [items]
    size = max(1, (len(items) + count - 1) // count)
    return [items[index:index + size] for index in range(0, len(items), size)]


def base_spec(template: dict) -> tuple[dict, str, str]:
    category = template.get("category", "Materiali didattici")
    kind, layout, default_action, interactive = CATEGORY_KIND.get(
        category, ("teaching", "flow", "speak", False)
    )
    if template.get("interactive"):
        interactive = True
    spec = {
        "id": template["id"],
        "title": template["name"],
        "description": template.get("description", ""),
        "category": category,
        "source_type": "template",
        "kind": kind,
        "interactive": interactive,
        "license": template.get("license", "CC BY-NC-SA 4.0"),
        "page_size": "A4",
        "orientation": (
            "landscape"
            if layout == "absolute"
            or (layout != "flow" and template.get("orientation") == "orizzontale")
            else "portrait"
        ),
        "pages": [],
    }
    return spec, layout, default_action


TOKEN_WORDS = {"tre": 3, "cinque": 5, "dieci": 10}


def build_token_board_spec(template: dict) -> dict:
    """Una token board reale: gettoni, obiettivo, attività e rinforzo.

    La vecchia specifica usava le parole del modello come celle, producendo
    tre caselle di testo: non era una token board. Il numero di gettoni viene
    dal titolo (tre, cinque, dieci), il resto è una struttura editoriale.
    """
    count = 5
    for word, value in TOKEN_WORDS.items():
        if word in template["name"].lower():
            count = value
            break
    title = template["name"]
    elements = [
        {
            "id": f"{template['id']}-badge",
            "kind": "badge",
            "text": "SUPPORTI",
            "x_mm": 10,
            "y_mm": 8,
            "w_mm": 30,
            "h_mm": 7,
            "style": {"font_size": 3.1, "color": "#ffffff", "background": "#0f766e", "border_radius": 2, "align": "center", "valign": "middle", "bold": True},
        },
        {
            "id": f"{template['id']}-title",
            "kind": "title",
            "text": title,
            "x_mm": 10,
            "y_mm": 17,
            "w_mm": 190,
            "h_mm": 12,
            "style": {"font_size": 8.4, "bold": True, "valign": "middle"},
        },
        {
            "id": f"{template['id']}-instr",
            "kind": "instruction",
            "text": f"Tocca un gettone quando completi un'attività. Quando ho {count} gettoni scelgo il rinforzo.",
            "x_mm": 10,
            "y_mm": 32,
            "w_mm": 190,
            "h_mm": 8,
            "style": {"font_size": 4.2, "color": "#0f766e", "bold": True},
        },
        {
            "id": f"{template['id']}-goal",
            "kind": "text_box",
            "text": f"Obiettivo: completare {count} attività concordate.",
            "x_mm": 10,
            "y_mm": 42,
            "w_mm": 190,
            "h_mm": 12,
            "style": {"font_size": 4.5, "background": "#ecfdf5", "border_color": "#0f766e", "border_width": 0.5, "border_radius": 2.5, "padding_mm": 2, "valign": "middle", "bold": True},
        },
    ]
    columns = 5 if count > 5 else count
    rows = 2 if count > 5 else 1
    slot_w = 32
    gap = 4
    total = columns * slot_w + (columns - 1) * gap
    start_x = (210 - total) / 2
    for index in range(count):
        row = index // columns
        column = index % columns
        elements.append({
            "id": f"{template['id']}-token-{index + 1}",
            "kind": "token_slot",
            "text": "",
            "x_mm": start_x + column * (slot_w + gap),
            "y_mm": 62 + row * (slot_w + gap),
            "w_mm": slot_w,
            "h_mm": slot_w,
            "action": {"type": "toggle"},
            "spoken_text": "gettone",
            "metadata": {"filled": False, "index": index + 1},
            "style": {"font_size": 3.2, "border_color": "#0f766e", "border_width": 0.8, "align": "center", "valign": "middle"},
        })
    reward_y = 62 + rows * (slot_w + gap) + 8
    elements.append({
        "id": f"{template['id']}-reward",
        "kind": "text_box",
        "text": "Il rinforzo si sceglie insieme e può cambiare: non è un premio per «essere bravo».",
        "x_mm": 10,
        "y_mm": reward_y,
        "w_mm": 190,
        "h_mm": 18,
        "style": {"font_size": 3.8, "background": "#fffbeb", "border_color": "#f59e0b", "border_width": 0.5, "border_radius": 2.5, "padding_mm": 2, "valign": "middle"},
    })
    activities = ["leggere", "scrivere", "disegnare", "controllare", "ordinare"][:count]
    for index, label in enumerate(activities[:count]):
        x = 10 + index * (190 / max(1, min(count, 5)))
        width = (190 / max(1, min(count, 5))) - 4
        elements.append({
            "id": f"{template['id']}-act-{index + 1}",
            "kind": "symbol",
            "text": label,
            "symbol": label,
            "x_mm": x,
            "y_mm": reward_y + 28,
            "w_mm": width,
            "h_mm": 30,
            "style": {"font_size": 3.1, "align": "center", "valign": "bottom"},
            "action": {"type": "speak"},
        })
        elements.append({
            "id": f"{template['id']}-check-{index + 1}",
            "kind": "checkbox",
            "text": "",
            "x_mm": x + width / 2 - 4,
            "y_mm": reward_y + 62,
            "w_mm": 8,
            "h_mm": 8,
            "style": {"font_size": 3.0, "align": "center"},
        })
    spec = {
        "id": template["id"],
        "title": title,
        "description": template.get("description", ""),
        "category": "Materiali ABA e supporti visivi",
        "source_type": "template",
        "kind": "aba",
        "interactive": True,
        "license": template.get("license", "CC BY-NC-SA 4.0"),
        "page_size": "A4",
        "orientation": "portrait",
        "pages": [{
            "id": "home",
            "title": title,
            "layout": "absolute",
            "home": True,
            "elements": elements,
        }],
    }
    return spec


def worksheet_style(size=4.3, bold=False, italic=False, align="left", color="#1f2937", background="", border_color="", border_width=0.0, radius=2.0):
    return {
        "font_family": "Arial",
        "font_size": size,
        "bold": bold,
        "italic": italic,
        "align": align,
        "valign": "top",
        "color": color,
        "background": background,
        "border_color": border_color,
        "border_width": border_width,
        "border_radius": radius,
        "padding_mm": 1.2,
    }


def worksheet_title(template: dict, subject: str, instruction_text: str) -> list[dict]:
    return [
        {
            "id": f"{template['id']}-badge",
            "kind": "badge",
            "text": subject.upper(),
            "x_mm": 10,
            "y_mm": 8,
            "w_mm": 34,
            "h_mm": 7,
            "style": worksheet_style(3.0, True, align="center", color="#ffffff", background="#0f766e", radius=2.0),
        },
        {
            "id": f"{template['id']}-title",
            "kind": "title",
            "text": template["name"],
            "x_mm": 10,
            "y_mm": 17,
            "w_mm": 190,
            "h_mm": 12,
            "style": worksheet_style(8.4, True),
        },
        {
            "id": f"{template['id']}-sub",
            "kind": "subtitle",
            "text": "Scheda operativa · materiale modificabile",
            "x_mm": 10,
            "y_mm": 30,
            "w_mm": 190,
            "h_mm": 6,
            "style": worksheet_style(3.9, color="#475569"),
        },
        {
            "id": f"{template['id']}-instr",
            "kind": "instruction",
            "text": instruction_text,
            "x_mm": 10,
            "y_mm": 40,
            "w_mm": 190,
            "h_mm": 10,
            "style": worksheet_style(4.2, True, color="#0f766e"),
        },
    ]


def math_operations(title: str) -> tuple[str, list[tuple[int, int, int]]]:
    lower = title.lower()
    limit = 20 if "20" in lower else 100 if "100" in lower else 10
    if "sottrazioni" in lower:
        ops = []
        for a, b in [(5, 2), (8, 3), (10, 4), (7, 5), (9, 6), (6, 2), (8, 1), (10, 7)]:
            a = min(limit, a)
            b = min(a, b)
            ops.append((a, b, a - b))
        return "Completa le sottrazioni. Conta all'indietro se ti serve.", ops
    if "moltiplicazioni" in lower:
        ops = [(2, 1, 2), (2, 2, 4), (2, 3, 6), (5, 2, 10), (5, 3, 15), (10, 1, 10), (10, 2, 20), (10, 4, 40)]
        return "Completa le moltiplicazioni con la tavola del 2, del 5 e del 10.", ops
    if "divisioni" in lower:
        ops = [(4, 2, 2), (6, 2, 3), (8, 2, 4), (10, 2, 5), (10, 5, 2), (15, 5, 3), (20, 5, 4), (20, 10, 2)]
        return "Completa le divisioni. Aiutati con le schieramenti o con i gettoni.", ops
    if "confront" in lower or "maggiore" in lower or "minore" in lower:
        ops = [(3, 5, 0), (7, 2, 0), (4, 4, 0), (8, 6, 0), (2, 9, 0), (10, 10, 0), (5, 1, 0), (6, 3, 0)]
        return "Confronta i numeri: scrivi >, < oppure = nella casella.", ops
    ops = []
    for a, b in [(2, 1), (3, 2), (4, 3), (1, 4), (5, 2), (6, 3), (2, 7), (8, 1)]:
        a = min(limit, a)
        b = min(limit - a, b) if limit - a >= b else max(0, limit - a)
        ops.append((a, b, a + b))
    return "Completa le addizioni. Puoi usare la linea dei numeri o i gettoni.", ops


def build_worksheet_spec(template: dict) -> dict:
    """Trasforma un vecchio elenco di parole in una scheda con esercizio e soluzioni."""
    title = template["name"]
    lower = title.lower()
    if "matematica" in lower or any(
        word in lower
        for word in ["addizion", "sottrazion", "moltiplic", "division", "numer", "contegg", "confront", "quantit", "forma", "geometr", "misur", "orologio", "denaro", "frazion"]
    ):
        subject = "MATEMATICA"
        instruction_text, operations = math_operations(title)
        elements = worksheet_title(template, subject, instruction_text)
        if any(word in lower for word in ["addizion", "sottrazion", "moltiplic", "division"]):
            for index, (a, b, result) in enumerate(operations):
                col = index % 2
                row = index // 2
                x = 10 + col * 96
                y = 60 + row * 30
                if "moltiplic" in lower:
                    symbol = "×"
                elif "division" in lower:
                    symbol = "÷"
                elif "sottrazion" in lower:
                    symbol = "-"
                else:
                    symbol = "+"
                elements.append({
                    "id": f"{template['id']}-op-{index}",
                    "kind": "question",
                    "text": f"{a} {symbol} {b} =",
                    "x_mm": x,
                    "y_mm": y,
                    "w_mm": 54,
                    "h_mm": 20,
                    "style": worksheet_style(7.6, True, align="right"),
                })
                elements.append({
                    "id": f"{template['id']}-ans-{index}",
                    "kind": "text_box",
                    "text": "",
                    "x_mm": x + 60,
                    "y_mm": y + 2,
                    "w_mm": 28,
                    "h_mm": 16,
                    "style": worksheet_style(7.0, align="center", border_color="#94a3b8", border_width=0.6, radius=2.0),
                })
            answers = [f"{a} {symbol} {b} = {result}" for a, b, result in operations]
        else:
            # Conteggi, confronti, forme e misure: una riga per risposta.
            items = template.get("items", [])[:8]
            for index, item in enumerate(items):
                y = 58 + index * 26
                elements.append({
                    "id": f"{template['id']}-prompt-{index}",
                    "kind": "question",
                    "text": item.get("label", f"Esercizio {index + 1}"),
                    "x_mm": 10,
                    "y_mm": y,
                    "w_mm": 110,
                    "h_mm": 12,
                    "style": worksheet_style(4.4, True),
                })
                elements.append({
                    "id": f"{template['id']}-line-{index}",
                    "kind": "answer_line",
                    "text": "",
                    "x_mm": 122,
                    "y_mm": y,
                    "w_mm": 72,
                    "h_mm": 14,
                    "style": worksheet_style(3.0),
                    "metadata": {"lines": 1},
                })
            answers = [item.get("label", "") for item in items]
        solutions = [
            {
                "id": f"{template['id']}-sol-title",
                "kind": "title",
                "text": "Soluzioni per il docente",
                "x_mm": 10,
                "y_mm": 18,
                "w_mm": 190,
                "h_mm": 12,
                "style": worksheet_style(7.4, True),
            },
            {
                "id": f"{template['id']}-sol-instr",
                "kind": "instruction",
                "text": "Risposte, obiettivo e indicazioni per l'adattamento.",
                "x_mm": 10,
                "y_mm": 32,
                "w_mm": 190,
                "h_mm": 8,
                "style": worksheet_style(3.8, color="#0f766e"),
            },
        ]
        for index, answer in enumerate(answers[:12]):
            solutions.append({
                "id": f"{template['id']}-sol-{index}",
                "kind": "text",
                "text": answer,
                "x_mm": 10 + (index % 2) * 96,
                "y_mm": 48 + (index // 2) * 16,
                "w_mm": 88,
                "h_mm": 12,
                "style": worksheet_style(4.2),
            })
        solutions.append({
            "id": f"{template['id']}-sol-note",
            "kind": "note",
            "text": "Obiettivo: consolidare il calcolo e il ragionamento matematico. Adattamento: ridurre gli esercizi, ingrandire il testo o fornire gettoni e linea dei numeri. Verificare le risposte prima della stampa.",
            "x_mm": 10,
            "y_mm": 190,
            "w_mm": 190,
            "h_mm": 34,
            "style": worksheet_style(3.5, italic=True, color="#475569"),
        })
        return {
            "id": template["id"],
            "title": title,
            "description": template.get("description", ""),
            "category": "Materiali didattici",
            "source_type": "template",
            "kind": "worksheet",
            "interactive": False,
            "license": template.get("license", "CC BY-NC-SA 4.0"),
            "page_size": "A4",
            "orientation": "portrait",
            "pages": [
                {"id": "home", "title": title, "layout": "absolute", "home": True, "elements": elements},
                {"id": "soluzioni", "title": "Soluzioni", "layout": "absolute", "elements": solutions},
            ],
        }

    # Scheda linguistica o disciplinare: immagine + parola + riga di scrittura.
    subject = "ITALIANO" if any(w in lower for w in ["vocal", "consonant", "sillab", "parol", "frase", "lettur", "comprension", "ortograf", "grammatic", "sinonim", "contrari"]) else "DISCIPLINA"
    instruction_text = "Osserva l'immagine, leggi la parola e ricopiala. Poi usala in una frase."
    if "sillab" in lower:
        instruction_text = "Leggi la parola, dividila in sillabe e ricopiala."
    elif "vocal" in lower:
        instruction_text = "Leggi la parola, cerchia la vocale iniziale e ricopiala."
    elif "giorni" in lower or "mesi" in lower:
        instruction_text = "Osserva, metti in ordine e ricopia la parola."
    elements = worksheet_title(template, subject, instruction_text)
    items = template.get("items", [])[:8]
    for index, item in enumerate(items):
        label = item.get("label", f"Parola {index + 1}")
        y = 56 + index * 27
        elements.append({
            "id": f"{template['id']}-img-{index}",
            "kind": "symbol",
            "text": "",
            "symbol": label,
            "x_mm": 10,
            "y_mm": y,
            "w_mm": 28,
            "h_mm": 24,
            "style": worksheet_style(2.8, align="center"),
        })
        elements.append({
            "id": f"{template['id']}-word-{index}",
            "kind": "text",
            "text": label,
            "x_mm": 42,
            "y_mm": y + 4,
            "w_mm": 46,
            "h_mm": 12,
            "style": worksheet_style(4.4, True),
        })
        elements.append({
            "id": f"{template['id']}-line-{index}",
            "kind": "answer_line",
            "text": "",
            "x_mm": 92,
            "y_mm": y + 2,
            "w_mm": 100,
            "h_mm": 18,
            "style": worksheet_style(3.0),
            "metadata": {"lines": 1},
        })
    solutions = [
        {
            "id": f"{template['id']}-sol-title",
            "kind": "title",
            "text": "Soluzioni e note per il docente",
            "x_mm": 10,
            "y_mm": 18,
            "w_mm": 190,
            "h_mm": 12,
            "style": worksheet_style(7.4, True),
        },
        {
            "id": f"{template['id']}-sol-list",
            "kind": "text",
            "text": "Parole della scheda: " + ", ".join(item.get("label", "") for item in items) + ".",
            "x_mm": 10,
            "y_mm": 38,
            "w_mm": 190,
            "h_mm": 40,
            "style": worksheet_style(4.2),
        },
        {
            "id": f"{template['id']}-sol-note",
            "kind": "note",
            "text": "Obiettivo: consolidare lettura, scrittura e uso del lessico. Adattamento: ridurre le parole, aumentare il carattere, fornire la prima lettera o leggere ad alta voce. Accettare risposte corrette anche con grafia non convenzionale.",
            "x_mm": 10,
            "y_mm": 90,
            "w_mm": 190,
            "h_mm": 34,
            "style": worksheet_style(3.5, italic=True, color="#475569"),
        },
    ]
    return {
        "id": template["id"],
        "title": title,
        "description": template.get("description", ""),
        "category": "Materiali didattici",
        "source_type": "template",
        "kind": "worksheet",
        "interactive": False,
        "license": template.get("license", "CC BY-NC-SA 4.0"),
        "page_size": "A4",
        "orientation": "portrait",
        "pages": [
            {"id": "home", "title": title, "layout": "absolute", "home": True, "elements": elements},
            {"id": "soluzioni", "title": "Soluzioni", "layout": "absolute", "elements": solutions},
        ],
    }


def build_social_story_spec(template: dict) -> dict:
    """Trasforma una vecchia storia di una pagina in una storia multipagina."""
    title = template["name"]
    raw_items = [item.get("label", "").strip() for item in template.get("items", []) if item.get("label", "").strip()]
    sentences = [sentence for sentence in raw_items if len(sentence.split()) >= 3]
    if not sentences:
        sentences = [
            "Questa storia parla di una situazione che può capitare.",
            "Posso osservare che cosa succede intorno a me.",
            "Posso chiedere aiuto a un adulto che conosco.",
            "Dopo un momento la situazione può cambiare.",
        ]
    pages = [
        {
            "id": "home",
            "title": "Copertina",
            "layout": "absolute",
            "home": True,
            "elements": [
                {
                    "id": f"{template['id']}-badge",
                    "kind": "badge",
                    "text": "STORIA",
                    "x_mm": 10,
                    "y_mm": 8,
                    "w_mm": 24,
                    "h_mm": 7,
                    "style": worksheet_style(3.0, True, align="center", color="#ffffff", background="#0f766e", radius=2.0),
                },
                {
                    "id": f"{template['id']}-title",
                    "kind": "title",
                    "text": title,
                    "x_mm": 10,
                    "y_mm": 24,
                    "w_mm": 190,
                    "h_mm": 32,
                    "style": worksheet_style(9.0, True),
                },
                {
                    "id": f"{template['id']}-desc",
                    "kind": "text_box",
                    "text": template.get("description", "Una storia da leggere una frase alla volta."),
                    "x_mm": 10,
                    "y_mm": 70,
                    "w_mm": 190,
                    "h_mm": 50,
                    "style": worksheet_style(4.4, background="#ecfdf5", border_color="#0f766e", border_width=0.4, radius=2.5),
                },
                {
                    "id": f"{template['id']}-note",
                    "kind": "note",
                    "text": "Storia descrittiva e di anticipazione. Non è una Social Story certificata secondo i criteri 10.4: la revisione dell'adulto è necessaria. Adattare le frasi alla persona.",
                    "x_mm": 10,
                    "y_mm": 150,
                    "w_mm": 190,
                    "h_mm": 40,
                    "style": worksheet_style(3.5, italic=True, color="#475569"),
                },
            ],
        }
    ]
    chunk_size = 2
    for index in range(0, len(sentences), chunk_size):
        chunk = sentences[index:index + chunk_size]
        page_number = len(pages) + 1
        elements = [
            {
                "id": f"{template['id']}-p{page_number}-title",
                "kind": "subtitle",
                "text": f"Parte {page_number}",
                "x_mm": 10,
                "y_mm": 12,
                "w_mm": 190,
                "h_mm": 8,
                "style": worksheet_style(4.0, color="#475569"),
            }
        ]
        for row, sentence in enumerate(chunk):
            y = 32 + row * 92
            elements.append({
                "id": f"{template['id']}-p{page_number}-s{row}",
                "kind": "text_box",
                "text": sentence,
                "x_mm": 10,
                "y_mm": y,
                "w_mm": 190,
                "h_mm": 70,
                "style": worksheet_style(5.2, background="#f8fafc", border_color="#94a3b8", border_width=0.4, radius=2.5, align="left"),
            })
        pages.append({
            "id": f"pagina-{page_number}",
            "title": f"Parte {page_number}",
            "layout": "absolute",
            "elements": elements,
        })
    pages.append({
        "id": "nota-adulto",
        "title": "Nota per l'adulto",
        "layout": "absolute",
        "elements": [
            {
                "id": f"{template['id']}-adult-title",
                "kind": "title",
                "text": "Nota per l'adulto",
                "x_mm": 10,
                "y_mm": 18,
                "w_mm": 190,
                "h_mm": 12,
                "style": worksheet_style(7.4, True),
            },
            {
                "id": f"{template['id']}-adult-text",
                "kind": "text_box",
                "text": "Leggere una frase alla volta, lasciare spazio alle domande e verificare che la storia sia comprensibile. Non presentare il testo come una prescrizione: la persona può avere reazioni diverse. Se la storia non aiuta, provare una sequenza visiva o una conversazione con un adulto.",
                "x_mm": 10,
                "y_mm": 38,
                "w_mm": 190,
                "h_mm": 80,
                "style": worksheet_style(4.2, background="#fffbeb", border_color="#f59e0b", border_width=0.5, radius=2.5),
            },
        ],
    })
    return {
        "id": template["id"],
        "title": title,
        "description": template.get("description", ""),
        "category": "Storie sociali",
        "source_type": "template",
        "kind": "social_story",
        "interactive": False,
        "license": template.get("license", "CC BY-NC-SA 4.0"),
        "page_size": "A4",
        "orientation": "portrait",
        "pages": pages,
    }


def build_template_spec(template: dict) -> dict:
    if template.get("id", "").startswith("lib_aba_token"):
        return build_token_board_spec(template)
    if template.get("category") == "Materiali didattici":
        return build_worksheet_spec(template)
    if template.get("category") == "Storie sociali":
        return build_social_story_spec(template)
    spec, layout, default_action = base_spec(template)
    items = template.get("items", [])
    grid = template.get("grid") or {}
    columns = int(grid.get("columns") or 3)
    rows = int(grid.get("rows") or max(1, (len(items) + columns - 1) // columns))
    desired_pages = max(1, int(template.get("pages") or 1))
    if layout == "flow":
        # Un elemento simbolo in flusso occupa circa 28 mm: evita pagine
        # sovraccariche e testi tagliati già in generazione.
        desired_pages = max(desired_pages, (len(items) + 5) // 6)
    chunks = split_chunks(items, desired_pages)

    if layout == "absolute":
        # Giochi e attività: una home con pulsanti e una pagina per attività.
        home = {
            "id": "home",
            "title": template["name"],
            "layout": "absolute",
            "home": True,
            "elements": [
                {
                    "id": f"{template['id']}-title",
                    "kind": "text",
                    "text": template["name"],
                    "x_mm": 15,
                    "y_mm": 14,
                    "w_mm": 260,
                    "h_mm": 20,
                    "style": {"font_size": 9, "bold": True, "align": "center"},
                }
            ],
        }
        for index, item in enumerate(items[:8]):
            home["elements"].append({
                "id": f"{template['id']}-btn-{index + 1}",
                "kind": "button",
                "text": item.get("label", f"Attività {index + 1}"),
                "symbol": item.get("label", ""),
                "concept": item.get("concept"),
                "x_mm": 18 + (index % 3) * 88,
                "y_mm": 42 + (index // 3) * 52,
                "w_mm": 78,
                "h_mm": 40,
                "action": {"type": "navigate", "target": f"activity-{index + 1}"},
                "style": {"font_size": 5, "border_width": 1, "border_color": "#456", "align": "center"},
            })
        spec["pages"].append(home)
        for index, item in enumerate(items[:8]):
            spec["pages"].append({
                "id": f"activity-{index + 1}",
                "title": item.get("label", f"Attività {index + 1}"),
                "layout": "absolute",
                "elements": [
                    {
                        "id": f"{template['id']}-a{index + 1}-title",
                        "kind": "text",
                        "text": item.get("label", f"Attività {index + 1}"),
                        "x_mm": 20,
                        "y_mm": 12,
                        "w_mm": 250,
                        "h_mm": 20,
                        "style": {"font_size": 9, "bold": True, "align": "center"},
                    },
                    {
                        "id": f"{template['id']}-a{index + 1}-symbol",
                        "kind": "symbol",
                        "text": item.get("label", ""),
                        "symbol": item.get("label", ""),
                        "concept": item.get("concept"),
                        "x_mm": 80,
                        "y_mm": 50,
                        "w_mm": 130,
                        "h_mm": 112,
                        "action": {"type": "speak"},
                    },
                    {
                        "id": f"{template['id']}-a{index + 1}-home",
                        "kind": "button",
                        "text": "Home",
                        "x_mm": 20,
                        "y_mm": 178,
                        "w_mm": 55,
                        "h_mm": 14,
                        "action": {"type": "home"},
                    },
                ],
            })
        return spec

    for page_index, chunk in enumerate(chunks):
        page = {
            "id": "home" if page_index == 0 else f"pagina-{page_index + 1}",
            "title": template["name"] if page_index == 0 else f"{template['name']} · {page_index + 1}",
            "layout": "grid" if layout == "grid" else "flow",
            "home": page_index == 0,
            "elements": [],
        }
        if layout == "grid":
            page["grid"] = {
                "columns": columns,
                "rows": max(rows, (len(chunk) + columns - 1) // columns or 1),
                "gap_mm": 3,
            }
            for index, item in enumerate(chunk):
                page["elements"].append({
                    "id": f"{template['id']}-p{page_index + 1}-{index + 1}",
                    "kind": "symbol",
                    "text": item.get("label", ""),
                    "symbol": item.get("label", ""),
                    "concept": item.get("concept"),
                    "spoken_text": item.get("label", ""),
                    "action": {"type": default_action},
                    "style": {"font_size": 4.2, "align": "center", "border_width": 1, "border_color": "#5b6575"},
                })
        else:
            page["flow"] = {"columns": 1, "gap_mm": 3}
            page["elements"].append({
                "id": f"{template['id']}-p{page_index + 1}-title",
                "kind": "text",
                "text": template["name"] if page_index == 0 else f"{template['name']} · {page_index + 1}",
                "style": {"font_size": 8, "bold": True},
            })
            for field in template.get("fields", []):
                if field.get("kind") == "text" and field.get("default_value"):
                    page["elements"].append({
                        "id": f"{template['id']}-p{page_index + 1}-field-{slug(field['key'])}",
                        "kind": "text",
                        "text": field["default_value"],
                        "style": {"font_size": 4.6, "italic": field["key"] == "instructions"},
                    })
            for index, item in enumerate(chunk):
                page["elements"].append({
                    "id": f"{template['id']}-p{page_index + 1}-{index + 1}",
                    "kind": "symbol",
                    "text": item.get("label", ""),
                    "symbol": item.get("label", ""),
                    "concept": item.get("concept"),
                    "spoken_text": item.get("label", ""),
                    "action": {"type": default_action},
                    "style": {"font_size": 4.6},
                })
        spec["pages"].append(page)
    return spec


def build_board_spec(board: dict) -> dict:
    spec = {
        "id": board["id"],
        "title": board["title"],
        "description": board.get("description", ""),
        "category": "Sistemi comunicativi",
        "source_type": "board",
        "kind": "communication_board",
        "interactive": True,
        "license": "CC BY-NC-SA 4.0",
        "page_size": "A4",
        "orientation": "portrait",
        "pages": [],
    }
    for page in board.get("pages", []):
        cells = page.get("cells", [])
        columns = 4
        rows = max(1, (len(cells) + columns - 1) // columns)
        elements = []
        for index, cell in enumerate(cells):
            action = cell.get("action") or {}
            target = action.get("targetPageId") or action.get("target_page")
            mapped = {
                "type": action.get("type", "append"),
            }
            if target:
                mapped["target"] = target
            elements.append({
                "id": cell.get("id", f"c{index + 1}"),
                "kind": "symbol",
                "text": cell.get("label", ""),
                # Le etichette `symbolLabel` storiche contenevano slug artificiali
                # (es. momenti_home): per i documenti nativi si usa il testo
                # visibile, che è la parola realmente cercata nel repertorio.
                "symbol": (cell.get("label") or cell.get("symbolLabel", "")).replace("_", " "),
                "spoken_text": cell.get("spokenText") or cell.get("label", ""),
                "action": mapped,
                "style": {"font_size": 4.2, "align": "center", "border_width": 1, "border_color": "#4b5563"},
            })
        spec["pages"].append({
            "id": page["id"],
            "title": page.get("title", page["id"]),
            "layout": "grid",
            "home": page["id"] == board.get("rootPageId"),
            "grid": {"columns": columns, "rows": rows, "gap_mm": 3},
            "elements": elements,
        })
    return spec


def dedupe_titles(spec: dict) -> None:
    """Rimuove il titolo ripetuto due volte nella stessa pagina.

    I vecchi modelli a flusso aggiungevano un titolo grande e un campo con lo
    stesso testo; non serve a nulla e il validatore lo segnala. Si mantiene la
    prima occorrenza.
    """
    title = spec.get("title", "").strip().lower()
    if not title:
        return
    for page in spec.get("pages", []):
        seen = False
        kept = []
        for element in page.get("elements", []):
            if element.get("kind") == "text" and element.get("text", "").strip().lower() == title:
                if seen:
                    continue
                seen = True
            kept.append(element)
        page["elements"] = kept


def main() -> int:
    TEMPLATES.mkdir(parents=True, exist_ok=True)
    BOARDS.mkdir(parents=True, exist_ok=True)
    TPL_OUT.mkdir(parents=True, exist_ok=True)
    BOARD_OUT.mkdir(parents=True, exist_ok=True)
    # Rimuove solo le specifiche generate: le fonti manuali eventuali restano.
    for path in TPL_OUT.rglob("*.json"):
        # Le specifiche pilota sono manutenute a mano dal design system e non
        # vengono rigenerate da questo script.
        if "piloti" in path.parts:
            continue
        path.unlink()
    for path in BOARD_OUT.glob("*.json"):
        path.unlink()

    counts = {}
    seen = set()
    for path in sorted(TEMPLATES.glob("*.json")):
        template = json.loads(path.read_text())
        if template["id"] in seen:
            raise SystemExit(f"ID duplicato: {template['id']}")
        seen.add(template["id"])
        spec = build_template_spec(template)
        dedupe_titles(spec)
        folder = TPL_OUT / slug(spec["category"])
        folder.mkdir(parents=True, exist_ok=True)
        (folder / f"{spec['id']}.json").write_text(
            json.dumps(spec, ensure_ascii=False, indent=2) + "\n"
        )
        counts[spec["category"]] = counts.get(spec["category"], 0) + 1

    board_count = 0
    for path in sorted(BOARDS.glob("*.json")):
        board = json.loads(path.read_text())
        if board["id"] in seen:
            raise SystemExit(f"ID duplicato: {board['id']}")
        seen.add(board["id"])
        spec = build_board_spec(board)
        (BOARD_OUT / f"{spec['id']}.json").write_text(
            json.dumps(spec, ensure_ascii=False, indent=2) + "\n"
        )
        board_count += 1

    manifest = {
        "version": 1,
        "templates": sum(counts.values()),
        "boards": board_count,
        "categories": counts,
        "note": "Specifiche sorgente; i documenti nativi sono generati da caa-cli template build-all.",
    }
    (SOURCES / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n"
    )
    print(json.dumps(manifest, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
