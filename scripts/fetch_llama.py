#!/usr/bin/env python3
"""Inserisce il runtime llama.cpp nel pacchetto `resources/ai/`.

Il modello GGUF è già incluso (`resources/ai/model.gguf`). Il binario invece
dipende da sistema operativo e architettura: questo script scarica la build
ufficiale di llama.cpp per la macchina corrente e ne estrae `llama-cli` con
le librerie necessarie.

Uso:
    python3 scripts/fetch_llama.py [--check] [--tag b12109]

`--check` non scarica nulla: dice solo quale asset servirebbe.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import stat
import sys
import tarfile
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AI_DIR = ROOT / "resources" / "ai"
RELEASES = "https://api.github.com/repos/ggml-org/llama.cpp/releases"
USER_AGENT = "CAAWriter/0.2"


def request_headers() -> dict[str, str]:
    """In CI usa il token per evitare il limite orario dell'API GitHub."""
    headers = {"User-Agent": USER_AGENT}
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    return headers


def asset_pattern() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "darwin":
        return "bin-macos-arm64" if machine in {"arm64", "aarch64"} else "bin-macos-x64"
    if system == "windows":
        return "bin-win-avx2-x64" if machine in {"amd64", "x86_64"} else f"bin-win-{machine}"
    return "bin-ubuntu-x64" if machine in {"x86_64", "amd64"} else "bin-ubuntu-arm64"


def get_json(url: str) -> dict | list:
    request = urllib.request.Request(url, headers=request_headers())
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.loads(response.read().decode("utf-8"))


def release_with_assets(pattern: str, tag: str | None) -> tuple[dict, dict]:
    """Alcune release non contengono binari: si scorre la lista finché si trova l'asset."""
    if tag:
        releases = [get_json(f"{RELEASES}/tags/{tag}")]
    else:
        releases = get_json(f"{RELEASES}?per_page=15")
    for release in releases:
        for asset in release.get("assets", []):
            name = asset["name"]
            if pattern in name and (name.endswith(".zip") or name.endswith(".tar.gz")):
                return release, asset
    return {}, {}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="mostra l'asset scelto senza scaricare")
    parser.add_argument("--tag", default=None, help="release specifica (es. b12109)")
    parser.add_argument(
        "--asset",
        default=None,
        help="forza il pattern dell'asset (es. bin-macos-x64 per una build Intel da host ARM)",
    )
    args = parser.parse_args()

    pattern = args.asset or asset_pattern()
    release, match = release_with_assets(pattern, args.tag)
    if not match:
        print(f"nessun asset «{pattern}» nelle ultime release", file=sys.stderr)
        return 1

    print(f"release {release.get('tag_name')} → {match['name']} ({match['size'] / 1e6:.1f} MB)")
    if args.check:
        return 0

    AI_DIR.mkdir(parents=True, exist_ok=True)
    suffix = ".tar.gz" if match["name"].endswith(".tar.gz") else ".zip"
    archive = AI_DIR / f"llama-runtime{suffix}"
    request = urllib.request.Request(match["browser_download_url"], headers=request_headers())
    with urllib.request.urlopen(request, timeout=600) as response, archive.open("wb") as out:
        shutil.copyfileobj(response, out)

    def wanted(base: str) -> bool:
        # `llama-server` serve per l'istanza condivisa: il modello viene caricato
        # una volta sola e tutte le funzionalità lo interrogano via HTTP locale.
        return base in {"llama-cli", "llama-cli.exe", "llama-server", "llama-server.exe"} or base.endswith(
            (".dylib", ".so", ".dll")
        )

    if suffix == ".zip":
        with zipfile.ZipFile(archive) as bundle:
            for name in bundle.namelist():
                base = os.path.basename(name)
                if not base or name.endswith("/") or not wanted(base):
                    continue
                with bundle.open(name) as src, (AI_DIR / base).open("wb") as dst:
                    shutil.copyfileobj(src, dst)
    else:
        with tarfile.open(archive) as bundle:
            for member in bundle.getmembers():
                base = os.path.basename(member.name)
                if not member.isfile() or not wanted(base):
                    continue
                src = bundle.extractfile(member)
                if src is None:
                    continue
                with src, (AI_DIR / base).open("wb") as dst:
                    shutil.copyfileobj(src, dst)

    # Le librerie sono nominate con la versione completa (libmtmd.0.8.0.dylib)
    # ma il binario le cerca con la sola major (libmtmd.0.dylib): i collegamenti
    # simbolici dell'archivio non vengono copiati e vanno ricreati.
    import re

    pattern = re.compile(r"^(lib.+)\.(\d+)\.\d+\.\d+\.(dylib|so)$")
    for library in AI_DIR.iterdir():
        match = pattern.match(library.name)
        if not match:
            continue
        short = AI_DIR / f"{match.group(1)}.{match.group(2)}.{match.group(3)}"
        if short.exists() or short.is_symlink():
            continue
        try:
            short.symlink_to(library.name)
            print("collegamento:", short.name, "→", library.name)
        except OSError as error:
            print(f"collegamento non creato per {short.name}: {error}", file=sys.stderr)

    for name in ("llama-cli", "llama-server"):
        binary = AI_DIR / (f"{name}.exe" if platform.system() == "Windows" else name)
        if binary.exists():
            binary.chmod(binary.stat().st_mode | stat.S_IEXEC)
            print("binario:", binary)
    archive.unlink(missing_ok=True)

    print("runtime pronto in", AI_DIR)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
