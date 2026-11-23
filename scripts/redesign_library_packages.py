#!/usr/bin/env python3
"""
Ridisegna tutti i pacchetti della libreria.

Per ognuno dei 350 documenti del catalogo:

  * applica un sistema grafico unico (raggi 2,4 mm, bordi leggeri, spaziature,
    margine pagina 10 mm) e un accento di genere — colore diverso per schede,
    agende, TEACCH, storie sociali, giochi, materiali comportamentali,
    stampabili, storie in CAA e comunicazione aumentativa — in modo che si
    riconosca a colpo d'occhio il tipo di materiale;
  * costruisce una gerarchia didattica costante: titolo (fascia piena) →
    sottotitolo → obiettivo (badge) → istruzione (riquadro tinto) → attività
    (domanda, risposta, caselle) → nota per l'adulto (testo attenuato);
  * colora le celle di vocabolario per classe di parola (chiave Fitzgerald) con
    liste esplicite; le parole non elencate restano neutre;
  * tratteggia i riquadri senza pittogramma invece di inventare un simbolo;
  * compila `accessibility.label` su ogni elemento e imposta `large_text` su
    titoli, istruzioni, domande, badge e navigazione.

Nessun testo, simbolo, licenza o metadato di contenuto viene modificato: si
toccano solo stile e metadati di accessibilità. Il `.caadoc` viene riscritto
sostituendo soltanto `document.json` — gli asset restano identici — e le
anteprime vengono rigenerate dal renderer ufficiale (`dev.library.preview`).

Uso:
  python3 scripts/redesign_library_packages.py                   # dry-run
  python3 scripts/redesign_library_packages.py --apply           # scrive
  python3 scripts/redesign_library_packages.py --category schede --limit 2
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

# ---------------------------------------------------------------- generi ---
# Ogni categoria ha un accento scuro (testo bianco sopra ≥4,5:1) e una tinta
# chiara per i riquadri didattici. Il resto della tavolozza è condiviso.
GENRES = {
    "aac": {"accent": "#0f766e", "tint": "#f0fdfa", "name": "comunicazione"},
    "agende": {"accent": "#0e7490", "tint": "#ecfeff", "name": "agende"},
    "comportamentali": {"accent": "#6d28d9", "tint": "#f5f3ff", "name": "comportamento"},
    "giochi": {"accent": "#b45309", "tint": "#fffbeb", "name": "giochi"},
    "schede": {"accent": "#1d4ed8", "tint": "#eff6ff", "name": "schede"},
    "storie-sociali": {"accent": "#be185d", "tint": "#fdf2f8", "name": "storie sociali"},
    "storie-caa": {"accent": "#4338ca", "tint": "#eef2ff", "name": "storie CAA"},
    "teacch": {"accent": "#15803d", "tint": "#f0fdf4", "name": "TEACCH"},
    "stampabili": {"accent": "#334155", "tint": "#f8fafc", "name": "stampabili"},
}
DEFAULT_GENRE = GENRES["stampabili"]

# Classi di parola (chiave Fitzgerald) e tinte chiare con bordo scuro: il testo
# predefinito del renderer è #111111, ogni accoppiamento supera 15:1.
PALETTE = {
    "nav": ("#ccfbf1", "#0f766e"),
    "home": ("#e0f2fe", "#0369a1"),
    "people": ("#fef3c7", "#b45309"),
    "action": ("#dcfce7", "#15803d"),
    "descriptor": ("#dbeafe", "#1d4ed8"),
    "noun": ("#ffedd5", "#c2410c"),
    "social": ("#fce7f3", "#be185d"),
    "urgent": ("#fee2e2", "#b91c1c"),
    "neutral": ("#ffffff", "#64748b"),
    "missing": ("#f1f5f9", "#94a3b8"),
}

INK = "#111111"
BODY_INK = "#1f2937"
MUTED_INK = "#475569"
LINE_INK = "#64748b"

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
    "completare", "cerchiare", "segnare", "abbina", "abbinare", "collega",
    "collegare", "copia", "copiare", "ritaglia", "ritagliare", "unisci",
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
    "uguale", "diverso", "contrario", "giusto", "sbagliato", "primo", "ultimo",
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
# Descrizioni per i componenti didattici privi di testo: dicono che cosa è
# l'elemento, senza attribuirgli un contenuto che non ha.
KIND_LABELS = {
    "answer_line": "Riga per scrivere",
    "handwriting_line": "Riga per scrivere a mano",
    "checkbox": "Casella da spuntare",
    "token_slot": "Casella per il gettone",
    "ten_frame": "Dieci caselle per contare",
    "number_line": "Linea dei numeri",
    "timeline": "Sequenza temporale",
    "text_box": "Riquadro per scrivere o incollare",
}

NUMERIC = re.compile(r"^\d{1,3}([.,]\d{1,2})?$")
SINGLE_LETTER = re.compile(r"^[a-zàèéìòù]$")


def norm(text: str) -> str:
    text = (text or "").strip().lower()
    text = re.sub(r"[«»\"'’.,!?;:()\[\]]+", "", text)
    return re.sub(r"\s+", " ", text).strip()


def word_class(text: str, action_type: str) -> str:
    """Classe visiva della cella; nessuna inferenza sulle parole non elencate."""
    if action_type == "home":
        return "home"
    if action_type in ("navigate", "back"):
        return "nav"
    key = norm(text)
    if not key:
        return "neutral"
    # Numeri, lettere e date («1», «15: compleanno») restano neutri: nelle
    # agende e nelle schede di matematica il colore di classe non comunica
    # nulla e riempirebbe la pagina di tinte forti.
    if NUMERIC.match(key) or SINGLE_LETTER.match(key) or re.search(r"\d", key):
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
        return "neutral"
    return "noun"


def clamp(value, low, high):
    try:
        value = float(value)
    except (TypeError, ValueError):
        return low
    return max(low, min(high, value))


def wrap_lines(text: str, max_chars: int) -> int:
    """Stima le righe come il renderer: parole intere, larghezza in caratteri."""
    words = text.split()
    if not words:
        return 0
    lines, current = 1, 0
    for word in words:
        need = len(word) + (1 if current else 0)
        if current + need > max_chars:
            lines += 1
            current = len(word)
        else:
            current += need
    return lines


def fits(element: dict, font: float, style: dict, cap: float = 1.0) -> bool:
    """True se il testo entra nell'elemento con la dimensione indicata."""
    text = (element.get("text") or "").strip()
    if not text:
        return True
    padding = style.get("padding_mm") or 0.0
    inner_w = max(1.0, (element.get("w_mm") or 0.0) - 2 * padding)
    inner_h = max(1.0, (element.get("h_mm") or 0.0) - 2 * padding)
    max_chars = max(3, int(inner_w / (font * 0.55)))
    lines = wrap_lines(text, max_chars)
    needed = lines * font * 1.28
    return needed <= inner_h * cap + 0.01


def readable_font(element: dict, style: dict, floor: float, cap: float = 1.0) -> float:
    """La dimensione più grande fra `floor` e quella attuale che entra nello
    spazio disponibile; se nulla entra, lascia la dimensione originale."""
    current = clamp(style.get("font_size"), 2.0, 40.0)
    if current >= floor:
        return current
    probe = round(floor, 2)
    while probe > current:
        if fits(element, probe, style, cap):
            return probe
        probe = round(probe - 0.1, 2)
    return current


def box(style: dict, *, background: str, ink: str, border: str, width: float,
        radius: float, padding: float, bold: bool = False,
        align: str = "", border_style: str = "") -> None:
    style["background"] = background
    style["color"] = ink
    style["border_color"] = border
    style["border_width"] = width
    style["border_style"] = border_style
    style["border_radius"] = radius
    style["padding_mm"] = padding
    style["bold"] = bold
    if align:
        style["align"] = align


def style_cell(cell: dict, genre: dict) -> None:
    """Cella di una griglia interna (frasi CAA, storie, tabelle)."""
    style = cell.setdefault("style", {})
    box(
        style,
        background="#ffffff",
        ink=INK,
        border="#94a3b8",
        width=0.6,
        radius=1.8,
        padding=1.2,
        align="center",
    )
    cell["style"] = style


def redesign_element(element: dict, genre: dict, primary_title: bool = True) -> str:
    kind = element.get("kind") or "symbol"
    action = (element.get("action") or {}).get("type") or "none"
    style = element.setdefault("style", {})
    metadata = element.get("metadata") or {}
    accent = genre["accent"]
    tint = genre["tint"]
    has_picture = bool((element.get("svg") or "").strip()) or bool(
        (element.get("image_asset") or "").strip()
    )
    foreground = style.get("background") or ""

    # Gerarchia didattica: titolo → sottotitolo → obiettivo → istruzione →
    # attività → nota per l'adulto.
    if kind == "title":
        if primary_title:
            # Intestazione di pagina: fascia piena con il colore del genere.
            box(style, background=accent, ink="#ffffff", border=accent, width=0.0,
                radius=2.6, padding=clamp(style.get("padding_mm"), 1.6, 3.0),
                bold=True, align=style.get("align") or "left")
            style["font_size"] = clamp(style.get("font_size"), 7.0, 10.0)
        else:
            # Titolo di sezione nella stessa pagina: niente seconda fascia,
            # altrimenti due bande piene competono e la gerarchia si appiattisce.
            box(style, background="", ink=accent, border="", width=0.0,
                radius=0.0, padding=0.0, bold=True,
                align=style.get("align") or "left")
            style["font_size"] = clamp(style.get("font_size"), 5.4, 7.4)
    elif kind == "subtitle":
        box(style, background="", ink="#334155", border="", width=0.0,
            radius=0.0, padding=0.0, bold=False,
            align=style.get("align") or "left")
        style["font_size"] = readable_font(element, style, 3.6)
    elif kind == "badge":
        box(style, background=accent, ink="#ffffff", border=accent, width=0.0,
            radius=2.4, padding=clamp(style.get("padding_mm"), 1.0, 2.0),
            bold=True, align="center")
        style["font_size"] = readable_font(element, style, 3.3)
    elif kind == "instruction":
        box(style, background=tint, ink=accent, border=accent, width=0.6,
            radius=2.4, padding=clamp(style.get("padding_mm"), 1.4, 2.6),
            bold=False, align=style.get("align") or "left")
        style["font_size"] = readable_font(element, style, 3.8)
    elif kind == "question":
        box(style, background="", ink=INK, border="", width=0.0, radius=0.0,
            padding=0.0, bold=True, align=style.get("align") or "left")
        style["font_size"] = clamp(style.get("font_size"), 4.4, 12.0)
    elif kind == "note":
        box(style, background="", ink=MUTED_INK, border="", width=0.0,
            radius=0.0, padding=0.0, bold=False,
            align=style.get("align") or "left")
        style["font_size"] = readable_font(element, style, 3.4)
    elif kind in ("answer_line", "handwriting_line"):
        box(style, background="", ink=MUTED_INK, border=LINE_INK, width=0.0,
            radius=0.0, padding=0.0, bold=False,
            align=style.get("align") or "left")
        style["font_size"] = readable_font(element, style, 3.2)
    elif kind == "checkbox":
        box(style, background="", ink=BODY_INK, border="#475569", width=0.5,
            radius=0.8, padding=0.0, bold=False)
        style["font_size"] = readable_font(element, style, 3.2)
    elif kind == "token_slot":
        box(style, background="", ink=BODY_INK, border="#475569", width=0.6,
            radius=2.0, padding=0.0, bold=False)
        style["font_size"] = readable_font(element, style, 3.2)
    elif kind in ("number_line", "ten_frame", "timeline"):
        box(style, background="#ffffff" if kind == "ten_frame" else "",
            ink=BODY_INK, border="#475569", width=0.5, radius=1.8,
            padding=0.0, bold=False)
    elif kind == "speech":
        box(style, background="#ffffff", ink="#172033", border="#1f2937",
            width=0.9, radius=3.4, padding=clamp(style.get("padding_mm"), 1.6, 3.0),
            bold=False, align="center")
        style["font_size"] = readable_font(element, style, 3.4, cap=0.9)
    elif kind == "shape":
        box(style, background=foreground or tint, ink=MUTED_INK,
            border=(style.get("border_color") or "#cbd5e1"), width=0.6,
            radius=2.6, padding=clamp(style.get("padding_mm"), 1.2, 2.6),
            bold=False, align=style.get("align") or "center")
    elif kind == "text_box":
        box(style, background=foreground or "#ffffff", ink=BODY_INK,
            border="#cbd5e1", width=0.6, radius=2.4,
            padding=clamp(style.get("padding_mm"), 1.4, 2.8), bold=False,
            align=style.get("align") or "left")
        style["font_size"] = readable_font(element, style, 3.6, cap=0.95)
    elif kind == "button":
        box(style, background=accent, ink="#ffffff", border=accent, width=0.9,
            radius=2.4, padding=clamp(style.get("padding_mm"), 1.2, 2.4),
            bold=True, align="center")
        style["font_size"] = readable_font(element, style, 4.0)
    elif kind in ("text", "grid"):
        if foreground:
            style["border_radius"] = 2.4
            style["color"] = style.get("color") or "#ffffff"
        else:
            box(style, background="", ink=MUTED_INK if kind == "text" else BODY_INK,
                border="", width=0.0, radius=0.0, padding=0.0, bold=False,
                align=style.get("align") or "left")
    elif metadata.get("missing_symbol") and not has_picture:
        background, border = PALETTE["missing"]
        box(style, background=background, ink=INK, border=border, width=0.8,
            radius=2.4, padding=clamp(style.get("padding_mm"), 1.4, 2.6),
            align="center", border_style="dashed")
        style["font_size"] = readable_font(element, style, 4.2, cap=0.6)
    else:
        background, border = PALETTE[word_class(element.get("text") or "", action)]
        bold = action in ("home", "back") or word_class(
            element.get("text") or "", action
        ) in ("nav", "home", "urgent")
        box(style, background=background, ink=INK, border=border,
            width=0.7 if background == "#ffffff" else 0.9, radius=2.4,
            padding=clamp(style.get("padding_mm"), 1.4, 2.6),
            bold=bold, align=style.get("align") or "center")
        style["font_size"] = readable_font(element, style, 4.2, cap=0.6)

    if element.get("cells"):
        for cell in element["cells"]:
            style_cell(cell, genre)
    grid = element.get("grid")
    if isinstance(grid, dict):
        for cell in grid.get("cells") or []:
            style_cell(cell, genre)

    # Nome accessibile: etichetta esplicita, poi il testo parlato, poi il testo,
    # infine una descrizione del componente (mai del contenuto inventato).
    accessibility = element.setdefault("accessibility", {})
    label = (accessibility.get("label") or "").strip()
    if not label:
        label = (
            (element.get("spoken_text") or "").strip()
            or (element.get("text") or "").strip()
            or (element.get("lemma") or "").strip()
            or KIND_LABELS.get(kind, "")
        )
    if not label and kind in ("symbol", "image"):
        symbol_id = (element.get("symbol_id") or "")
        label = (
            "Pittogramma ARASAAC" if symbol_id.startswith("arasaac")
            else "Pittogramma" if symbol_id else "Immagine"
        )
    if not label and kind == "grid":
        grid = element.get("grid") or {}
        columns = grid.get("columns") or 0
        rows = grid.get("rows") or 0
        label = (
            f"Griglia di {rows} righe e {columns} colonne"
            if rows and columns else "Griglia di celle"
        )
    accessibility["label"] = label[:120]
    important = kind in ("title", "subtitle", "badge", "instruction", "question")
    if action in ("home", "back") or norm(element.get("text") or "") in CORE_WORDS:
        important = True
    if important:
        accessibility["large_text"] = True
    accessibility.setdefault("large_text", False)
    return kind


def redesign_document(document: dict, genre: dict) -> Counter:
    stats = Counter()
    page_settings = document.setdefault("page", {})
    margin = page_settings.get("margin_mm")
    if margin is None or margin > 10:
        page_settings["margin_mm"] = 10.0
    page_settings["background"] = "#ffffff"
    document.setdefault("accessibility", {})
    if not (document["accessibility"].get("label") or "").strip():
        document["accessibility"]["label"] = (document.get("title") or "").strip()[:120]

    for page in document.get("pages") or []:
        page["background"] = ""
        for container in ("grid", "flow"):
            block = page.get(container)
            if isinstance(block, dict) and block.get("gap_mm", 0) > 2.4:
                block["gap_mm"] = 2.4
            if isinstance(block, dict):
                for cell in block.get("cells") or []:
                    style_cell(cell, genre)
        elements = page.get("elements") or []
        titles = [
            element for element in elements
            if (element.get("kind") or "") == "title"
        ]
        # L'intestazione di pagina è il titolo più in alto, non il primo
        # nell'ordine di salvataggio degli elementi.
        primary_title_id = min(
            titles,
            key=lambda item: (item.get("y_mm") or 0.0, -(item.get("w_mm") or 0.0)),
        ).get("id") if titles else None
        for element in elements:
            kind = redesign_element(
                element, genre, primary_title=element.get("id") == primary_title_id
            )
            stats["elementi"] += 1
            stats[f"tipo:{kind}"] += 1
            action = (element.get("action") or {}).get("type") or "none"
            if action in NAV_ACTIONS:
                stats["navigazione"] += 1
    return stats


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
    with tempfile.TemporaryDirectory(prefix="caa-library-") as tmp:
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


def run_cli(requests, method_timeout: int = 600) -> dict:
    env = dict(os.environ)
    env["CAA_WRITER_DEV_LIBRARY"] = "1"
    command = [str(CLI), "--no-bridge", "--resources", str(RESOURCES), "rpc"]
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
        process.wait(timeout=method_timeout)
        stderr = process.stderr.read()
        if stderr.strip():
            print(stderr.strip()[:2000], file=sys.stderr)
    return results


def main() -> int:
    parser = argparse.ArgumentParser(description="Ridisegna i pacchetti della libreria.")
    parser.add_argument("--apply", action="store_true", help="scrive i documenti")
    parser.add_argument("--category", default="", help="solo una categoria")
    parser.add_argument("--limit", type=int, default=0, help="numero massimo di documenti")
    parser.add_argument("--only", default="", help="solo un documento (id)")
    parser.add_argument("--no-preview", action="store_true", help="non rigenera le anteprime")
    args = parser.parse_args()

    if not CATALOG.exists():
        print(f"Catalogo non trovato: {CATALOG}", file=sys.stderr)
        return 2
    if args.apply and not CLI.exists():
        print(f"CLI di debug non trovata: {CLI}", file=sys.stderr)
        return 2

    catalog = load_catalog()
    targets = [e for e in catalog.get("entries", []) if e.get("category_id") in GENRES]
    if args.category:
        targets = [e for e in targets if e["category_id"] == args.category]
    if args.only:
        targets = [e for e in targets if e.get("id") == args.only]
    targets.sort(key=lambda entry: entry.get("path", ""))
    if args.limit:
        targets = targets[: args.limit]

    print(f"Documenti selezionati: {len(targets)} (apply={args.apply})")
    stats_total = Counter()
    per_category = Counter()
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
        genre = GENRES.get(entry["category_id"], DEFAULT_GENRE)
        stats = redesign_document(document, genre)
        stats_total.update(stats)
        per_category[entry["category_id"]] += 1
        prepared[index] = (entry, payload)

    print(f"Elementi ridisegnati: {stats_total['elementi']}")
    print("Documenti per categoria:", dict(per_category))
    for kind, count in sorted(stats_total.items()):
        if kind.startswith("tipo:"):
            print(f"  {kind[5:]:16} {count}")
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

    preview_ok = 0
    if not args.no_preview:
        requests = [
            {
                "id": index,
                "method": "dev.library.preview",
                "params": {"path": prepared[index][0]["path"]},
            }
            for index in written
        ]
        previews = run_cli(requests) if requests else {}
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

    catalog = load_catalog()
    saved = {prepared[index][0]["id"]: prepared[index][0] for index in written}
    updated = 0
    for entry in catalog.get("entries", []):
        if entry.get("id") not in saved:
            continue
        genre = GENRES.get(entry["category_id"], DEFAULT_GENRE)
        tags = entry.setdefault("tags", [])
        for tag in ("accessibilità", "gerarchia didattica", "design 2026", genre["name"]):
            if tag not in tags:
                tags.append(tag)
                updated += 1
    catalog["generated_at"] = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    CATALOG.write_text(json.dumps(catalog, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"Catalogo: {updated} etichette aggiunte.")

    for document_id, error in failed[:10]:
        print(f"  ! {document_id}: {error}", file=sys.stderr)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
