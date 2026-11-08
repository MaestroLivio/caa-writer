#!/usr/bin/env python3
"""Genera le specifiche dei dodici modelli pilota della nuova biblioteca.

Le specifiche sono dati versionati: il documento finale viene costruito dalla
CLI (`caa-cli template build`), non da questo script. Lo script non disegna
nulla: compone strutture (pagine, elementi, celle, azioni) secondo il design
system condiviso.
"""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "templates" / "sources" / "templates" / "piloti"

INK = "#1f2937"
MUTED = "#475569"
ACCENT = "#0f766e"
ACCENT_SOFT = "#ecfdf5"
BORDER = "#94a3b8"
PAPER = "#ffffff"
WARM = "#f59e0b"
ROSE = "#be123c"
BLUE = "#1d4ed8"

BASE_PAGE = {
    "page_size": "A4",
    "orientation": "portrait",
    "margin_mm": 10,
    "license": "CC BY-NC-SA 4.0",
    "source_type": "template",
}


def style(
    size=4.4,
    color=INK,
    background="",
    border_color="",
    border_width=0.0,
    radius=2.0,
    padding=1.2,
    align="left",
    valign="top",
    bold=False,
    italic=False,
):
    return {
        "font_family": "Arial",
        "font_size": size,
        "color": color,
        "background": background,
        "border_color": border_color,
        "border_width": border_width,
        "border_radius": radius,
        "padding_mm": padding,
        "align": align,
        "valign": valign,
        "bold": bold,
        "italic": italic,
    }


def el(
    eid,
    kind="text",
    text="",
    x=10,
    y=10,
    w=190,
    h=8,
    st=None,
    symbol=None,
    concept=None,
    action=None,
    metadata=None,
    absolute=False,
    **extra,
):
    item = {
        "id": eid,
        "kind": kind,
        "text": text,
        "x_mm": x,
        "y_mm": y,
        "w_mm": w,
        "h_mm": h,
        "style": st or style(),
    }
    if symbol is not None:
        item["symbol"] = symbol
    if concept is not None:
        item["concept"] = concept
    if action is not None:
        item["action"] = action
    if metadata is not None:
        item["metadata"] = metadata
    if absolute:
        item["absolute"] = True
    item.update(extra)
    return item


def header(elements, subject, title, subtitle, page_no=None, total=None, footer_y=284):
    elements.append(
        el(
            f"hdr-badge-{page_no or 1}",
            "badge",
            subject.upper(),
            x=10,
            y=8,
            w=34,
            h=7,
            st=style(size=3.1, color="#ffffff", background=ACCENT, radius=2.0, align="center", valign="middle", bold=True),
            absolute=True,
        )
    )
    elements.append(
        el(
            f"hdr-title-{page_no or 1}",
            "title",
            title,
            x=10,
            y=17,
            w=190,
            h=12,
            st=style(size=8.6, bold=True, valign="middle"),
            absolute=True,
        )
    )
    elements.append(
        el(
            f"hdr-sub-{page_no or 1}",
            "subtitle",
            subtitle,
            x=10,
            y=30,
            w=190,
            h=6,
            st=style(size=4.0, color=MUTED),
            absolute=True,
        )
    )
    if page_no:
        elements.append(
            el(
                f"footer-{page_no}",
                "text",
                f"CAA Writer · CC BY-NC-SA 4.0 · pagina {page_no}" + (f" di {total}" if total else ""),
                x=10,
                y=footer_y,
                w=190,
                h=5,
                st=style(size=2.8, color=MUTED),
                absolute=True,
            )
        )


def instruction(elements, eid, text, y=38, h=8, size=4.3):
    elements.append(
        el(
            eid,
            "instruction",
            text,
            x=10,
            y=y,
            w=190,
            h=h,
            st=style(size=size, color=ACCENT, bold=True),
        )
    )


def symbol(elements, eid, label, x, y, w, h, size=3.6, action=None, metadata=None, symbol_label=None, symbol_id=None, caption=None):
    elements.append(
        el(
            eid,
            "symbol",
            label if caption is None else caption,
            x=x,
            y=y,
            w=w,
            h=h,
            st=style(size=size, align="center", valign="bottom"),
            symbol=symbol_label or label,
            symbol_id=symbol_id,
            action=action,
            metadata=metadata,
        )
    )


def page(pid, title, elements, layout="absolute", grid=None, home=False, notes=""):
    data = {
        "id": pid,
        "title": title,
        "layout": layout,
        "home": home,
        "elements": elements,
    }
    if grid:
        data["grid"] = grid
    if notes:
        data["notes"] = notes
    return data


def base_spec(pid, title, description, category, kind, taxonomy, interactive=False, orientation="portrait", pages=None):
    return {
        "id": pid,
        "title": title,
        "description": description,
        "category": category,
        "source_type": "template",
        "kind": kind,
        "interactive": interactive,
        "license": "CC BY-NC-SA 4.0",
        "page_size": "A4",
        "orientation": orientation,
        "margin_mm": 10,
        "taxonomy": taxonomy,
        "pages": pages or [],
    }


# --------------------------------------------------------------------- 1. CAA 4x4
def pilot_caa_4x4():
    els = []
    header(els, "Comunicazione", "Comunicazione — 16 parole essenziali", "Tocca o indica un pittogramma, poi costruisci la frase.", page_no=1, total=1, footer_y=196)
    els.append(
        el(
            "msg-strip",
            "text_box",
            "La mia frase: ________________________________________________",
            x=10,
            y=38,
            w=277,
            h=11,
            st=style(size=4.4, background="#f8fafc", border_color=BORDER, border_width=0.5, radius=2.5, padding=2.0, valign="middle"),
            absolute=True,
        )
    )
    words = [
        ("io", "io"), ("tu", "tu"), ("volere", "volere"), ("avere", "avere"),
        ("andare", "andare"), ("fare", "fare"), ("bene", "bene"), ("male", "male"),
        ("aiuto", "aiuto"), ("pausa", "riposo"), ("mangiare", "mangiare"), ("bere", "bere"),
        ("giocare", "giocare"), ("casa", "casa"), ("scuola", "scuola"), ("amico", "amico"),
    ]
    for index, (label, symbol_label) in enumerate(words):
        els.append(
            el(
                f"cella-{index+1}",
                "symbol",
                label,
                row=index // 4,
                column=index % 4,
                st=style(size=4.0, border_color=BORDER, border_width=0.5, radius=3.0, padding=1.4, align="center", valign="bottom"),
                symbol=symbol_label,
                action={"type": "append"},
                spoken_text=label,
            )
        )
    grid = {
        "columns": 4,
        "rows": 4,
        "gap_mm": 3.0,
        "origin_x_mm": 10,
        "origin_y_mm": 54,
        "content_width_mm": 277,
        "content_height_mm": 136,
        "align": "left",
        "valign": "top",
    }
    taxonomy = {
        "category_id": "aac",
        "subcategory_id": "tabelle",
        "audience": "primaria",
        "objective": "Esprimere bisogni, azioni e preferenze con 16 parole ad alta frequenza.",
        "tags": ["comunicazione", "vocabolario", "scuola", "bisogni", "frase"],
        "complexity": "base",
        "needs_symbols": True,
        "printable": True,
        "customizable": True,
        "featured": True,
    }
    return base_spec(
        "pilota_tabella_caa_4x4",
        "Tabella CAA 4 × 4",
        "Tabella di comunicazione con sedici parole essenziali, barra della frase e celle modificabili.",
        "Tabelle comunicative",
        "communication_grid",
        taxonomy,
        interactive=True,
        orientation="landscape",
        pages=[page("home", "Comunicazione", els, layout="grid", grid=grid, home=True)],
    )


# ------------------------------------------------------- 2. Quaderno multipagina
def pilot_quaderno():
    def category_page(pid, title, intro, words, next_pages):
        els = []
        header(els, "Comunicazione", title, intro, page_no=next_pages["index"], total=10)
        for i, entry in enumerate(words):
            label, symbol_label = entry if isinstance(entry, tuple) else (entry, entry)
            els.append(
                el(
                    f"{pid}-w{i+1}",
                    "symbol",
                    label,
                    row=i // 3,
                    column=i % 3,
                    st=style(size=4.0, border_color=BORDER, border_width=0.5, radius=3.0, padding=1.4, align="center", valign="bottom"),
                    symbol=symbol_label,
                    action={"type": "append"},
                    spoken_text=label,
                )
            )
        els.append(
            el(
                f"{pid}-home",
                "button",
                "⌂ Home",
                x=10,
                y=274,
                w=36,
                h=9,
                st=style(size=3.6, background=ACCENT_SOFT, border_color=ACCENT, border_width=0.5, radius=2.0, align="center", valign="middle", bold=True),
                action={"type": "home"},
                spoken_text="Home",
                absolute=True,
            )
        )
        els.append(
            el(
                f"{pid}-back",
                "button",
                "‹ Indietro",
                x=50,
                y=274,
                w=36,
                h=9,
                st=style(size=3.6, background="#f8fafc", border_color=BORDER, border_width=0.5, radius=2.0, align="center", valign="middle"),
                action={"type": "back"},
                spoken_text="Indietro",
                absolute=True,
            )
        )
        grid = {
            "columns": 3,
            "rows": 4,
            "gap_mm": 3.0,
            "origin_x_mm": 10,
            "origin_y_mm": 42,
            "content_width_mm": 190,
            "content_height_mm": 218,
            "align": "left",
            "valign": "top",
        }
        return page(pid, title, els, layout="grid", grid=grid)

    home_els = []
    header(home_els, "Comunicazione", "Il mio quaderno", "Scegli una pagina. Ogni parola può essere pronunciata o aggiunta alla frase.", page_no=1, total=10)
    sections = [
        ("famiglia", "Famiglia", "famiglia"),
        ("scuola", "Scuola", "scuola"),
        ("casa", "Casa", "casa"),
        ("cibo", "Cibo", "cibo"),
        ("giochi", "Giochi", "giocare"),
        ("emozioni", "Emozioni", "emozioni"),
        ("bisogni", "Bisogni", "bisogno"),
        ("corpo", "Corpo", "corpo"),
        ("aiuto", "Aiuto", "aiuto"),
    ]
    for i, (pid, label, symbol_label) in enumerate(sections):
        home_els.append(
            el(
                f"home-{pid}",
                "symbol",
                label,
                row=i // 3,
                column=i % 3,
                st=style(size=4.2, border_color=ACCENT, border_width=0.6, radius=3.0, padding=1.4, align="center", valign="bottom", bold=True),
                symbol=symbol_label,
                action={"type": "navigate", "target": pid},
                spoken_text=label,
            )
        )
    home_grid = {
        "columns": 3,
        "rows": 4,
        "gap_mm": 3.0,
        "origin_x_mm": 10,
        "origin_y_mm": 42,
        "content_width_mm": 190,
        "content_height_mm": 218,
        "align": "left",
        "valign": "top",
    }
    pages = [page("home", "Il mio quaderno", home_els, layout="grid", grid=home_grid, home=True)]
    pages.append(category_page("famiglia", "Famiglia", "Persone della famiglia e azioni con loro.", ["mamma", "papà", "nonna", "nonno", "fratello", "sorella", "abbracciare", "aiutare", "parlare"], {"index": 2}))
    pages.append(category_page("scuola", "Scuola", "Persone, luoghi e materiali della scuola.", ["scuola", "maestra", "compagno", ("classe", "aula"), "quaderno", "matita", "leggere", "scrivere", "intervallo"], {"index": 3}))
    pages.append(category_page("cibo", "Cibo", "Cibi e bevande, per scegliere o rifiutare.", ["mela", "pane", "pasta", "acqua", "latte", "banana", "mangiare", "bere", "biscotto"], {"index": 4}))
    pages.append(category_page("bisogni", "Bisogni", "Che cosa mi serve adesso.", ["aiuto", ("pausa", "riposo"), "bagno", "acqua", "riposare", "ascoltare", ("capire", "comprendere"), "spazio", "abbraccio"], {"index": 5}))
    pages.append(category_page("giochi", "Giochi", "Giochi e attività da scegliere.", ["giocare", "puzzle", "palla", "musica", "libro", "disegnare", "costruire", "correre", "tablet"], {"index": 6}))
    pages.append(category_page("emozioni", "Emozioni", "Come mi sento e che cosa posso fare.", ["felice", "triste", "arrabbiato", "preoccupato", "calmo", "stanco", "respirare", "abbraccio", "parlare"], {"index": 7}))
    pages.append(category_page("casa", "Casa", "Stanze, oggetti e azioni di casa.", ["casa", "cucina", ("camera", "camera da letto"), "bagno", "divano", "letto", "mangiare", "dormire", "guardare"], {"index": 8}))
    pages.append(category_page("corpo", "Corpo", "Parti del corpo e bisogni fisici.", ["testa", "mano", "pancia", "piede", "occhio", "orecchio", "dolore", "freddo", "caldo"], {"index": 9}))
    pages.append(category_page("aiuto", "Aiuto", "Chiedere aiuto e spiegare che cosa serve.", ["aiuto", "aiutare", ("capire", "comprendere"), "ripetere", "ascoltare", "aspettare", ("insieme", "unire"), "adulto", ("sicuro", "protezione")], {"index": 10}))
    taxonomy = {
        "category_id": "aac",
        "subcategory_id": "quaderni",
        "audience": "primaria",
        "objective": "Offrire un vocabolario organizzato per comunicare in casa e a scuola.",
        "tags": ["quaderno", "comunicazione", "categorie", "navigazione", "famiglia"],
        "complexity": "intermedio",
        "needs_symbols": True,
        "printable": True,
        "customizable": True,
        "featured": True,
    }
    return base_spec(
        "pilota_quaderno_comunicazione",
        "Quaderno di comunicazione",
        "Quaderno multipagina con sei sezioni, navigazione Home/Indietro e parole pronunciabili.",
        "Sistemi comunicativi",
        "communication_board",
        taxonomy,
        interactive=True,
        orientation="portrait",
        pages=pages,
    )


# --------------------------------------------------------- 3. Agenda settimanale
def pilot_agenda():
    els = []
    header(els, "Organizzazione", "La mia settimana", "Guarda il giorno, poi l'attività. Spunta ciò che hai finito.", page_no=1, total=2, footer_y=196)
    days = ["lunedì", "martedì", "mercoledì", "giovedì", "venerdì"]
    for i, day in enumerate(days):
        els.append(
            el(
                f"day-{i}",
                "text",
                day,
                row=0,
                column=i,
                st=style(size=4.2, color="#ffffff", background=ACCENT, radius=2.0, align="center", valign="middle", bold=True),
            )
        )
    routine = [
        ("scuola", "scuola"), ("pranzo", "pranzo"), ("giocare", "giocare"), ("compiti", "compiti"),
    ]
    for day_index in range(5):
        for activity_index, (label, symbol_label) in enumerate(routine):
            els.append(
                el(
                    f"act-{day_index}-{activity_index}",
                    "symbol",
                    label,
                    row=activity_index + 1,
                    column=day_index,
                    st=style(size=3.4, border_color=BORDER, border_width=0.4, radius=2.0, padding=1.0, align="center", valign="bottom"),
                    symbol=symbol_label,
                    action={"type": "speak"},
                    spoken_text=f"{days[day_index]} {label}",
                )
            )
    grid = {
        "columns": 5,
        "rows": 5,
        "gap_mm": 2.5,
        "origin_x_mm": 10,
        "origin_y_mm": 42,
        "content_width_mm": 277,
        "content_height_mm": 232,
        "row_heights": [0.55, 1.0, 1.0, 1.0, 1.0],
        "align": "left",
        "valign": "top",
    }
    page1 = page("home", "La mia settimana", els, layout="grid", grid=grid, home=True)
    # Pagina 2: cartellini da ritagliare.
    els2 = []
    header(els2, "Organizzazione", "Cartellini delle attività", "Ritaglia i cartellini e usali per cambiare o completare la settimana.", page_no=2, total=2, footer_y=196)
    cut = ["sveglia", "colazione", "scuola", "pranzo", "giocare", "compiti", "cena", "letto"]
    for i, label in enumerate(cut):
        els2.append(
            el(
                f"cut-{i}",
                "symbol",
                label,
                row=i // 4,
                column=i % 4,
                st=style(size=3.4, border_color=BORDER, border_width=0.5, radius=2.0, padding=1.0, align="center", valign="bottom"),
                symbol=label,
                action={"type": "speak"},
                spoken_text=label,
            )
        )
    grid2 = {
        "columns": 4,
        "rows": 2,
        "gap_mm": 4.0,
        "origin_x_mm": 10,
        "origin_y_mm": 44,
        "content_width_mm": 190,
        "content_height_mm": 100,
        "align": "left",
        "valign": "top",
    }
    page2 = page("ritagli", "Cartellini", els2, layout="grid", grid=grid2)
    taxonomy = {
        "category_id": "agende",
        "subcategory_id": "settimanale",
        "audience": "primaria",
        "objective": "Rendere prevedibile la settimana scolastica e mostrare i cambiamenti.",
        "tags": ["agenda", "settimana", "routine", "scuola", "transizioni"],
        "complexity": "base",
        "needs_symbols": True,
        "printable": True,
        "customizable": True,
        "featured": True,
    }
    return base_spec(
        "pilota_agenda_settimanale",
        "Agenda visiva settimanale",
        "Cinque giorni con quattro attività ricorrenti, più cartellini da ritagliare per i cambiamenti.",
        "Agende e calendari visivi",
        "visual_schedule",
        taxonomy,
        interactive=True,
        orientation="landscape",
        pages=[page1, page2],
    )


# --------------------------------------------------------- 4. Sequenza TEACCH
def pilot_teacch():
    els = []
    header(els, "TEACCH", "Il mio lavoro di oggi", "Segui i passi. Spunta ogni attività quando è finita.", page_no=1, total=2)
    questions = [
        ("Che cosa devo fare?", "Cinque attività, una alla volta."),
        ("Quanto lavoro devo fare?", "Cinque passi. Li vedi tutti qui sotto."),
        ("Come so di aver finito?", "Tutte le caselle sono spuntate."),
        ("Che cosa succede dopo?", "Scelgo una pausa o un gioco."),
    ]
    for i, (question, answer) in enumerate(questions):
        x = 10 + (i % 2) * 96
        y = 42 + (i // 2) * 23
        els.append(
            el(
                f"q-{i}",
                "text_box",
                f"{question}\n{answer}",
                x=x,
                y=y,
                w=92,
                h=20,
                st=style(size=3.5, background=ACCENT_SOFT, border_color=ACCENT, border_width=0.4, radius=2.5, padding=1.8, valign="top"),
            )
        )
    steps = [
        ("1", "leggere", "Leggo la consegna."),
        ("2", "scrivere", "Faccio la prima parte."),
        ("3", "disegnare", "Completo con un disegno."),
        ("4", "controllare", "Controllo il lavoro."),
        ("5", "ordinare", "Metto a posto i materiali."),
    ]
    start_y = 94
    for i, (number, symbol_label, text) in enumerate(steps):
        y = start_y + i * 32
        els.append(
            el(
                f"step-{i}-num",
                "badge",
                number,
                x=10,
                y=y,
                w=10,
                h=10,
                st=style(size=5.0, color="#ffffff", background=ACCENT, radius=5.0, align="center", valign="middle", bold=True),
            )
        )
        symbol(els, f"step-{i}-sym", symbol_label, 24, y - 4, 28, 28, size=3.4)
        els.append(
            el(
                f"step-{i}-text",
                "text",
                text,
                x=56,
                y=y + 2,
                w=110,
                h=10,
                st=style(size=4.4, valign="middle"),
            )
        )
        els.append(
            el(
                f"step-{i}-done",
                "checkbox",
                "Fatto",
                x=170,
                y=y,
                w=30,
                h=10,
                st=style(size=3.4, valign="middle"),
            )
        )
    els.append(
        el(
            "finish",
            "badge",
            "FINITO → pausa",
            x=56,
            y=258,
            w=100,
            h=12,
            st=style(size=4.6, color="#ffffff", background=ACCENT, radius=3.0, align="center", valign="middle", bold=True),
        )
    )
    page1 = page("home", "Il mio lavoro", els, home=True)
    # Pagina 2: materiali da ritagliare.
    els2 = []
    header(els2, "TEACCH", "Materiali da ritagliare", "Ritaglia i cartellini e mettili nella sequenza del lavoro.", page_no=2, total=2)
    for i, (number, symbol_label, text) in enumerate(steps):
        els2.append(
            el(
                f"cut-{i}",
                "text_box",
                f"{number}. {text}",
                x=10 + (i % 2) * 96,
                y=44 + (i // 2) * 42,
                w=90,
                h=34,
                st=style(size=3.8, border_color=BORDER, border_width=0.5, radius=2.5, padding=1.8, valign="top"),
            )
        )
        symbol(els2, f"cut-{i}-sym", symbol_label, 12 + (i % 2) * 96, 66 + (i // 2) * 42, 24, 22, size=3.0)
    page2 = page("materiali", "Cartellini del lavoro", els2)
    taxonomy = {
        "category_id": "teacch",
        "subcategory_id": "sequenze-lavoro",
        "audience": "primaria",
        "objective": "Rendere visibili che cosa fare, quanto lavoro, come riconoscere la fine e cosa viene dopo.",
        "tags": ["teacch", "lavoro", "autonomia", "sequenza", "inizio", "fine"],
        "complexity": "intermedio",
        "needs_symbols": True,
        "printable": True,
        "customizable": True,
        "featured": True,
    }
    return base_spec(
        "pilota_teacch_sequenza",
        "Sequenza TEACCH di cinque attività",
        "Sistema di lavoro con quattro domande, cinque passi, caselle di completamento e cartellini.",
        "Materiali TEACCH",
        "teacch",
        taxonomy,
        interactive=False,
        orientation="portrait",
        pages=[page1, page2],
    )


# --------------------------------------------------------- 5. Token board
def pilot_token_board():
    els = []
    header(els, "Supporti", "La mia token board", "Tocca un gettone quando completi un'attività. Alla fine scegli il rinforzo.", page_no=1, total=1)
    els.append(
        el(
            "goal",
            "text_box",
            "Obiettivo: finire le 5 attività della giornata.",
            x=10,
            y=40,
            w=190,
            h=12,
            st=style(size=4.6, background=ACCENT_SOFT, border_color=ACCENT, border_width=0.5, radius=2.5, padding=2.0, valign="middle", bold=True),
        )
    )
    for i in range(5):
        els.append(
            el(
                f"token-{i}",
                "token_slot",
                "",
                x=12 + i * 38,
                y=62,
                w=32,
                h=32,
                st=style(size=3.2, border_color=ACCENT, border_width=0.8, align="center", valign="middle"),
                action={"type": "toggle"},
                spoken_text="gettone",
                metadata={"filled": False, "index": i + 1},
            )
        )
    els.append(
        el(
            "reward",
            "text_box",
            "Quando ho 5 gettoni posso scegliere: puzzle, musica o tablet.",
            x=10,
            y=106,
            w=190,
            h=14,
            st=style(size=4.0, background="#fffbeb", border_color=WARM, border_width=0.6, radius=2.5, padding=2.0, valign="middle"),
        )
    )
    els.append(
        el(
            "reward-title",
            "title",
            "Scegli il rinforzo (concordato con un adulto)",
            x=10,
            y=126,
            w=190,
            h=8,
            st=style(size=4.4, bold=True, color=ACCENT),
        )
    )
    for i, (label, symbol_label) in enumerate([("puzzle", "puzzle"), ("musica", "musica"), ("tablet", "tablet")]):
        symbol(els, f"reward-choice-{i}", label, 20 + i * 58, 138, 46, 42, size=3.4)
    activities = [("leggere", "leggere"), ("scrivere", "scrivere"), ("disegnare", "disegnare"), ("controllare", "controllare"), ("riordinare", "ordinare")]
    for i, (label, symbol_label) in enumerate(activities):
        symbol(els, f"act-{i}", symbol_label, 12 + i * 38, 190, 32, 32, size=3.2)
        els.append(
            el(
                f"act-{i}-check",
                "checkbox",
                "",
                x=12 + i * 38,
                y=226,
                w=32,
                h=8,
                st=style(size=3.0, align="center"),
            )
        )
    els.append(
        el(
            "reset-note",
            "note",
            "La token board non è un premio per «essere bravo»: serve a rendere visibile un percorso concordato e può essere modificata.",
            x=10,
            y=250,
            w=190,
            h=20,
            st=style(size=3.2, color=MUTED, italic=True),
        )
    )
    taxonomy = {
        "category_id": "comportamentali",
        "subcategory_id": "token",
        "audience": "primaria",
        "objective": "Rendere visibile l'avanzamento verso un obiettivo concordato, senza giudicare la persona.",
        "tags": ["token board", "gettoni", "rinforzo", "autonomia", "pausa"],
        "complexity": "base",
        "needs_symbols": True,
        "printable": True,
        "customizable": True,
        "featured": True,
    }
    return base_spec(
        "pilota_token_board",
        "Token board interattiva",
        "Cinque gettoni selezionabili, attività da spuntare e scelta del rinforzo, con nota metodologica.",
        "Materiali ABA e supporti visivi",
        "aba",
        taxonomy,
        interactive=True,
        orientation="portrait",
        pages=[page("home", "Token board", els, home=True)],
    )


# ------------------------------------------------------- 6. Addizioni entro il 10
def pilot_addizioni():
    els = []
    header(els, "Matematica", "Addizioni entro il 10", "Classe prima · Scheda di calcolo con appoggio visivo", page_no=1, total=2)
    instruction(els, "instr", "Completa ogni addizione. Puoi aiutarti con la linea dei numeri.", y=40, h=8)
    els.append(
        el(
            "number-line",
            "number_line",
            "",
            x=10,
            y=52,
            w=190,
            h=18,
            st=style(size=3.0, color=INK),
            metadata={"min": 0, "max": 10, "step": 1},
        )
    )
    operations = [(2, 1), (3, 2), (4, 3), (1, 4), (5, 2), (6, 3), (2, 7), (8, 1), (4, 5), (7, 2)]
    for i, (a, b) in enumerate(operations):
        col = i % 2
        row = i // 2
        x = 10 + col * 96
        y = 74 + row * 30
        els.append(
            el(
                f"op-{i}",
                "question",
                f"{a} + {b} =",
                x=x,
                y=y,
                w=52,
                h=22,
                st=style(size=7.8, bold=True, valign="middle", align="right"),
            )
        )
        els.append(
            el(
                f"op-{i}-answer",
                "text_box",
                "",
                x=x + 58,
                y=y + 3,
                w=28,
                h=16,
                st=style(size=7.4, border_color=BORDER, border_width=0.6, radius=2.5, align="center", valign="middle"),
            )
        )
    els.append(
        el(
            "example-title",
            "title",
            "Aiutino: composizione del numero",
            x=10,
            y=230,
            w=190,
            h=9,
            st=style(size=4.6, bold=True, color=ACCENT),
        )
    )
    for i, (a, b) in enumerate([(4, 3), (6, 2)]):
        x = 10 + i * 90
        els.append(
            el(
                f"frame-{i}",
                "ten_frame",
                "",
                x=x,
                y=242,
                w=36,
                h=16,
                st=style(size=3.0, border_color=BORDER, border_width=0.4),
                metadata={"filled": a + b},
            )
        )
        els.append(
            el(
                f"frame-{i}-label",
                "text",
                f"{a} + {b} = {a+b}",
                x=x + 40,
                y=244,
                w=34,
                h=12,
                st=style(size=3.8, valign="middle"),
            )
        )
    # Soluzioni
    els2 = []
    header(els2, "Matematica", "Soluzioni per il docente", "Scheda «Addizioni entro il 10» · risposte e indicazioni", page_no=2, total=2)
    instruction(els2, "instr2", "Obiettivo: calcolare addizioni entro il 10 con supporto visivo. Errori comuni: contare due volte l'addendo iniziale.", y=40, h=12, size=3.9)
    lines = [f"{a} + {b} = {a+b}" for a, b in operations]
    for i, line in enumerate(lines):
        els2.append(
            el(
                f"sol-{i}",
                "text",
                line,
                x=10 + (i % 2) * 96,
                y=64 + (i // 2) * 16,
                w=88,
                h=12,
                st=style(size=5.0, bold=True, valign="middle"),
            )
        )
    els2.append(
        el(
            "sol-note",
            "note",
            "Accettare anche risposte corrette con rappresentazioni diverse (dita, gettoni, linea dei numeri). La scheda è modificabile: cambiare gli addendi non altera le soluzioni finché si aggiorna la pagina docente.",
            x=10,
            y=156,
            w=190,
            h=30,
            st=style(size=3.6, color=MUTED, italic=True),
        )
    )
    taxonomy = {
        "category_id": "schede",
        "subcategory_id": "matematica",
        "audience": "primaria",
        "objective": "Calcolare addizioni entro il 10 e collegare numero, quantità e linea dei numeri.",
        "tags": ["matematica", "addizioni", "calcolo", "classe prima", "linea dei numeri"],
        "complexity": "base",
        "needs_symbols": False,
        "printable": True,
        "customizable": True,
        "featured": True,
    }
    return base_spec(
        "pilota_addizioni_10",
        "Addizioni entro il 10",
        "Dieci addizioni con linea dei numeri, dieci frame e pagina delle soluzioni per il docente.",
        "Materiali didattici",
        "worksheet",
        taxonomy,
        interactive=False,
        orientation="portrait",
        pages=[page("home", "Addizioni entro il 10", els, home=True), page("soluzioni", "Soluzioni", els2)],
    )


# ------------------------------------------------------ 7. Scheda sillabica
def pilot_sillabe():
    els = []
    header(els, "Italiano", "Sillabe e immagini", "Classe prima · Leggi, scegli la sillaba, completa la parola", page_no=1, total=2)
    instruction(els, "instr", "Osserva l'immagine. Cerchia la sillaba giusta e scrivila nella riga.", y=40, h=8)
    items = [
        ("cane", "___NE", ["CA", "CO", "CU"], "CA"),
        ("sole", "SO___", ["LE", "LA", "LO"], "LE"),
        ("mela", "ME___", ["LA", "LU", "LO"], "LA"),
        ("luna", "LU___", ["NA", "NE", "NO"], "NA"),
        ("pesce", "PE___", ["SCE", "SCO", "SCI"], "SCE"),
        ("topo", "TO___", ["PO", "PA", "PE"], "PO"),
    ]
    start_y = 54
    for i, (label, prompt, choices, correct) in enumerate(items):
        y = start_y + i * 38
        symbol(els, f"item-{i}-img", label, 10, y, 30, 30, size=3.0)
        els.append(
            el(
                f"item-{i}-word",
                "text",
                prompt,
                x=44,
                y=y + 4,
                w=40,
                h=12,
                st=style(size=7.2, bold=True, valign="middle"),
            )
        )
        for j, choice in enumerate(choices):
            els.append(
                el(
                    f"item-{i}-choice-{j}",
                    "checkbox",
                    choice,
                    x=88 + j * 26,
                    y=y + 5,
                    w=24,
                    h=10,
                    st=style(size=3.6, valign="middle"),
                    metadata={"correct": choice == correct, "group": f"item-{i}"},
                )
            )
        els.append(
            el(
                f"item-{i}-line",
                "answer_line",
                "",
                x=44,
                y=y + 22,
                w=146,
                h=8,
                st=style(size=3.0, border_color=BORDER),
                metadata={"lines": 1},
            )
        )
    # Soluzioni
    els2 = []
    header(els2, "Italiano", "Soluzioni e note", "Scheda «Sillabe e immagini»", page_no=2, total=2)
    answers = ["CA", "LE", "LA", "NA", "SCE", "PO"]
    for i, answer in enumerate(answers):
        els2.append(
            el(
                f"sol-{i}",
                "text",
                f"{items[i][0]}: {answer}" + (f" → {items[i][0]}" if False else ""),
                x=10 + (i % 2) * 96,
                y=48 + (i // 2) * 18,
                w=88,
                h=12,
                st=style(size=4.8, bold=True, valign="middle"),
            )
        )
    els2.append(
        el(
            "note",
            "note",
            "Obiettivo: riconoscere la sillaba iniziale o finale e completare parole bisillabe. Errori comuni: confondere la forma scritta della sillaba con il suono. Accettare la lettura ad alta voce come risposta.",
            x=10,
            y=120,
            w=190,
            h=32,
            st=style(size=3.6, color=MUTED, italic=True),
        )
    )
    taxonomy = {
        "category_id": "schede",
        "subcategory_id": "italiano",
        "audience": "primaria",
        "objective": "Riconoscere sillabe e completare parole bisillabe con il supporto dell'immagine.",
        "tags": ["italiano", "sillabe", "lettura", "classe prima", "pregrafismo"],
        "complexity": "base",
        "needs_symbols": True,
        "printable": True,
        "customizable": True,
        "featured": True,
    }
    return base_spec(
        "pilota_sillabe_immagini",
        "Sillabe e immagini",
        "Sei parole bisillabe con immagine, tre scelte e riga di risposta; soluzioni per il docente.",
        "Materiali didattici",
        "worksheet",
        taxonomy,
        interactive=False,
        orientation="portrait",
        pages=[page("home", "Sillabe e immagini", els, home=True), page("soluzioni", "Soluzioni", els2)],
    )


# --------------------------------------------------- 8. Comprensione del testo
def pilot_comprensione():
    els = []
    header(els, "Italiano", "Leggo e comprendo", "Classe seconda · Una storia breve e sei domande", page_no=1, total=2)
    instruction(els, "instr", "Leggi la storia due volte. Poi rispondi con una crocetta o con una frase.", y=40, h=8)
    story = (
        "Lina ha un gatto che si chiama Nuvola. Nuvola dorme sul divano tutto il giorno. "
        "Quando Lina torna da scuola, Nuvola la aspetta alla porta. Lina gli dà da mangiare e poi giocano con un gomitolo rosso. "
        "Una sera il gomitolo finisce sotto il tavolo. Nuvola lo cerca e Lina lo aiuta."
    )
    els.append(
        el(
            "story",
            "text_box",
            story,
            x=10,
            y=52,
            w=190,
            h=44,
            st=style(size=4.2, background="#f8fafc", border_color=BORDER, border_width=0.5, radius=2.5, padding=2.2, valign="top"),
        )
    )
    els.append(
        el(
            "q-title",
            "title",
            "Rispondi",
            x=10,
            y=102,
            w=190,
            h=8,
            st=style(size=5.2, bold=True, color=ACCENT),
        )
    )
    multiple = [
        ("1. Come si chiama il gatto?", ["Nuvola", "Lina", "Rosso"], "Nuvola"),
        ("2. Dove dorme Nuvola?", ["A scuola", "Sul divano", "Sotto il tavolo"], "Sul divano"),
        ("3. Che cosa fa Lina quando torna da scuola?", ["Dà da mangiare al gatto", "Va a dormire", "Legge un libro"], "Dà da mangiare al gatto"),
        ("4. Di che colore è il gomitolo?", ["Blu", "Verde", "Rosso"], "Rosso"),
    ]
    for i, (question, options, correct) in enumerate(multiple):
        y = 114 + i * 25
        els.append(
            el(
                f"mc-{i}-q",
                "question",
                question,
                x=10,
                y=y,
                w=190,
                h=7,
                st=style(size=4.0, bold=True),
            )
        )
        for j, option in enumerate(options):
            els.append(
                el(
                    f"mc-{i}-opt-{j}",
                    "checkbox",
                    option,
                    x=14 + j * 62,
                    y=y + 8,
                    w=58,
                    h=8,
                    st=style(size=3.4),
                    metadata={"correct": option == correct, "group": f"mc-{i}"},
                )
            )
    els.append(
        el(
            "open-title",
            "title",
            "Scrivi tu",
            x=10,
            y=218,
            w=190,
            h=8,
            st=style(size=5.2, bold=True, color=ACCENT),
        )
    )
    els.append(
        el(
            "open-1",
            "question",
            "5. Che cosa fa Nuvola quando Lina torna da scuola?",
            x=10,
            y=230,
            w=190,
            h=7,
            st=style(size=3.8, bold=True),
        )
    )
    els.append(el("open-1-line", "answer_line", "", x=10, y=238, w=190, h=14, st=style(size=3.0), metadata={"lines": 2}))
    els.append(
        el(
            "open-2",
            "question",
            "6. Perché Lina aiuta Nuvola?",
            x=10,
            y=256,
            w=190,
            h=7,
            st=style(size=3.8, bold=True),
        )
    )
    els.append(el("open-2-line", "answer_line", "", x=10, y=264, w=190, h=14, st=style(size=3.0), metadata={"lines": 2}))
    # Soluzioni
    els2 = []
    header(els2, "Italiano", "Soluzioni per il docente", "Scheda «Leggo e comprendo»", page_no=2, total=2)
    answers = [
        "1. Nuvola", "2. Sul divano", "3. Dà da mangiare al gatto", "4. Rosso",
        "5. Nuvola aspetta Lina alla porta.", "6. Perché il gomitolo è finito sotto il tavolo e Lina vuole aiutare il gatto a trovarlo.",
    ]
    for i, answer in enumerate(answers):
        els2.append(
            el(
                f"sol-{i}",
                "text",
                answer,
                x=10,
                y=44 + i * 18,
                w=190,
                h=14,
                st=style(size=4.0, valign="middle"),
            )
        )
    els2.append(
        el(
            "note",
            "note",
            "Obiettivo: comprendere un testo narrativo breve, individuare informazioni esplicite e inferire un motivo. Per la risposta 6 accettare formulazioni equivalenti. Errori comuni: confondere il nome del gatto con il colore del gomitolo.",
            x=10,
            y=160,
            w=190,
            h=28,
            st=style(size=3.6, color=MUTED, italic=True),
        )
    )
    taxonomy = {
        "category_id": "schede",
        "subcategory_id": "italiano",
        "audience": "primaria",
        "objective": "Comprendere un testo narrativo breve e rispondere a domande esplicite e inferenziali.",
        "tags": ["italiano", "comprensione", "lettura", "classe seconda", "domande"],
        "complexity": "intermedio",
        "needs_symbols": False,
        "printable": True,
        "customizable": True,
        "featured": True,
    }
    return base_spec(
        "pilota_comprensione_testo",
        "Comprensione del testo",
        "Storia breve, quattro domande a scelta multipla, due domande aperte e soluzioni commentate.",
        "Materiali didattici",
        "worksheet",
        taxonomy,
        interactive=False,
        orientation="portrait",
        pages=[page("home", "Leggo e comprendo", els, home=True), page("soluzioni", "Soluzioni", els2)],
    )


# ------------------------------------------------------ 9. Storia sociale
def pilot_storia_sociale():
    def story_page(pid, title, sentences, symbols, page_no, color=ACCENT_SOFT):
        els = []
        header(els, "Storia", title, "Una storia da leggere insieme, una frase alla volta.", page_no=page_no, total=6)
        for i, sentence in enumerate(sentences):
            y = 44 + i * 76
            if i < len(symbols):
                entry = symbols[i]
                label, symbol_label = entry if isinstance(entry, tuple) else (entry, entry)
                symbol(els, f"{pid}-sym-{i}", label, 10, y, 58, 58, size=3.6, symbol_label=symbol_label)
            els.append(
                el(
                    f"{pid}-text-{i}",
                    "text_box",
                    sentence,
                    x=74,
                    y=y + 2,
                    w=126,
                    h=54,
                    st=style(size=4.8, background=color, border_color=BORDER, border_width=0.4, radius=2.5, padding=2.4, valign="middle"),
                )
            )
        return page(pid, title, els, home=False)

    pages = []
    cover = []
    header(cover, "Storia", "Quando la routine cambia", "Una storia per capire che cosa può succedere quando la giornata cambia.", page_no=1, total=6)
    symbol(cover, "cover-sym", "scuola", 55, 64, 100, 100, size=4.0)
    cover.append(
        el(
            "cover-note",
            "note",
            "Storia descrittiva e di anticipazione. Non è una Social Story certificata secondo i criteri 10.4: la revisione dell'adulto è necessaria.",
            x=10,
            y=180,
            w=190,
            h=30,
            st=style(size=3.4, color=MUTED, italic=True),
        )
    )
    pages.append(page("home", "Copertina", cover, home=True))
    pages.append(
        story_page(
            "contesto",
            "Dove sono",
            [
                "Sono a scuola, nella mia classe.",
                "Oggi l'insegnante potrebbe cambiare l'ordine delle attività.",
                "La campanella può suonare in un momento diverso dal solito.",
            ],
            ["scuola", "maestra", ("campanella", "campana")],
            2,
        )
    )
    pages.append(
        story_page(
            "succede",
            "Che cosa può succedere",
            [
                "È possibile che la lezione di musica venga dopo il pranzo.",
                "È possibile che un'attività duri più o meno del previsto.",
                "Non sempre conosco in anticipo che cosa cambierà.",
            ],
            ["musica", "pranzo", "orologio"],
            3,
            color="#f8fafc",
        )
    )
    pages.append(
        story_page(
            "sento",
            "Come posso sentirmi",
            [
                "Posso sentirmi sorpreso o preoccupato.",
                "Posso sentirmi arrabbiato se avevo programmato qualcosa.",
                "Posso anche sentirmi curioso di sapere che cosa succederà.",
            ],
            ["sorpreso", "arrabbiato", "curioso"],
            4,
            color="#fff7ed",
        )
    )
    pages.append(
        story_page(
            "faccio",
            "Che cosa posso fare",
            [
                "Posso chiedere all'insegnante che cosa cambia.",
                "Posso guardare l'agenda visiva e cercare il nuovo ordine.",
                "Posso chiedere una pausa e poi riprendere l'attività.",
            ],
            ["chiedere", "agenda", ("pausa", "riposo")],
            5,
            color=ACCENT_SOFT,
        )
    )
    final = []
    header(final, "Storia", "Che cosa può succedere dopo", "La giornata può cambiare e poi continuare.", page_no=6, total=6)
    symbol(final, "final-sym", "insegnante", 65, 60, 80, 80, size=4.0)
    final.append(
        el(
            "final-text",
            "text_box",
            "Dopo il cambiamento la giornata continua. Posso chiedere aiuto e avere informazioni. "
            "Le persone della scuola possono spiegarmi che cosa succede. Se la storia non mi aiuta, "
            "possiamo provare un altro supporto, per esempio una sequenza visiva o una conversazione con un adulto.",
            x=10,
            y=152,
            w=190,
            h=52,
            st=style(size=4.4, background="#f8fafc", border_color=BORDER, border_width=0.4, radius=2.5, padding=2.4, valign="top"),
        )
    )
    final.append(
        el(
            "adult-note",
            "note",
            "Nota per l'adulto: usare frasi brevi, verificare con la persona che la storia sia comprensibile e non presentare il testo come una prescrizione. Aggiornare i dettagli quando la situazione cambia.",
            x=10,
            y=214,
            w=190,
            h=28,
            st=style(size=3.3, color=MUTED, italic=True),
        )
    )
    pages.append(page("finale", "Conclusione", final))
    taxonomy = {
        "category_id": "storie-sociali",
        "subcategory_id": "anticipazione",
        "audience": "primaria",
        "objective": "Anticipare un cambiamento di routine, riconoscere emozioni possibili e offrire alternative.",
        "tags": ["storia sociale", "anticipazione", "routine", "scuola", "emozioni", "cambiamento"],
        "complexity": "intermedio",
        "needs_symbols": True,
        "printable": True,
        "customizable": True,
        "featured": True,
    }
    return base_spec(
        "pilota_storia_sociale",
        "Quando la routine cambia",
        "Storia illustrata e multipagina per anticipare un cambiamento a scuola, con nota metodologica per l'adulto.",
        "Storie sociali",
        "social_story",
        taxonomy,
        interactive=False,
        orientation="portrait",
        pages=pages,
    )


# ------------------------------------------------------- 10. Libro CAA
def pilot_libro_caa():
    def book_page(pid, title, sentence, words, symbol_label, page_no):
        els = []
        header(els, "Libro CAA", title, f"Pagina {page_no - 1} di 4", page_no=page_no, total=6)
        symbol(els, f"{pid}-big", symbol_label, 55, 40, 100, 110, size=4.0, caption="")
        # Striscia di pittogrammi sopra il testo: una cella per unità di significato.
        cells = []
        for word in words:
            label, symbol_label = word if isinstance(word, tuple) else (word, word)
            cells.append({
                "text": label,
                "symbol": symbol_label,
                "action": {"type": "speak"},
                "spoken_text": label,
            })
        els.append(
            el(
                f"{pid}-strip",
                "grid",
                "",
                x=10,
                y=156,
                w=190,
                h=40,
                st=style(size=3.6, border_color=BORDER, border_width=0.4, radius=2.0, padding=1.0),
                columns=len(cells),
                rows=1,
                gap_mm=2.0,
                cells=cells,
            )
        )
        els.append(
            el(
                f"{pid}-sentence",
                "text_box",
                sentence,
                x=10,
                y=204,
                w=190,
                h=30,
                st=style(size=5.2, background="#f8fafc", border_color=BORDER, border_width=0.4, radius=2.5, padding=2.5, align="center", valign="middle", bold=True),
            )
        )
        return page(pid, title, els, home=pid == "home")

    pages = []
    cover = []
    header(cover, "Libro CAA", "Il gatto e la farfalla", "Un libro con pittogrammi, testo e una domanda finale.", page_no=1, total=6)
    symbol(cover, "cover-cat", "gatto", 40, 58, 60, 60, size=3.8)
    symbol(cover, "cover-butterfly", "farfalla", 110, 58, 60, 60, size=3.8)
    cover.append(
        el(
            "cover-line",
            "text_box",
            "Leggi la storia. Puoi toccare i pittogrammi per ascoltare le parole.",
            x=30,
            y=132,
            w=150,
            h=20,
            st=style(size=4.0, background=ACCENT_SOFT, border_color=ACCENT, border_width=0.4, radius=2.5, padding=2.0, align="center", valign="middle"),
        )
    )
    pages.append(page("home", "Copertina", cover, home=True))
    pages.append(book_page("p1", "Il gatto vede", "Il gatto vede una farfalla.", ["gatto", "vedere", "farfalla"], "gatto", 2))
    pages.append(book_page("p2", "La farfalla vola", "La farfalla vola sopra il prato.", ["farfalla", "volare", "prato"], "farfalla", 3))
    pages.append(book_page("p3", "Il gatto salta", "Il gatto salta per giocare.", ["gatto", "saltare", "giocare"], "gatto", 4))
    pages.append(book_page("p4", "Insieme", "Il gatto e la farfalla restano insieme.", ["gatto", "farfalla", ("insieme", "unire")], "farfalla", 5))
    questions = []
    header(questions, "Libro CAA", "Domande", "Rispondi con una frase o con i pittogrammi.", page_no=6, total=6)
    for i, question in enumerate([
        "Che cosa vede il gatto?",
        "Dove vola la farfalla?",
        "Che cosa fa il gatto per giocare?",
    ]):
        y = 46 + i * 54
        questions.append(
            el(
                f"q-{i}",
                "question",
                question,
                x=10,
                y=y,
                w=190,
                h=8,
                st=style(size=4.6, bold=True),
            )
        )
        questions.append(el(f"q-{i}-line", "answer_line", "", x=10, y=y + 12, w=190, h=22, st=style(size=3.0), metadata={"lines": 2}))
    pages.append(page("domande", "Domande", questions))
    taxonomy = {
        "category_id": "storie-caa",
        "subcategory_id": "sopra-testo",
        "audience": "primaria",
        "objective": "Leggere una storia breve in CAA, collegare pittogrammi e parole e rispondere a domande.",
        "tags": ["libro", "caa", "lettura", "storia", "pittogrammi", "comprensione"],
        "complexity": "intermedio",
        "needs_symbols": True,
        "printable": True,
        "customizable": True,
        "featured": True,
    }
    return base_spec(
        "pilota_libro_caa",
        "Il gatto e la farfalla",
        "Libro CAA di quattro pagine con striscia di pittogrammi, testo e domande di comprensione.",
        "Storie sociali",
        "story",
        taxonomy,
        interactive=False,
        orientation="portrait",
        pages=pages,
    )


# --------------------------------------------------- 11. Abbinamento interattivo
def pilot_abbinamento():
    els = []
    header(els, "Gioco", "Collega l'animale al suo ambiente", "Leggi le due colonne. Scrivi la lettera giusta nella casella.", page_no=1, total=2)
    instruction(els, "instr", "Ogni animale ha un ambiente. Scrivi la lettera dell'ambiente accanto all'animale.", y=40, h=8)
    animals = [("cane", "cane", "casa"), ("pesce", "pesce", "acqua"), ("uccello", "uccello", "nido"), ("cammello", "cammello", "deserto"), ("pinguino", "pinguino", "ghiaccio")]
    environments = [("A", "deserto", "deserto"), ("B", "casa", "casa"), ("C", "acqua", "acqua"), ("D", "ghiaccio", "ghiaccio"), ("E", "nido", "nido")]
    for i, (label, symbol_label, target) in enumerate(animals):
        y = 54 + i * 40
        symbol(els, f"animal-{i}", symbol_label, 10, y, 34, 34, size=3.4,
               symbol_id="arasaac.picto.2520" if symbol_label == "pesce" else None)
        els.append(
            el(
                f"animal-{i}-box",
                "text_box",
                "",
                x=94,
                y=y + 7,
                w=22,
                h=18,
                st=style(size=5.0, border_color=BORDER, border_width=0.6, radius=2.0, align="center", valign="middle"),
            )
        )
    for i, (letter, label, symbol_label) in enumerate(environments):
        y = 54 + i * 40
        symbol(els, f"env-{i}", symbol_label, 146, y, 34, 34, size=3.4)
        els.append(el(f"env-{i}-label", "text", f"{letter} · {label}", x=182, y=y + 9, w=26, h=14, st=style(size=3.2, bold=True, valign="middle")))
    page1 = page("home", "Collega", els, home=True)
    # Pagina interattiva: scelta dell'ambiente corretto per ogni animale.
    els2 = []
    header(els2, "Gioco", "Verifica: scegli l'ambiente", "Tocca l'ambiente giusto. Il materiale mostra subito se la scelta è corretta.", page_no=2, total=2)
    instruction(els2, "instr2", "Tocca una sola risposta per ogni animale.", y=40, h=8)
    for i, (label, symbol_label, target) in enumerate(animals):
        y = 52 + i * 43
        symbol(els2, f"q-{i}-animal", symbol_label, 10, y, 28, 28, size=3.0,
               symbol_id="arasaac.picto.2520" if symbol_label == "pesce" else None)
        els2.append(el(f"q-{i}-label", "text", label, x=42, y=y + 7, w=34, h=12, st=style(size=3.8, bold=True, valign="middle")))
        for j, (letter, option, option_symbol) in enumerate(environments):
            els2.append(
                el(
                    f"q-{i}-opt-{j}",
                    "checkbox",
                    f"{letter} {option}",
                    x=80 + j * 22,
                    y=y + 6,
                    w=21,
                    h=14,
                    st=style(size=2.9, valign="middle"),
                    action={"type": "toggle"},
                    metadata={"correct": option == target, "group": f"q-{i}", "feedback": f"{label}: {option}"},
                )
            )
    page2 = page("verifica", "Verifica", els2)
    taxonomy = {
        "category_id": "giochi",
        "subcategory_id": "associazioni",
        "audience": "primaria",
        "objective": "Associare animali e ambienti, motivando la scelta e verificandola con feedback.",
        "tags": ["gioco", "abbinamento", "associazioni", "animali", "ambienti", "scienze"],
        "complexity": "base",
        "needs_symbols": True,
        "printable": True,
        "customizable": True,
        "featured": True,
    }
    return base_spec(
        "pilota_abbinamento",
        "Collega l'animale al suo ambiente",
        "Scheda di abbinamento stampabile e verifica interattiva con feedback immediato.",
        "Giochi e attività interattive",
        "game",
        taxonomy,
        interactive=True,
        orientation="portrait",
        pages=[page1, page2],
    )


# ----------------------------------------------- 12. Classificazione scientifica
def pilot_classificazione():
    els = []
    header(els, "Scienze", "Classifica gli animali", "Classe seconda · Mammiferi, uccelli e pesci", page_no=1, total=2)
    instruction(els, "instr", "Osserva gli animali. Scrivi il nome nella colonna giusta: mammiferi, uccelli o pesci.", y=40, h=10)
    animals = ["cane", "gatto", "cavallo", "rondine", "gufo", "pinguino", "squalo", "trota", "pesce"]
    for i, label in enumerate(animals):
        symbol(els, f"bank-{i}", label, 10 + (i % 9) * 21, 54, 20, 24, size=2.6,
               symbol_id="arasaac.picto.2520" if label == "pesce" else None)
    columns = [
        ("Mammiferi", 10, ACCENT_SOFT),
        ("Uccelli", 75, "#eff6ff"),
        ("Pesci", 140, "#f0fdf4"),
    ]
    for i, (title, x, color) in enumerate(columns):
        els.append(
            el(
                f"col-{i}-title",
                "text",
                title,
                x=x,
                y=88,
                w=60,
                h=9,
                st=style(size=4.2, color="#ffffff", background=ACCENT, radius=2.0, align="center", valign="middle", bold=True),
            )
        )
        els.append(
            el(
                f"col-{i}-box",
                "text_box",
                "",
                x=x,
                y=99,
                w=60,
                h=128,
                st=style(size=3.0, background=color, border_color=BORDER, border_width=0.5, radius=2.5, padding=2.0),
            )
        )
        for line in range(3):
            els.append(
                el(
                    f"col-{i}-line-{line}",
                    "answer_line",
                    "",
                    x=x + 2,
                    y=104 + line * 40,
                    w=56,
                    h=34,
                    st=style(size=3.0, border_color=BORDER),
                    metadata={"lines": 2},
                )
            )
    els.append(
        el(
            "hint",
            "note",
            "Indizio: i mammiferi hanno pelo e allattano i piccoli; gli uccelli hanno piume e becco; i pesci respirano con le branchie.",
            x=10,
            y=232,
            w=190,
            h=24,
            st=style(size=3.4, color=MUTED, italic=True),
        )
    )
    els2 = []
    header(els2, "Scienze", "Soluzioni e note per il docente", "Scheda «Classifica gli animali»", page_no=2, total=2)
    sol = [
        "Mammiferi: cane, gatto, cavallo.",
        "Uccelli: rondine, gufo, pinguino.",
        "Pesci: squalo, trota, pesce.",
    ]
    for i, line in enumerate(sol):
        els2.append(el(f"sol-{i}", "text", line, x=10, y=48 + i * 18, w=190, h=14, st=style(size=4.8, bold=True, valign="middle")))
    els2.append(
        el(
            "note",
            "note",
            "Obiettivo: classificare gli animali in base a caratteristiche osservabili. Errori comuni: collocare il pinguino tra i pesci perché nuota, o lo squalo tra i mammiferi perché partorisce alcuni piccoli. Il pinguino è un uccello; lo squalo è un pesce. Accettare risposte motivate e verificare insieme le caratteristiche.",
            x=10,
            y=112,
            w=190,
            h=42,
            st=style(size=3.6, color=MUTED, italic=True),
        )
    )
    els2.append(
        el(
            "teacher-box",
            "text_box",
            "Adattamento facilitato: ridurre a sei animali (due per categoria) e fornire le etichette da ritagliare. "
            "Adattamento avanzato: aggiungere anfibi e rettili e chiedere un criterio per ogni gruppo.",
            x=10,
            y=166,
            w=190,
            h=34,
            st=style(size=3.8, background=ACCENT_SOFT, border_color=ACCENT, border_width=0.4, radius=2.5, padding=2.0, valign="top"),
        )
    )
    taxonomy = {
        "category_id": "schede",
        "subcategory_id": "scienze",
        "audience": "primaria",
        "objective": "Classificare animali in mammiferi, uccelli e pesci usando caratteristiche osservabili.",
        "tags": ["scienze", "classificazione", "animali", "mammiferi", "uccelli", "pesci"],
        "complexity": "intermedio",
        "needs_symbols": True,
        "printable": True,
        "customizable": True,
        "featured": True,
    }
    return base_spec(
        "pilota_classificazione_animali",
        "Classifica gli animali",
        "Nove animali da collocare in tre colonne, con indizi, soluzioni, errori comuni e adattamenti.",
        "Materiali didattici",
        "worksheet",
        taxonomy,
        interactive=False,
        orientation="portrait",
        pages=[page("home", "Classifica gli animali", els, home=True), page("soluzioni", "Soluzioni", els2)],
    )


BUILDERS = [
    pilot_caa_4x4,
    pilot_quaderno,
    pilot_agenda,
    pilot_teacch,
    pilot_token_board,
    pilot_addizioni,
    pilot_sillabe,
    pilot_comprensione,
    pilot_storia_sociale,
    pilot_libro_caa,
    pilot_abbinamento,
    pilot_classificazione,
]


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for builder in BUILDERS:
        spec = builder()
        path = OUT / f"{spec['id']}.json"
        path.write_text(json.dumps(spec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(path.relative_to(ROOT))
    print(f"{len(BUILDERS)} specifiche pilota scritte in {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
