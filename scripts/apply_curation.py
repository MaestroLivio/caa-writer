#!/usr/bin/env python3
"""Applica alla libreria pubblicata le scelte di `curate_library.py`.

`template build-all` ricostruisce i documenti dalle specifiche, ma senza il
catalogo ARASAAC nel repository i pittogrammi già incorporati andrebbero persi.
Questa operazione è quindi volutamente conservativa: mantiene i documenti e le
anteprime esistenti, rimuove soltanto le voci archiviate e riscrive il manifest
con le stesse voci rimaste, allineate alla tassonomia corrente.

Uso:
    git checkout -- resources/library
    python3 scripts/apply_curation.py --confirm
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIBRARY = ROOT / "resources" / "library"
MANIFEST = LIBRARY / "manifest.json"
ARCHIVE = ROOT / "templates" / "archive"

# Specifiche ripristinate dopo la curatela: il sistema comunicativo è uno dei
# 30 richiesti dal controllo di completezza ed è un materiale utile.
RESTORED = {"boards.comunicazione-sociale-adulta"}


def archived_ids() -> set[str]:
    ids: set[str] = set()
    for path in ARCHIVE.rglob("*.json"):
        if path.name == "manifest.json":
            continue
        try:
            spec = json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            continue
        spec_id = spec.get("id")
        if isinstance(spec_id, str):
            ids.add(spec_id)
    return ids - RESTORED


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--confirm", action="store_true")
    args = parser.parse_args()

    raw = subprocess.run(
        ["git", "show", "HEAD:resources/library/manifest.json"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    manifest = json.loads(raw)
    removed = archived_ids()
    entries = manifest.get("entries", [])
    kept = [entry for entry in entries if entry["id"] not in removed]
    dropped = [entry for entry in entries if entry["id"] in removed]

    print(f"manifest: {len(entries)} voci")
    print(f"da mantenere: {len(kept)}")
    print(f"da rimuovere: {len(dropped)}")

    if not args.confirm:
        print("\ndry-run: nessun file modificato.")
        return 0

    for entry in dropped:
        for relative in (entry.get("path"), entry.get("preview")):
            if not relative:
                continue
            target = ROOT / "resources" / relative.removeprefix("resources/")
            if target.exists() and target.is_file():
                target.unlink()
        preview = entry.get("preview") or f"library/previews/{entry['id']}.svg"
        base = ROOT / "resources" / preview.removeprefix("resources/")
        for candidate in [base, *base.parent.glob(f"{entry['id']}-p*.svg")]:
            if candidate.exists() and candidate.is_file():
                candidate.unlink()

    manifest["entries"] = kept
    report = manifest.get("report")
    if isinstance(report, dict):
        boards = sum(1 for entry in kept if entry.get("source_type") == "board")
        report["built"] = len(kept)
        report["valid"] = len(kept)
        report["modifiable"] = len(kept)
        report["templates_built"] = len(kept) - boards
        report["boards_built"] = boards
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    for junk in LIBRARY.rglob("._*"):
        if junk.is_file():
            junk.unlink()

    print(f"\nmanifest riscritto con {len(kept)} voci.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
