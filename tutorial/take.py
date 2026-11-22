#!/usr/bin/env python3
"""Registra una presa Recordly per un capitolo del tutorial.

    python3 tutorial/take.py 03              # prepara, registra, archivia
    python3 tutorial/take.py 03 --prepare    # solo preparazione
    python3 tutorial/take.py 03 --dry        # stampa i passaggi senza eseguire

Ogni capitolo ha due liste:
  prepare  azioni fatte PRIMA di premere Record (non entrano nel video)
  take     azioni registrate, cronometrate sulla durata della narrazione

Segmenti: ("click", x, y), ("move", x, y), ("drag", x, y, x2, y2),
("type", testo), ("key", codice, [modificatori]), ("wait", secondi),
("shot", nome) per un fermo immagine di controllo.

Coordinate in punti schermo, con finestra CAA Writer a 1440x803 in (0,29):
i riferimenti sono presi da screenshot a 900 px di larghezza (fattore 1,6).
"""
import glob
import json
import os
import shutil
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.abspath(__file__))
PROJECT = os.path.dirname(ROOT)
TAKES = os.path.join(ROOT, "takes")
APP = "CAA Writer"

sys.path.insert(0, ROOT)
import uia  # noqa: E402

# --- viste -----------------------------------------------------------------

HOME = {"creator": (452, 610), "luca": (720, 450)}

DASH = {
    "home_tab": (83, 181), "apri": (304, 181), "library": (480, 181), "packages": (634, 181),
    "settings": (741, 181), "users": (880, 181), "back_util": (1026, 181),
    "wp_card": (272, 334), "tables_card": (581, 334), "agenda_card": (818, 334),
    "aba_card": (1080, 334), "teacch_card": (272, 557), "smart_card": (581, 557),
    "free_card": (818, 557), "story": (1197, 731), "controls": (45, 838),
}

ED = {
    "page": (400, 300), "page2": (400, 380),
    "tab_home": (69, 139), "tab_doc": (133, 139), "tab_insert": (232, 139),
    "tab_layout": (326, 139), "tab_picto": (418, 139), "tab_review": (512, 139),
    "tab_ai": (624, 139), "tab_view": (734, 139), "tab_users": (813, 139),
    "save": (827, 86), "export": (981, 86), "print": (1170, 86), "commands": (1213, 86),
    "panel_symbol": (1098, 192), "panel_collections": (1210, 192),
    "panel_studio": (1298, 192), "panel_ai": (1389, 192),
    "search": (1200, 374), "search_go": (1385, 374),
    "alt1": (1107, 432), "alt2": (1185, 432), "alt3": (1264, 432),
    "controls": (40, 806), "font_plus": (454, 723), "font_minus": (382, 723),
    "listen": (350, 677), "controls_close": (662, 677), "caps": (792, 758),
    "zoom_out": (1250, 792),
}

STUDENT = {"card1": (250, 380), "card2": (250, 500), "close": (68, 97)}

BOARD = {
    "lib_apri1": (334, 748), "cell": (412, 554),
    "caa_edit": (1241, 557), "caa_text": (288, 360),
    "caa_close": (1394, 96), "caa_apply": (1335, 835),
    "pict2": (302, 240), "alt2": (1224, 425), "alt3": (1224, 490),
    "block": (1145, 251), "search": (1217, 294), "search_go": (1371, 294),
}

BOARD_PREP = [
    ("click", HOME["creator"] [0], HOME["creator"][1]), ("wait", 3),
    ("click", *DASH["library"]), ("wait", 4),
    ("click", *BOARD["lib_apri1"]), ("wait", 7),
]


def cdp(js, timeout=60):
    env = dict(os.environ, CDP_FILTER="hud-overlay")
    return subprocess.run(
        ["node", os.path.join(ROOT, "cdp.js"), js],
        capture_output=True, text=True, timeout=timeout, cwd=PROJECT, env=env,
    ).stdout.strip()


def activate_terminal():
    subprocess.run(["osascript", "-e", 'tell application "Terminal" to activate'], check=False)
    time.sleep(1.2)


def activate_app():
    """Porta CAA Writer davanti e verifica che lo sia davvero."""
    for _ in range(4):
        subprocess.run(["open", "-a", APP], check=False)
        subprocess.run(["osascript", "-e", f'tell application "{APP}" to activate'], check=False)
        time.sleep(1.2)
        front = subprocess.run(
            ["osascript", "-e",
             'tell application "System Events" to get name of first process whose frontmost is true'],
            capture_output=True, text=True).stdout.strip()
        if front == "caa-writer":
            return
    print(f"attenzione: in primo piano c'è {front!r}")


def fix_window():
    script = ('tell application "System Events" to tell process "caa-writer"\n'
              'set position of window 1 to {0, 29}\n'
              'set size of window 1 to {1440, 803}\n'
              'end tell')
    subprocess.run(["osascript", "-e", script], check=False)
    time.sleep(1.2)


def pixel(x, y, name="state"):
    """Colore del pixel (x, y) dello schermo, letto da uno screenshot temporaneo."""
    shot = os.path.join(ROOT, "tmp", f"{name}.png")
    subprocess.run(["caffeinate", "-u", "-t", "1"], check=False)
    subprocess.run(["screencapture", "-x", shot], check=False)
    out = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", shot, "-vf", f"crop=1:1:{x}:{y}",
         "-f", "rawvideo", "-pix_fmt", "rgb24", "-"],
        capture_output=True).stdout
    return tuple(out[:3]) if len(out) >= 3 else (0, 0, 0)


def dismiss_recovery():
    """Chiude il banner giallo della bozza recuperata, che copre la pagina."""
    shot = os.path.join(ROOT, "tmp", "recovery.png")
    subprocess.run(["caffeinate", "-u", "-t", "1"], check=False)
    subprocess.run(["screencapture", "-x", shot], check=False)
    top, height = 80, 260
    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", shot, "-vf", f"crop=1440:{height}:0:{top}",
         "-f", "rawvideo", "-pix_fmt", "rgb24", "-"],
        capture_output=True).stdout
    rows = []
    for y in range(height):
        offset = y * 1440 * 3
        row_bytes = raw[offset:offset + 1440 * 3]
        yellow = sum(1 for i in range(0, len(row_bytes), 3)
                     if row_bytes[i] > 238 and row_bytes[i + 1] > 228 and 170 < row_bytes[i + 2] < 228)
        if yellow > 800:
            rows.append(top + y)
    if not rows:
        return False
    uia.click(628, (rows[0] + rows[-1]) // 2)
    time.sleep(1.8)
    return True


def app_state():
    r, g, b = pixel(30, 300)
    if r + g + b < 330:
        return "home"          # sfondo scuro della schermata iniziale
    if r + g + b > 690 and pixel(83, 181)[2] > 150:
        return "dashboard"     # scheda Italiano attiva, sfondo chiaro
    return "altro"


def ensure_dashboard():
    for _ in range(3):
        state = app_state()
        if state == "dashboard":
            return
        if state == "home":
            uia.click(*HOME["creator"])
        else:
            uia.click(1362, 91)      # pulsante Area Creator della vista Utilizzatore
        time.sleep(3.5)
    raise RuntimeError(f"Area Creator non raggiunta (stato: {app_state()})")


def restart_app():
    subprocess.run(["osascript", "-e", f'quit app "{APP}"'], check=False)
    time.sleep(3)
    subprocess.run(["open", "-a", APP], check=False)
    time.sleep(6)
    fix_window()


def pages():
    out = subprocess.run(["curl", "-s", "--max-time", "3", "http://127.0.0.1:9222/json/list"],
                         capture_output=True, text=True).stdout
    try:
        return json.loads(out) if out.strip() else []
    except json.JSONDecodeError:
        return []


def close_editor():
    for p in pages():
        if "windowType=editor" in p.get("url", ""):
            subprocess.run(["curl", "-s", f"http://127.0.0.1:9222/json/close/{p['id']}"],
                           capture_output=True, text=True)
            time.sleep(2)


def ensure_recordly(force=False):
    """Riavvia Recordly con la porta CDP. Serve anche a ricostruire l'elenco delle
    finestre: un elenco vecchio può associare il nome sbagliato alla finestra."""
    if not force and any("hud-overlay" in p.get("url", "") for p in pages()):
        return
    close_editor()
    subprocess.run(["osascript", "-e", 'quit app "Recordly"'], check=False)
    time.sleep(3)
    pids = subprocess.run(["pgrep", "-f", "^/Applications/Recordly.app/Contents/MacOS/Recordly"],
                          capture_output=True, text=True).stdout.split()
    for pid in pids:
        subprocess.run(["kill", pid], check=False)
    time.sleep(1)
    subprocess.run(["open", "-a", "Recordly", "--args", "--remote-debugging-port=9222"], check=False)
    for _ in range(20):
        time.sleep(1)
        if any("hud-overlay" in p.get("url", "") for p in pages()):
            time.sleep(1.5)
            return
    raise RuntimeError("la HUD di Recordly non è disponibile")


def ensure_source(name="CAA Writer"):
    # si riseleziona sempre: il titolo del pulsante non dice quale finestra sia
    # agganciata, e un'altra finestra può avere lo stesso nome.
    cdp('(async()=>{const s=ms=>new Promise(r=>setTimeout(r,ms));'
        'const b=[...document.querySelectorAll("button")];'
        'b.find(e=>e.title==="Screen").click(); await s(1800); return "ok"})()')
    # il titolo della finestra deve combaciare: un browser aperto su un documento
    # chiamato "CAA Writer" comparirebbe altrimenti come prima corrispondenza
    res = cdp('(async()=>{const s=ms=>new Promise(r=>setTimeout(r,ms));'
              'const items=[...document.querySelectorAll("button.source-selector-item")];'
              f'const rows=items.map(e=>({{e, first:(e.innerText||"").split("\\n")[0].trim()}}));'
              f'const exact=rows.filter(r=>r.first==={name!r}).map(r=>r.e);'
              'if(!exact.length) return "assente";'
              'if(exact.length>1) return "ambiguo:"+exact.length;'
              'exact[0].click(); await s(1200); return "ok"})()')
    if res != "ok":
        raise RuntimeError(f"sorgente {name!r} non trovata: {res}")


def hud_buttons():
    out = cdp('JSON.stringify([...document.querySelectorAll("button")].map(b=>b.title||""))')
    try:
        return json.loads(out)
    except json.JSONDecodeError:
        return []


def start_recording():
    cdp('(async()=>{const s=ms=>new Promise(r=>setTimeout(r,ms));'
        'const b=[...document.querySelectorAll("button")];'
        'b.find(e=>e.title==="Registra").click(); await s(600); return "ok"})()')
    for _ in range(40):
        if "Stop" in hud_buttons():
            return True
        time.sleep(0.5)
    raise RuntimeError("Recordly non è entrato in registrazione")


def stop_recording():
    cdp('(async()=>{const s=ms=>new Promise(r=>setTimeout(r,ms));'
        'const b=[...document.querySelectorAll("button")];'
        'const stop=b.find(e=>e.title==="Stop"); if(stop) stop.click(); await s(1500); return "ok"})()')


def newest_recording():
    files = glob.glob(os.path.expanduser(
        "~/Library/Application Support/Recordly/recordings/recording-*.mp4"))
    return max(files, key=os.path.getmtime) if files else None


def play(steps):
    for step in steps:
        action = step[0]
        if action == "click":
            uia.click(step[1], step[2])
        elif action == "move":
            uia.move(step[1], step[2])
        elif action == "drag":
            uia.drag(step[1], step[2], step[3], step[4])
        elif action == "scroll":
            uia.scroll(step[1], step[2], step[3])
        elif action == "dismiss":
            for _ in range(3):
                if not dismiss_recovery():
                    break
        elif action == "activate":
            activate_app()
        elif action == "type":
            uia.type_text(step[1])
        elif action == "key":
            uia.key(step[1], step[2] if len(step) > 2 else ())
        elif action == "wait":
            time.sleep(step[1])
        elif action == "shot":
            subprocess.run(["caffeinate", "-u", "-t", "1"], check=False)
            subprocess.run(["screencapture", "-x", os.path.join(ROOT, "tmp", f"take-{step[1]}.png")],
                           check=False)
        else:
            raise ValueError(f"azione sconosciuta: {action}")


def archive(chapter):
    src = newest_recording()
    if not src:
        raise RuntimeError("nessuna registrazione trovata")
    os.makedirs(TAKES, exist_ok=True)
    dst = os.path.join(TAKES, f"ch{chapter}.mp4")
    shutil.move(src, dst)
    probe = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", dst],
        capture_output=True, text=True).stdout.strip()
    print(f"presa {chapter}: {dst} ({float(probe):.1f}s)")
    return dst


def run(chapter, prepare_only=False, dry=False):
    spec = CHAPTERS[chapter]
    if dry:
        print(json.dumps(spec, indent=1, ensure_ascii=False))
        return
    ensure_recordly(force=True)
    activate_app()
    if spec.get("restart"):
        restart_app()
        activate_app()
    print(f"— preparazione capitolo {chapter}")
    if spec.get("creator"):
        ensure_dashboard()
    play(spec.get("prepare", []))
    if prepare_only:
        return
    print("— registrazione")
    ensure_source(spec.get("source", SCREEN))
    start_recording()
    time.sleep(0.4)
    if spec.get("terminal"):
        activate_terminal()
    else:
        activate_app()          # i tasti vanno all'app in primo piano, non a Recordly
    play(spec["take"])
    time.sleep(0.6)
    stop_recording()
    time.sleep(3)
    archive(chapter)
    close_editor()


# La cattura per finestra è immune ai cambi di primo piano; il nome della
# finestra deve però essere univoco nell'elenco di Recordly, altrimenti ci si
# ferma invece di registrare la finestra sbagliata.
SCREEN = "Screen 1 (Primary)"

CREATOR_ENTRY = [("wait", 0.5)]
FRESH_DOC = CREATOR_ENTRY + [
    ("click", *DASH["wp_card"]), ("wait", 5),
    ("dismiss",), ("wait", 1.0),
    ("click", 1397, 790), ("wait", 1.0),
    ("click", 1397, 790), ("wait", 1.5),
    ("click", *ED["page"]), ("wait", 1),
]
PHRASE = "Luca va a scuola con lo zaino e il quaderno."

CHAPTERS = {
    "01": {
        "restart": True,
        "prepare": [],
        "take": [
            ("wait", 2.0),
            ("move", HOME["luca"][0], HOME["luca"][1]), ("wait", 4.0),
            ("move", 900, 380), ("wait", 3.0),
            ("move", HOME["luca"][0], HOME["luca"][1]), ("wait", 4.0),
            ("move", *HOME["creator"]), ("wait", 4.0),
            ("move", 700, 640), ("wait", 3.5),
            ("move", 640, 300), ("wait", 3.5),
            ("move", *HOME["creator"]), ("wait", 5.0),
            ("move", 760, 680), ("wait", 3.5),
            ("move", 420, 430), ("wait", 4.0),
        ],
    },
    "02": {
        "restart": True,
        "creator": True,
        "prepare": CREATOR_ENTRY,
        "take": [
            ("wait", 1.5),
            ("move", *DASH["wp_card"]), ("wait", 2.0),
            ("move", *DASH["tables_card"]), ("wait", 2.0),
            ("move", *DASH["smart_card"]), ("wait", 2.5),
            ("click", *DASH["library"]), ("wait", 5.0),
            ("move", 700, 420), ("wait", 3.0),
            ("click", 68, 97), ("wait", 3.0),
            ("click", *DASH["users"]), ("wait", 5.0),
            ("move", 700, 380), ("wait", 3.0),
            ("click", 1181, 140), ("wait", 2.0),
            ("click", *DASH["settings"]), ("wait", 4.5),
            ("move", 700, 420), ("wait", 3.0),
        ],
    },
    "03": {
        "restart": True,
        "prepare": BOARD_PREP + [
            ("click", *BOARD["cell"]), ("wait", 3),
            ("click", *BOARD["caa_edit"]), ("wait", 4),
            ("click", *BOARD["caa_text"]), ("wait", 1),
        ],
        "take": [
            ("wait", 1.0),
            ("type", PHRASE), ("wait", 7.0),
            ("move", 300, 240), ("wait", 3.0),
            ("click", *BOARD["pict2"]), ("wait", 4.0),
            ("move", 1224, 300), ("wait", 3.0),
            ("move", 1224, 425), ("wait", 3.0),
            ("move", 600, 400), ("wait", 3.0),
            ("move", 1224, 490), ("wait", 3.0),
            ("move", 300, 250), ("wait", 4.0),
            ("move", 900, 600), ("wait", 4.0),
        ],
    },
    "04": {
        "restart": True,
        "prepare": BOARD_PREP + [
            ("click", *BOARD["cell"]), ("wait", 3),
            ("click", *BOARD["caa_edit"]), ("wait", 4),
            ("click", *BOARD["caa_text"]), ("wait", 1),
            ("type", PHRASE), ("wait", 6),
        ],
        "take": [
            ("wait", 1.0),
            ("move", 300, 240), ("wait", 3.0),
            ("click", *BOARD["pict2"]), ("wait", 4.0),
            ("move", 1224, 300), ("wait", 2.5),
            ("move", *BOARD["alt2"]), ("wait", 2.5),
            ("click", *BOARD["alt2"]), ("wait", 4.0),
            ("move", 300, 240), ("wait", 3.0),
            ("move", *BOARD["block"]), ("wait", 3.0),
            ("move", *BOARD["search"]), ("wait", 3.0),
            ("move", *BOARD["search_go"]), ("wait", 2.5),
            ("move", 1224, 450), ("wait", 3.0),
            ("move", 700, 300), ("wait", 3.5),
        ],
    },
    "05": {
        "restart": True,
        "prepare": BOARD_PREP + [
            ("click", *BOARD["cell"]), ("wait", 3),
            ("click", *BOARD["caa_edit"]), ("wait", 4),
            ("click", *BOARD["caa_text"]), ("wait", 1),
            ("type", "Il mio bassotto si chiama Pippo."), ("wait", 7),
        ],
        "take": [
            ("wait", 1.0),
            ("move", 200, 240), ("wait", 3.0),
            ("click", 250, 240), ("wait", 4.0),
            ("move", *BOARD["search"]), ("wait", 2.5),
            ("click", *BOARD["search"]), ("wait", 1.0),
            ("type", "bassotto"), ("wait", 2.0),
            ("click", *BOARD["search_go"]), ("wait", 4.5),
            ("move", 1224, 430), ("wait", 3.0),
            ("move", 1224, 500), ("wait", 3.0),
            ("move", 300, 240), ("wait", 3.5),
            ("move", 700, 350), ("wait", 4.0),
        ],
    },
    "06": {
        "restart": True,
        "creator": True,
        "prepare": CREATOR_ENTRY + [("click", *DASH["smart_card"]), ("wait", 5)],
        "take": [
            ("wait", 1.5),
            ("move", 1150, 300), ("wait", 3.0),
            ("move", 1240, 480), ("wait", 2.5),
            ("click", 1240, 480), ("wait", 2.5),
            ("move", 1300, 600), ("wait", 2.5),
            ("click", 1254, 560), ("wait", 4.0),
            ("move", 700, 350), ("wait", 3.0),
            ("click", 1389, 192), ("wait", 4.0),
            ("move", 1200, 500), ("wait", 3.5),
        ],
    },
    "07": {
        "restart": True,
        "prepare": [("click", *HOME["luca"]), ("wait", 3.5),
                    ("click", *STUDENT["card1"]), ("wait", 5)],
        "take": [
            ("wait", 1.5),
            ("move", 700, 200), ("wait", 3.0),
            ("click", 420, 300), ("wait", 3.0),
            ("move", 700, 300), ("wait", 2.5),
            ("click", 620, 300), ("wait", 3.0),
            ("move", 500, 480), ("wait", 3.0),
            ("click", 500, 480), ("wait", 3.0),
            ("move", 700, 600), ("wait", 3.5),
        ],
    },
    "08": {
        "restart": True,
        "prepare": BOARD_PREP,
        "take": [
            ("wait", 1.5),
            ("click", *ED["controls"]), ("wait", 3.0),
            ("move", *ED["font_plus"]), ("wait", 2.0),
            ("click", *ED["font_plus"]), ("wait", 3.0),
            ("click", *ED["font_plus"]), ("wait", 3.0),
            ("move", *ED["listen"]), ("wait", 2.5),
            ("click", *ED["listen"]), ("wait", 4.5),
            ("move", 700, 500), ("wait", 3.0),
            ("move", *ED["controls"]), ("wait", 2.5),
            ("click", *ED["controls"]), ("wait", 3.0),
            ("move", 700, 300), ("wait", 4.0),
        ],
    },
    "09": {
        "restart": True,
        "prepare": BOARD_PREP,
        "take": [
            ("wait", 1.5),
            ("click", *ED["save"]), ("wait", 3.5),
            ("move", 900, 500), ("wait", 3.0),
            ("key", 53), ("wait", 2.5),
            ("click", *ED["commands"]), ("wait", 3.0),
            ("move", 1150, 420), ("wait", 4.0),
            ("key", 53), ("wait", 2.0),
            ("click", *ED["print"]), ("wait", 5.0),
            ("key", 53), ("wait", 3.0),
            ("click", *ED["panel_collections"]), ("wait", 3.5),
            ("move", 1150, 500), ("wait", 3.0),
            ("move", 700, 400), ("wait", 3.0),
        ],
    },
    "10": {
        "restart": False,
        "terminal": True,
        "prepare": [
            ("wait", 0.5),
        ],
        "take": [
            ("wait", 1.0),
            ("type", "./src-tauri/target/release/caa-cli --version"), ("key", 36), ("wait", 3.5),
            ("type", "./src-tauri/target/release/caa-cli info"), ("key", 36), ("wait", 5.0),
            ("move", 600, 300), ("wait", 3.0),
            ("type", "./src-tauri/target/release/caa-cli document inspect --help"), ("key", 36), ("wait", 5.0),
            ("move", 700, 400), ("wait", 4.0),
            ("type", "./src-tauri/target/release/caa-cli symbolize '{\"text\":\"Il gatto dorme sul divano.\"}'"), ("key", 36), ("wait", 5.0),
        ],
    },
}


def main(argv):
    if argv and argv[0] == "--state":
        activate_app()
        print(app_state())
        return 0
    if not argv or argv[0] not in CHAPTERS:
        print("capitoli disponibili:", ", ".join(sorted(CHAPTERS)))
        return 2
    chapter = argv[0]
    run(chapter, prepare_only="--prepare" in argv, dry="--dry" in argv)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
