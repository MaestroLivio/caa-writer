#!/usr/bin/env python3
"""Prepara gli asset che non stanno nel repository, per una build di release.

Il repository versiona codice, lessico, SVG originali e template. Non versiona:

- il runtime llama.cpp, specifico per sistema operativo e architettura;
- il catalogo ARASAAC completo (439 MB, ricostruibile dall'API pubblica).

Il modello linguistico GGUF non è più incluso: l'utente lo sceglie dal
catalogo dell'app e lo scarica separatamente.

Uso:
    python3 scripts/fetch_release_assets.py
    python3 scripts/fetch_release_assets.py --skip-arasaac
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AI_DIR = ROOT / "resources" / "ai"
ARASAAC = ROOT / "resources" / "symbol-packs" / "arasaac"


def run(script: str, *args: str) -> None:
    command = [sys.executable, str(ROOT / "scripts" / script), *args]
    print("eseguo:", " ".join(command))
    subprocess.run(command, check=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--skip-arasaac",
        action="store_true",
        help="non scaricare il catalogo ARASAAC (build con i soli simboli originali)",
    )
    parser.add_argument(
        "--llama-asset",
        default=None,
        help="forza il runtime llama.cpp (es. bin-macos-x64 per la build Intel)",
    )
    parser.add_argument(
        "--force-llama",
        action="store_true",
        help="riscarica il runtime anche se è già presente",
    )
    args = parser.parse_args()

    runtime_present = (AI_DIR / "llama-server").exists() or (AI_DIR / "llama-server.exe").exists()
    if runtime_present and not args.force_llama:
        print(f"runtime llama.cpp già presente: {AI_DIR}")
    else:
        extra = ["--asset", args.llama_asset] if args.llama_asset else []
        run("fetch_llama.py", *extra)
    if args.skip_arasaac:
        print("catalogo ARASAAC saltato su richiesta")
    elif (ARASAAC / "manifest.json").exists() and any((ARASAAC / "svg").glob("*.svg")):
        print(f"catalogo ARASAAC già presente: {ARASAAC}")
    else:
        run("fetch_arasaac.py")
    print("asset di release pronti")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
