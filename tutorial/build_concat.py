#!/usr/bin/env python3
"""Monta il tutorial dentro Concat.

Struttura richiesta:
  * il video completo, suddiviso in singole clip, su una traccia video;
  * la narrazione di ogni capitolo come clip separate su una traccia audio.

Gira in un'unica sessione MCP perche' Concat tiene il progetto aperto solo nel
processo che lo apre.
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from concat_mcp import Concat  # noqa: E402

ROOT = os.path.dirname(os.path.abspath(__file__))
PROJECT = "/Users/livioluppino/Movies/CAA-Writer-Tutorial-Docenti"
script = json.load(open(os.path.join(ROOT, "script.json"), encoding="utf-8"))

client = Concat()
try:
    try:
        state = client.call("project_create", {
            "location": os.path.dirname(PROJECT),
            "name": os.path.basename(PROJECT),
        })
    except RuntimeError:
        state = client.call("project_open", {"path": PROJECT})
    tracks = state["timeline"]["tracks"]
    video_track = tracks[0]["id"]
    audio_track = tracks[1]["id"]
    print(f"progetto aperto: {state['project']['name']} · tracce {[t['id'] for t in tracks]}")
    print(f"video -> {video_track}, audio -> {audio_track}")

    cursor = 0.0
    for chapter in script["chapters"]:
        cid = chapter["id"]
        clip = os.path.join(ROOT, "clips", f"ch{cid}.mp4")
        audio = os.path.join(ROOT, "audio", f"ch{cid}.wav")
        for path in (clip, audio):
            if not os.path.exists(path):
                raise SystemExit(f"manca {path}")

        video_media = client.call("media_import", {"path": PROJECT, "file": clip})
        audio_media = client.call("media_import", {"path": PROJECT, "file": audio})
        video_id = video_media["createdId"]
        audio_id = audio_media["createdId"]
        seconds = next(
            float(item["duration"]) for item in video_media["media"]
            if item["id"] == video_id
        )

        client.call("clip_add", {
            "path": PROJECT, "mediaId": video_id,
            "trackId": video_track, "start": round(cursor, 3),
        })
        client.call("clip_add", {
            "path": PROJECT, "mediaId": audio_id,
            "trackId": audio_track, "start": round(cursor, 3),
        })

        print(f"ch{cid}: video e voce a {cursor:6.2f}s (clip {seconds:.2f}s)")
        cursor += seconds

    timeline = client.call("timeline_read", {"path": PROJECT})
    print(f"timeline: {len(timeline['timeline']['clips'])} clip, "
          f"durata {timeline['timeline']['duration']:.2f}s")
    client.call("project_save", {"path": PROJECT})
    print("progetto salvato")

    job = client.call("export_start", {
        "path": PROJECT,
        "output": "/Users/livioluppino/Movies/CAA-Writer-Tutorial-Docenti.mp4",
        "width": 1920, "height": 1080,
    })
    job_id = job.get("job") or job.get("id")
    print(f"export avviato: {job_id}")
    result = client.call("export_wait", {"job": job_id, "timeoutSeconds": 900})
    print(json.dumps(result, ensure_ascii=False)[:600])
finally:
    client.close()
