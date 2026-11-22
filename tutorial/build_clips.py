#!/usr/bin/env python3
"""Compone un clip 1920x1080 per ogni capitolo dalle riprese Recordly.

Per ogni capitolo:
  * la ripresa reale (`takes/chNN.mp4`) viene ritagliata in 16:9 e portata
    alla durata esatta della narrazione (rallentata se serve, oppure ferma
    sull'ultimo fotogramma quando lo scarto è grande);
  * sopra entrano le motion graphics: etichetta di capitolo, titolo,
    sottotitolo con dissolvenza, filo superiore e barra di avanzamento;
  * il clip resta muto: la voce viaggia su un WAV separato.

Le riprese sono di due tipi: finestra (2880x1686, include la barra del titolo)
o schermo intero (2880x1864, la finestra dell'app sta a y=58).
"""
import json
import os
import subprocess
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.abspath(__file__))
W, H, FPS = 1920, 1080, 30
BOLD = "/System/Library/Fonts/Supplemental/Arial Bold.ttf"
REG = "/System/Library/Fonts/Supplemental/Arial.ttf"
ACCENT = (134, 216, 198)
TEXT = (243, 248, 246)
MUTED = (170, 200, 194)
PANEL = (10, 32, 31, 205)

script = json.load(open(os.path.join(ROOT, "script.json"), encoding="utf-8"))
tmp_dir = os.path.join(ROOT, "tmp")
clips_dir = os.path.join(ROOT, "clips")
takes_dir = os.path.join(ROOT, "takes")
os.makedirs(tmp_dir, exist_ok=True)
os.makedirs(clips_dir, exist_ok=True)


def duration_of(path):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "default=nw=1:nk=1", path],
        capture_output=True, text=True,
    )
    return float(out.stdout.strip() or 0)


def size_of(path):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0",
         "-show_entries", "stream=width,height", "-of", "csv=p=0", path],
        capture_output=True, text=True,
    ).stdout.strip().split(",")
    return int(out[0]), int(out[1])


def wrap(draw, text, font, max_width):
    words, lines, line = text.split(), [], ""
    for word in words:
        candidate = f"{line} {word}".strip()
        if draw.textlength(candidate, font=font) <= max_width or not line:
            line = candidate
        else:
            lines.append(line)
            line = word
    if line:
        lines.append(line)
    return lines


def caption(chapter):
    """Etichetta con capitolo, titolo e sottotitolo, in basso a sinistra."""
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    f_label = ImageFont.truetype(BOLD, 22)
    f_title = ImageFont.truetype(BOLD, 42)
    f_sub = ImageFont.truetype(REG, 24)

    title_lines = wrap(draw, chapter["title"], f_title, 980)
    sub_lines = wrap(draw, chapter["subtitle"], f_sub, 980)
    height = 40 + len(title_lines) * 50 + len(sub_lines) * 32

    top = H - height - 58
    draw.rounded_rectangle([64, top, 1120, top + height], radius=18, fill=PANEL)
    draw.rectangle([64, top, 68, top + height], fill=ACCENT)

    y = top + 18
    draw.text((92, y), f"CAPITOLO {chapter['id']}", font=f_label, fill=ACCENT)
    y += 30
    for line in title_lines:
        draw.text((92, y), line, font=f_title, fill=TEXT)
        y += 50
    for line in sub_lines:
        draw.text((92, y), line, font=f_sub, fill=MUTED)
        y += 32
    path = os.path.join(tmp_dir, f"cap{chapter['id']}.png")
    img.save(path)
    return path


def time_remap(source_duration, target):
    """Filtri per far combaciare la ripresa alla durata della narrazione."""
    if source_duration <= 0:
        return f"trim=duration={target}"
    ratio = target / source_duration
    if ratio <= 1.0:
        return f"trim=duration={target}"
    if ratio <= 1.35:
        return f"setpts=PTS*{ratio:.6f},trim=duration={target}"
    stretched = source_duration * 1.35
    return (f"setpts=PTS*1.35,tpad=stop_mode=clone:stop_duration="
            f"{max(0.0, target - stretched):.3f},trim=duration={target}")


def build(chapter):
    cid = chapter["id"]
    take = os.path.join(takes_dir, f"ch{cid}.mp4")
    audio = os.path.join(ROOT, "audio", f"ch{cid}.wav")
    if not (os.path.exists(take) and os.path.exists(audio)):
        print(f"ch{cid}: manca la ripresa o la voce", file=sys.stderr)
        return None

    target = round(duration_of(audio) + 0.35, 2)
    source_duration = duration_of(take)
    width, height = size_of(take)
    crop_h = min(1620, height)
    top = max(0, (height - crop_h) // 2)        # centro della finestra o dello schermo
    cap = caption(chapter)

    graph = (
        f"[0:v]crop={width}:{crop_h}:0:{top},scale={W}:{H},fps={FPS},"
        f"{time_remap(source_duration, target)},setpts=PTS-STARTPTS[v0];"
        f"[1:v]format=rgba,fade=t=in:st=0.35:d=0.7:alpha=1,"
        f"fade=t=out:st={max(0.0, target - 5.0):.2f}:d=0.9:alpha=1[cap];"
        f"[v0][cap]overlay=0:0:shortest=0,"
        f"drawbox=x=0:y=0:w={W}:h=6:color=0x86D8C6@0.9:t=fill,"
        f"drawbox=x=0:y={H - 6}:w='{W}*t/{target}':h=6:color=0x86D8C6@0.85:t=fill[out]"
    )

    out = os.path.join(clips_dir, f"ch{cid}.mp4")
    cmd = ["ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
           "-i", take, "-loop", "1", "-t", str(target), "-i", cap,
           "-filter_complex", graph, "-map", "[out]", "-an",
           "-c:v", "libx264", "-preset", "medium", "-crf", "20",
           "-pix_fmt", "yuv420p", "-r", str(FPS), "-t", str(target),
           "-movflags", "+faststart", out]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"ERRORE ch{cid}: {result.stderr.strip()[:800]}", file=sys.stderr)
        return None
    print(f"ch{cid}: ripresa {source_duration:5.1f}s -> clip {target:5.1f}s "
          f"({os.path.getsize(out) / 1e6:4.1f} MB)", flush=True)
    return out


total = 0.0
for chapter in script["chapters"]:
    cid = chapter["id"]
    clip = os.path.join(clips_dir, f"ch{cid}.mp4")
    take = os.path.join(takes_dir, f"ch{cid}.mp4")
    if (os.path.exists(clip) and os.path.getsize(clip) > 300000
            and os.path.getmtime(clip) >= os.path.getmtime(take)):
        print(f"ch{cid}: clip gia aggiornato", flush=True)
        total += duration_of(clip)
        continue
    if build(chapter):
        total += duration_of(os.path.join(clips_dir, f"ch{chapter['id']}.mp4"))

print(f"durata complessiva: {total:.1f}s ({total / 60:.2f} min)")
