#!/usr/bin/env python3
"""Importa un repertorio di pittogrammi SVG in un pacchetto locale di CAA Writer.

Il pacchetto prodotto è compatibile con `resources/symbol-packs/<id>/manifest.json`
e viene importato nel database dall'applicazione (tabella dei pacchetti).

Cosa fa:
  1. legge tutti gli .svg dalla cartella sorgente indicata (nessuna rete);
  2. valida ogni SVG: XML ben formato, presenza del viewBox, nessuno script,
     nessun evento inline, nessun riferimento esterno, dimensione contenuta;
  3. copia i file validi in `resources/symbol-packs/<pack-id>/svg/`;
  4. scrive un manifest con metadati di licenza completi e stato
     `da verificare` (la revisione semantica resta umana).

Non scarica nulla da ARASAAC o da altri servizi: l'importazione è locale e
richiede che l'utente abbia già ottenuto i file nel rispetto della licenza.

Esempio:
  python3 scripts/import_symbols.py \
    --source ~/Downloads/mio-repertorio \
    --pack-id mio-repertorio \
    --name "Mio repertorio" \
    --license "CC BY 4.0" \
    --license-url https://creativecommons.org/licenses/by/4.0/ \
    --source-url https://example.org/repertorio \
    --author "Nome Cognome" \
    --concept-map concept-map.json
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACKS = ROOT / "resources" / "symbol-packs"

FORBIDDEN_TAGS = {"script", "foreignObject", "iframe", "audio", "video", "animate", "animateTransform"}
FORBIDDEN_ATTRS = {"onload", "onclick", "onmouseover", "onerror", "onbegin", "href"}
EXTERNAL_PATTERN = re.compile(r"(?:https?:)?//", re.IGNORECASE)
MAX_BYTES = 512 * 1024


def slugify(text: str) -> str:
    text = text.lower().strip()
    text = re.sub(r"[^a-z0-9]+", "-", text)
    return text.strip("-") or "simbolo"


def validate_svg(path: Path) -> tuple[str | None, list[str]]:
    problems: list[str] = []
    raw = path.read_text(encoding="utf-8", errors="replace")
    if len(raw.encode("utf-8")) > MAX_BYTES:
        problems.append("file troppo grande (> 512 kB)")
    if EXTERNAL_PATTERN.search(raw) and "<image" in raw:
        problems.append("contiene un riferimento esterno")
    try:
        tree = ET.fromstring(raw)
    except ET.ParseError as error:
        return None, [f"XML non valido: {error}"]

    tag = tree.tag.split("}")[-1]
    if tag != "svg":
        problems.append(f"radice inattesa: <{tag}>")
    if not tree.get("viewBox"):
        problems.append("manca l'attributo viewBox")

    for element in tree.iter():
        local = element.tag.split("}")[-1]
        if local in FORBIDDEN_TAGS:
            problems.append(f"elemento non ammesso: <{local}>")
        for name, value in element.attrib.items():
            attr = name.split("}")[-1].lower()
            if attr in FORBIDDEN_ATTRS and (value or "").strip().lower().startswith(
                ("http", "javascript:", "//")
            ):
                problems.append(f"attributo non ammesso: {attr}")
            if attr in {"href", "xlink:href"} and "data:" not in value:
                problems.append("riferimento esterno tramite href")

    if problems:
        return None, problems
    return raw, []


def main() -> int:
    parser = argparse.ArgumentParser(description="Importa pittogrammi SVG in un pacchetto locale")
    parser.add_argument("--source", required=True, help="cartella con i file .svg")
    parser.add_argument("--pack-id", required=True, help="identificativo del pacchetto")
    parser.add_argument("--name", required=True)
    parser.add_argument("--license", required=True)
    parser.add_argument("--license-url", default=None)
    parser.add_argument("--source-url", default=None)
    parser.add_argument("--author", default=None)
    parser.add_argument("--rights-holder", default=None)
    parser.add_argument("--constraints", default=None)
    parser.add_argument("--commercial-ok", action="store_true")
    parser.add_argument("--concept-map", default=None, help="JSON {file: concept_id}")
    parser.add_argument("--label-map", default=None, help="JSON {file: etichetta}")
    args = parser.parse_args()

    source = Path(args.source).expanduser()
    if not source.is_dir():
        print(f"Cartella non trovata: {source}", file=sys.stderr)
        return 1

    concept_map: dict[str, str] = {}
    label_map: dict[str, str] = {}
    if args.concept_map:
        concept_map = json.loads(Path(args.concept_map).read_text(encoding="utf-8"))
    if args.label_map:
        label_map = json.loads(Path(args.label_map).read_text(encoding="utf-8"))

    pack_id = slugify(args.pack_id)
    target = PACKS / pack_id
    svg_dir = target / "svg"
    svg_dir.mkdir(parents=True, exist_ok=True)

    symbols = []
    rejected = []
    for file in sorted(source.glob("*.svg")):
        content, problems = validate_svg(file)
        if content is None:
            rejected.append({"file": file.name, "problems": problems})
            continue
        slug = slugify(file.stem)
        destination = svg_dir / f"{slug}.svg"
        destination.write_text(content, encoding="utf-8")
        label = label_map.get(file.name) or file.stem.replace("_", " ").replace("-", " ")
        symbols.append(
            {
                "id": f"{pack_id}.{slug}",
                "concept": concept_map.get(file.name),
                "label": label,
                "meaning": None,
                "file": f"svg/{slug}.svg",
                "status": "da verificare",
            }
        )

    manifest = {
        "pack": {
            "id": pack_id,
            "name": args.name,
            "version": "1.0.0",
            "description": f"Repertorio importato da {source.name}",
            "author": args.author,
            "rights_holder": args.rights_holder or args.author,
            "license": args.license,
            "license_url": args.license_url,
            "source_url": args.source_url,
            "commercial_ok": bool(args.commercial_ok),
            "share_alike": "SA" in args.license.upper().replace(" ", ""),
            "no_derivatives": "ND" in args.license.upper().replace(" ", ""),
            "constraints": args.constraints,
            "builtin": False,
            "imported_at": datetime.now(timezone.utc).isoformat(),
        },
        "symbols": symbols,
    }
    (target / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8"
    )

    print(f"Pacchetto: {target}")
    print(f"  simboli importati: {len(symbols)}")
    print(f"  simboli rifiutati: {len(rejected)}")
    for item in rejected:
        print(f"    - {item['file']}: {'; '.join(item['problems'])}")
    print("  stato: 'da verificare' (serve l'approvazione di un professionista)")
    if not symbols:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
