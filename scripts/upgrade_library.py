#!/usr/bin/env python3
"""
Arricchisce le tabelle di comunicazione e le agende visive di CAA Writer.

Per ogni documento .caadoc selezionato:
  * aggiunge una pagina "Vocabolario essenziale" con parole ad alta frequenza;
  * collega la pagina dalla home con una cella "Vocabolario";
  * aggiunge un pulsante Home nella nuova pagina;
  * aggiorna plain.txt, document.json e il catalogo della libreria.

I simboli usati provengono dai documenti esistenti (ARASAAC già incorporati)
oppure dal pack locale CAA Writer Core. Nessun simbolo viene inventato.

Uso:
  python3 scripts/upgrade_library.py --dry-run
  python3 scripts/upgrade_library.py --confirm

Senza --confirm non viene scritto nessun file.
"""

from __future__ import annotations

import argparse
import copy
import json
import os
import re
import shutil
import sys
import tempfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RESOURCES = ROOT / "resources"
LIBRARY = RESOURCES / "library"
CATALOG_PATH = LIBRARY / "manifest.json"
CORE_PACK = RESOURCES / "symbol-packs" / "core"

# Parole ad alta frequenza (core vocabulary) utili in ogni contesto.
# L'ordine è deliberato: pronomi, verbi, bisogni, emozioni, contesti.
CORE_WORDS = [
    "io", "tu", "lui", "lei", "noi", "voi", "loro",
    "voglio", "vuoi", "avere", "fare", "andare", "stare",
    "bene", "male", "più", "basta", "aiuto", "grazie", "sì", "no",
    "ancora", "finito", "mangiare", "bere", "giocare", "uscire",
    "guardare", "ascoltare", "leggere", "scrivere", "parlare",
    "dormire", "lavarsi le mani", "felice", "triste", "arrabbiato",
    "paura", "stanco", "casa", "scuola", "acqua", "pane", "pasta",
]

BOOK_SVG = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" '
    'role="img" aria-label="Vocabolario">'
    '<rect x="14" y="18" width="72" height="64" rx="6" fill="#ffffff" '
    'stroke="#1f2937" stroke-width="4"/>'
    '<path d="M50 18 V82" stroke="#1f2937" stroke-width="3"/>'
    '<path d="M22 30 H42 M22 40 H42 M22 50 H42 M58 30 H78 M58 40 H78 M58 50 H78" '
    'stroke="#0f766e" stroke-width="3" stroke-linecap="round"/>'
    '</svg>'
)

HOME_SVG = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" '
    'role="img" aria-label="Home">'
    '<path d="M50 14 L88 46 H76 V84 H24 V46 H12 Z" fill="#ffffff" '
    'stroke="#1f2937" stroke-width="4" stroke-linejoin="round"/>'
    '<rect x="42" y="58" width="16" height="26" rx="2" fill="#0f766e"/>'
    '</svg>'
)


def slugify(text: str) -> str:
    text = text.lower().strip()
    text = re.sub(r"[^a-z0-9]+", "_", text, flags=re.UNICODE)
    return text.strip("_") or "voce"


def norm(text: str) -> str:
    return re.sub(r"\s+", " ", (text or "").strip().lower())


def load_json_zip(path: Path, name: str):
    with zipfile.ZipFile(path) as zf:
        return json.loads(zf.read(name))


def collect_arasaac_symbols() -> dict[str, dict]:
    """Raccoglie i simboli già presenti nei board, con SVG incorporato."""
    found: dict[str, dict] = {}
    for path in sorted((LIBRARY / "boards").glob("*.caadoc")):
        if path.name.startswith("._") or path.name.startswith("."):
            continue
        try:
            doc = load_json_zip(path, "document.json")
        except Exception as exc:  # pragma: no cover - diagnostica
            print(f"  ! salto {path.name}: {exc}", file=sys.stderr)
            continue
        for page in doc.get("universal", {}).get("pages", []):
            for el in page.get("elements", []):
                svg = el.get("svg")
                if not svg:
                    continue
                for candidate in (el.get("text"), el.get("spoken_text"), el.get("lemma")):
                    key = norm(candidate or "")
                    if not key:
                        continue
                    if key not in found:
                        found[key] = {
                            "svg": svg,
                            "symbol_id": el.get("symbol_id"),
                            "source": path.name,
                        }
    return found


def collect_core_pack_symbols() -> dict[str, dict]:
    """Carica i simboli del pack locale CAA Writer Core."""
    manifest_path = CORE_PACK / "manifest.json"
    if not manifest_path.exists():
        return {}
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    result: dict[str, dict] = {}
    for symbol in manifest.get("symbols", []):
        label = norm(symbol.get("label", ""))
        if not label or label in result:
            continue
        file_path = CORE_PACK / symbol.get("file", "")
        if not file_path.exists():
            continue
        result[label] = {
            "svg": file_path.read_text(encoding="utf-8"),
            "symbol_id": symbol.get("id"),
            "source": "core",
        }
    return result


def build_core_elements(prototype: dict, arasaac: dict[str, dict], core: dict[str, dict], start_z: int):
    """Costruisce le celle della pagina Vocabolario essenziale."""
    elements = []
    used_ids: set[str] = set()
    idx = 0
    for word in CORE_WORDS:
        key = norm(word)
        entry = arasaac.get(key) or core.get(key)
        if not entry:
            continue
        el = copy.deepcopy(prototype)
        base = f"core_{slugify(word)}"
        el_id = base
        suffix = 2
        while el_id in used_ids:
            el_id = f"{base}_{suffix}"
            suffix += 1
        used_ids.add(el_id)
        el["id"] = el_id
        el["text"] = word
        el["spoken_text"] = word
        el["lemma"] = None
        el["kind"] = "symbol"
        el["symbol_id"] = entry.get("symbol_id")
        el["svg"] = entry["svg"]
        el["metadata"] = None
        el["children"] = []
        el["group_id"] = None
        el["link_page"] = None
        el["image_asset"] = None
        el["audio_asset"] = None
        el["action"] = {
            "target_document": None,
            "target_page": None,
            "type": "append",
            "url": None,
        }
        el["column"] = idx % 4
        el["row"] = idx // 4
        el["z"] = start_z + idx + 1
        elements.append(el)
        idx += 1
        if idx >= 24:
            break
    return elements


def build_nav_element(prototype: dict, el_id: str, text: str, spoken: str, action_type: str,
                      target_page: str | None, svg: str, column: int, row: int, z: int,
                      symbol_id: str | None = None):
    el = copy.deepcopy(prototype)
    el["id"] = el_id
    el["text"] = text
    el["spoken_text"] = spoken
    el["lemma"] = None
    el["kind"] = "symbol"
    el["symbol_id"] = symbol_id
    el["svg"] = svg
    el["metadata"] = None
    el["children"] = []
    el["group_id"] = None
    el["link_page"] = None
    el["image_asset"] = None
    el["audio_asset"] = None
    el["action"] = {
        "target_document": None,
        "target_page": target_page,
        "type": action_type,
        "url": None,
    }
    el["column"] = column
    el["row"] = row
    el["z"] = z
    return el


def next_slot(page: dict, index: int):
    grid = page.get("grid") or {}
    columns = int(grid.get("columns") or 4)
    return index % columns, index // columns


def ensure_capacity(page: dict, need: int):
    grid = page.setdefault("grid", {})
    columns = int(grid.get("columns") or 4)
    rows = int(grid.get("rows") or 1)
    if need > columns * rows:
        rows = (need + columns - 1) // columns
        grid["rows"] = rows
        heights = grid.get("row_heights")
        if heights:
            if len(heights) < rows:
                heights.extend([1.0] * (rows - len(heights)))
            elif len(heights) > rows:
                del heights[rows:]
        widths = grid.get("column_widths")
        if widths:
            if len(widths) < columns:
                widths.extend([1.0] * (columns - len(widths)))
            elif len(widths) > columns:
                del widths[columns:]


def upgrade_document(path: Path, arasaac: dict[str, dict], core: dict[str, dict], dry_run: bool):
    """Restituisce un dizionario con il riepilogo delle modifiche."""
    with zipfile.ZipFile(path) as zf:
        names = zf.namelist()
        doc = json.loads(zf.read("document.json"))
        manifest = json.loads(zf.read("manifest.json"))
        plain = zf.read("plain.txt").decode("utf-8")
        extras = {name: zf.read(name) for name in names
                  if name not in {"document.json", "manifest.json", "plain.txt"}}

    pages = doc.get("universal", {}).get("pages", [])
    if not pages:
        return None
    if any(p.get("id", "").startswith("core_") or p.get("title") == "Vocabolario essenziale" for p in pages):
        return {"id": path.stem, "skipped": True}

    home = next((p for p in pages if p.get("is_home")), pages[0])
    page_ids = {p.get("id", "") for p in pages}
    core_page_id = "core_vocabolario"
    suffix = 2
    while core_page_id in page_ids:
        core_page_id = f"core_vocabolario_{suffix}"
        suffix += 1

    prototype = copy.deepcopy(home["elements"][0]) if home.get("elements") else None
    if prototype is None:
        return {"id": path.stem, "skipped": True, "reason": "home senza elementi"}

    # z massimo attuale
    max_z = 0
    for page in pages:
        for el in page.get("elements", []):
            try:
                max_z = max(max_z, int(el.get("z") or 0))
            except (TypeError, ValueError):
                pass

    core_elements = build_core_elements(prototype, arasaac, core, max_z)
    if not core_elements:
        return {"id": path.stem, "skipped": True, "reason": "nessun simbolo core disponibile"}

    core_grid_cols = 4
    core_grid_rows = (len(core_elements) + 1 + core_grid_cols - 1) // core_grid_cols

    # Pulsante Home in fondo alla pagina core
    home_idx = len(core_elements)
    home_el = build_nav_element(
        prototype,
        f"{core_page_id}_home",
        "Home",
        "Torna alla home",
        "home",
        None,
        HOME_SVG,
        home_idx % core_grid_cols,
        home_idx // core_grid_cols,
        max_z + len(core_elements) + 1,
        symbol_id=(core.get("casa") or {}).get("symbol_id"),
    )

    core_page = {
        "background": "",
        "elements": core_elements + [home_el],
        "flow": copy.deepcopy(home.get("flow") or {"columns": 1, "gap_mm": 3.0, "padding_mm": 0.0}),
        "grid": {
            "align": "",
            "column_widths": [],
            "columns": core_grid_cols,
            "content_height_mm": None,
            "content_width_mm": None,
            "gap_mm": 3.0,
            "origin_x_mm": None,
            "origin_y_mm": None,
            "row_heights": [],
            "rows": core_grid_rows,
            "valign": "",
        },
        "id": core_page_id,
        "is_home": False,
        "layout": "grid",
        "notes": "Parole ad alta frequenza per comunicare in ogni contesto.",
        "title": "Vocabolario essenziale",
    }

    # Cella di collegamento nella home
    home_elements = home.setdefault("elements", [])
    ensure_capacity(home, len(home_elements) + 1)
    link_index = len(home_elements)
    link_el = build_nav_element(
        prototype,
        f"{core_page_id}_link",
        "Vocabolario",
        "Apri il vocabolario essenziale",
        "navigate",
        core_page_id,
        BOOK_SVG,
        link_index % int(home.get("grid", {}).get("columns") or 4),
        link_index // int(home.get("grid", {}).get("columns") or 4),
        max_z + len(core_elements) + 2,
        symbol_id=(core.get("libro") or {}).get("symbol_id"),
    )
    home_elements.append(link_el)

    pages.append(core_page)

    # plain text aggiornato
    new_plain_lines = [line for line in plain.splitlines() if line.strip()]
    new_plain_lines.append("Vocabolario essenziale")
    for word in [e["text"] for e in core_elements]:
        new_plain_lines.append(word)
    new_plain_lines.append("Home")
    new_plain = "\n".join(new_plain_lines) + "\n"

    doc["plain_text"] = new_plain
    doc["saved_at"] = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    manifest["plain_text_chars"] = len(new_plain)

    summary = {
        "id": path.stem,
        "path": str(path.relative_to(ROOT)),
        "added_page": core_page_id,
        "added_elements": len(core_page["elements"]) + 1,
        "core_words": [e["text"] for e in core_elements],
        "skipped": False,
    }

    if dry_run:
        return summary

    tmp_dir = Path(tempfile.mkdtemp(prefix="caa-upgrade-"))
    try:
        (tmp_dir / "document.json").write_text(json.dumps(doc, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
        (tmp_dir / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
        (tmp_dir / "plain.txt").write_text(new_plain, encoding="utf-8")
        for name, data in extras.items():
            target = tmp_dir / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
        tmp_zip = tmp_dir / "out.caadoc"
        with zipfile.ZipFile(tmp_zip, "w", zipfile.ZIP_DEFLATED) as zf:
            for item in sorted(tmp_dir.rglob("*")):
                if item.is_file() and item.name != "out.caadoc":
                    zf.write(item, item.relative_to(tmp_dir).as_posix())
        shutil.move(str(tmp_zip), path)
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description="Arricchisce tabelle di comunicazione e agende visive.")
    parser.add_argument("--confirm", action="store_true", help="scrive davvero i file (senza questo flag è dry-run)")
    parser.add_argument("--dry-run", action="store_true", help="forza la modalità dry-run")
    parser.add_argument("--limit", type=int, default=0, help="numero massimo di documenti da elaborare")
    args = parser.parse_args()
    dry_run = args.dry_run or not args.confirm

    if not CATALOG_PATH.exists():
        print(f"Catalogo non trovato: {CATALOG_PATH}", file=sys.stderr)
        return 2
    catalog = json.loads(CATALOG_PATH.read_text(encoding="utf-8"))

    targets = []
    for entry in catalog.get("entries", []):
        is_board = entry.get("source_type") == "board"
        is_agenda = entry.get("category_id") == "agende"
        is_grid = entry.get("kind") in ("communication_grid", "communication_board")
        if is_board or is_agenda or is_grid:
            path = RESOURCES / entry.get("path", "")
            if path.suffix == ".caadoc" and path.exists():
                targets.append((entry, path))
    targets.sort(key=lambda item: str(item[1]))

    if args.limit:
        targets = targets[: args.limit]

    print(f"Documenti da elaborare: {len(targets)} (dry-run={dry_run})")
    arasaac = collect_arasaac_symbols()
    core = collect_core_pack_symbols()
    print(f"Simboli disponibili: {len(arasaac)} dai board, {len(core)} dal pack core")

    summaries = []
    for entry, path in targets:
        try:
            summary = upgrade_document(path, arasaac, core, dry_run)
        except Exception as exc:  # pragma: no cover - diagnostica
            print(f"  ! errore su {path.name}: {exc}", file=sys.stderr)
            summaries.append({"id": path.stem, "error": str(exc)})
            continue
        if summary is None:
            continue
        summaries.append(summary)
        if summary.get("skipped"):
            print(f"  - salto {path.name}: {summary.get('reason', 'già aggiornato')}")
        else:
            print(f"  + {path.name}: +{summary['added_elements']} elementi, "
                  f"{len(summary['core_words'])} parole core")
            if not dry_run:
                entry["pages"] = int(entry.get("pages", 1)) + 1
                entry["elements"] = int(entry.get("elements", 0)) + summary["added_elements"]
                entry["objective"] = (entry.get("objective") or "") + " Include vocabolario essenziale e navigazione."

    if not dry_run:
        CATALOG_PATH.write_text(json.dumps(catalog, ensure_ascii=False, indent=2), encoding="utf-8")
        print(f"Catalogo aggiornato: {CATALOG_PATH}")
    else:
        print("Modalità dry-run: nessun file è stato modificato.")

    updated = sum(1 for s in summaries if not s.get("skipped") and not s.get("error"))
    skipped = sum(1 for s in summaries if s.get("skipped"))
    errors = sum(1 for s in summaries if s.get("error"))
    print(f"Riepilogo: aggiornati={updated} saltati={skipped} errori={errors}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
