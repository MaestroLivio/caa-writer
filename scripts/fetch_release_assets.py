#!/usr/bin/env python3
"""Prepara gli asset che non stanno nel repository, per una build di release.

Il repository versiona codice, lessico, SVG originali e template. Non versiona:

- il modello Qwen2.5-0.5B-Instruct GGUF (491 MB, oltre i limiti di GitHub);
- il runtime llama.cpp, specifico per sistema operativo e architettura;
- il catalogo ARASAAC completo (439 MB, ricostruibile dall'API pubblica).

Questo script li scarica e verifica il modello con lo sha256. Va eseguito prima
di `npm run tauri build` su una macchina pulita o in CI.

Uso:
    python3 scripts/fetch_release_assets.py
    python3 scripts/fetch_release_assets.py --skip-arasaac
"""

from __future__ import annotations

import argparse
import hashlib
import subprocess
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AI_DIR = ROOT / "resources" / "ai"
MODEL = AI_DIR / "model.gguf"
ARASAAC = ROOT / "resources" / "symbol-packs" / "arasaac"

MODEL_URL = (
    "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/"
    "qwen2.5-0.5b-instruct-q4_k_m.gguf"
)
MODEL_SHA256 = "74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db"
MODEL_SIZE = 491_400_032


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def download_model() -> None:
    AI_DIR.mkdir(parents=True, exist_ok=True)
    if MODEL.exists() and MODEL.stat().st_size == MODEL_SIZE and sha256(MODEL) == MODEL_SHA256:
        print(f"modello già presente e verificato: {MODEL}")
        return
    partial = MODEL.with_suffix(".gguf.part")
    print(f"scarico il modello ({MODEL_SIZE / 1e9:.2f} GB)…")
    request = urllib.request.Request(MODEL_URL, headers={"User-Agent": "CAAWriter/0.2"})
    with urllib.request.urlopen(request, timeout=600) as response, partial.open("wb") as out:
        total = int(response.headers.get("Content-Length") or MODEL_SIZE)
        done = 0
        while True:
            block = response.read(1024 * 1024)
            if not block:
                break
            out.write(block)
            done += len(block)
            if done % (32 * 1024 * 1024) < 1024 * 1024:
                print(f"  {done / total * 100:5.1f}% · {done / 1e6:.0f} MB", flush=True)
    if partial.stat().st_size != MODEL_SIZE:
        raise SystemExit(f"dimensione inattesa: {partial.stat().st_size} byte")
    actual = sha256(partial)
    if actual != MODEL_SHA256:
        partial.unlink(missing_ok=True)
        raise SystemExit(f"sha256 errato: {actual}")
    partial.replace(MODEL)
    print(f"modello verificato: {MODEL}")


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

    download_model()
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
