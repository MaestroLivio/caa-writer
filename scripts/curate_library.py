#!/usr/bin/env python3
"""Riduce la libreria a un insieme curato di materiali realmente utili.

La selezione non inventa contenuti: sposta soltanto le specifiche escluse in
`templates/archive/` (fuori dal bundle) e cancella i documenti e le anteprime
corrispondenti. Le specifiche archiviate restano nel repository e possono
essere ripristinate.

Criteri, per tipo:
  * si tiene un numero massimo dichiarato in CAPS;
  * si eliminano prima i duplicati con lo stesso titolo normalizzato,
    preferendo la versione interattiva e con più pagine;
  * a parità, si preferiscono i materiali interattivi, con più pagine, senza
    pittogrammi mancanti e già segnalati come in evidenza.

Uso:
    python3 scripts/curate_library.py            # dry-run
    python3 scripts/curate_library.py --confirm
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIBRARY = ROOT / "resources" / "library"
MANIFEST = LIBRARY / "manifest.json"
SOURCES = ROOT / "templates" / "sources"
ARCHIVE = ROOT / "templates" / "archive"

CAPS = {
    "worksheet": 70,
    "communication_grid": 45,
    "communication_board": 30,
    "teacch": 30,
    "visual_schedule": 25,
    "social_story": 25,
    "visual_sequence": 22,
    "aba": 20,
    "emotions": 20,
    "printable": 16,
    "autonomy": 14,
    "game": 12,
    "story": 10,
    "activity": 10,
}


def normalize(title: str) -> str:
    text = title.lower().strip()
    text = re.sub(r"[^a-z0-9àèéìòù]+", " ", text)
    return re.sub(r"\s+", " ", text).strip()


def score(entry: dict) -> tuple:
    return (
        1 if entry.get("interactive") else 0,
        int(entry.get("pages") or 0),
        -int(entry.get("missing_symbols") or 0),
        1 if entry.get("featured") else 0,
        int(entry.get("elements") or 0),
    )


def index_sources() -> dict[str, Path]:
    found: dict[str, Path] = {}
    for path in SOURCES.rglob("*.json"):
        if path.name == "manifest.json":
            continue
        try:
            spec = json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            continue
        spec_id = spec.get("id")
        if isinstance(spec_id, str):
            found[spec_id] = path
    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--confirm", action="store_true", help="scrive davvero le modifiche")
    args = parser.parse_args()

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    entries: list[dict] = manifest.get("entries", [])
    sources = index_sources()

    # Un solo materiale per titolo: le versioni a più pagine e interattive
    # sostituiscono i doppioni brevi.
    unique: dict[str, dict] = {}
    for entry in entries:
        key = normalize(entry.get("title", ""))
        current = unique.get(key)
        if current is None or score(entry) > score(current):
            unique[key] = entry

    by_kind: dict[str, list[dict]] = {}
    for entry in unique.values():
        by_kind.setdefault(entry.get("kind", ""), []).append(entry)

    keep: set[str] = set()
    removed: list[dict] = []
    for kind, items in by_kind.items():
        items.sort(key=score, reverse=True)
        cap = CAPS.get(kind, 20)
        for position, entry in enumerate(items):
            if position < cap:
                keep.add(entry["id"])
            else:
                removed.append(entry)

    # I duplicati scartati dal dizionario `unique` vanno rimossi anch'essi.
    for entry in entries:
        if entry["id"] not in {item["id"] for item in unique.values()}:
            removed.append(entry)

    removed = [entry for entry in removed if entry["id"] not in keep]
    removed.sort(key=lambda item: item.get("id", ""))

    print(f"documenti attuali: {len(entries)}")
    print(f"da mantenere:     {len(keep)}")
    print(f"da archiviare:    {len(removed)}")
    for kind, cap in CAPS.items():
        kept = sum(1 for entry in entries if entry.get("kind") == kind and entry["id"] in keep)
        print(f"  {kind:22} {kept:3}/{cap}")

    if not args.confirm:
        print("\ndry-run: nessun file modificato. Usa --confirm per applicare.")
        return 0

    ARCHIVE.mkdir(parents=True, exist_ok=True)
    for entry in removed:
        source = sources.get(entry["id"])
        if source and source.exists():
            relative = source.relative_to(SOURCES)
            target = ARCHIVE / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.move(str(source), str(target))
        document = LIBRARY / entry.get("path", "").removeprefix("library/")
        if document.exists() and document.is_file():
            document.unlink()
        preview = entry.get("preview") or f"library/previews/{entry['id']}.svg"
        preview_path = ROOT / "resources" / preview.removeprefix("resources/")
        for candidate in [preview_path, *preview_path.parent.glob(f"{entry['id']}-p*.svg")]:
            if candidate.exists() and candidate.is_file():
                candidate.unlink()

    # File AppleDouble creati da macOS, mai parte della libreria.
    for junk in LIBRARY.rglob("._*"):
        if junk.is_file():
            junk.unlink()

    print(f"\narchiviati {len(removed)} materiali in {ARCHIVE.relative_to(ROOT)}")
    print("Ora rigenera manifest e anteprime con caa-cli template build-all.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
