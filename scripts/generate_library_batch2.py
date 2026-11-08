#!/usr/bin/env python3
"""Secondo lotto della nuova biblioteca: 60 modelli costruiti con lo stesso
design system dei piloti. Le specifiche sono dati; i documenti vengono creati
dalla CLI (`caa-cli template build-all`)."""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
from generate_pilot_templates import (  # noqa: E402
    ACCENT,
    ACCENT_SOFT,
    BORDER,
    INK,
    MUTED,
    PAPER,
    ROSE,
    WARM,
    base_spec,
    el,
    header,
    instruction,
    page,
    style,
    symbol,
)

OUT = ROOT / "templates" / "sources" / "templates" / "lotto2"


def tax(category_id, subcategory_id, audience, objective, tags, complexity="base", interactive=False, needs_symbols=True, featured=False):
    return {
        "category_id": category_id,
        "subcategory_id": subcategory_id,
        "audience": audience,
        "objective": objective,
        "tags": tags,
        "complexity": complexity,
        "needs_symbols": needs_symbols,
        "printable": True,
        "customizable": True,
        "interactive": interactive,
        "featured": featured,
    }


def save(spec):
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / f"{spec['id']}.json").write_text(json.dumps(spec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


# ------------------------------------------------------------------ CAA


def caa_table(pid, title, desc, words, category="Tabelle comunicative", sub="tabelle", landscape=True, featured=False):
    els = []
    header(els, "Comunicazione", title, "Tocca o indica un pittogramma, poi costruisci la frase.", page_no=1, total=1, footer_y=196 if landscape else 284)
    els.append(el("msg", "text_box", "La mia frase: ________________________________________________", x=10, y=38, w=277 if landscape else 190, h=11, st=style(size=4.3, background="#f8fafc", border_color=BORDER, border_width=0.5, radius=2.5, padding=2, valign="middle"), absolute=True))
    for i, word in enumerate(words):
        label, symbol_label = word if isinstance(word, tuple) else (word, word)
        els.append(el(f"c{i+1}", "symbol", label, row=i // 4, column=i % 4, st=style(size=3.9, border_color=BORDER, border_width=0.5, radius=3, padding=1.2, align="center", valign="bottom"), symbol=symbol_label, action={"type": "append"}, spoken_text=label))
    grid = {"columns": 4, "rows": (len(words) + 3) // 4, "gap_mm": 3, "origin_x_mm": 10, "origin_y_mm": 54, "content_width_mm": 277 if landscape else 190, "content_height_mm": 136 if landscape else 210, "align": "left", "valign": "top"}
    return base_spec(pid, title, desc, category, "communication_grid", tax("aac", sub, "primaria", desc, [w.lower() for w in words[:6]], "base", True, True, featured), interactive=True, orientation="landscape" if landscape else "portrait", pages=[page("home", title, els, layout="grid", grid=grid, home=True)])


def caa_notebook(pid, title, desc, sections, sub="quaderni"):
    pages = []
    home = []
    header(home, "Comunicazione", title, "Scegli una categoria. Ogni parola può essere aggiunta alla frase.", page_no=1, total=len(sections) + 1)
    for i, (slug, label, words) in enumerate(sections):
        label_text, label_symbol = label if isinstance(label, tuple) else (label, label)
        home.append(el(f"h{slug}", "symbol", label_text, row=i // 3, column=i % 3, st=style(size=4.1, border_color=ACCENT, border_width=0.6, radius=3, padding=1.2, align="center", valign="bottom", bold=True), symbol=label_symbol, action={"type": "navigate", "target": slug}, spoken_text=label_text))
    pages.append(page("home", title, home, layout="grid", grid={"columns": 3, "rows": (len(sections) + 2) // 3, "gap_mm": 3, "origin_x_mm": 10, "origin_y_mm": 42, "content_width_mm": 190, "content_height_mm": 215, "align": "left", "valign": "top"}, home=True))
    for index, (slug, label, words) in enumerate(sections, start=2):
        label_text, _label_symbol = label if isinstance(label, tuple) else (label, label)
        els = []
        header(els, "Comunicazione", label_text, "Tocca una parola per aggiungerla alla frase.", page_no=index, total=len(sections) + 1)
        for i, word in enumerate(words[:9]):
            label, symbol_label = word if isinstance(word, tuple) else (word, word)
            els.append(el(f"{slug}{i}", "symbol", label, row=i // 3, column=i % 3, st=style(size=3.8, border_color=BORDER, border_width=0.5, radius=3, padding=1.2, align="center", valign="bottom"), symbol=symbol_label, action={"type": "append"}, spoken_text=label))
        els.append(el(f"{slug}-home", "button", "⌂ Home", x=10, y=274, w=36, h=9, st=style(size=3.4, background=ACCENT_SOFT, border_color=ACCENT, border_width=0.5, radius=2, align="center", valign="middle", bold=True), action={"type": "home"}, spoken_text="Home", absolute=True))
        els.append(el(f"{slug}-back", "button", "‹ Indietro", x=50, y=274, w=36, h=9, st=style(size=3.4, background="#f8fafc", border_color=BORDER, border_width=0.5, radius=2, align="center", valign="middle"), action={"type": "back"}, spoken_text="Indietro", absolute=True))
        pages.append(page(slug, label_text, els, layout="grid", grid={"columns": 3, "rows": 3, "gap_mm": 3, "origin_x_mm": 10, "origin_y_mm": 42, "content_width_mm": 190, "content_height_mm": 215, "align": "left", "valign": "top"}))
    return base_spec(pid, title, desc, "Sistemi comunicativi", "communication_board", tax("aac", sub, "primaria", desc, [(s[1][0] if isinstance(s[1], tuple) else s[1]).lower() for s in sections], "intermedio", True, True), interactive=True, orientation="portrait", pages=pages)


def caa_story(pid, title, desc, pages_data, featured=False):
    pages = []
    cover = []
    header(cover, "Libro CAA", title, "Leggi la storia. Puoi toccare i pittogrammi per ascoltare le parole.", page_no=1, total=len(pages_data) + 2)
    symbol(cover, "cover-sym", pages_data[0]["hero"], 55, 56, 100, 100, size=4.0)
    cover.append(el("cover-note", "text_box", desc, x=25, y=170, w=160, h=34, st=style(size=4.0, background=ACCENT_SOFT, border_color=ACCENT, border_width=0.4, radius=2.5, padding=2.2, align="center", valign="middle")))
    pages.append(page("home", "Copertina", cover, home=True))
    for index, data in enumerate(pages_data, start=2):
        els = []
        header(els, "Libro CAA", data["title"], f"Pagina {index - 1} di {len(pages_data)}", page_no=index, total=len(pages_data) + 2)
        symbol(els, f"p{index}-hero", data["hero"], 55, 42, 100, 104, size=4.0, caption="")
        cells = []
        for word in data["words"]:
            label, symbol_label = word if isinstance(word, tuple) else (word, word)
            cells.append({"text": label, "symbol": symbol_label, "action": {"type": "speak"}, "spoken_text": label})
        els.append(el(f"p{index}-strip", "grid", "", x=10, y=154, w=190, h=38, st=style(size=3.4, border_color=BORDER, border_width=0.4, radius=2, padding=1), columns=len(cells), rows=1, gap_mm=2, cells=cells))
        els.append(el(f"p{index}-sentence", "text_box", data["sentence"], x=10, y=202, w=190, h=30, st=style(size=5.0, background="#f8fafc", border_color=BORDER, border_width=0.4, radius=2.5, padding=2.4, align="center", valign="middle", bold=True)))
        pages.append(page(f"p{index}", data["title"], els))
    final = []
    header(final, "Libro CAA", "Domande", "Rispondi con una frase o con i pittogrammi.", page_no=len(pages_data) + 2, total=len(pages_data) + 2)
    for i, question in enumerate(["Che cosa succede nella storia?", "Chi sono i personaggi?", "Che cosa ti è piaciuto di più?"]):
        y = 48 + i * 60
        final.append(el(f"q{i}", "question", question, x=10, y=y, w=190, h=8, st=style(size=4.4, bold=True)))
        final.append(el(f"q{i}-line", "answer_line", "", x=10, y=y + 12, w=190, h=26, st=style(size=3.0), metadata={"lines": 2}))
    pages.append(page("domande", "Domande", final))
    return base_spec(pid, title, desc, "Storie sociali", "story", tax("storie-caa", "sopra-testo", "primaria", desc, ["libro", "caa", "lettura", "storia"], "intermedio", False, True, featured), interactive=False, orientation="portrait", pages=pages)


def social_story(pid, title, desc, situation, pages_text, note, featured=False):
    pages = []
    cover = []
    header(cover, "Storia", title, "Una storia da leggere insieme, una frase alla volta.", page_no=1, total=len(pages_text) + 2)
    cover.append(el("cover-box", "text_box", situation, x=20, y=54, w=170, h=70, st=style(size=6.2, background=ACCENT_SOFT, border_color=ACCENT, border_width=0.5, radius=3, padding=3, align="center", valign="middle", bold=True)))
    cover.append(el("cover-note", "note", "Storia descrittiva e di anticipazione. Non è una Social Story certificata secondo i criteri 10.4: la revisione dell'adulto è necessaria.", x=10, y=160, w=190, h=34, st=style(size=3.3, color=MUTED, italic=True)))
    pages.append(page("home", "Copertina", cover, home=True))
    for idx, sentences in enumerate(pages_text, start=2):
        els = []
        header(els, "Storia", f"Parte {idx - 1}", "Leggi una frase, poi lascia spazio alle domande.", page_no=idx, total=len(pages_text) + 2)
        for j, sentence in enumerate(sentences):
            y = 46 + j * 92
            els.append(el(f"p{idx}s{j}", "text_box", sentence, x=10, y=y, w=190, h=72, st=style(size=5.0, background="#f8fafc", border_color=BORDER, border_width=0.4, radius=2.5, padding=2.5, valign="middle")))
        pages.append(page(f"p{idx}", f"Parte {idx - 1}", els))
    end = []
    header(end, "Storia", "Nota per l'adulto", "Da leggere prima di usare la storia.", page_no=len(pages_text) + 2, total=len(pages_text) + 2)
    end.append(el("adult", "text_box", note, x=10, y=42, w=190, h=100, st=style(size=4.1, background="#fffbeb", border_color=WARM, border_width=0.5, radius=2.5, padding=2.5)))
    pages.append(page("nota", "Nota per l'adulto", end))
    return base_spec(pid, title, desc, "Storie sociali", "social_story", tax("storie-sociali", "anticipazione", "primaria", desc, ["storia sociale", "anticipazione", "emozioni"], "intermedio", False, True, featured), interactive=False, orientation="portrait", pages=pages)


# --------------------------------------------------------------- agenda


def agenda_weekly(pid, title, desc, routine, days=None, featured=False):
    days = days or ["lunedì", "martedì", "mercoledì", "giovedì", "venerdì"]
    els = []
    header(els, "Organizzazione", title, "Guarda il giorno, poi l'attività. Spunta ciò che hai finito.", page_no=1, total=1, footer_y=196)
    for i, day in enumerate(days):
        els.append(el(f"d{i}", "text", day, row=0, column=i, st=style(size=4.1, color="#ffffff", background=ACCENT, radius=2, align="center", valign="middle", bold=True)))
    for day_index in range(len(days)):
        for a, (label, symbol_label) in enumerate(routine):
            els.append(el(f"a{day_index}-{a}", "symbol", label, row=a + 1, column=day_index, st=style(size=3.3, border_color=BORDER, border_width=0.4, radius=2, padding=1, align="center", valign="bottom"), symbol=symbol_label, action={"type": "speak"}, spoken_text=f"{days[day_index]} {label}"))
    grid = {"columns": len(days), "rows": len(routine) + 1, "gap_mm": 2.5, "origin_x_mm": 10, "origin_y_mm": 42, "content_width_mm": 277, "content_height_mm": 224, "row_heights": [0.55] + [1] * len(routine), "align": "left", "valign": "top"}
    return base_spec(pid, title, desc, "Agende e calendari visivi", "visual_schedule", tax("agende", "settimanale", "primaria", desc, ["agenda", "settimana", "routine"], "intermedio", True), interactive=True, orientation="landscape", pages=[page("home", title, els, layout="grid", grid=grid, home=True)])


def agenda_daily(pid, title, desc, activities, featured=False):
    els = []
    header(els, "Organizzazione", title, "Segui la giornata dall'alto verso il basso. Spunta le attività finite.", page_no=1, total=1)
    for i, (time_label, label, symbol_label) in enumerate(activities[:8]):
        y = 46 + i * 29
        els.append(el(f"t{i}", "text", time_label, x=10, y=y, w=26, h=22, st=style(size=3.2, color="#ffffff", background=ACCENT, radius=2, align="center", valign="middle", bold=True)))
        symbol(els, f"s{i}", symbol_label, 40, y, 24, 22, size=2.8, caption="")
        els.append(el(f"l{i}", "text", label, x=68, y=y + 4, w=80, h=14, st=style(size=4.0, valign="middle")))
        els.append(el(f"c{i}", "checkbox", "", x=170, y=y + 3, w=20, h=14, st=style(size=3.0)))
    return base_spec(pid, title, desc, "Agende e calendari visivi", "visual_schedule", tax("agende", "giornaliera", "primaria", desc, ["agenda", "giornata", "routine", "transizioni"], "base", False), interactive=False, orientation="portrait", pages=[page("home", title, els, home=True)])


def agenda_first_then(pid, title, desc, first, then, featured=False):
    els = []
    header(els, "Organizzazione", title, "Prima l'attività, poi il momento scelto.", page_no=1, total=1)
    for i, (heading, item, color) in enumerate([("PRIMA", first, ACCENT_SOFT), ("POI", then, "#fffbeb")]):
        x = 10 + i * 96
        els.append(el(f"h{i}", "text", heading, x=x, y=46, w=92, h=10, st=style(size=5.0, color="#ffffff", background=ACCENT, radius=2, align="center", valign="middle", bold=True)))
        els.append(el(f"b{i}", "text_box", "", x=x, y=60, w=92, h=130, st=style(size=3.0, background=color, border_color=ACCENT if i == 0 else WARM, border_width=0.5, radius=3, padding=2)))
        symbol(els, f"s{i}", item, x + 18, 80, 56, 56, size=3.6)
        els.append(el(f"t{i}", "text", item, x=x, y=144, w=92, h=14, st=style(size=4.0, align="center", valign="middle", bold=True)))
    return base_spec(pid, title, desc, "Sequenze e routine", "visual_sequence", tax("agende", "prima-poi", "primaria", desc, ["prima", "poi", "transizione"], "base", True), interactive=True, orientation="portrait", pages=[page("home", title, els, home=True)])


def agenda_checklist(pid, title, desc, tasks, featured=False):
    els = []
    header(els, "Organizzazione", title, "Controlla una voce alla volta. La checklist è finita quando tutte le caselle sono spunte.", page_no=1, total=1)
    for i, task in enumerate(tasks[:10]):
        y = 46 + i * 23
        els.append(el(f"c{i}", "checkbox", "", x=14, y=y, w=10, h=10, st=style(size=3.0)))
        els.append(el(f"t{i}", "text", task, x=32, y=y, w=160, h=12, st=style(size=4.2, valign="middle")))
    return base_spec(pid, title, desc, "Sequenze e routine", "visual_sequence", tax("agende", "checklist", "primaria", desc, ["checklist", "routine", "autonomia"], "base", False), interactive=False, orientation="portrait", pages=[page("home", title, els, home=True)])


# ---------------------------------------------------------------- TEACCH


def teacch_sequence(pid, title, desc, steps, featured=False):
    els = []
    header(els, "TEACCH", title, "Segui i passi. Spunta ogni attività quando è finita.", page_no=1, total=1)
    for i, (symbol_label, text) in enumerate(steps[:5]):
        y = 46 + i * 43
        els.append(el(f"n{i}", "badge", str(i + 1), x=10, y=y, w=12, h=12, st=style(size=5.2, color="#ffffff", background=ACCENT, radius=6, align="center", valign="middle", bold=True)))
        symbol(els, f"s{i}", symbol_label, 28, y - 4, 36, 36, size=3.2, caption="")
        els.append(el(f"t{i}", "text", text, x=70, y=y + 4, w=104, h=14, st=style(size=4.2, valign="middle")))
        els.append(el(f"c{i}", "checkbox", "Fatto", x=174, y=y + 2, w=26, h=12, st=style(size=3.2, valign="middle")))
    return base_spec(pid, title, desc, "Materiali TEACCH", "teacch", tax("teacch", "sequenze-lavoro", "primaria", desc, ["teacch", "sequenza", "lavoro", "autonomia"], "base", False), interactive=False, orientation="portrait", pages=[page("home", title, els, home=True)])


def teacch_matching(pid, title, desc, pairs, featured=False):
    els = []
    header(els, "TEACCH", title, "Abbina ogni immagine a quella uguale. Scrivi il numero nella casella.", page_no=1, total=1)
    for i, (left, right) in enumerate(pairs[:5]):
        y = 50 + i * 42
        symbol(els, f"l{i}", left, 10, y, 32, 32, size=3.0, caption="")
        els.append(el(f"b{i}", "text_box", "", x=64, y=y + 6, w=24, h=18, st=style(size=5.0, border_color=BORDER, border_width=0.6, radius=2, align="center", valign="middle")))
        symbol(els, f"r{i}", right, 120, y, 32, 32, size=3.0, caption="")
        els.append(el(f"n{i}", "text", str(i + 1), x=162, y=y + 8, w=12, h=12, st=style(size=4.0, bold=True)))
    return base_spec(pid, title, desc, "Materiali TEACCH", "teacch", tax("teacch", "abbinamenti", "primaria", desc, ["teacch", "abbinamento", "corrispondenza"], "base", True), interactive=True, orientation="portrait", pages=[page("home", title, els, home=True)])


def teacch_sorting(pid, title, desc, categories, featured=False):
    els = []
    header(els, "TEACCH", title, "Classifica le immagini nella colonna giusta.", page_no=1, total=1)
    bank = []
    for name, words in categories:
        bank.extend(words)
    for i, word in enumerate(bank[:9]):
        symbol(els, f"b{i}", word, 10 + (i % 9) * 21, 46, 20, 24, size=2.5, caption="")
    for i, (name, words) in enumerate(categories):
        x = 10 + i * 65
        els.append(el(f"h{i}", "text", name, x=x, y=76, w=60, h=9, st=style(size=3.8, color="#ffffff", background=ACCENT, radius=2, align="center", valign="middle", bold=True)))
        els.append(el(f"box{i}", "text_box", "", x=x, y=88, w=60, h=150, st=style(size=3.0, background=ACCENT_SOFT if i % 2 == 0 else "#eff6ff", border_color=BORDER, border_width=0.5, radius=2.5, padding=2)))
        for j in range(3):
            els.append(el(f"line{i}-{j}", "answer_line", "", x=x + 2, y=94 + j * 46, w=56, h=40, st=style(size=3.0), metadata={"lines": 2}))
    return base_spec(pid, title, desc, "Materiali TEACCH", "teacch", tax("teacch", "classificazioni", "primaria", desc, ["teacch", "classificazione", "categorie"], "intermedio", False), interactive=False, orientation="portrait", pages=[page("home", title, els, home=True)])


def teacch_cards(pid, title, desc, cards, featured=False):
    els = []
    header(els, "TEACCH", title, "Ritaglia i cartellini e usali per l'attività.", page_no=1, total=1, footer_y=196)
    for i, label in enumerate(cards[:12]):
        els.append(el(f"c{i}", "symbol", label, row=i // 4, column=i % 4, st=style(size=3.1, border_color=BORDER, border_width=0.5, radius=2, padding=1, align="center", valign="bottom"), symbol=label, action={"type": "speak"}, spoken_text=label))
    return base_spec(pid, title, desc, "Materiali TEACCH", "teacch", tax("teacch", "ritagliare", "primaria", desc, ["teacch", "cartellini", "ritagliare"], "base", False), interactive=False, orientation="landscape", pages=[page("home", title, els, layout="grid", grid={"columns": 4, "rows": 3, "gap_mm": 4, "origin_x_mm": 10, "origin_y_mm": 44, "content_width_mm": 277, "content_height_mm": 140, "align": "left", "valign": "top"}, home=True)])


# -------------------------------------------------------------- behavior


def token_board(pid, title, desc, count, featured=False):
    els = []
    header(els, "Supporti", title, "Tocca un gettone quando completi un'attività. Alla fine scegli il rinforzo.", page_no=1, total=1)
    els.append(el("goal", "text_box", f"Obiettivo: completare {count} attività concordate.", x=10, y=40, w=190, h=12, st=style(size=4.4, background=ACCENT_SOFT, border_color=ACCENT, border_width=0.5, radius=2.5, padding=2, valign="middle", bold=True)))
    columns = 5 if count > 5 else count
    rows = 2 if count > 5 else 1
    slot = 32
    gap = 4
    total = columns * slot + (columns - 1) * gap
    start = (210 - total) / 2
    for i in range(count):
        els.append(el(f"tok{i}", "token_slot", "", x=start + (i % columns) * (slot + gap), y=62 + (i // columns) * (slot + gap), w=slot, h=slot, st=style(size=3.2, border_color=ACCENT, border_width=0.8, align="center", valign="middle"), action={"type": "toggle"}, spoken_text="gettone", metadata={"filled": False, "index": i + 1}))
    reward_y = 62 + rows * (slot + gap) + 8
    els.append(el("reward", "text_box", "Il rinforzo si sceglie insieme e può cambiare: non è un premio per «essere bravo».", x=10, y=reward_y, w=190, h=18, st=style(size=3.7, background="#fffbeb", border_color=WARM, border_width=0.5, radius=2.5, padding=2, valign="middle")))
    for i, label in enumerate(["leggere", "scrivere", "disegnare", "controllare", "ordinare"][:count]):
        x = 10 + i * (190 / max(1, min(count, 5)))
        width = (190 / max(1, min(count, 5))) - 4
        symbol(els, f"act{i}", label, x, reward_y + 28, width, 30, size=3.0)
        els.append(el(f"chk{i}", "checkbox", "", x=x + width / 2 - 4, y=reward_y + 62, w=8, h=8, st=style(size=3.0)))
    return base_spec(pid, title, desc, "Materiali ABA e supporti visivi", "aba", tax("comportamentali", "token", "primaria", desc, ["token board", "gettoni", "rinforzo", "autonomia"], "base", True), interactive=True, orientation="portrait", pages=[page("home", title, els, home=True)])


def choice_board(pid, title, desc, choices, n=2, featured=False):
    els = []
    header(els, "Supporti", title, "Scegli una delle alternative. Non c'è una risposta sbagliata.", page_no=1, total=1)
    for i, choice in enumerate(choices[:n]):
        x = 10 + i * (190 // n)
        w = (190 // n) - 6
        symbol(els, f"c{i}", choice, x, 62, w, 110, size=4.0, action={"type": "speak"})
        els.append(el(f"l{i}", "text", choice, x=x, y=180, w=w, h=16, st=style(size=5.0, align="center", valign="middle", bold=True)))
    return base_spec(pid, title, desc, "Materiali ABA e supporti visivi", "aba", tax("comportamentali", "scelte", "primaria", desc, ["scelta", "comunicazione", "autonomia"], "base", True), interactive=True, orientation="portrait", pages=[page("home", title, els, home=True)])


def break_card(pid, title, desc, featured=False):
    els = []
    header(els, "Supporti", title, "Mostra questa carta quando hai bisogno di una pausa.", page_no=1, total=1)
    els.append(el("big", "badge", "PAUSA", x=30, y=70, w=150, h=50, st=style(size=18, color="#ffffff", background=ACCENT, radius=6, align="center", valign="middle", bold=True)))
    symbol(els, "pause-sym", "riposo", 70, 140, 70, 70, size=4.0, caption="")
    els.append(el("steps", "text_box", "1. Mostro la carta.\n2. Vado nello spazio della pausa.\n3. Respiri e aspetto.\n4. Torno all'attività quando sono pronto.", x=20, y=220, w=170, h=46, st=style(size=4.0, background="#f8fafc", border_color=BORDER, border_width=0.5, radius=2.5, padding=2, valign="top")))
    return base_spec(pid, title, desc, "Materiali ABA e supporti visivi", "aba", tax("comportamentali", "pausa", "primaria", desc, ["pausa", "autoregolazione", "comunicazione"], "base", True), interactive=True, orientation="portrait", pages=[page("home", title, els, home=True)])


def emotion_scale(pid, title, desc, featured=False):
    els = []
    header(els, "Supporti", title, "Come mi sento? Indica il grado e scegli una strategia.", page_no=1, total=1)
    levels = [("1", "calmo", "#dcfce7"), ("2", "un po' teso", "#ecfccb"), ("3", "teso", "#fef9c3"), ("4", "molto teso", "#ffedd5"), ("5", "ho bisogno di aiuto", "#fee2e2")]
    for i, (number, label, color) in enumerate(levels):
        y = 52 + i * 40
        els.append(el(f"n{i}", "badge", number, x=10, y=y, w=16, h=16, st=style(size=5.0, color="#ffffff", background=ACCENT, radius=8, align="center", valign="middle", bold=True)))
        els.append(el(f"b{i}", "text_box", label, x=32, y=y, w=92, h=16, st=style(size=4.0, background=color, border_color=BORDER, border_width=0.4, radius=2, padding=2, valign="middle")))
        els.append(el(f"c{i}", "checkbox", "", x=130, y=y + 2, w=12, h=12, st=style(size=3.0)))
    return base_spec(pid, title, desc, "Emozioni e autoregolazione", "emotions", tax("comportamentali", "scale", "primaria", desc, ["emozioni", "intensità", "autoregolazione"], "base", True), interactive=True, orientation="portrait", pages=[page("home", title, els, home=True)])


# --------------------------------------------------------------- schede


def math_sheet(pid, title, desc, operations, instruction_text, subject="MATEMATICA", featured=False):
    els = []
    header(els, "Matematica", title, "Scheda operativa · materiale modificabile", page_no=1, total=2)
    instruction(els, "instr", instruction_text, y=40, h=10)
    for i, (a, op, b, result) in enumerate(operations[:10]):
        x = 10 + (i % 2) * 96
        y = 60 + (i // 2) * 30
        els.append(el(f"q{i}", "question", f"{a} {op} {b} =", x=x, y=y, w=54, h=20, st=style(size=7.4, bold=True, align="right", valign="middle")))
        els.append(el(f"a{i}", "text_box", "", x=x + 60, y=y + 2, w=28, h=16, st=style(size=7.0, border_color=BORDER, border_width=0.6, radius=2.5, align="center", valign="middle")))
    sol = []
    header(sol, "Matematica", "Soluzioni per il docente", "Risposte e indicazioni", page_no=2, total=2)
    for i, (a, op, b, result) in enumerate(operations[:10]):
        sol.append(el(f"s{i}", "text", f"{a} {op} {b} = {result}", x=10 + (i % 2) * 96, y=48 + (i // 2) * 18, w=88, h=14, st=style(size=4.6, bold=True, valign="middle")))
    sol.append(el("note", "note", "Obiettivo: consolidare il calcolo. Adattamento: ridurre gli esercizi, usare gettoni o linea dei numeri, aumentare il carattere.", x=10, y=160, w=190, h=28, st=style(size=3.6, color=MUTED, italic=True)))
    return base_spec(pid, title, desc, "Materiali didattici", "worksheet", tax("schede", "matematica", "primaria", desc, ["matematica", "calcolo", "esercizi"], "base", False, False, featured), interactive=False, orientation="portrait", pages=[page("home", title, els, home=True), page("soluzioni", "Soluzioni", sol)])


def counting_sheet(pid, title, desc, counts, featured=False):
    els = []
    header(els, "Matematica", title, "Scheda operativa · materiale modificabile", page_no=1, total=2)
    instruction(els, "instr", "Conta gli oggetti nel riquadro e scrivi il numero nella casella.", y=40, h=10)
    for i, count in enumerate(counts[:8]):
        x = 10 + (i % 2) * 96
        y = 60 + (i // 2) * 48
        els.append(el(f"f{i}", "ten_frame", "", x=x, y=y, w=52, h=22, st=style(size=3.0, border_color=BORDER, border_width=0.5), metadata={"filled": count}))
        els.append(el(f"a{i}", "text_box", "", x=x + 62, y=y + 2, w=24, h=18, st=style(size=7.0, border_color=BORDER, border_width=0.6, radius=2.5, align="center", valign="middle")))
    sol = []
    header(sol, "Matematica", "Soluzioni per il docente", "Risposte e indicazioni", page_no=2, total=2)
    sol.append(el("answers", "text", "Risposte: " + ", ".join(str(count) for count in counts[:8]) + ".", x=10, y=48, w=190, h=30, st=style(size=4.6, bold=True)))
    sol.append(el("note", "note", "Obiettivo: collegare quantità e numero. Adattamento: contare ad alta voce, toccare un oggetto per volta, usare gettoni reali.", x=10, y=96, w=190, h=30, st=style(size=3.6, color=MUTED, italic=True)))
    return base_spec(pid, title, desc, "Materiali didattici", "worksheet", tax("schede", "matematica", "primaria", desc, ["matematica", "conteggio", "quantità"], "base", False, False, featured), interactive=False, orientation="portrait", pages=[page("home", title, els, home=True), page("soluzioni", "Soluzioni", sol)])


def language_sheet(pid, title, desc, items, prompt, instruction_text, featured=False, with_symbol=True, subject="Italiano"):
    els = []
    header(els, subject, title, "Scheda operativa · materiale modificabile", page_no=1, total=2)
    instruction(els, "instr", instruction_text, y=40, h=10)
    for i, word in enumerate(items[:6]):
        y = 58 + i * 36
        if with_symbol:
            symbol(els, f"img{i}", word, 10, y, 30, 30, size=3.0, caption="")
        else:
            els.append(el(f"img{i}", "text", word, x=10, y=y + 6, w=30, h=16, st=style(size=4.2, bold=True, align="center")))
        els.append(el(f"p{i}", "text", prompt(word), x=44, y=y + 4, w=54, h=16, st=style(size=6.2, bold=True, valign="middle")))
        els.append(el(f"line{i}", "answer_line", "", x=104, y=y + 18, w=86, h=12, st=style(size=3.0), metadata={"lines": 1}))
    sol = []
    header(sol, subject, "Soluzioni per il docente", "Risposte e indicazioni", page_no=2, total=2)
    sol.append(el("list", "text", "Parole: " + ", ".join(items[:6]) + ".", x=10, y=48, w=190, h=40, st=style(size=4.2)))
    sol.append(el("note", "note", "Obiettivo: consolidare lettura e scrittura. Accettare risposte corrette anche con grafia non convenzionale; leggere ad alta voce se necessario.", x=10, y=100, w=190, h=30, st=style(size=3.6, color=MUTED, italic=True)))
    return base_spec(pid, title, desc, "Materiali didattici", "worksheet", tax("schede", "italiano", "primaria", desc, ["italiano", "lettura", "scrittura"], "base", False, True, featured), interactive=False, orientation="portrait", pages=[page("home", title, els, home=True), page("soluzioni", "Soluzioni", sol)])


def science_sheet(pid, title, desc, categories, animals, featured=False):
    els = []
    header(els, "Scienze", title, "Classe seconda · classifica e motiva", page_no=1, total=2)
    instruction(els, "instr", "Osserva gli animali e scrivi il nome nella colonna giusta.", y=40, h=10)
    for i, animal in enumerate(animals[:9]):
        symbol(els, f"a{i}", animal, 10 + (i % 9) * 21, 54, 20, 24, size=2.5)
    for i, (name, color) in enumerate(categories):
        x = 10 + i * 65
        els.append(el(f"h{i}", "text", name, x=x, y=86, w=60, h=9, st=style(size=3.8, color="#ffffff", background=ACCENT, radius=2, align="center", valign="middle", bold=True)))
        els.append(el(f"b{i}", "text_box", "", x=x, y=97, w=60, h=130, st=style(size=3.0, background=color, border_color=BORDER, border_width=0.5, radius=2.5, padding=2)))
        for j in range(3):
            els.append(el(f"l{i}-{j}", "answer_line", "", x=x + 2, y=103 + j * 41, w=56, h=36, st=style(size=3.0), metadata={"lines": 2}))
    sol = []
    header(sol, "Scienze", "Soluzioni per il docente", "Classificazione e errori comuni", page_no=2, total=2)
    for i, (name, color) in enumerate(categories):
        sol.append(el(f"s{i}", "text", name, x=10, y=48 + i * 22, w=190, h=16, st=style(size=4.6, bold=True)))
    sol.append(el("note", "note", "Obiettivo: classificare in base a caratteristiche osservabili. Errori comuni: confondere animali che nuotano con i pesci. Verificare insieme le caratteristiche.", x=10, y=130, w=190, h=34, st=style(size=3.6, color=MUTED, italic=True)))
    return base_spec(pid, title, desc, "Materiali didattici", "worksheet", tax("schede", "scienze", "primaria", desc, ["scienze", "classificazione", "animali"], "intermedio", False, True, featured), interactive=False, orientation="portrait", pages=[page("home", title, els, home=True), page("soluzioni", "Soluzioni", sol)])


def game_cards(pid, title, desc, cards, sub, kind="game", interactive=True, featured=False):
    els = []
    header(els, "Gioco", title, "Ritaglia le carte, mescola e gioca secondo la regola indicata.", page_no=1, total=1, footer_y=196)
    for i, label in enumerate(cards[:12]):
        els.append(el(f"c{i}", "symbol", label, row=i // 4, column=i % 4, st=style(size=3.2, border_color=BORDER, border_width=0.5, radius=2, padding=1, align="center", valign="bottom"), symbol=label, action={"type": "speak"} if interactive else None, spoken_text=label))
    return base_spec(pid, title, desc, "Giochi e attività interattive", kind, tax("giochi", sub, "primaria", desc, ["gioco", sub, "carte"], "base", interactive), interactive=interactive, orientation="landscape", pages=[page("home", title, els, layout="grid", grid={"columns": 4, "rows": 3, "gap_mm": 4, "origin_x_mm": 10, "origin_y_mm": 44, "content_width_mm": 277, "content_height_mm": 140, "align": "left", "valign": "top"}, home=True)])


# ------------------------------------------------------------------ build


def main():
    specs = []
    tables = [
        ("gen2_caa_scuola", "Comunicazione: scuola", "Sedici parole per la giornata scolastica.", ["io", "maestra", "compagno", "aula", "quaderno", "matita", "leggere", "scrivere", "intervallo", "pranzo", "giocare", "aiuto", "bagno", "zaino", "campana", "casa"]),
        ("gen2_caa_casa", "Comunicazione: casa", "Parole per comunicare in casa.", ["mamma", "papà", "fratello", "sorella", "cucina", "camera da letto", "bagno", "divano", "letto", "mangiare", "dormire", "guardare", "aiuto", "riposo", "giocare", "casa"]),
        ("gen2_caa_cibo", "Comunicazione: cibo", "Cibi e bevande per scegliere.", ["mela", "banana", "pane", "pasta", "riso", "latte", "acqua", "succo", "biscotto", "formaggio", "mangiare", "bere", "ancora", "finito", "buono", "freddo"]),
        ("gen2_caa_animali", "Comunicazione: animali", "Animali e azioni per parlarne.", ["cane", "gatto", "uccello", "pesce", "cavallo", "mucca", "pecora", "topo", "mangiare", "bere", "giocare", "dormire", "accarezzare", "guardare", "grande", "piccolo"]),
        ("gen2_caa_emozioni", "Comunicazione: emozioni", "Parole per dire come mi sento.", ["felice", "triste", "arrabbiato", "preoccupato", "spaventato", "calmo", "stanco", "sorpreso", "abbraccio", "riposo", "parlare", "aiuto", "respirare", "ascoltare", "bene", "male"]),
        ("gen2_caa_corpo", "Comunicazione: corpo", "Parti del corpo e bisogni fisici.", ["testa", "occhio", "orecchio", "naso", "bocca", "mano", "pancia", "piede", "dolore", "caldo", "freddo", "bagno", "acqua", "riposare", "medico", "aiuto"]),
        ("gen2_caa_vestiti", "Comunicazione: vestiti", "Vestiti e azioni per vestirsi.", ["maglietta", "pantaloni", "scarpe", "calze", "giacca", "cappello", "vestirsi", "svestirsi", "allacciare", "aprire", "chiudere", "caldo", "freddo", "grande", "piccolo", "bello"]),
        ("gen2_caa_trasporti", "Comunicazione: trasporti", "Mezzi e spostamenti.", ["macchina", "autobus", "treno", "aereo", "bicicletta", "camminare", "andare", "fermare", "aspettare", "salire", "scendere", "casa", "scuola", "lontano", "vicino", "aiuto"]),
        ("gen2_caa_gioco", "Comunicazione: giochi", "Giochi e scelte per il tempo libero.", ["giocare", "palla", "puzzle", "musica", "libro", "disegnare", "costruire", "correre", "tablet", "fuori", "dentro", "ancora", "finito", "unire", "aspettare", "scegliere"]),
        ("gen2_caa_tempo", "Comunicazione: tempo e meteo", "Giorni, momenti e condizioni del tempo.", ["oggi", "domani", "ieri", "mattina", "pranzo", "pomeriggio", "notte", "sole", "pioggia", "nuvola", "vento", "freddo", "caldo", "adesso", "dopo", "prima"]),
    ]
    for args in tables:
        specs.append(caa_table(*args))

    specs.append(caa_notebook("gen2_quaderno_scuola", "Quaderno: scuola e casa", "Quaderno multipagina con quattro categorie.", [("scuola", "Scuola", ["maestra", "compagno", "aula", "quaderno", "matita", "leggere", "scrivere", "intervallo", "pranzo"]), ("casa", "Casa", ["mamma", "papà", "cucina", "camera da letto", "bagno", "divano", "letto", "mangiare", "dormire"]), ("giochi", ("Giochi", "giocare"), ["giocare", "palla", "puzzle", "musica", "libro", "disegnare", "correre", "tablet", "unire"]), ("bisogni", "Bisogni", ["aiuto", "riposo", "bagno", "acqua", "riposare", "ascoltare", "comprendere", "spazio", "abbraccio"])]))
    specs.append(caa_notebook("gen2_quaderno_emozioni", "Quaderno: emozioni e aiuto", "Quaderno per riconoscere e comunicare stati interni.", [("emozioni", "Emozioni", ["felice", "triste", "arrabbiato", "preoccupato", "calmo", "stanco", "sorpreso", "spaventato", "bene"]), ("corpo", "Corpo", ["testa", "mano", "pancia", "piede", "occhio", "orecchio", "dolore", "freddo", "caldo"]), ("aiuto", "Aiuto", ["aiuto", "aiutare", "comprendere", "ripetere", "ascoltare", "aspettare", "unire", "adulto", "protezione"]), ("strategie", ("Strategie", "calmarsi"), ["respirare", "riposo", "riposare", "abbraccio", "spazio", "parlare", "bere", "camminare", "ascoltare"])]))

    stories = [
        ("gen2_storia_cane_palla", "Il cane e la palla", "Il cane trova una palla e la porta al bambino.", [{"title": "Il cane trova", "hero": "cane", "words": ["cane", "trovare", "palla"], "sentence": "Il cane trova una palla nel giardino."}, {"title": "Il cane corre", "hero": "palla", "words": ["cane", "correre", "palla"], "sentence": "Il cane corre con la palla in bocca."}, {"title": "Il bambino gioca", "hero": "bambino", "words": ["bambino", "giocare", "cane"], "sentence": "Il bambino gioca con il cane."}]),
        ("gen2_storia_gatta_topo", "La gatta e il topo", "Una gatta curiosa incontra un topo.", [{"title": "La gatta vede", "hero": "gatto", "words": ["gatto", "vedere", "topo"], "sentence": "La gatta vede un topo vicino al muro."}, {"title": "Il topo corre", "hero": "topo", "words": ["topo", "correre", "casa"], "sentence": "Il topo corre verso la sua casa."}, {"title": "La gatta aspetta", "hero": "gatto", "words": ["gatto", "aspettare", "topo"], "sentence": "La gatta aspetta e poi torna a dormire."}]),
        ("gen2_storia_parco", "Un pomeriggio al parco", "Un bambino va al parco con la mamma.", [{"title": "Al parco", "hero": "bambino", "words": ["bambino", "andare", "parco"], "sentence": "Il bambino va al parco con la mamma."}, {"title": "Lo scivolo", "hero": "scivolo", "words": ["bambino", "salire", "scivolo"], "sentence": "Il bambino sale sullo scivolo e scende."}, {"title": "La merenda", "hero": "mela", "words": ["mamma", "mela", "mangiare"], "sentence": "La mamma dà una mela e insieme fanno merenda."}]),
        ("gen2_storia_farfalla", "La bambina e la farfalla", "Una farfalla colorata entra nel giardino.", [{"title": "La farfalla", "hero": "farfalla", "words": ["farfalla", "volare", "giardino"], "sentence": "Una farfalla vola nel giardino."}, {"title": "La bambina guarda", "hero": "bambina", "words": ["bambina", "guardare", "farfalla"], "sentence": "La bambina guarda la farfalla con attenzione."}, {"title": "Insieme", "hero": "fiore", "words": ["farfalla", "fiore", "unire"], "sentence": "La farfalla si posa su un fiore e resta un momento."}]),
        ("gen2_storia_pinguino", "Il pinguino e la neve", "Un pinguino piccolo scopre la neve.", [{"title": "La neve", "hero": "pinguino", "words": ["pinguino", "neve", "freddo"], "sentence": "Il pinguino guarda la neve che cade."}, {"title": "Il pinguino cammina", "hero": "pinguino", "words": ["pinguino", "camminare", "neve"], "sentence": "Il pinguino cammina piano sulla neve."}, {"title": "Insieme", "hero": "pinguino", "words": ["pinguino", "amico", "giocare"], "sentence": "Il pinguino incontra un amico e giocano insieme."}]),
        ("gen2_storia_gufo_luna", "Il gufo e la luna", "Un gufo veglia nella notte.", [{"title": "La notte", "hero": "gufo", "words": ["gufo", "notte", "luna"], "sentence": "Il gufo si sveglia quando arriva la notte."}, {"title": "La luna", "hero": "luna", "words": ["gufo", "guardare", "luna"], "sentence": "Il gufo guarda la luna nel cielo."}, {"title": "Il ritorno", "hero": "nido", "words": ["gufo", "andare", "nido"], "sentence": "Quando arriva il mattino il gufo torna al nido."}]),
    ]
    for args in stories:
        specs.append(caa_story(*args))

    socials = [
        ("gen2_sociale_primo_giorno", "Il primo giorno di scuola", "Anticipare il primo giorno in una nuova classe.", "Domani comincia la scuola.", [["Domani andrò a scuola.", "Conoscerò una nuova maestra e nuovi compagni."], ["La mattina saluterò la mia famiglia.", "Entrerò in classe e cercherò il mio posto."], ["Posso sentirmi emozionato o preoccupato.", "Posso chiedere aiuto a un adulto se non so dove andare."], ["Dopo la campanella la giornata continuerà.", "Posso portare un oggetto che mi aiuta a stare tranquillo."]], "Leggere la storia il giorno prima e rileggerla la mattina. Non promettere che tutto andrà come previsto; lasciare spazio alle domande."),
        ("gen2_sociale_medico", "La visita dal medico", "Prepararsi a una visita medica.", "Oggi andrò dal medico.", [["Oggi andrò dal medico con un adulto.", "In sala d'attesa aspetterò il mio turno."], ["Il medico potrà guardarmi la gola o ascoltarmi il respiro.", "Potrebbe chiedermi di aprire la bocca o di dire come mi sento."], ["Posso sentirmi nervoso.", "Posso chiedere una pausa o spiegare che cosa mi aiuta."], ["La visita dura poco e poi tornerò a casa.", "Posso fare domande al medico o all'adulto che è con me."]], "Non descrivere la visita come dolorosa se non è necessario. Chiedere al medico di spiegare i passaggi e concordare un segnale per la pausa."),
        ("gen2_sociale_rumore", "Un rumore forte", "Che cosa posso fare quando un rumore mi disturba.", "A volte sento un rumore forte.", [["A scuola possono esserci rumori forti.", "Per esempio la campanella, un trapano o un urlo improvviso."], ["Il rumore può farmi male alle orecchie o spaventarmi.", "Posso coprirmi le orecchie o allontanarmi se è sicuro."], ["Posso chiedere una pausa o usare le cuffie.", "Posso dire a un adulto che il rumore mi disturba."], ["Il rumore può finire dopo poco.", "Quando sto meglio posso tornare all'attività."]], "Offrire cuffie o un luogo tranquillo senza obbligare a sopportare il rumore. Verificare che la richiesta di pausa sia accettata."),
        ("gen2_sociale_turno", "Aspettare il proprio turno", "Allenarsi ad aspettare senza ansia.", "A volte devo aspettare il mio turno.", [["In classe o nel gioco può esserci una fila.", "Altre persone parlano o usano il materiale prima di me."], ["Aspettare può essere difficile.", "Posso guardare un timer, contare o tenere le mani occupate."], ["Posso chiedere quanto manca.", "Posso ricordare che il mio turno arriverà."], ["Dopo il mio turno potrò fare la stessa cosa.", "Se ho bisogno di aiuto posso chiamare un adulto."]], "Non usare la frase «devi aspettare» da sola: mostrare un timer e dare un tempo prevedibile. Riconoscere lo sforzo senza premiare la soppressione del disagio."),
        ("gen2_sociale_gita", "La gita al museo", "Anticipare una gita scolastica.", "Oggi andrò in gita al museo.", [["Andrò al museo con la classe e con un insegnante.", "Viaggerò in autobus e resterò vicino al gruppo."], ["Nel museo ci saranno opere, persone e regole.", "Non potrò toccare tutto e dovrò seguire le indicazioni."], ["Posso stancarmi o annoiarmi.", "Posso chiedere una pausa o guardare la mappa."], ["Dopo il museo tornerò a scuola.", "Posso raccontare che cosa ho visto."]], "Mostrare prima la mappa e le regole del museo. Concordare un punto di ritrovo e un adulto di riferimento."),
        ("gen2_sociale_compleanno", "Un compleanno affollato", "Prepararsi a una festa con molte persone.", "Oggi parteciperò a una festa di compleanno.", [["Ci saranno persone, musica e giochi.", "Non conosco tutti e potrei sentirmi confuso."], ["Posso stare vicino a una persona conosciuta.", "Posso chiedere una pausa o andare in un posto più tranquillo."], ["Potrei essere invitato a giocare o a cantare.", "Posso partecipare a modo mio, anche solo guardando."], ["La festa finisce e poi tornerò a casa.", "Posso raccontare che cosa mi è piaciuto."]], "Non obbligare a cantare, abbracciare o stare in mezzo al gruppo. Concordare un segnale per la pausa e un adulto di riferimento."),
    ]
    for args in socials:
        specs.append(social_story(*args))

    specs.append(agenda_weekly("gen2_agenda_settimana_scuola", "Settimana scolastica", "Cinque giorni con quattro attività ricorrenti.", [("scuola", "scuola"), ("pranzo", "pranzo"), ("giocare", "giocare"), ("compiti", "compiti")]))
    specs.append(agenda_weekly("gen2_agenda_settimana_casa", "Settimana a casa", "Cinque giorni con routine domestiche.", [("sveglia", "svegliare"), ("colazione", "far colazione"), ("giocare", "giocare"), ("letto", "dormire")]))
    specs.append(agenda_daily("gen2_agenda_giornata_infanzia", "La mia giornata all'infanzia", "Sequenza giornaliera per la scuola dell'infanzia.", [("8:00", "entrata", "scuola"), ("9:00", "gioco", "giocare"), ("10:30", "merenda", "mela"), ("11:30", "laboratorio", "disegnare"), ("12:30", "pranzo", "pranzo"), ("13:30", "riposo", "riposare"), ("15:30", "uscita", "casa")]))
    specs.append(agenda_first_then("gen2_agenda_prima_poi", "Prima lavoro, poi scelgo", "Struttura prima/poi per una transizione.", "compiti", "giocare"))
    specs.append(agenda_checklist("gen2_checklist_compiti", "Checklist dei compiti", "Controllo i passi prima di riporre lo zaino.", ["Apro il quaderno", "Leggo la consegna", "Faccio il primo esercizio", "Controllo le risposte", "Metto il quaderno nello zaino"]))
    specs.append(agenda_daily("gen2_agenda_gita", "La giornata della gita", "Anticipare orari e attività di una gita.", [("8:00", "ritrovo", "scuola"), ("8:30", "autobus", "autobus"), ("9:30", "museo", "museo"), ("12:00", "pranzo", "pranzo"), ("14:00", "laboratorio", "disegnare"), ("15:30", "ritorno", "scuola")]))

    specs.append(teacch_sequence("gen2_teacch_lettura", "Sequenza: leggere un testo", "Cinque passi per leggere e comprendere.", [("leggere", "Leggo il titolo e osservo le immagini."), ("guardare", "Leggo una frase alla volta."), ("comprendere", "Mi chiedo che cosa succede."), ("scrivere", "Rispondo alle domande."), ("controllare", "Rileggo e controllo.")]))
    specs.append(teacch_sequence("gen2_teacch_compito", "Sequenza: completare un compito", "Sistema di lavoro per un compito autonomo.", [("prendere", "Prendo il materiale."), ("leggere", "Leggo la consegna."), ("fare", "Faccio il primo passo."), ("controllare", "Controllo il lavoro."), ("ordinare", "Metto a posto.")]))
    specs.append(teacch_matching("gen2_teacch_abbinamento_animali", "Abbina gli animali", "Coppie di animali uguali.", [("cane", "cane"), ("gatto", "gatto"), ("uccello", "uccello"), ("pesce", "pesce"), ("cavallo", "cavallo")]))
    specs.append(teacch_sorting("gen2_teacch_classifica_cibi", "Classifica i cibi", "Frutta, verdure e bevande.", [("Frutta", ["mela", "banana", "pera"]), ("Verdure", ["carota", "insalata", "pomodoro"]), ("Bevande", ["acqua", "latte", "succo"])]))
    specs.append(teacch_cards("gen2_teacch_cartellini_routine", "Cartellini della routine", "Cartellini da ritagliare per la giornata.", ["sveglia", "colazione", "vestirsi", "scuola", "pranzo", "giocare", "compiti", "cena", "letto", "lavarsi", "ordinare", "riposo"]))
    specs.append(teacch_sequence("gen2_teacch_igiene", "Sequenza: lavarsi le mani", "Passi per l'igiene delle mani.", [("aprire", "Apro il rubinetto."), ("acqua", "Bagno le mani."), ("sapone", "Metto il sapone."), ("strofinare", "Strofino per venti secondi."), ("asciugare", "Risciacquo e asciugo.")]))

    specs.append(token_board("gen2_token_cinque", "Token board: cinque gettoni", "Cinque gettoni per un obiettivo concordato.", 5, True))
    specs.append(choice_board("gen2_scelta_due", "Scelgo tra due attività", "Due alternative visive.", ["giocare", "leggere"], 2))
    specs.append(break_card("gen2_pausa", "Carta pausa", "Chiedo una pausa e poi riprendo.", True))
    specs.append(emotion_scale("gen2_scale_emozioni", "Come mi sento?", "Scala visiva da 1 a 5 con strategie.", True))

    specs.append(counting_sheet("gen2_mate_conteggio10", "Conteggio entro il 10", "Conta gli oggetti e scrivi il numero.", [3, 5, 2, 7, 4, 6, 8, 9]))
    specs.append(math_sheet("gen2_mate_addizioni20", "Addizioni entro il 20", "Otto addizioni con risultato entro 20.", [(8, "+", 5, 13), (7, "+", 6, 13), (9, "+", 4, 13), (10, "+", 7, 17), (6, "+", 8, 14), (5, "+", 9, 14), (12, "+", 3, 15), (11, "+", 4, 15)], "Completa le addizioni. Puoi usare la linea dei numeri.", "MATEMATICA"))
    specs.append(math_sheet("gen2_mate_sottrazioni10", "Sottrazioni entro il 10", "Otto sottrazioni con risultato non negativo.", [(8, "-", 3, 5), (9, "-", 4, 5), (7, "-", 2, 5), (10, "-", 6, 4), (6, "-", 1, 5), (5, "-", 3, 2), (9, "-", 7, 2), (10, "-", 2, 8)], "Completa le sottrazioni. Conta all'indietro se ti serve.", "MATEMATICA"))
    specs.append(language_sheet("gen2_ita_vocali", "Le vocali", "Riconosci la vocale iniziale.", ["ape", "elefante", "isola", "oca", "uva", "albero"], lambda w: "___" + w[1:], "Scrivi la vocale iniziale di ogni parola.", True))
    specs.append(language_sheet("gen2_ita_sillabe", "Sillabe e parole", "Completa parole bisillabe.", ["cane", "sole", "mela", "luna", "pesce", "topo"], lambda w: w[:2] + "___", "Scrivi la sillaba che manca.", True))
    specs.append(language_sheet("gen2_ita_frasi", "Completa le frasi", "Usa la parola giusta per completare.", ["cane", "scuola", "mela", "casa", "acqua", "amico"], lambda w: "___", "Scrivi la parola che completa la frase.", True))
    specs.append(language_sheet("gen2_ita_contrari", "Sinonimi e contrari", "Trova la parola contraria.", ["grande", "caldo", "felice", "pieno", "aprire", "giorno"], lambda w: "contrario di " + w, "Scrivi il contrario di ogni parola.", True))
    specs.append(science_sheet("gen2_scienze_classifica", "Classifica gli animali", "Mammiferi, uccelli e pesci.", [("Mammiferi", ACCENT_SOFT), ("Uccelli", "#eff6ff"), ("Pesci", "#f0fdf4")], ["cane", "gatto", "cavallo", "rondine", "gufo", "pinguino", "squalo", "trota", "pesce"], True))
    specs.append(game_cards("gen2_memory_animali", "Memory degli animali", "Quattro coppie di animali.", ["cane", "cane", "gatto", "gatto", "pesce", "pesce", "uccello", "uccello"], "memory", featured=True))
    specs.append(game_cards("gen2_tombola_scuola", "Tombola della scuola", "Cartelle con nove parole della scuola.", ["scuola", "maestra", "quaderno", "matita", "zaino", "intervallo", "pranzo", "compagno", "aula"], "tombola"))
    specs.append(game_cards("gen2_domino_casa", "Domino della casa", "Tessere con oggetti di casa.", ["casa", "cucina", "camera da letto", "bagno", "divano", "letto", "mangiare", "dormire", "guardare"], "domino"))
    specs.append(game_cards("gen2_memory_emozioni", "Memory delle emozioni", "Coppie di espressioni emotive.", ["felice", "felice", "triste", "triste", "arrabbiato", "arrabbiato", "calmo", "calmo"], "memory"))

    # Flashcard e materiali stampabili
    specs.append(game_cards("gen2_flashcard_animali", "Flashcard: animali", "Nove flashcard con immagine e parola.", ["cane", "gatto", "uccello", "pesce", "cavallo", "mucca", "pecora", "topo", "farfalla"], "flashcard", kind="worksheet", interactive=False))
    specs.append(game_cards("gen2_flashcard_cibo", "Flashcard: cibo", "Nove flashcard per il lessico del cibo.", ["mela", "banana", "pane", "pasta", "latte", "acqua", "formaggio", "biscotto", "succo"], "flashcard", kind="worksheet", interactive=False))
    specs.append(game_cards("gen2_etichette_casa", "Etichette per la casa", "Etichette da stampare e applicare agli oggetti.", ["cucina", "camera da letto", "bagno", "divano", "letto", "finestra", "porta", "tavolo", "sedia"], "flashcard", kind="worksheet", interactive=False))
    specs.append(game_cards("gen2_poster_emozioni", "Poster delle emozioni", "Poster con nove emozioni e parole.", ["felice", "triste", "arrabbiato", "preoccupato", "calmo", "stanco", "sorpreso", "spaventato", "bene"], "flashcard", kind="worksheet", interactive=False))

    for spec in specs:
        if spec["id"].startswith("gen2_flashcard"):
            spec["category"] = "Materiali stampabili"
            spec["kind"] = "printable"
            spec["taxonomy"].update({"category_id": "stampabili", "subcategory_id": "flashcard"})
        elif spec["id"].startswith("gen2_etichette"):
            spec["category"] = "Materiali stampabili"
            spec["kind"] = "printable"
            spec["taxonomy"].update({"category_id": "stampabili", "subcategory_id": "etichette"})
        elif spec["id"].startswith("gen2_poster"):
            spec["category"] = "Materiali stampabili"
            spec["kind"] = "printable"
            spec["taxonomy"].update({"category_id": "stampabili", "subcategory_id": "poster"})
        save(spec)
    print(f"{len(specs)} specifiche del secondo lotto scritte in {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
