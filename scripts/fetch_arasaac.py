#!/usr/bin/env python3
"""Costruisce il pacchetto ARASAAC completo incluso in CAA Writer.

Il catalogo italiano di ARASAAC (`/api/pictograms/all/it`) contiene tutti i
pittogrammi pubblicati con almeno una parola chiave italiana. Vengono scaricati
tutti, più i pittogrammi scelti per i concetti del lessico di CAA Writer.

ARASAAC distribuisce i pittogrammi in PNG (500 px), non in SVG. Poiché la
pipeline di CAA Writer gestisce SVG, ogni PNG viene incorporato in un SVG con
immagine in `data:` URI: il risultato è un SVG valido, senza rete al momento
dell'uso, con la stessa resa del file originale.

Licenza dei pittogrammi: **CC BY-NC-SA 4.0** (ARASAAC, autore Sergio Palao).
Il pacchetto è quindi `commercial_ok: false` e `share_alike: true`;
l'attribuzione è registrata per ogni simbolo nel manifest e nel database.

Uso (richiede rete, una volta sola):
    python3 scripts/fetch_arasaac.py                 # scarica e crea il manifest
    python3 scripts/fetch_arasaac.py --workers 12    # più connessioni
    python3 scripts/fetch_arasaac.py --manifest-only # ricrea solo il manifest
"""

from __future__ import annotations

import argparse
import base64
import json
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEXICON = ROOT / "resources" / "dictionaries" / "it-core.json"
PACK = ROOT / "resources" / "symbol-packs" / "arasaac"
SVG_DIR = PACK / "svg"
CACHE = ROOT / "scripts" / ".cache"
CATALOG = CACHE / "arasaac-catalog-it.json"

CATALOG_URL = "https://api.arasaac.org/api/pictograms/all/it"
SEARCH_URL = "https://api.arasaac.org/api/pictograms/it/search/{term}"
PNG_URL = "https://api.arasaac.org/api/pictograms/{id}?download=true"
USER_AGENT = "CAAWriter/0.1 (non-commercial CAA project; contact: caa-writer)"

LICENSE = "CC BY-NC-SA 4.0"
LICENSE_URL = "https://creativecommons.org/licenses/by-nc-sa/4.0/"
SOURCE_URL = "https://arasaac.org/"
AUTHOR = "Sergio Palao"
RIGHTS_HOLDER = "ARASAAC — Governo di Aragona"
CONSTRAINTS = (
    "Attribuzione obbligatoria ad ARASAAC e all'autore; uso non commerciale; "
    "opere derivate distribuite con la stessa licenza. Modifica applicata: "
    "PNG originale incorporato in un contenitore SVG."
)

MAX_KEYWORDS = 12
_print_lock = threading.Lock()


def log(message: str) -> None:
    with _print_lock:
        print(message, flush=True)


def fetch(url: str, timeout: int = 60) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=timeout) as response:
        return response.read()


def fetch_with_retry(url: str, attempts: int = 4) -> bytes:
    last: Exception | None = None
    for attempt in range(attempts):
        try:
            return fetch(url)
        except urllib.error.HTTPError as error:
            last = error
            if error.code == 404:
                raise
            time.sleep(1.5 * (attempt + 1))
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            last = error
            time.sleep(1.5 * (attempt + 1))
    assert last is not None
    raise last


def load_catalog(force: bool) -> list[dict]:
    if CATALOG.exists() and not force:
        return json.loads(CATALOG.read_text(encoding="utf-8"))
    CACHE.mkdir(parents=True, exist_ok=True)
    log("scarico il catalogo italiano di ARASAAC…")
    raw = fetch_with_retry(CATALOG_URL)
    data = json.loads(raw.decode("utf-8"))
    if not isinstance(data, list) or not data:
        raise SystemExit("catalogo ARASAAC vuoto o inatteso")
    CATALOG.write_bytes(raw)
    return data


def png_to_svg(png: bytes) -> str:
    encoded = base64.b64encode(png).decode("ascii")
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500 500" '
        'width="500" height="500" role="img">'
        f'<image x="0" y="0" width="500" height="500" href="data:image/png;base64,{encoded}"/>'
        "</svg>"
    )


def keywords_of(entry: dict) -> list[str]:
    """Parole chiave italiane, singolare e plurale, senza duplicati."""
    out: list[str] = []
    for item in entry.get("keywords") or []:
        for word in (item.get("keyword"), item.get("plural")):
            if not word:
                continue
            word = word.strip()
            if word and word not in out:
                out.append(word)
    return out[:MAX_KEYWORDS]


def label_of(entry: dict, pictogram_id: int) -> str:
    words = keywords_of(entry)
    return words[0] if words else f"Pittogramma {pictogram_id}"


def download_one(entry: dict, force: bool) -> tuple[int, str | None]:
    pictogram_id = entry["_id"]
    target = SVG_DIR / f"picto_{pictogram_id}.svg"
    if target.exists() and target.stat().st_size > 200 and not force:
        return pictogram_id, None
    try:
        png = fetch_with_retry(PNG_URL.format(id=pictogram_id))
    except Exception as error:  # noqa: BLE001 - si riporta e si continua
        return pictogram_id, f"errore di rete: {error}"
    if not png.startswith(b"\x89PNG") or len(png) < 500:
        return pictogram_id, "risposta non PNG"
    tmp = target.with_suffix(".svg.part")
    tmp.write_text(png_to_svg(png), encoding="utf-8")
    tmp.replace(target)
    return pictogram_id, None


def search_pictogram(term: str) -> int | None:
    try:
        raw = fetch_with_retry(SEARCH_URL.format(term=urllib.parse.quote(term)))
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        raise
    results = json.loads(raw.decode("utf-8"))
    if isinstance(results, list) and results:
        return results[0]["_id"]
    return None


def build_manifest(catalog: list[dict], mappings: list[dict]) -> dict:
    by_id = {entry["_id"]: entry for entry in catalog}
    symbols: list[dict] = []

    # 1. Simboli legati ai concetti del lessico: il motore li usa per la
    #    simbolizzazione, quindi hanno priorità nella ricerca.
    for mapping in mappings:
        pictogram_id = mapping["arasaac_id"]
        symbols.append(
            {
                "id": f"arasaac.{mapping['concept']}",
                "concept": mapping["concept"],
                "label": mapping["label"],
                "meaning": mapping["meaning"],
                "keywords": keywords_of(by_id.get(pictogram_id, {})),
                "file": f"svg/picto_{pictogram_id}.svg",
                "arasaac_id": pictogram_id,
                "source": f"https://arasaac.org/pictograms/{pictogram_id}",
                "status": "incluso",
            }
        )

    # 2. Intera libreria ARASAAC, cercabile per parola chiave.
    for entry in sorted(catalog, key=lambda item: item["_id"]):
        pictogram_id = entry["_id"]
        target = SVG_DIR / f"picto_{pictogram_id}.svg"
        if not target.exists():
            continue
        symbols.append(
            {
                "id": f"arasaac.picto.{pictogram_id}",
                "label": label_of(entry, pictogram_id),
                "keywords": keywords_of(entry),
                "file": f"svg/picto_{pictogram_id}.svg",
                "arasaac_id": pictogram_id,
                "source": f"https://arasaac.org/pictograms/{pictogram_id}",
                "status": "incluso",
            }
        )

    return {
        "pack": {
            "id": "arasaac",
            "name": "ARASAAC — pittogrammi",
            "version": "2.0.0",
            "description": (
                f"Catalogo ARASAAC completo ({len(catalog)} pittogrammi italiani) e "
                "pittogrammi dei concetti del lessico. PNG ufficiali incorporati in "
                "contenitori SVG."
            ),
            "author": AUTHOR,
            "rights_holder": RIGHTS_HOLDER,
            "license": LICENSE,
            "license_url": LICENSE_URL,
            "source_url": SOURCE_URL,
            "constraints": CONSTRAINTS,
            "commercial_ok": False,
            "share_alike": True,
            "no_derivatives": False,
            "builtin": True,
            "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        },
        "symbols": symbols,
    }


def concept_mappings() -> tuple[list[dict], list[str]]:
    lexicon = json.loads(LEXICON.read_text(encoding="utf-8"))
    concepts = lexicon.get("concepts", [])
    mappings: list[dict] = []
    missing: list[str] = []
    for index, concept in enumerate(concepts, start=1):
        concept_id = concept["id"]
        label = concept.get("label") or concept_id.split(".")[-1]
        try:
            pictogram_id = search_pictogram(label)
            if pictogram_id is None:
                pictogram_id = search_pictogram(
                    concept_id.split(".")[-1].replace("_", " ")
                )
            if pictogram_id is None:
                missing.append(label)
                continue
            mappings.append(
                {
                    "concept": concept_id,
                    "label": label,
                    "meaning": concept.get("meaning") or label,
                    "arasaac_id": pictogram_id,
                }
            )
        except Exception as error:  # noqa: BLE001
            missing.append(label)
            log(f"[concetti {index}/{len(concepts)}] {label}: {error}")
        if index % 25 == 0:
            log(f"[concetti {index}/{len(concepts)}] …")
    return mappings, missing


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--force", action="store_true", help="riscarica anche i file presenti")
    parser.add_argument("--workers", type=int, default=10, help="richieste in parallelo")
    parser.add_argument("--manifest-only", action="store_true", help="ricrea solo il manifest")
    parser.add_argument("--refresh-catalog", action="store_true", help="riscarica il catalogo")
    args = parser.parse_args()

    SVG_DIR.mkdir(parents=True, exist_ok=True)
    catalog = load_catalog(args.refresh_catalog)
    log(f"catalogo: {len(catalog)} pittogrammi italiani")

    mappings, missing = concept_mappings()
    log(f"concetti del lessico associati: {len(mappings)} (senza: {len(missing)})")

    # I pittogrammi scelti per i concetti devono esistere anche se il catalogo
    # non li elencasse (l'API di ricerca può restituirli comunque).
    catalog_ids = {entry["_id"] for entry in catalog}
    for mapping in mappings:
        if mapping["arasaac_id"] not in catalog_ids:
            catalog.append({"_id": mapping["arasaac_id"], "keywords": []})

    if not args.manifest_only:
        pending = [entry for entry in catalog]
        done = 0
        failures: list[str] = []
        started = time.time()
        with ThreadPoolExecutor(max_workers=max(1, args.workers)) as pool:
            futures = [pool.submit(download_one, entry, args.force) for entry in pending]
            for future in as_completed(futures):
                pictogram_id, error = future.result()
                done += 1
                if error:
                    failures.append(f"{pictogram_id}: {error}")
                if done % 250 == 0 or done == len(pending):
                    rate = done / max(time.time() - started, 1e-6)
                    log(
                        f"[{done}/{len(pending)}] {rate:.1f} file/s · errori {len(failures)}"
                    )
        if failures:
            log("errori (si può rilanciare lo script, riprende da dove era):")
            for line in failures[:20]:
                log(f"  {line}")

    manifest = build_manifest(catalog, mappings)
    PACK.joinpath("manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )

    files = len(list(SVG_DIR.glob("*.svg")))
    total = sum(path.stat().st_size for path in SVG_DIR.glob("*.svg"))
    log(f"file SVG: {files} ({total / 1024 / 1024:.1f} MB)")
    log(f"simboli nel manifest: {len(manifest['symbols'])}")
    log(f"concetti senza pittogramma: {len(missing)} {missing[:10]}")
    log(f"manifest: {PACK / 'manifest.json'}")
    return 0 if files else 1


if __name__ == "__main__":
    raise SystemExit(main())
