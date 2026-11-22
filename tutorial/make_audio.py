#!/usr/bin/env python3
"""Genera la narrazione di ogni capitolo con VoiceStudio (voce Italian Explainer, italiano)."""
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
script = json.load(open(os.path.join(ROOT, "script.json"), encoding="utf-8"))
voice = script["voice"]
audio_dir = os.path.join(ROOT, "audio")
os.makedirs(audio_dir, exist_ok=True)

for chapter in script["chapters"]:
    out = os.path.join(audio_dir, f"ch{chapter['id']}.wav")
    if os.path.exists(out) and os.path.getsize(out) > 50000:
        print(f"ch{chapter['id']}: gia presente, salto")
        continue
    payload = {
        "model": "tts-1",
        "voice": voice["profile_id"],
        "input": chapter["narration"],
        "response_format": "wav",
        "language": voice["language"],
    }
    tmp = os.path.join("/tmp", f"speech-{chapter['id']}.json")
    with open(tmp, "w", encoding="utf-8") as handle:
        json.dump(payload, handle, ensure_ascii=False)
    print(f"ch{chapter['id']}: sintesi in corso...", flush=True)
    result = subprocess.run(
        [
            "curl", "-sS", "--fail-with-body", "-m", "300",
            "http://localhost:3900/v1/audio/speech",
            "-H", "Content-Type: application/json",
            "--data-binary", f"@{tmp}",
            "--output", out,
        ],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"ERRORE {chapter['id']}: {result.stderr.strip()}", file=sys.stderr)
        continue
    probe = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "default=nw=1:nk=1", out],
        capture_output=True, text=True,
    )
    duration = float(probe.stdout.strip() or 0)
    size = os.path.getsize(out)
    print(f"ch{chapter['id']}: {duration:6.2f}s  {size:>8} byte  {out}")
