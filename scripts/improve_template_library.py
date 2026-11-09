#!/usr/bin/env python3
"""Ricognizione e miglioramento della libreria di modelli.

Obiettivi:
- aggiungere a ogni modello un'intestazione leggibile (categoria, titolo,
  istruzione per l'adulto) e un piè di pagina con licenza;
- liberare l'area contenuti dalle intestazioni, così la griglia non parte a
  ridosso del bordo;
- completare la tassonomia (obiettivo, pubblico, tag, complessità) per la
  ricerca nella libreria;
- togliere i testi tecnici generati per errore («checklist», «verticale»);
- non toccare i modelli che hanno già un'intestazione completa.

Il formato di origine resta JSON versionato: i documenti `.caadoc` vengono
rigenerati da `caa-cli template build-all`.
"""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCES = ROOT / "templates" / "sources" / "templates"

CATEGORY_INFO = {
    "Materiali ABA e supporti visivi": {
        "badge": "ABA",
        "category_id": "aba",
        "subcategory_id": "supporti_visivi",
        "subcategory": "Supporti visivi ABA",
        "instruction": "Mostra il supporto, indica il passo e rinforza ogni risposta corretta.",
    },
    "Agende e calendari visivi": {
        "badge": "AGENDA",
        "category_id": "agenda",
        "subcategory_id": "agenda",
        "subcategory": "Agende e calendari",
        "instruction": "Anticipa la giornata: indica l'attività, spostala quando è conclusa e mostra quella successiva.",
    },
    "Autonomia personale": {
        "badge": "AUTONOMIA",
        "category_id": "autonomia",
        "subcategory_id": "routine",
        "subcategory": "Routine di autonomia",
        "instruction": "Segui la sequenza un passo alla volta; spunta o sposta il pittogramma quando il passo è concluso.",
    },
    "Tabelle comunicative": {
        "badge": "COMUNICAZIONE",
        "category_id": "aac",
        "subcategory_id": "tabelle",
        "subcategory": "Tabelle comunicative",
        "instruction": "Indica o tocca i pittogrammi per costruire la frase; le parole restano modificabili.",
    },
    "Materiali didattici": {
        "badge": "DIDATTICA",
        "category_id": "didattica",
        "subcategory_id": "schede",
        "subcategory": "Schede didattiche",
        "instruction": "Leggi la consegna, svolgi l'esercizio e verifica insieme il risultato.",
    },
    "Emozioni e autoregolazione": {
        "badge": "EMOZIONI",
        "category_id": "emozioni",
        "subcategory_id": "regolazione",
        "subcategory": "Emozioni e regolazione",
        "instruction": "Nomina l'emozione, indica l'intensità e scegli una strategia di regolazione.",
    },
    "Giochi e attività interattive": {
        "badge": "GIOCO",
        "category_id": "giochi",
        "subcategory_id": "attivita",
        "subcategory": "Giochi e attività",
        "instruction": "Gioca a turni: indica la carta, pronuncia la parola e verifica la risposta.",
    },
    "Sequenze e routine": {
        "badge": "SEQUENZA",
        "category_id": "sequenze",
        "subcategory_id": "sequenze",
        "subcategory": "Sequenze e routine",
        "instruction": "Metti in ordine i passi e verbalizza prima/dopo; poi esegui la sequenza.",
    },
    "Storie sociali": {
        "badge": "STORIA",
        "category_id": "storie",
        "subcategory_id": "storie_sociali",
        "subcategory": "Storie sociali",
        "instruction": "Leggi la storia insieme, soffermati sulle emozioni e sulle scelte del protagonista.",
    },
    "Materiali TEACCH": {
        "badge": "TEACCH",
        "category_id": "teacch",
        "subcategory_id": "strutturato",
        "subcategory": "Materiali TEACCH",
        "instruction": "Segui il sistema di lavoro: cosa fare, quanti, come so che ho finito, cosa dopo.",
    },
    "Materiali stampabili": {
        "badge": "STAMPA",
        "category_id": "stampabili",
        "subcategory_id": "velcro",
        "subcategory": "Materiali da plastificare",
        "instruction": "Stampa, plastifica e applica il velcro; ritaglia lungo le linee tratteggiate.",
    },
    "Sistemi comunicativi": {
        "badge": "SISTEMA",
        "category_id": "aac",
        "subcategory_id": "quaderni",
        "subcategory": "Quaderni di comunicazione",
        "instruction": "Sfoglia il quaderno, indica i pittogrammi e costruisci la frase.",
    },
}

TIP_BY_KEYWORD = [
    ("prima e poi", "Mostra PRIMA e POI, indica cosa succede e lascia scegliere il rinforzo."),
    ("prima", "Mostra PRIMA e POI, indica cosa succede e lascia scegliere il rinforzo."),
    ("token", "Consegna un gettone dopo ogni risposta o passo; quando la board è piena arriva il premio."),
    ("domino", "Distribuisci le tessere, gioca a turni e pronuncia l'immagine accoppiata."),
    ("storia", "Leggi la storia insieme e soffermati sulle emozioni e sulle scelte."),
    ("agenda", "Anticipa la giornata e sposta il pittogramma dell'attività conclusa."),
    ("sequenza", "Metti in ordine i passi e verbalizza prima/dopo."),
    ("velcro", "Stampa, plastifica e applica il velcro; ritaglia lungo le linee tratteggiate."),
    ("checklist", "Spunta ogni passo concluso e annota chi ha svolto l'attività."),
    ("abbin", "Abbina le coppie uguali e verbalizza ciò che vedi."),
    ("ordina", "Ordina dal più piccolo al più grande o secondo la sequenza."),
]

TECHNICAL_TEXTS = {"checklist", "verticale", "orizzontale", "griglia", "griglia libera"}


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def save(path: Path, spec: dict) -> None:
    path.write_text(json.dumps(spec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def title_of(spec: dict) -> str:
    return str(spec.get("title") or spec.get("name") or spec.get("id") or "").strip()


def item_labels(page: dict, limit: int = 10) -> list[str]:
    labels: list[str] = []
    for element in page.get("elements", []):
        ident = str(element.get("id") or "")
        if element.get("absolute") or ident.startswith(("hdr-", "footer-", "msg-")):
            continue
        text = str(element.get("text") or element.get("label") or "").strip()
        lowered = text.lower()
        if not text or lowered in TECHNICAL_TEXTS:
            continue
        if "caa writer" in lowered or lowered.startswith(("mostra ", "indica ", "leggi ", "segui ")):
            continue
        labels.append(text)
    return labels[:limit]


def instruction_for(spec: dict) -> str:
    category = str(spec.get("category") or "")
    base = CATEGORY_INFO.get(category, CATEGORY_INFO["Materiali didattici"])
    text = base["instruction"]
    lowered = title_of(spec).lower()
    description = str(spec.get("description") or "").lower()
    for keyword, tip in TIP_BY_KEYWORD:
        pattern = r"(?<![\w])" + re.escape(keyword) + r"(?![\w])"
        if re.search(pattern, lowered) or re.search(pattern, description):
            text = tip
            break
    if spec.get("printable"):
        if not text.endswith("."):
            text += "."
        text += " Stampa, plastifica e applica il velcro."
    return text


def badge_for(spec: dict) -> str:
    category = str(spec.get("category") or "")
    return CATEGORY_INFO.get(category, CATEGORY_INFO["Materiali didattici"])["badge"]


def page_dimensions(spec: dict) -> tuple[float, float]:
    size = str(spec.get("page_size") or "A4").upper()
    landscape = str(spec.get("orientation") or "portrait") == "landscape"
    sizes = {"A4": (210.0, 297.0), "A3": (297.0, 420.0), "A5": (148.0, 210.0), "LETTER": (216.0, 279.0)}
    width, height = sizes.get(size, sizes["A4"])
    if landscape:
        width, height = height, width
    return width, height


def text_element(element_id: str, kind: str, text: str, x: float, y: float, w: float, h: float,
                 font: float, color: str, *, bold: bool = False, align: str = "left",
                 background: str = "", border_color: str = "", border_width: float = 0.0,
                 border_style: str = "solid", italic: bool = False, radius: float = 2.0,
                 valign: str = "middle", absolute: bool = True) -> dict:
    return {
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
            "padding_mm": 1.2,
            "align": align,
            "valign": valign,
            "bold": bold,
            "italic": italic,
            "underline": False,
        },
        "absolute": absolute,
    }


def add_header(page: dict, spec: dict, width: float, height: float, margin: float,
               content_top: float) -> None:
    title = title_of(spec)
    badge = badge_for(spec)
    instruction = instruction_for(spec)
    # 1 mm di sicurezza: il validatore considera oltre il margine un elemento
    # che arriva esattamente sul bordo.
    usable = width - 2 * margin - 2
    page.setdefault("elements", []).extend([
        text_element(f"hdr-badge-{page.get('id','p1')}", "badge", badge, margin, margin - 2,
                     42, 7, 3.1, "#ffffff", bold=True, align="center",
                     background="#0f766e", radius=2.0),
        text_element(f"hdr-title-{page.get('id','p1')}", "title", title, margin, margin + 7,
                     usable, 12, 8.2, "#1f2937", bold=True),
        text_element(f"hdr-sub-{page.get('id','p1')}", "subtitle", instruction, margin, margin + 21,
                     usable, 7, 3.9, "#475569", italic=False, valign="top"),
        text_element(f"footer-{page.get('id','p1')}", "text",
                     "CAA Writer · CC BY-NC-SA 4.0 · materiale modificabile",
                     margin, height - margin - 5, usable, 4, 2.8, "#475569",
                     valign="top"),
    ])
    page["margin_mm"] = margin


def add_flow_header(page: dict, spec: dict) -> None:
    elements = page.setdefault("elements", [])
    title = title_of(spec)
    instruction = instruction_for(spec)
    # Rimuove i testi tecnici del generatore e li sostituisce con un'istruzione.
    cleaned = []
    for element in elements:
        text = str(element.get("text") or "").strip()
        ident = str(element.get("id") or "")
        if ident.endswith("-field-layout") or text.lower() in TECHNICAL_TEXTS:
            continue
        cleaned.append(element)
    page["elements"] = cleaned
    if not cleaned or not any(str(e.get("text") or "").strip() == title for e in cleaned):
        cleaned.insert(0, {
            "id": f"{page.get('id','p1')}-title",
            "kind": "text",
            "text": title,
            "style": {"font_size": 8, "bold": True, "color": "#1f2937"},
        })
    cleaned.insert(1, {
        "id": f"{page.get('id','p1')}-instruction",
        "kind": "text",
        "text": instruction,
        "style": {"font_size": 4.1, "color": "#475569", "italic": False},
    })
    page["flow"] = {"columns": 1, "gap_mm": 4}


def improve_taxonomy(spec: dict) -> None:
    category = str(spec.get("category") or "")
    info = CATEGORY_INFO.get(category, CATEGORY_INFO["Materiali didattici"])
    taxonomy = spec.setdefault("taxonomy", {})
    taxonomy.setdefault("category_id", info["category_id"])
    taxonomy.setdefault("subcategory_id", info["subcategory_id"])
    taxonomy.setdefault("subcategory", info["subcategory"])
    taxonomy.setdefault("audience", "primaria")
    taxonomy.setdefault("objective", title_of(spec))
    tags = list(taxonomy.get("tags") or [])
    for label in item_labels((spec.get("pages") or [{}])[0]):
        lowered = label.lower()
        if lowered not in [t.lower() for t in tags]:
            tags.append(lowered)
    taxonomy["tags"] = tags[:14]
    pages = spec.get("pages") or []
    elements = sum(len(p.get("elements", [])) for p in pages)
    taxonomy.setdefault("complexity", "articolato" if elements > 30 or len(pages) > 2 else "base")
    taxonomy.setdefault("needs_symbols", True)
    taxonomy.setdefault("printable", True)
    taxonomy.setdefault("customizable", True)
    taxonomy.setdefault("interactive", bool(spec.get("interactive")))
    spec["taxonomy"] = taxonomy


def improve_description(spec: dict) -> None:
    title = title_of(spec)
    current = str(spec.get("description") or "").strip()
    # Toglie la formula generata «N voci, tra cui ...» che non descrive nulla.
    current = re.sub(r"\s*\d+\s+voci,\s+tra cui.*?(?:\.|$)", ".", current, flags=re.IGNORECASE)
    current = re.sub(r"\s+", " ", current).strip(" .")
    instruction = instruction_for(spec)
    if not current:
        spec["description"] = f"{title}. {instruction}"
    elif instruction not in current:
        spec["description"] = f"{current.rstrip('.')}. {instruction}"
    else:
        spec["description"] = current


def has_full_header(page: dict) -> bool:
    """Intestazione completa: badge + titolo, oppure bad/mini-intestazione."""
    elements = page.get("elements", [])
    kinds = {str(element.get("kind") or "") for element in elements}
    ids = {str(element.get("id") or "") for element in elements}
    if "badge" in kinds and "title" in kinds:
        return True
    if any(identifier.startswith("hdr-badge-") for identifier in ids) and any(
        identifier.startswith("hdr-sub-") for identifier in ids
    ):
        return True
    if page.get("layout") == "flow" and any(identifier.endswith("-instruction") for identifier in ids):
        return True
    return False


def fix_footer_positions(page: dict, width: float, height: float, margin: float) -> None:
    """Riporta i piè di pagina dentro il margine inferiore.

    I modelli della seconda generazione avevano un footer a 196 mm su una
    pagina landscape da 210 mm: finiva 1 mm oltre il margine e il validatore
    lo segnalava come elemento fuori pagina.
    """
    for element in page.get("elements", []):
        ident = str(element.get("id") or "")
        if not ident.startswith("footer"):
            continue
        element["y_mm"] = round(height - margin - 6, 2)
        element["h_mm"] = min(float(element.get("h_mm") or 4), 4.0)
        width_mm = float(element.get("w_mm") or 0)
        if width_mm > 0:
            element["w_mm"] = round(min(width_mm, width - 2 * margin - 2), 2)


def dedupe_flow_instructions(page: dict) -> None:
    """Evita istruzioni duplicate se lo script viene rieseguito."""
    seen = False
    kept = []
    for element in page.get("elements", []):
        ident = str(element.get("id") or "")
        if ident.endswith("-instruction"):
            if seen:
                continue
            seen = True
        kept.append(element)
    page["elements"] = kept


def clean_generated_texts(page: dict, title: str) -> None:
    """Toglie titoli e testi tecnici che finirebbero dentro la griglia."""
    cleaned = []
    for element in page.get("elements", []):
        ident = str(element.get("id") or "")
        text = str(element.get("text") or element.get("label") or "").strip()
        if ident.endswith("-field-layout"):
            continue
        if text.lower() in TECHNICAL_TEXTS:
            continue
        if ident.endswith("-title") and text == title:
            continue
        if ident.startswith("hdr-sub-") or ident.startswith("footer-"):
            continue
        cleaned.append(element)
    page["elements"] = cleaned


def improve_spec(spec: dict) -> bool:
    pages = spec.get("pages") or []
    if not pages:
        return False
    changed = False
    width, height = page_dimensions(spec)
    margin = float(spec.get("margin_mm") or 10)
    spec["margin_mm"] = margin
    for page in pages:
        layout = str(page.get("layout") or "grid")
        dedupe_flow_instructions(page)
        fix_footer_positions(page, width, height, margin)
        if has_full_header(page):
            continue
        if layout == "flow":
            add_flow_header(page, spec)
            changed = True
            continue
        if layout == "grid":
            clean_generated_texts(page, title_of(spec))
            content_top = margin + 34
            if spec.get("category") == "Tabelle comunicative":
                # Striscia per comporre la frase sotto l'intestazione.
                page.setdefault("elements", []).append(
                    text_element(
                        f"msg-{page.get('id','p1')}",
                        "text_box",
                        "La mia frase: ________________________________________________",
                        margin,
                        content_top,
                        width - 2 * margin - 2,
                        9,
                        4.2,
                        "#1f2937",
                        background="#f8fafc",
                        border_color="#94a3b8",
                        border_width=0.5,
                        radius=2.5,
                        align="left",
                    )
                )
                content_top += 12
            add_header(page, spec, width, height, margin, content_top)
            grid = page.setdefault("grid", {})
            grid["origin_x_mm"] = margin
            grid["origin_y_mm"] = content_top
            grid["content_width_mm"] = width - 2 * margin - 2
            grid["content_height_mm"] = height - content_top - margin - 8
            grid["align"] = grid.get("align") or "left"
            grid["valign"] = grid.get("valign") or "top"
            page["margin_mm"] = margin
            changed = True
        elif layout == "absolute":
            # I modelli assoluti hanno già il titolo: aggiunge badge, istruzione
            # e piè di pagina senza spostare il contenuto.
            existing_ids = {str(e.get("id")) for e in page.get("elements", [])}
            title_element = next(
                (e for e in page.get("elements", []) if str(e.get("id", "")).endswith("-title")),
                None,
            )
            top = float(title_element.get("y_mm", margin)) if title_element else margin
            if f"hdr-badge-{page.get('id','p1')}" not in existing_ids:
                page.setdefault("elements", []).insert(0, text_element(
                    f"hdr-badge-{page.get('id','p1')}", "badge", badge_for(spec),
                    margin, max(2.0, top - 8), 42, 7, 3.1, "#ffffff", bold=True,
                    align="center", background="#0f766e", radius=2.0))
            if not any(str(e.get("id", "")).startswith("hdr-sub-") for e in page.get("elements", [])):
                page.setdefault("elements", []).append(text_element(
                    f"hdr-sub-{page.get('id','p1')}", "subtitle", instruction_for(spec),
                    margin, top + 20, width - 2 * margin - 2, 6, 3.9, "#475569", valign="top"))
            if not any(str(e.get("id", "")).startswith("footer-") for e in page.get("elements", [])):
                page.setdefault("elements", []).append(text_element(
                    f"footer-{page.get('id','p1')}", "text",
                    "CAA Writer · CC BY-NC-SA 4.0 · materiale modificabile",
                    margin, height - margin - 5, width - 2 * margin - 2, 4, 2.8, "#475569",
                    valign="top"))
            changed = True
    improve_description(spec)
    improve_taxonomy(spec)
    spec.setdefault("license", "CC BY-NC-SA 4.0")
    spec.setdefault("source_type", "template")
    return changed


def main() -> None:
    files = sorted(
        path for path in SOURCES.rglob("*.json") if not path.name.startswith("._")
    )
    changed = 0
    for path in files:
        spec = load(path)
        before = json.dumps(spec, ensure_ascii=False, sort_keys=True)
        improve_spec(spec)
        after = json.dumps(spec, ensure_ascii=False, sort_keys=True)
        if before != after:
            save(path, spec)
            changed += 1
    print(f"Modelli esaminati: {len(files)} · aggiornati: {changed}")


if __name__ == "__main__":
    main()
