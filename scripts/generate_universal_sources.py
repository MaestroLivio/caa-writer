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


def build_template_spec(template: dict) -> dict:
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
                "symbol": (cell.get("symbolLabel") or cell.get("label", "")).replace("_", " "),
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


def main() -> int:
    TEMPLATES.mkdir(parents=True, exist_ok=True)
    BOARDS.mkdir(parents=True, exist_ok=True)
    TPL_OUT.mkdir(parents=True, exist_ok=True)
    BOARD_OUT.mkdir(parents=True, exist_ok=True)
    # Rimuove solo le specifiche generate: le fonti manuali eventuali restano.
    for path in TPL_OUT.rglob("*.json"):
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
