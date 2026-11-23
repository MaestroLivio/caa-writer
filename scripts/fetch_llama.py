#!/usr/bin/env python3
"""Inserisce il runtime llama.cpp in `resources/ai/runtime/`.

Il runtime (binari e librerie) è incluso nell'installer; il modello GGUF non
lo è più. Il binario dipende da sistema operativo e architettura: questo
script scarica la build ufficiale di llama.cpp per la macchina corrente e ne
estrae `llama-cli`, `llama-server` e le librerie necessarie.

Uso:
    python3 scripts/fetch_llama.py [--check] [--tag b12109]

`--check` non scarica nulla: dice solo quale asset servirebbe.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import shutil
import stat
import sys
import tarfile
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AI_DIR = ROOT / "resources" / "ai"
AI_RUNTIME = AI_DIR / "runtime"
RELEASES = "https://api.github.com/repos/ggml-org/llama.cpp/releases"
USER_AGENT = "CAAWriter/0.2"


def request_headers() -> dict[str, str]:
    """In CI usa il token per evitare il limite orario dell'API GitHub."""
    headers = {"User-Agent": USER_AGENT}
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    return headers


def asset_patterns() -> list[str]:
    """Pattern in ordine di preferenza.

    Il nome degli asset Windows è cambiato: le release recenti pubblicano
    `bin-win-cpu-x64` (che contiene le varianti CPU con dispatch a runtime)
    al posto di `bin-win-avx2-x64`. Il secondo resta come ripiego per le
    release più vecchie.
    """
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "darwin":
        return ["bin-macos-arm64" if machine in {"arm64", "aarch64"} else "bin-macos-x64"]
    if system == "windows":
        if machine in {"amd64", "x86_64"}:
            return ["bin-win-cpu-x64", "bin-win-avx2-x64"]
        return [f"bin-win-{machine}"]
    return ["bin-ubuntu-x64" if machine in {"x86_64", "amd64"} else "bin-ubuntu-arm64"]


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


def create_version_links(directory: Path) -> None:
    """Ricrea i nomi «major» delle librerie versionate.

    Gli archivi di llama.cpp contengono collegamenti simbolici
    (`libggml-base.so.0` → `libggml-base.so.0.9.7`), ma l'estrazione copia
    solo i file regolari: senza questi nomi il caricatore dinamico non trova
    le dipendenze. macOS nomina `libmtmd.0.8.0.dylib` → `libmtmd.0.dylib`,
    Linux `libggml-base.so.0.9.7` → `libggml-base.so.0`.
    """
    macos_pattern = re.compile(r"^(lib.+?)\.(\d+)(?:\.\d+)+\.dylib$")
    linux_pattern = re.compile(r"^(lib.+\.so)\.(\d+)(?:\.\d+)+$")
    for library in sorted(directory.iterdir()):
        match = macos_pattern.match(library.name)
        if match:
            short = directory / f"{match.group(1)}.{match.group(2)}.dylib"
        else:
            match = linux_pattern.match(library.name)
            if not match:
                continue
            short = directory / f"{match.group(1)}.{match.group(2)}"
        if short.exists() or short.is_symlink():
            continue
        try:
            short.symlink_to(library.name)
            print("collegamento:", short.name, "→", library.name)
        except OSError as error:
            print(f"collegamento non creato per {short.name}: {error}", file=sys.stderr)


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

    pattern = args.asset or asset_patterns()
    if isinstance(pattern, str):
        pattern = [pattern]
    release, match = {}, {}
    pattern_used = ""
    for candidate in pattern:
        release, match = release_with_assets(candidate, args.tag)
        if match:
            pattern_used = candidate
            break
    if not match:
        print(f"nessun asset «{', '.join(pattern)}» nelle ultime release", file=sys.stderr)
        return 1
    pattern = pattern_used

    print(f"release {release.get('tag_name')} → {match['name']} ({match['size'] / 1e6:.1f} MB)")
    if args.check:
        return 0

    AI_RUNTIME.mkdir(parents=True, exist_ok=True)
    suffix = ".tar.gz" if match["name"].endswith(".tar.gz") else ".zip"
    archive = AI_DIR / f"llama-runtime{suffix}"
    request = urllib.request.Request(match["browser_download_url"], headers=request_headers())
    with urllib.request.urlopen(request, timeout=600) as response, archive.open("wb") as out:
        shutil.copyfileobj(response, out)

    def wanted(base: str) -> bool:
        # `llama-server` serve per l'istanza condivisa: il modello viene caricato
        # una volta sola e tutte le funzionalità lo interrogano via HTTP locale.
        if base in {"llama-cli", "llama-cli.exe", "llama-server", "llama-server.exe"}:
            return True
        # Su Linux le librerie sono versionate *dopo* l'estensione
        # (`libggml-base.so.0.9.7`), quindi il solo suffisso `.so` non basta:
        # senza questi file il bundle AppImage non risolve le dipendenze di
        # `libggml-cpu-*.so` e `llama-server` non parte.
        return (
            base.endswith((".so", ".dylib", ".dll"))
            or re.search(r"\.so(\.\d+)+$", base) is not None
        )

    if suffix == ".zip":
        with zipfile.ZipFile(archive) as bundle:
            for name in bundle.namelist():
                base = os.path.basename(name)
                if not base or name.endswith("/") or not wanted(base):
                    continue
                with bundle.open(name) as src, (AI_RUNTIME / base).open("wb") as dst:
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
                with src, (AI_RUNTIME / base).open("wb") as dst:
                    shutil.copyfileobj(src, dst)

    create_version_links(AI_RUNTIME)

    for name in ("llama-cli", "llama-server"):
        binary = AI_RUNTIME / (f"{name}.exe" if platform.system() == "Windows" else name)
        if binary.exists():
            binary.chmod(binary.stat().st_mode | stat.S_IEXEC)
            print("binario:", binary)
    archive.unlink(missing_ok=True)

    print("runtime pronto in", AI_RUNTIME)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
