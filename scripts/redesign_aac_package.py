#!/usr/bin/env python3
"""
Ridisegna il pacchetto «Comunicazione aumentativa e alternativa».

Per ognuno dei documenti della categoria `aac` della libreria:

  * applica un sistema grafico coerente (raggi, bordi, riempimenti, tipografia);
  * colora le celle per classe di parola con la chiave Fitzgerald, la
    convenzione usata nelle tabelle CAA (persone, azioni, descrittori, nomi,
    parole sociali, urgenze, navigazione);
  * rende riconoscibili i riquadri senza pittogramma (bordo tratteggiato, non
    un simbolo inventato) e i comandi di navigazione (Home, Indietro);
  * compila `accessibility.label` su ogni elemento, che era vuota su tutti i
    5.141 elementi del pacchetto, e imposta `large_text` su navigazione,
    titoli e vocabolario essenziale;
  * uniforma margini (10 mm) e spaziature della griglia (2,4 mm).

Nessun testo, simbolo, licenza o metadato di contenuto viene modificato:
cambiano solo stile e metadati di accessibilità. Il `.caadoc` viene riscritto
sostituendo soltanto `document.json` — gli asset già presenti restano intatti,
senza duplicare i pittogrammi — e le anteprime vengono rigenerate dal renderer
ufficiale con `dev.library.preview`.

Uso:
  python3 scripts/redesign_aac_package.py                 # dry-run
  python3 scripts/redesign_aac_package.py --apply         # scrive
  python3 scripts/redesign_aac_package.py --apply --limit 1
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RESOURCES = ROOT / "resources"
CATALOG = RESOURCES / "library" / "manifest.json"
CLI = ROOT / "src-tauri" / "target" / "debug" / "caa-cli"

# ---------------------------------------------------------------- palette ---
# Tinte chiare + bordo scuro: il testo predefinito del renderer è #111111,
# quindi ogni accoppiamento supera 4,5:1 sul riempimento. I bordi scuri
# superano 3:1 sulla tinta chiara (WCAG 1.4.11, elementi non testuali).
PALETTE = {
    "nav": ("#ccfbf1", "#0f766e"),        # comandi di navigazione
    "home": ("#e0f2fe", "#0369a1"),       # ritorno alla home
    "people": ("#fef3c7", "#b45309"),     # pronomi e persone
    "action": ("#dcfce7", "#15803d"),     # verbi e azioni
    "descriptor": ("#dbeafe", "#1d4ed8"), # aggettivi e descrittori
    "noun": ("#ffedd5", "#c2410c"),       # nomi e oggetti
    "social": ("#fce7f3", "#be185d"),     # saluti, cortesie, domande
    "urgent": ("#fee2e2", "#b91c1c"),     # no, stop, aiuto, dolore
    "neutral": ("#ffffff", "#64748b"),    # parole non classificate
    "missing": ("#f1f5f9", "#94a3b8"),    # riquadro senza pittogramma
}

INK = "#111111"
TITLE_BG = "#0f766e"
TITLE_INK = "#ffffff"
MUTED_INK = "#334155"

PEOPLE = {
    "io", "tu", "lui", "lei", "noi", "voi", "loro", "mio", "mia", "tuo", "tua",
    "mamma", "papà", "papa", "babbo", "nonna", "nonno", "fratello", "sorella",
    "bambino", "bambina", "bambini", "ragazzo", "ragazza", "maestra", "maestro",
    "insegnante", "educatore", "educatrice", "amico", "amica", "amici",
    "famiglia", "genitori", "persona", "persone", "compagno", "compagna",
    "dottore", "dottoressa", "infermiere", "medico", "dentista", "logopedista",
    "psicologo", "assistente", "vicino", "conoscente", "ospite", "ospiti",
}
ACTIONS = {
    "voglio", "vorrei", "vuoi", "volere", "avere", "ho", "hai", "essere", "sono",
    "sei", "fare", "faccio", "andare", "vado", "va", "venire", "vengo", "stare",
    "sto", "mangiare", "mangio", "bere", "bevo", "dormire", "dormo", "giocare",
    "gioco", "uscire", "esco", "guardare", "guardo", "ascoltare", "ascolto",
    "leggere", "leggo", "scrivere", "scrivo", "parlare", "parlo", "camminare",
    "correre", "aspettare", "finire", "finito", "lavarsi", "lavare",
    "vestirsi", "aprire", "chiudere", "chiedere", "aiutare", "prendere", "dare",
    "mettere", "togliere", "salutare", "abbracciare", "respirare", "contare",
    "cantare", "ballare", "disegnare", "colorare", "tagliare", "incollare",
    "capire", "ricordare", "provare", "ripetere", "scegliere", "continuare",
}
DESCRIPTORS = {
    "grande", "piccolo", "piccola", "alto", "bassa", "basso", "lungo", "corto",
    "caldo", "freddo", "bello", "brutta", "brutto", "buono", "cattivo",
    "importante", "facile", "difficile", "veloce", "lento", "pulito", "sporco",
    "pieno", "vuoto", "pesante", "leggero", "duro", "morbido", "rumoroso",
    "silenzioso", "buio", "luminoso", "stanco", "stanca", "pronto", "pronta",
    "bene", "male", "meglio", "peggio", "più", "meno", "molto", "poco", "troppo",
    "ancora", "adesso", "dopo", "prima", "oggi", "domani", "ieri", "sempre",
    "mai", "a volte", "forte", "piano", "dolce", "amaro", "salato", "acido",
}
SOCIAL = {
    "ciao", "buongiorno", "buonasera", "buonanotte", "salve", "grazie",
    "prego", "scusa", "scusi", "per favore", "per piacere", "come stai",
    "sto bene", "piacere", "arrivederci", "a dopo", "a domani", "sì", "si",
    "va bene", "ok", "d'accordo", "come", "perché", "perche", "dove", "quando",
    "chi", "cosa", "quale", "quanto", "posso", "puoi", "vuoi", "ti va",
    "mi aiuti", "mi scusi", "non capisco", "ripeti", "più lentamente",
    "puoi spiegarmi meglio", "secondo me", "penso che", "non sono d'accordo",
}
URGENT = {
    "no", "basta", "stop", "aiuto", "dolore", "male", "fa male", "paura",
    "spaventato", "arrabbiato", "arrabbiata", "emergenza", "pericolo",
    "vomito", "febbre", "sangue", "soffoco", "non respiro", "bagno",
}
CORE_WORDS = {
    "io", "tu", "lui", "lei", "noi", "voi", "loro", "voglio", "vuoi", "avere",
    "fare", "andare", "stare", "bene", "male", "più", "basta", "aiuto",
    "grazie", "sì", "no", "ancora", "finito", "mangiare", "bere", "giocare",
    "uscire", "guardare", "ascoltare", "leggere", "scrivere", "parlare",
    "dormire", "lavarsi le mani", "felice", "triste", "arrabbiato", "paura",
    "stanco", "casa", "scuola", "acqua", "pane", "pasta",
}

NAV_ACTIONS = {"home", "back", "navigate"}


def norm(text: str) -> str:
    text = (text or "").strip().lower()
    text = re.sub(r"[«»\"'’.,!?;:()\[\]]+", "", text)
    return re.sub(r"\s+", " ", text).strip()


def word_class(text: str, action_type: str) -> str:
    """Classe visiva della cella. Le liste sono esplicite: nessuna inferenza
    sulle parole non elencate, che restano neutre."""
    if action_type == "home":
        return "home"
    if action_type in ("navigate", "back"):
        return "nav"
    key = norm(text)
    if not key:
        return "neutral"
    if key in URGENT:
        return "urgent"
    if key in PEOPLE:
        return "people"
    if key in ACTIONS:
        return "action"
    if key in DESCRIPTORS:
        return "descriptor"
    if key in SOCIAL:
        return "social"
    if len(key.split()) > 4:
        # Istruzioni e frasi: restano su fondo bianco per non perdere leggibilità.
        return "neutral"
    return "noun"


def clamp(value, low, high):
    try:
        value = float(value)
    except (TypeError, ValueError):
        return low
    return max(low, min(high, value))


def style_symbol(element: dict, kind: str) -> dict:
    style = element.setdefault("style", {})
    bg, border = PALETTE[kind]
    style["background"] = bg
    style["border_color"] = border
    style["border_width"] = 0.7 if kind == "neutral" else 0.9
    style["border_radius"] = 2.4
    style["padding_mm"] = clamp(style.get("padding_mm"), 1.4, 2.6)
    style["align"] = style.get("align") or "center"
    style["bold"] = kind in ("nav", "home", "urgent")
    if not (style.get("color") or "").strip():
        style["color"] = INK
    style["font_size"] = clamp(style.get("font_size"), 4.2, 9.0)
    return style


def style_missing(element: dict) -> dict:
    """Riquadro senza pittogramma: fondo chiaro e bordo tratteggiato.
    Il tratteggio segnala che il pittogramma manca davvero (nessun simbolo
    inventato) e insieme dà un aspetto intenzionale al riquadro."""
    style = element.setdefault("style", {})
    bg, border = PALETTE["missing"]
    style["background"] = bg
    style["border_color"] = border
    style["border_width"] = 0.8
    style["border_style"] = "dashed"
    style["border_radius"] = 2.4
    style["padding_mm"] = clamp(style.get("padding_mm"), 1.4, 2.6)
    style["align"] = "center"
    style["bold"] = False
    if not (style.get("color") or "").strip():
        style["color"] = INK
    return style


def redesign_element(element: dict) -> dict:
    kind = element.get("kind") or "symbol"
    action = (element.get("action") or {}).get("type") or "none"
    style = element.setdefault("style", {})
    metadata = element.get("metadata") or {}
    has_picture = bool((element.get("svg") or "").strip()) or bool(
        (element.get("image_asset") or "").strip()
    )

    if kind == "title":
        style.update(
            {
                "background": TITLE_BG,
                "color": TITLE_INK,
                "border_color": TITLE_BG,
                "border_width": 0.0,
                "border_radius": 2.6,
                "padding_mm": clamp(style.get("padding_mm"), 1.6, 3.0),
                "bold": True,
                "align": style.get("align") or "left",
                "font_size": clamp(style.get("font_size"), 7.0, 10.0),
            }
        )
        element.setdefault("accessibility", {})["large_text"] = True
    elif kind == "subtitle":
        style.update(
            {
                "background": "",
                "color": MUTED_INK,
                "border_width": 0.0,
                "border_radius": 0.0,
                "padding_mm": 0.0,
                "bold": False,
                "align": style.get("align") or "left",
                "font_size": clamp(style.get("font_size"), 3.4, 5.0),
            }
        )
    elif kind == "badge":
        style.update(
            {
                "background": "#e2e8f0",
                "color": "#1f2937",
                "border_color": "#cbd5e1",
                "border_width": 0.6,
                "border_radius": 2.2,
                "padding_mm": clamp(style.get("padding_mm"), 1.0, 2.0),
                "bold": True,
                "font_size": clamp(style.get("font_size"), 3.0, 4.2),
            }
        )
    elif kind == "button":
        style.update(
            {
                "background": TITLE_BG,
                "color": TITLE_INK,
                "border_color": "#0b5c56",
                "border_width": 0.9,
                "border_radius": 2.6,
                "padding_mm": clamp(style.get("padding_mm"), 1.4, 2.6),
                "bold": True,
                "align": "center",
                "font_size": clamp(style.get("font_size"), 4.2, 6.0),
            }
        )
        element.setdefault("accessibility", {})["large_text"] = True
    elif kind in ("text", "text_box"):
        style.update(
            {
                "background": "",
                "color": MUTED_INK,
                "border_width": 0.0,
                "border_radius": 0.0,
                "padding_mm": 0.0,
                "bold": False,
                "font_size": clamp(style.get("font_size"), 3.2, 5.0),
            }
        )
    elif metadata.get("missing_symbol") and not has_picture:
        style_missing(element)
        element.setdefault("accessibility", {})["large_text"] = True
    else:
        style_symbol(element, word_class(element.get("text") or "", action))
        if action in ("home", "back"):
            element.setdefault("accessibility", {})["large_text"] = True

    # Nome accessibile: etichetta esplicita, poi il testo parlato, poi il testo.
    accessibility = element.setdefault("accessibility", {})
    label = (accessibility.get("label") or "").strip()
    if not label:
        label = (
            (element.get("spoken_text") or "").strip()
            or (element.get("text") or "").strip()
        )
    accessibility["label"] = label[:120]
    accessibility.setdefault("large_text", False)
    return element


def redesign_document(document: dict) -> tuple[dict, Counter]:
    stats = Counter()
    page_settings = document.setdefault("page", {})
    if page_settings.get("margin_mm") is None or page_settings.get("margin_mm", 0) > 10:
        page_settings["margin_mm"] = 10.0
    page_settings["background"] = "#ffffff"
    document.setdefault("accessibility", {})
    if not (document["accessibility"].get("label") or "").strip():
        document["accessibility"]["label"] = (document.get("title") or "").strip()[:120]

    for page in document.get("pages") or []:
        page["background"] = ""
        grid = page.get("grid")
        if isinstance(grid, dict) and grid.get("gap_mm", 0) > 2.4:
            grid["gap_mm"] = 2.4
        flow = page.get("flow")
        if isinstance(flow, dict) and flow.get("gap_mm", 0) > 2.4:
            flow["gap_mm"] = 2.4
        for element in page.get("elements") or []:
            redesign_element(element)
            stats["elementi"] += 1
            action = (element.get("action") or {}).get("type") or "none"
            if action in NAV_ACTIONS:
                stats["navigazione"] += 1
            if norm(element.get("text") or "") in CORE_WORDS:
                stats["core"] += 1
    return document, stats


def load_catalog() -> dict:
    return json.loads(CATALOG.read_text(encoding="utf-8"))


def read_payload(path: Path) -> dict:
    with zipfile.ZipFile(path) as zf:
        return json.loads(zf.read("document.json"))


def rewrite_document(path: Path, payload: dict) -> None:
    """Sostituisce solo `document.json` nel `.caadoc`: gli asset restano
    identici, quindi il documento non raddoppia i pittogrammi."""
    with zipfile.ZipFile(path) as zf:
        entries = [(info.filename, zf.read(info.filename)) for info in zf.infolist()]
    with tempfile.TemporaryDirectory(prefix="caa-aac-") as tmp:
        tmpdir = Path(tmp)
        for name, data in entries:
            if name == "document.json":
                continue
            target = tmpdir / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
        (tmpdir / "document.json").write_text(
            json.dumps(payload, ensure_ascii=False, separators=(",", ":")),
            encoding="utf-8",
        )
        out = tmpdir / "out.caadoc"
        with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zf:
            for item in sorted(tmpdir.rglob("*")):
                if item.is_file() and item.name != "out.caadoc":
                    zf.write(item, item.relative_to(tmpdir).as_posix())
        shutil.move(str(out), path)


def run_cli(requests, resources: Path) -> dict:
    """Invia richieste alla CLI (rpc JSONL) e raccoglie le risposte per id."""
    env = dict(os.environ)
    env["CAA_WRITER_DEV_LIBRARY"] = "1"
    command = [str(CLI), "--no-bridge", "--resources", str(resources), "rpc"]
    process = subprocess.Popen(
        command,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    results = {}
    try:
        for request in requests:
            process.stdin.write(json.dumps(request, ensure_ascii=False) + "\n")
            process.stdin.flush()
            line = process.stdout.readline()
            if not line:
                break
            try:
                results[request["id"]] = json.loads(line)
            except json.JSONDecodeError:
                results[request["id"]] = {"ok": False, "error": "risposta non JSON"}
    finally:
        try:
            process.stdin.close()
        except Exception:
            pass
        process.wait(timeout=600)
        stderr = process.stderr.read()
        if stderr.strip():
            print(stderr.strip()[:2000], file=sys.stderr)
    return results


def main() -> int:
    parser = argparse.ArgumentParser(description="Ridisegna il pacchetto AAC.")
    parser.add_argument("--apply", action="store_true", help="scrive i documenti")
    parser.add_argument("--limit", type=int, default=0, help="numero massimo di documenti")
    parser.add_argument("--only", default="", help="solo un documento (id)")
    args = parser.parse_args()

    if not CATALOG.exists():
        print(f"Catalogo non trovato: {CATALOG}", file=sys.stderr)
        return 2
    if args.apply and not CLI.exists():
        print(f"CLI di debug non trovata: {CLI}", file=sys.stderr)
        return 2

    catalog = load_catalog()
    targets = [e for e in catalog.get("entries", []) if e.get("category_id") == "aac"]
    if args.only:
        targets = [e for e in targets if e.get("id") == args.only]
    targets.sort(key=lambda entry: entry.get("path", ""))
    if args.limit:
        targets = targets[: args.limit]

    print(f"Documenti selezionati: {len(targets)} (apply={args.apply})")
    stats_total = Counter()
    prepared = {}
    for index, entry in enumerate(targets, start=1):
        path = RESOURCES / entry["path"]
        if not path.exists():
            print(f"  ! manca {path}", file=sys.stderr)
            continue
        payload = read_payload(path)
        document = payload.get("universal") or {}
        if not document.get("pages"):
            print(f"  - {entry['id']}: nessuna pagina universale, salto")
            continue
        payload["universal"], stats = redesign_document(document)
        stats_total.update(stats)
        prepared[index] = (entry, payload)

    print(
        f"Elementi ridisegnati: {stats_total['elementi']}, "
        f"navigazione: {stats_total['navigazione']}, "
        f"core: {stats_total['core']}"
    )
    if not args.apply:
        print("Dry-run: nessun file modificato. Usa --apply per scrivere.")
        return 0

    written, failed = [], []
    for index, (entry, payload) in prepared.items():
        try:
            rewrite_document(RESOURCES / entry["path"], payload)
            written.append(index)
        except Exception as error:  # noqa: BLE001 - riportato, non nascosto
            failed.append((entry["id"], f"scrittura: {error}"))
    print(f"Documenti riscritti: {len(written)} — errori: {len(failed)}")

    # Anteprime rigenerate dal renderer ufficiale.
    requests = [
        {
            "id": index,
            "method": "dev.library.preview",
            "params": {"path": prepared[index][0]["path"]},
        }
        for index in written
    ]
    previews = run_cli(requests, RESOURCES) if requests else {}
    preview_ok = 0
    for index in written:
        entry = prepared[index][0]
        response = previews.get(index) or {}
        result = response.get("result") or {}
        svg = result.get("svg") if isinstance(result, dict) else None
        if response.get("ok") and svg:
            target = RESOURCES / entry["preview"]
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(svg, encoding="utf-8")
            preview_ok += 1
        else:
            failed.append((entry["id"], "anteprima: " + str(response.get("error"))))
    print(f"Anteprime rigenerate: {preview_ok}/{len(written)}")

    # Catalogo: etichette editoriali e data di generazione.
    catalog = load_catalog()
    saved_ids = {prepared[index][0]["id"] for index in written}
    updated = 0
    for entry in catalog.get("entries", []):
        if entry.get("id") not in saved_ids:
            continue
        tags = entry.setdefault("tags", [])
        for tag in ("accessibilità", "chiave fitzgerald", "design 2026"):
            if tag not in tags:
                tags.append(tag)
                updated += 1
    catalog["generated_at"] = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    CATALOG.write_text(
        json.dumps(catalog, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    print(f"Catalogo: {updated} etichette aggiunte.")

    for document_id, error in failed[:10]:
        print(f"  ! {document_id}: {error}", file=sys.stderr)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
