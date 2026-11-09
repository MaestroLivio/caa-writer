#!/usr/bin/env python3
"""Genera materiali pronti da stampare, plastificare e usare con il velcro,
fumetti CAA completi e supporti ABA/TEACCH strutturati.

I modelli sono specifiche JSON versionate: `caa-cli template build-all`
produce i `.caadoc` modificabili e le anteprime. Ogni elemento resta
modificabile nell'editor; le griglie delle frasi hanno righe e colonne
modificabili dal pannello.
"""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCES = ROOT / "templates" / "sources" / "templates"

BADGE_COLORS = {
    "ABA": "#7c3aed",
    "TEACCH": "#0f766e",
    "VELCRO": "#b45309",
    "FUMETTO": "#be123c",
}

SYMBOL_LEMMA = {
    "stai": "stare",
    "sto": "stare",
    "andiamo": "andare",
    "ho": "avere",
    "sono": "essere",
    "è": "essere",
    "vuoi": "volere",
    "voglio": "volere",
    "prendo": "prendere",
    "giochiamo": "giocare",
    "aiuti": "aiutare",
    "serve": "servire",
    "posso": "potere",
    "devo": "dovere",
    "fammi": "fare",
    "dammi": "dare",
    "mangio": "mangiare",
    "bevo": "bere",
    "dormo": "dormire",
    "lavo": "lavare",
}


def text_element(element_id, kind, text, x, y, w, h, font=3.5, color="#1f2937", *,
                 bold=False, italic=False, align="left", valign="middle",
                 background="", border_color="", border_width=0.0,
                 border_style="solid", radius=2.0, absolute=True, padding=1.2,
                 action=None):
    element = {
        "id": element_id,
        "kind": kind,
        "text": text,
        "x_mm": round(x, 2),
        "y_mm": round(y, 2),
        "w_mm": round(w, 2),
        "h_mm": round(h, 2),
        "style": {
            "font_family": "Arial",
            "font_size": font,
            "color": color,
            "background": background,
            "border_color": border_color,
            "border_width": border_width,
            "border_style": border_style,
            "border_radius": radius,
            "padding_mm": padding,
            "align": align,
            "valign": valign,
            "bold": bold,
            "italic": italic,
            "underline": False,
        },
        "absolute": absolute,
    }
    if action:
        element["action"] = action
    return element


def header_elements(title, badge, instruction, width, margin=10):
    color = BADGE_COLORS.get(badge, "#0f766e")
    usable = width - 2 * margin - 2
    return [
        text_element("hdr-badge", "badge", badge, margin, 8, 46, 7, 3.1, "#ffffff",
                     bold=True, align="center", background=color, radius=2.0),
        text_element("hdr-title", "title", title, margin, 17, usable, 12, 8.2, bold=True),
        text_element("hdr-sub", "subtitle", instruction, margin, 31, usable, 8, 3.9,
                     "#475569", valign="top"),
    ]


def footer_element(width, height, margin=10):
    return text_element(
        "footer", "text",
        "CAA Writer · CC BY-NC-SA 4.0 · materiale modificabile",
        margin, height - margin - 5, width - 2 * margin - 2, 4, 2.8, "#475569", valign="top",
    )


def card(element_id, label, x, y, w, h, font=3.3, symbol=None, fill="#ffffff",
         border="#475569", dashed=True, radius=2.5):
    return {
        "id": element_id,
        "kind": "symbol",
        "text": label,
        "spoken_text": label,
        "symbol": symbol or label,
        "x_mm": round(x, 2),
        "y_mm": round(y, 2),
        "w_mm": round(w, 2),
        "h_mm": round(h, 2),
        "style": {
            "font_family": "Arial",
            "font_size": font,
            "color": "#1f2937",
            "background": fill,
            "border_color": border,
            "border_width": 0.6,
            "border_style": "dashed" if dashed else "solid",
            "border_radius": radius,
            "padding_mm": 1.4,
            "align": "center",
            "valign": "bottom",
            "bold": False,
            "italic": False,
            "underline": False,
        },
        "absolute": True,
        "action": {"type": "speak"},
    }


def slot(element_id, label, x, y, w, h, fill="#f8fafc"):
    return text_element(
        element_id, "shape", label, x, y, w, h, 4.2, "#475569",
        align="center", valign="middle", background=fill, border_color="#94a3b8",
        border_width=0.6, border_style="dashed", radius=3.0,
    )


def phrase_cells(phrase):
    cells = []
    for raw in phrase.split():
        if raw in {"?", "!", ".", ",", "…"}:
            if cells:
                cells[-1]["text"] = cells[-1]["text"] + raw
            continue
        punctuation = ""
        word = raw
        while word and word[-1] in "?!.,;:":
            punctuation = word[-1] + punctuation
            word = word[:-1]
        if not word:
            continue
        symbol = SYMBOL_LEMMA.get(word.lower(), word)
        cells.append({
            "text": word + punctuation,
            "spoken_text": word,
            "symbol": symbol,
            "style": {
                "font_size": 3.0,
                "background": "transparent",
                "border_color": "transparent",
                "border_width": 0.0,
                "border_style": "none",
                "align": "center",
                "valign": "middle",
            },
        })
    return cells


def speech(element_id, x, y, w, h, phrase, fill="#ffffff", border="#be123c", columns=None):
    cells = phrase_cells(phrase)
    return {
        "id": element_id,
        "kind": "speech",
        "text": phrase,
        "spoken_text": phrase,
        "x_mm": round(x, 2),
        "y_mm": round(y, 2),
        "w_mm": round(w, 2),
        "h_mm": round(h, 2),
        "columns": columns or max(1, len(cells)),
        "rows": 1,
        "gap_mm": 1.0,
        "cells": cells,
        "style": {
            "font_family": "Arial",
            "font_size": 3.2,
            "color": "#172033",
            "background": fill,
            "border_color": border,
            "border_width": 0.9,
            "border_style": "solid",
            "border_radius": 7.0,
            "padding_mm": 2.0,
            "align": "center",
            "valign": "middle",
            "bold": False,
            "italic": False,
            "underline": False,
        },
        "absolute": True,
    }


def character(element_id, symbol, x, y, w, h, label=None):
    return {
        "id": element_id,
        "kind": "symbol",
        "text": label or symbol,
        "spoken_text": label or symbol,
        "symbol": symbol,
        "x_mm": round(x, 2),
        "y_mm": round(y, 2),
        "w_mm": round(w, 2),
        "h_mm": round(h, 2),
        "style": {
            "font_family": "Arial",
            "font_size": 3.0,
            "color": "#1f2937",
            "background": "",
            "border_color": "",
            "border_width": 0.0,
            "border_style": "solid",
            "border_radius": 2.0,
            "padding_mm": 0.5,
            "align": "center",
            "valign": "bottom",
            "bold": False,
            "italic": False,
            "underline": False,
        },
        "absolute": True,
    }


def template(identifier, title, description, category, badge, instruction, width, height,
             elements, taxonomy, orientation="landscape"):
    return {
        "id": identifier,
        "title": title,
        "description": description,
        "category": category,
        "source_type": "template",
        "kind": (
            "printable" if category == "Materiali stampabili"
            else "teacch" if category == "Materiali TEACCH"
            else "aba"
        ),
        "interactive": False,
        "license": "CC BY-NC-SA 4.0",
        "page_size": "A4",
        "orientation": orientation,
        "margin_mm": 10,
        "taxonomy": taxonomy,
        "pages": [{
            "id": "home",
            "title": title,
            "layout": "absolute",
            "home": True,
            "elements": header_elements(title, badge, instruction, width) + elements + [
                footer_element(width, height)
            ],
        }],
    }


def save(path: Path, spec: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(spec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


COMICS = [
    ("comic_ciao_come_stai", "Fumetto CAA: ciao, come stai?",
     "Tre vignette con dialogo e pittogrammi: saluta, chiedi come sta e rispondi.",
     [("Ciao!", "bambino"), ("Come stai?", "bambina"), ("Sto bene, grazie.", "bambino")]),
    ("comic_ho_fame", "Fumetto CAA: ho fame",
     "Dialogo per chiedere da mangiare e scegliere un alimento.",
     [("Ho fame.", "bambino"), ("Vuoi la mela?", "mamma"), ("Sì, grazie.", "bambino")]),
    ("comic_andiamo_parco", "Fumetto CAA: andiamo al parco",
     "Dialogo per proporre un'uscita e scegliere un gioco.",
     [("Andiamo al parco?", "bambina"), ("Sì! Prendo la palla.", "bambino"), ("Giochiamo insieme.", "bambina")]),
    ("comic_mi_aiuti", "Fumetto CAA: mi aiuti?",
     "Dialogo per chiedere aiuto e spiegare cosa serve.",
     [("Mi aiuti?", "bambino"), ("Certo, cosa serve?", "maestra"), ("Aprire il barattolo.", "bambino")]),
    ("comic_sono_felice", "Fumetto CAA: sono felice",
     "Dialogo sulle emozioni: riconoscere la felicità e spiegarne il motivo.",
     [("Oggi sono felice.", "bambina"), ("Perché sei felice?", "maestra"), ("Gioco con gli amici.", "bambina")]),
    ("comic_ora_di_dormire", "Fumetto CAA: è ora di dormire",
     "Routine della sera con richiesta, compromesso e buonanotte.",
     [("È ora di dormire.", "mamma"), ("Ancora cinque minuti?", "bambino"), ("Va bene, poi a letto.", "mamma")]),
]


def build_comics() -> None:
    for identifier, title, description, panels in COMICS:
        elements = []
        panel_w, panel_h = 91.0, 142.0
        xs = [10.0, 103.0, 196.0]
        colors = ["#fff1f2", "#eff6ff", "#f0fdf4"]
        for index, (phrase, symbol) in enumerate(panels):
            x = xs[index]
            elements.append(text_element(
                f"panel-{index + 1}", "shape", "", x, 46, panel_w, panel_h, 3.0,
                background=colors[index], border_color="#94a3b8", border_width=0.5,
                radius=3.0,
            ))
            elements.append(speech(
                f"bubble-{index + 1}", x + 5, 52, panel_w - 10, 38, phrase,
                fill="#ffffff", border="#be123c",
            ))
            elements.append(character(
                f"char-{index + 1}", symbol, x + panel_w / 2 - 18, 96, 36, 54,
                label=symbol,
            ))
            elements.append(text_element(
                f"caption-{index + 1}", "note", f"Vignetta {index + 1}", x, 174,
                panel_w, 5, 2.8, "#64748b", align="center",
            ))
        save(
            SOURCES / "materiali-stampabili" / f"{identifier}.json",
            template(
                identifier, title, description, "Materiali stampabili", "FUMETTO",
                "Leggi le vignette, indica i pittogrammi e recita il dialogo. Poi cambia le parole nelle celle.",
                297, 210, elements,
                {
                    "category_id": "stampabili",
                    "subcategory_id": "fumetti",
                    "subcategory": "Fumetti CAA",
                    "audience": "primaria",
                    "objective": description,
                    "tags": ["fumetto", "dialogo", "caa", "comunicazione", "storia"],
                    "complexity": "base",
                    "needs_symbols": True,
                    "printable": True,
                    "customizable": True,
                    "interactive": False,
                    "featured": True,
                },
            ),
        )


def build_velcro() -> None:
    # 1. Striscia delle frasi con banca delle parole.
    elements = [
        text_element("strip-title", "title", "La mia frase", 10, 44, 120, 9, 5.0, bold=True),
    ]
    for index in range(5):
        elements.append(slot(f"slot-{index + 1}", str(index + 1), 14 + index * 34, 56, 28, 28))
    bank = ["io", "voglio", "mangiare", "bere", "giocare", "palla", "mela", "acqua",
            "casa", "scuola", "mamma", "aiuto"]
    for index, label in enumerate(bank):
        column = index % 6
        row = index // 6
        elements.append(card(f"word-{index + 1}", label, 10 + column * 46, 98 + row * 44, 42, 38))
    elements.append(text_element(
        "velcro-note", "note",
        "Ritaglia le carte lungo le linee tratteggiate, plastifica e applica il velcro.",
        10, 186, 277, 6, 3.0, "#64748b", italic=True,
    ))
    save(SOURCES / "materiali-stampabili" / "velcro_striscia_frase.json", template(
        "velcro_striscia_frase", "Velcro: striscia delle frasi",
        "Striscia per comporre frasi con cinque spazi e dodici carte parole da ritagliare e plastificare.",
        "Materiali stampabili", "VELCRO",
        "Stampa, plastifica e applica il velcro. Costruisci la frase spostando le carte nella striscia.",
        297, 210, elements,
        {
            "category_id": "stampabili", "subcategory_id": "velcro",
            "subcategory": "Materiali con velcro", "audience": "primaria",
            "objective": "Comporre frasi con parole e pittogrammi usando una striscia velcro.",
            "tags": ["velcro", "frase", "striscia", "parole", "caa"],
            "complexity": "base", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": True,
        },
    ))

    # 2. Carte dei bisogni.
    needs = ["sete", "fame", "bagno", "riposo", "aiuto", "abbraccio", "caldo", "freddo",
             "silenzio", "luce", "paura", "felice"]
    elements = [text_element("cards-title", "title", "Carte dei bisogni", 10, 43, 150, 9, 5.0, bold=True)]
    for index, label in enumerate(needs):
        column = index % 4
        row = index // 4
        elements.append(card(f"need-{index + 1}", label, 10 + column * 70, 56 + row * 45, 64, 40))
    elements.append(text_element(
        "velcro-note", "note",
        "Ritaglia, plastifica e metti le carte a disposizione nel quaderno o sulla scrivania.",
        10, 190, 277, 6, 3.0, "#64748b", italic=True,
    ))
    save(SOURCES / "materiali-stampabili" / "velcro_carte_bisogni.json", template(
        "velcro_carte_bisogni", "Velcro: carte dei bisogni",
        "Dodici carte dei bisogni da ritagliare, plastificare e attaccare con il velcro.",
        "Materiali stampabili", "VELCRO",
        "Stampa, plastifica e applica il velcro. Usa le carte per chiedere o indicare un bisogno.",
        297, 210, elements,
        {
            "category_id": "stampabili", "subcategory_id": "velcro",
            "subcategory": "Materiali con velcro", "audience": "primaria",
            "objective": "Esprimere bisogni e stati fisici con carte velcro.",
            "tags": ["velcro", "bisogni", "carte", "comunicazione"],
            "complexity": "base", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": True,
        },
    ))

    # 3. Categorie: cibo, animali, giochi.
    elements = [
        text_element("cat-title", "title", "Categorie: cibo, animali, giochi", 10, 43, 230, 9, 4.8, bold=True),
    ]
    mats = ["cibo", "animali", "giochi"]
    for index, label in enumerate(mats):
        x = 10 + index * 94
        elements.append(text_element(f"mat-{index + 1}", "shape", label, x, 56, 86, 84, 4.6,
                                     align="center", valign="middle", background="#f8fafc",
                                     border_color="#0f766e", border_width=0.7, radius=3.0))
    items = [("mela", "cibo"), ("pane", "cibo"), ("acqua", "cibo"), ("cane", "animali"),
             ("gatto", "animali"), ("pesce", "animali"), ("palla", "giochi"),
             ("macchina", "giochi"), ("puzzle", "giochi"), ("banana", "cibo"),
             ("cavallo", "animali"), ("bambola", "giochi")]
    for index, (label, _) in enumerate(items):
        column = index % 6
        row = index // 6
        elements.append(card(f"cat-card-{index + 1}", label, 10 + column * 46, 148 + row * 26, 42, 23))
    save(SOURCES / "materiali-stampabili" / "velcro_categorie.json", template(
        "velcro_categorie", "Velcro: categorie da ordinare",
        "Tre tappetini per cibo, animali e giochi con dodici carte da ritagliare e attaccare.",
        "Materiali stampabili", "VELCRO",
        "Stampa, plastifica e applica il velcro. Ordina le carte nella categoria giusta e verbalizza la scelta.",
        297, 210, elements,
        {
            "category_id": "stampabili", "subcategory_id": "velcro",
            "subcategory": "Materiali con velcro", "audience": "primaria",
            "objective": "Classificare immagini per categoria con supporto velcro.",
            "tags": ["velcro", "categorie", "cibo", "animali", "giochi", "ordinare"],
            "complexity": "base", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": False,
        },
    ))

    # 4. Token board con gettoni.
    elements = [
        text_element("token-title", "title", "Token board: cinque gettoni", 10, 43, 200, 9, 5.0, bold=True),
        text_element("token-labels", "subtitle", "Un gettone per ogni passo. Quando la board è piena, arriva il premio.",
                     10, 54, 277, 6, 3.8, "#475569"),
    ]
    for index in range(5):
        elements.append(text_element(f"token-slot-{index + 1}", "token_slot", "", 14 + index * 38, 68, 32, 32))
    elements.append(text_element("reward-panel", "shape", "PREMIO", 210, 62, 70, 44, 4.0,
                                 align="center", valign="middle", background="#fef3c7",
                                 border_color="#b45309", border_width=0.7, radius=3.0))
    elements.append(card("reward", "giocare", 224, 74, 42, 26, 3.2))
    for index in range(5):
        elements.append(text_element(f"cut-token-{index + 1}", "token_slot", "", 10 + index * 22, 140, 18, 18))
    elements.append(text_element("velcro-note", "note",
                                 "Ritaglia i gettoni, plastificali e attaccali con il velcro. I gettoni si spostano dalla striscia alla board.",
                                 10, 176, 277, 8, 3.0, "#64748b", italic=True))
    save(SOURCES / "materiali-stampabili" / "velcro_token_board.json", template(
        "velcro_token_board", "Velcro: token board con gettoni",
        "Token economy con cinque gettoni, premio finale e gettoni da ritagliare.",
        "Materiali stampabili", "VELCRO",
        "Consegna un gettone dopo ogni risposta o passo. Quando la board è piena, mostra il premio scelto.",
        297, 210, elements,
        {
            "category_id": "stampabili", "subcategory_id": "velcro",
            "subcategory": "Materiali con velcro", "audience": "primaria",
            "objective": "Rinforzare i comportamenti con una token economy a cinque gettoni.",
            "tags": ["velcro", "token", "gettone", "rinforzo", "aba"],
            "complexity": "base", "needs_symbols": False, "printable": True,
            "customizable": True, "interactive": False, "featured": True,
        },
    ))

    # 5. Prima e poi con velcro.
    elements = [
        text_element("prima-title", "title", "PRIMA", 10, 47, 130, 11, 8.0, bold=True, align="center"),
        text_element("poi-title", "title", "POI", 157, 47, 130, 11, 8.0, bold=True, align="center"),
        slot("prima-slot", "attività", 20, 62, 110, 62),
        slot("poi-slot", "premio", 167, 62, 110, 62),
        text_element("arrow", "text", "→", 138, 80, 12, 20, 12.0, bold=True, align="center"),
    ]
    choices = ["lavorare", "leggere", "puzzle", "riposo", "giocare", "palla", "musica", "disegnare"]
    for index, label in enumerate(choices):
        column = index % 4
        row = index // 4
        elements.append(card(f"choice-{index + 1}", label, 10 + column * 70, 140 + row * 26, 64, 23))
    save(SOURCES / "materiali-stampabili" / "velcro_prima_poi.json", template(
        "velcro_prima_poi", "Velcro: prima e poi",
        "Board prima/poi con due spazi grandi e otto carte attività da ritagliare.",
        "Materiali stampabili", "VELCRO",
        "Mostra PRIMA e POI. Attacca l'attività richiesta e il rinforzo scelto; quando PRIMA è finita, passa a POI.",
        297, 210, elements,
        {
            "category_id": "stampabili", "subcategory_id": "velcro",
            "subcategory": "Materiali con velcro", "audience": "primaria",
            "objective": "Anticipare attività e rinforzo con una board prima/poi.",
            "tags": ["velcro", "prima", "poi", "rinforzo", "aba"],
            "complexity": "base", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": True,
        },
    ))

    # 6. Agenda della giornata con velcro.
    elements = [
        text_element("agenda-title", "title", "Agenda della giornata", 10, 43, 220, 9, 5.0, bold=True),
    ]
    times = ["mattina", "scuola", "pranzo", "pomeriggio", "merenda", "gioco", "cena", "letto"]
    for index, label in enumerate(times):
        y = 56 + index * 15
        elements.append(text_element(f"time-{index + 1}", "text", label, 10, y, 40, 13, 3.4,
                                     align="left", valign="middle"))
        elements.append(slot(f"slot-{index + 1}", "", 54, y, 120, 13, fill="#ffffff"))
    activities = ["svegliarsi", "colazione", "scuola", "pranzo", "giocare", "merenda",
                  "compiti", "cena", "dormire", "lavarsi", "vestirsi", "leggere"]
    for index, label in enumerate(activities):
        column = index % 4
        row = index // 4
        elements.append(card(f"act-{index + 1}", label, 188 + column * 25, 56 + row * 28, 22, 25, font=2.5))
    elements.append(text_element("velcro-note", "note",
                                 "Ritaglia le carte e attaccale con il velcro. Sposta la carta nell'orario quando l'attività è conclusa.",
                                 10, 190, 277, 7, 3.0, "#64748b", italic=True))
    save(SOURCES / "materiali-stampabili" / "velcro_agenda_giornata.json", template(
        "velcro_agenda_giornata", "Velcro: agenda della giornata",
        "Agenda verticale con otto momenti e dodici carte attività da ritagliare e plastificare.",
        "Materiali stampabili", "VELCRO",
        "Anticipa la giornata. Attacca la carta nel momento giusto e spostala quando l'attività è conclusa.",
        297, 210, elements,
        {
            "category_id": "stampabili", "subcategory_id": "velcro",
            "subcategory": "Materiali con velcro", "audience": "primaria",
            "objective": "Organizzare la giornata con un'agenda visiva velcro.",
            "tags": ["velcro", "agenda", "giornata", "routine", "tempo"],
            "complexity": "articolato", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": False,
        },
    ))

    # 7. Scelte: voglio...
    elements = [
        text_element("voglio-strip", "shape", "IO VOGLIO", 10, 47, 80, 22, 5.0, bold=True,
                     align="center", valign="middle", background="#eff6ff", border_color="#2463eb",
                     border_width=0.8, radius=3.0),
        slot("voglio-slot", "", 98, 47, 90, 22),
    ]
    choices = ["mela", "acqua", "pane", "giocare", "palla", "puzzle", "musica", "libro",
               "riposo", "aiuto", "bagno", "abbraccio"]
    for index, label in enumerate(choices):
        column = index % 4
        row = index // 4
        elements.append(card(f"choice-{index + 1}", label, 10 + column * 70, 84 + row * 32, 64, 28))
    elements.append(text_element("velcro-note", "note",
                                 "Ritaglia le carte, plastifica e applica il velcro. Componi «Io voglio…» e leggi la frase ad alta voce.",
                                 10, 190, 277, 7, 3.0, "#64748b", italic=True))
    save(SOURCES / "materiali-stampabili" / "velcro_scelte.json", template(
        "velcro_scelte", "Velcro: io voglio",
        "Striscia «Io voglio» con dodici carte scelta per richieste e preferenze.",
        "Materiali stampabili", "VELCRO",
        "Usa la striscia per formulare richieste: «Io voglio» + carta. Verbalizza la frase completa.",
        297, 210, elements,
        {
            "category_id": "stampabili", "subcategory_id": "velcro",
            "subcategory": "Materiali con velcro", "audience": "primaria",
            "objective": "Formulare richieste con una striscia velcro e carte scelta.",
            "tags": ["velcro", "voglio", "richieste", "scelte", "caa"],
            "complexity": "base", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": True,
        },
    ))

    # 8. Sequenza lavarsi le mani con velcro.
    elements = [
        text_element("seq-title", "title", "Lavarsi le mani: sequenza", 10, 43, 220, 9, 5.0, bold=True),
    ]
    steps = ["bagnare", "sapone", "strofinare", "risciacquare", "asciugare", "chiudere"]
    for index, label in enumerate(steps):
        y = 56 + index * 20
        elements.append(text_element(f"step-num-{index + 1}", "badge", str(index + 1), 10, y, 12, 15,
                                     4.5, "#ffffff", bold=True, align="center", background="#0f766e",
                                     radius=2.0))
        elements.append(slot(f"step-slot-{index + 1}", label, 26, y, 110, 15, fill="#ffffff"))
        elements.append(text_element(f"check-{index + 1}", "checkbox", "fatto", 142, y, 28, 15, 3.2))
    for index, label in enumerate(steps):
        elements.append(card(
            f"step-card-{index + 1}", label, 184, 56 + index * 20, 56, 17, font=3.0,
            symbol="acqua" if label == "bagnare" else label,
        ))
    elements.append(text_element("velcro-note", "note",
                                 "Ritaglia le sei carte, plastifica e attacca il velcro. Sposta la carta nella sequenza e spunta il passo concluso.",
                                 10, 182, 277, 8, 3.0, "#64748b", italic=True))
    save(SOURCES / "materiali-stampabili" / "velcro_lavarsi_mani.json", template(
        "velcro_lavarsi_mani", "Velcro: lavarsi le mani",
        "Sequenza in sei passi con caselle da spuntare e carte da ritagliare.",
        "Materiali stampabili", "VELCRO",
        "Segui i passi, sposta la carta e spunta «fatto». Lascia che il bambino esegua in autonomia.",
        297, 210, elements,
        {
            "category_id": "stampabili", "subcategory_id": "velcro",
            "subcategory": "Materiali con velcro", "audience": "primaria",
            "objective": "Eseguire la routine di lavarsi le mani in autonomia.",
            "tags": ["velcro", "sequenza", "igiene", "mani", "autonomia"],
            "complexity": "base", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": False,
        },
    ))

    # 9. Contare fino a 10 con velcro.
    elements = [
        text_element("count-title", "title", "Contare fino a 10", 10, 43, 220, 9, 5.0, bold=True),
        text_element("ten-frame", "ten_frame", "", 10, 58, 160, 64, 4.0,
                     background="#ffffff", border_color="#0f766e", border_width=0.5),
        text_element("count-note", "note", "Metti un gettone in ogni casella e conta ad alta voce.",
                     10, 128, 160, 8, 3.4, "#475569"),
    ]
    numbers = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]
    for index, label in enumerate(numbers):
        column = index % 5
        row = index // 5
        elements.append(card(f"num-{index + 1}", label, 184 + column * 21, 58 + row * 34, 19, 30,
                             font=5.0, symbol=label, dashed=True))
    elements.append(text_element("velcro-note", "note",
                                 "Ritaglia le carte numero e i gettoni. Plastifica tutto e applica il velcro.",
                                 10, 190, 277, 7, 3.0, "#64748b", italic=True))
    save(SOURCES / "materiali-stampabili" / "velcro_contare_10.json", template(
        "velcro_contare_10", "Velcro: contare fino a 10",
        "Ten frame con dieci carte numero da ritagliare e attaccare con il velcro.",
        "Materiali stampabili", "VELCRO",
        "Metti un gettone in ogni casella, conta ad alta voce e abbina la carta numero.",
        297, 210, elements,
        {
            "category_id": "stampabili", "subcategory_id": "velcro",
            "subcategory": "Materiali con velcro", "audience": "primaria",
            "objective": "Contare fino a dieci e associare numero e quantità.",
            "tags": ["velcro", "numeri", "contare", "ten frame", "matematica"],
            "complexity": "base", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": False,
        },
    ))

    # 10. Abbinare forme e colori.
    elements = [
        text_element("shape-title", "title", "Abbinare forme e colori", 10, 43, 230, 9, 5.0, bold=True),
    ]
    for index, label in enumerate(["cerchio", "quadrato", "triangolo"]):
        elements.append(text_element(f"mat-{index + 1}", "shape", label, 10 + index * 94, 56, 86, 58,
                                     4.6, align="center", valign="middle", background="#f8fafc",
                                     border_color="#0f766e", border_width=0.7, radius=3.0))
    shapes = ["cerchio", "quadrato", "triangolo", "cerchio", "quadrato", "triangolo",
              "cerchio", "quadrato", "triangolo"]
    for index, label in enumerate(shapes):
        column = index % 5
        row = index // 5
        elements.append(card(f"shape-card-{index + 1}", label, 10 + column * 46, 126 + row * 30, 42, 26,
                             symbol=label))
    elements.append(text_element("velcro-note", "note",
                                 "Ritaglia le carte, plastifica e attacca il velcro. Abbina ogni forma al tappetino giusto.",
                                 10, 190, 277, 7, 3.0, "#64748b", italic=True))
    save(SOURCES / "materiali-stampabili" / "velcro_forme_colori.json", template(
        "velcro_forme_colori", "Velcro: forme e colori",
        "Tre tappetini per cerchio, quadrato e triangolo con nove carte da ritagliare.",
        "Materiali stampabili", "VELCRO",
        "Abbina le forme ai tappetini e verbalizza nome e colore di ogni carta.",
        297, 210, elements,
        {
            "category_id": "stampabili", "subcategory_id": "velcro",
            "subcategory": "Materiali con velcro", "audience": "primaria",
            "objective": "Riconoscere e abbinare forme e colori.",
            "tags": ["velcro", "forme", "colori", "abbinamento", "percezione"],
            "complexity": "base", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": False,
        },
    ))


def build_aba_teacch() -> None:
    # 1. Prima e poi strutturato.
    elements = [
        text_element("prima-title", "title", "PRIMA", 10, 46, 130, 12, 8.5, bold=True, align="center"),
        text_element("poi-title", "title", "POI", 157, 46, 130, 12, 8.5, bold=True, align="center"),
        slot("prima-slot", "attività", 20, 62, 110, 64),
        slot("poi-slot", "premio", 167, 62, 110, 64),
        text_element("arrow", "text", "→", 138, 82, 12, 18, 11.0, bold=True, align="center"),
        text_element("steps-note", "note",
                     "1. Mostra PRIMA. 2. Indica cosa serve fare. 3. Quando è finito, passa a POI e mostra il rinforzo.",
                     10, 136, 277, 12, 3.4, "#475569"),
    ]
    choices = ["lavorare", "leggere", "puzzle", "riposo", "giocare", "palla", "musica", "disegnare"]
    for index, label in enumerate(choices):
        column = index % 4
        row = index // 4
        elements.append(card(f"choice-{index + 1}", label, 10 + column * 70, 154 + row * 23, 64, 20))
    save(SOURCES / "materiali-aba-e-supporti-visivi" / "aba_prima_poi_board.json", template(
        "aba_prima_poi_board", "ABA: board PRIMA e POI",
        "Board prima/poi con due spazi grandi, otto carte attività e istruzioni per l'adulto.",
        "Materiali ABA e supporti visivi", "ABA",
        "Mostra PRIMA e POI. Attacca l'attività richiesta e il rinforzo scelto; quando PRIMA è finita, passa a POI.",
        297, 210, elements,
        {
            "category_id": "aba", "subcategory_id": "prima_dopo",
            "subcategory": "Prima e poi", "audience": "primaria",
            "objective": "Anticipare attività e rinforzo con una board prima/poi.",
            "tags": ["aba", "prima", "poi", "rinforzo", "velcro"],
            "complexity": "base", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": True,
        },
    ))

    # 2. Token economy 10 gettoni + scheda dati.
    elements = [
        text_element("token-title", "title", "Token economy: dieci gettoni", 10, 43, 230, 9, 5.0, bold=True),
        text_element("token-sub", "subtitle", "Un gettone per ogni risposta o passo. Il premio si sceglie prima.",
                     10, 54, 277, 6, 3.6, "#475569"),
    ]
    for index in range(10):
        column = index % 5
        row = index // 5
        elements.append(text_element(f"token-slot-{index + 1}", "token_slot", "", 14 + column * 40,
                                     68 + row * 38, 30, 30))
    elements.append(text_element("reward-panel", "shape", "PREMIO", 224, 62, 60, 68, 4.0,
                                 align="center", valign="middle", background="#fef3c7",
                                 border_color="#b45309", border_width=0.7, radius=3.0))
    elements.append(character("reward", "giocare", 236, 82, 36, 40, label="giocare"))
    elements.append(text_element("data-title", "subtitle", "Scheda rapida: data, attività, gettoni consegnati, premio.",
                                 10, 154, 277, 6, 3.3, "#475569", bold=True))
    for index in range(3):
        elements.append(text_element(f"data-line-{index + 1}", "answer_line", "data / attività / gettoni / premio",
                                     10, 164 + index * 12, 277, 9, 3.0, "#475569",
                                     border_color="#94a3b8", border_width=0.3, valign="top"))
    save(SOURCES / "materiali-aba-e-supporti-visivi" / "aba_token_economy.json", template(
        "aba_token_economy", "ABA: token economy completa",
        "Token board con dieci gettoni, premio, gettoni da ritagliare e scheda rapida per i dati.",
        "Materiali ABA e supporti visivi", "ABA",
        "Scegli il premio, consegna un gettone dopo ogni risposta corretta e registra i dati nella scheda.",
        297, 210, elements,
        {
            "category_id": "aba", "subcategory_id": "token",
            "subcategory": "Token economy", "audience": "primaria",
            "objective": "Rinforzare risposte e comportamenti con una token economy.",
            "tags": ["aba", "token economy", "rinforzo", "dati", "gettone"],
            "complexity": "articolato", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": False,
        },
    ))

    # 3. Sistema di lavoro TEACCH.
    boxes = [
        ("Cosa fare", "lavorare", 10, 46),
        ("Quanti", "tre", 76, 46),
        ("Come so che ho finito", "finire", 142, 46),
        ("Cosa dopo", "giocare", 208, 46),
    ]
    elements = []
    for index, (label, symbol, x, y) in enumerate(boxes):
        elements.append(text_element(f"box-{index + 1}", "shape", label, x, y, 76, 62, 4.2,
                                     bold=True, align="center", valign="top", background="#f8fafc",
                                     border_color="#0f766e", border_width=0.7, radius=3.0))
        elements.append(slot(f"box-slot-{index + 1}", "", x + 8, y + 30, 60, 26, fill="#ffffff"))
        elements.append(character(f"box-symbol-{index + 1}", symbol, x + 22, y + 12, 32, 18, label=symbol))
    elements.append(text_element("teacch-note", "note",
                                 "Sistema di lavoro TEACCH: indica cosa fare, quanti passi, come si capisce che è finito e cosa viene dopo.",
                                 10, 118, 277, 12, 3.4, "#475569"))
    for index in range(3):
        elements.append(text_element(f"work-check-{index + 1}", "checkbox", f"vassoio {index + 1}",
                                     10, 138 + index * 14, 120, 12, 3.4))
    save(SOURCES / "materiali-teacch" / "teacch_sistema_lavoro.json", template(
        "teacch_sistema_lavoro", "TEACCH: sistema di lavoro",
        "Sistema di lavoro strutturato con quattro riquadri e caselle per i vassoi.",
        "Materiali TEACCH", "TEACCH",
        "Presenta i quattro riquadri nell'ordine. L'alunno sposta il pittogramma e spunta il vassoio concluso.",
        297, 210, elements,
        {
            "category_id": "teacch", "subcategory_id": "strutturato",
            "subcategory": "Sistema di lavoro", "audience": "primaria",
            "objective": "Strutturare il lavoro con cosa fare, quanti, fine e cosa dopo.",
            "tags": ["teacch", "sistema di lavoro", "struttura", "vassoio", "autonomia"],
            "complexity": "articolato", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": True,
        },
    ))

    # 4. Task analysis lavarsi le mani.
    steps = ["bagnare le mani", "prendere il sapone", "strofinare", "risciacquare",
             "asciugare", "chiudere l'acqua", "gettare la carta", "controllare"]
    elements = [text_element("task-title", "title", "Analisi del compito: lavarsi le mani", 10, 43, 277, 9,
                             4.8, bold=True)]
    for index, label in enumerate(steps):
        y = 55 + index * 16
        elements.append(text_element(f"task-num-{index + 1}", "badge", str(index + 1), 10, y, 11, 13,
                                     4.0, "#ffffff", bold=True, align="center", background="#0f766e",
                                     radius=2.0))
        elements.append(text_element(f"task-step-{index + 1}", "text", label, 25, y, 150, 13, 3.6,
                                     valign="middle"))
        elements.append(text_element(f"task-check-{index + 1}", "checkbox", "", 182, y, 10, 13, 3.2))
        elements.append(text_element(f"task-prompt-{index + 1}", "text", "I  G  V  F", 196, y, 80, 13,
                                     3.0, "#475569", valign="middle"))
    elements.append(text_element("prompt-key", "note",
                                 "Codici: I = indipendente, G = gesto, V = verbale, F = fisico. Registra il livello più alto raggiunto.",
                                 10, 190, 277, 8, 3.0, "#64748b", italic=True))
    save(SOURCES / "materiali-teacch" / "teacch_task_analysis_lavarsi.json", template(
        "teacch_task_analysis_lavarsi", "TEACCH: analisi del compito — lavarsi le mani",
        "Otto passi con caselle e codici di prompt per registrare l'autonomia.",
        "Materiali TEACCH", "TEACCH",
        "Esegui i passi nell'ordine, registra il prompt usato e riducilo quando l'alunno diventa autonomo.",
        297, 210, elements,
        {
            "category_id": "teacch", "subcategory_id": "task_analysis",
            "subcategory": "Analisi del compito", "audience": "primaria",
            "objective": "Insegnare una routine di igiene con analisi del compito e prompt.",
            "tags": ["teacch", "task analysis", "igiene", "prompt", "autonomia"],
            "complexity": "articolato", "needs_symbols": False, "printable": True,
            "customizable": True, "interactive": False, "featured": False,
        },
    ))

    # 5. Scheda dati richieste.
    elements = [
        text_element("data-title", "title", "Scheda dati: richieste", 10, 43, 240, 9, 5.0, bold=True),
        text_element("data-sub", "subtitle", "Segna il livello di prompt per ogni prova. I = indipendente, G = gesto, V = verbale, F = fisico.",
                     10, 54, 277, 7, 3.3, "#475569"),
    ]
    headers = ["Prova", "Data", "Cosa ha chiesto", "Prompt", "Esito", "Note"]
    xs = [10, 34, 58, 176, 214, 240]
    widths = [24, 24, 118, 38, 26, 47]
    for index, label in enumerate(headers):
        elements.append(text_element(f"th-{index + 1}", "badge", label, xs[index], 66, widths[index], 10,
                                     3.0, "#ffffff", bold=True, align="center", background="#0f766e",
                                     radius=1.5))
    for row in range(10):
        y = 78 + row * 11
        elements.append(text_element(f"row-{row + 1}-num", "text", str(row + 1), xs[0], y, widths[0], 10,
                                     3.0, align="center"))
        for column in range(1, 6):
            elements.append(text_element(f"cell-{row + 1}-{column}", "answer_line", "", xs[column], y,
                                         widths[column], 10, 3.0, border_color="#94a3b8",
                                         border_width=0.2, valign="top"))
    elements.append(text_element("data-note", "note",
                                 "Raccogli i dati solo su una o due risposte per volta. Il rinforzo resta coerente con il programma.",
                                 10, 190, 277, 7, 3.0, "#64748b", italic=True))
    save(SOURCES / "materiali-aba-e-supporti-visivi" / "aba_scheda_dati_richieste.json", template(
        "aba_scheda_dati_richieste", "ABA: scheda dati — richieste",
        "Scheda a dieci prove con prompt, esito e note per raccogliere dati sulle richieste.",
        "Materiali ABA e supporti visivi", "ABA",
        "Registra ogni prova con il livello di prompt e l'esito. Usa i dati per decidere quando ridurre l'aiuto.",
        297, 210, elements,
        {
            "category_id": "aba", "subcategory_id": "dati",
            "subcategory": "Schede dati", "audience": "primaria",
            "objective": "Raccogliere dati sulle richieste e sui prompt.",
            "tags": ["aba", "dati", "richieste", "prompt", "annotazione"],
            "complexity": "base", "needs_symbols": False, "printable": True,
            "customizable": True, "interactive": False, "featured": False,
        },
    ))

    # 6. Abbinamento uguali.
    pairs = ["cane", "gatto", "mela", "palla", "macchina", "fiore", "sole", "casa"]
    elements = [text_element("match-title", "title", "Abbinamento: immagini uguali", 10, 43, 240, 9, 5.0, bold=True)]
    for index, label in enumerate(pairs):
        column = index % 4
        row = index // 4
        elements.append(card(f"match-{index + 1}", label, 10 + column * 70, 58 + row * 48, 64, 42))
        elements.append(card(f"match-copy-{index + 1}", label, 10 + column * 70, 160 + row * 0.1, 64, 1,
                             symbol=label))
    # Le copie da ritagliare stanno in una striscia separata in basso.
    elements = [e for e in elements if not str(e["id"]).startswith("match-copy-")]
    for index, label in enumerate(pairs):
        elements.append(card(f"copy-{index + 1}", label, 10 + index * 34, 164, 30, 28, font=2.8))
    elements.append(text_element("match-note", "note",
                                 "Ritaglia le copie in basso, plastifica e applica il velcro. Abbina ogni immagine al suo uguale.",
                                 10, 190, 277, 6, 3.0, "#64748b", italic=True))
    save(SOURCES / "materiali-teacch" / "teacch_abbinamento_uguali.json", template(
        "teacch_abbinamento_uguali", "TEACCH: abbina le immagini uguali",
        "Otto coppie di immagini con copie da ritagliare e attaccare con il velcro.",
        "Materiali TEACCH", "TEACCH",
        "Presenta una coppia alla volta, indica l'immagine uguale e verbalizza il nome.",
        297, 210, elements,
        {
            "category_id": "teacch", "subcategory_id": "matching",
            "subcategory": "Abbinamento", "audience": "primaria",
            "objective": "Abbinare immagini identiche e verbalizzarne il nome.",
            "tags": ["teacch", "abbinamento", "matching", "immagini", "velcro"],
            "complexity": "base", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": False,
        },
    ))

    # 7. Categorizza: cibo, animali, veicoli.
    elements = [text_element("cat-title", "title", "Categorizza: cibo, animali, veicoli", 10, 43, 250, 9, 4.8, bold=True)]
    mats = ["cibo", "animali", "veicoli"]
    for index, label in enumerate(mats):
        elements.append(text_element(f"cat-mat-{index + 1}", "shape", label, 10 + index * 94, 58, 86, 76, 4.6,
                                     align="center", valign="middle", background="#f8fafc",
                                     border_color="#0f766e", border_width=0.7, radius=3.0))
    items = ["mela", "pane", "acqua", "cane", "gatto", "pesce", "macchina", "treno", "bici",
             "banana", "cavallo", "autobus"]
    for index, label in enumerate(items):
        column = index % 6
        row = index // 6
        elements.append(card(f"cat-card-{index + 1}", label, 10 + column * 46, 144 + row * 24, 42, 21))
    elements.append(text_element("cat-note", "note",
                                 "Ritaglia le carte, plastifica e applica il velcro. Ordina ogni carta nella categoria giusta.",
                                 10, 190, 277, 6, 3.0, "#64748b", italic=True))
    save(SOURCES / "materiali-teacch" / "teacch_categorizza.json", template(
        "teacch_categorizza", "TEACCH: categorizza",
        "Tre tappetini per cibo, animali e veicoli con dodici carte da ritagliare.",
        "Materiali TEACCH", "TEACCH",
        "Ordina le carte nei tappetini e verbalizza la categoria e il nome dell'immagine.",
        297, 210, elements,
        {
            "category_id": "teacch", "subcategory_id": "categorizzazione",
            "subcategory": "Categorizzazione", "audience": "primaria",
            "objective": "Classificare immagini in categorie semantiche.",
            "tags": ["teacch", "categorie", "cibo", "animali", "veicoli", "velcro"],
            "complexity": "base", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": True,
        },
    ))

    # 8. Sequenza quotidiana.
    steps = ["svegliarsi", "lavarsi", "vestirsi", "colazione", "zaino", "scuola"]
    elements = [text_element("seq-title", "title", "Sequenza: pronto per la scuola", 10, 43, 240, 9, 4.8, bold=True)]
    for index, label in enumerate(steps):
        x = 10 + index * 46
        elements.append(text_element(f"seq-num-{index + 1}", "badge", str(index + 1), x, 58, 12, 12,
                                     4.0, "#ffffff", bold=True, align="center", background="#0f766e",
                                     radius=2.0))
        elements.append(card(f"seq-card-{index + 1}", label, x, 72, 42, 44, font=3.0))
    for index, label in enumerate(steps):
        elements.append(text_element(f"seq-check-{index + 1}", "checkbox", label, 10, 130 + index * 11, 120, 10, 3.2))
    elements.append(text_element("seq-note", "note",
                                 "Metti in ordine le carte, segui la sequenza e spunta ogni passo concluso.",
                                 10, 190, 277, 6, 3.0, "#64748b", italic=True))
    save(SOURCES / "materiali-teacch" / "teacch_sequenza_scuola.json", template(
        "teacch_sequenza_scuola", "TEACCH: sequenza pronto per la scuola",
        "Sei passi illustrati con caselle da spuntare per la routine del mattino.",
        "Materiali TEACCH", "TEACCH",
        "Mostra la sequenza, esegui un passo alla volta e spunta quello concluso.",
        297, 210, elements,
        {
            "category_id": "teacch", "subcategory_id": "sequenze",
            "subcategory": "Sequenze", "audience": "primaria",
            "objective": "Eseguire la routine del mattino in autonomia.",
            "tags": ["teacch", "sequenza", "routine", "scuola", "autonomia"],
            "complexity": "base", "needs_symbols": True, "printable": True,
            "customizable": True, "interactive": False, "featured": False,
        },
    ))


def main() -> None:
    build_comics()
    build_velcro()
    build_aba_teacch()
    files = list((SOURCES / "materiali-stampabili").glob("*.json"))
    files += list((SOURCES / "materiali-aba-e-supporti-visivi").glob("aba_*.json"))
    print(f"Nuovi modelli generati: {len(files)}")


if __name__ == "__main__":
    main()
