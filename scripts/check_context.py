#!/usr/bin/env python3
"""Verifica il file di memoria del progetto `Contesto.md`.

Controlli:
 1. il file esiste nella radice;
 2. contiene le 21 sezioni richieste (numerazione e titoli);
 3. è escluso da Git (`/Contesto.md` in .gitignore) e non è tracciato;
 4. `AGENTS.md` e `CLAUDE.md` rimandano a `Contesto.md` senza copiarlo;
 5. la sezione sullo stato dichiara attività completate e attività aperte.

Esce con codice 1 se un controllo fallisce.

Uso: python3 scripts/check_context.py
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONTEXT = ROOT / "Contesto.md"

EXPECTED_SECTIONS = [
    "Identità e finalità del progetto",
    "Requisiti funzionali",
    "Requisiti non funzionali",
    "Vincoli hardware",
    "Piattaforme supportate",
    "Architettura generale",
    "Tecnologie e dipendenze",
    "Struttura delle directory",
    "Modello dei dati",
    "Gestione dei repertori pittografici",
    "Motore semantico e LLM",
    "Generazione SVG e MCP",
    "Editor documentale",
    "Generatore di risorse CAA/ABA/TEACCH",
    "Decisioni architetturali e motivazioni",
    "Problemi risolti",
    "Problemi aperti",
    "Test e risultati verificati",
    "Stato effettivo dell'implementazione",
    "Prossime attività, ordinate per priorità",
    "Informazioni necessarie per riprendere lo sviluppo",
]

errors: list[str] = []
notes: list[str] = []


def check_presence() -> str:
    if not CONTEXT.exists():
        errors.append("Contesto.md non esiste nella radice del progetto.")
        return ""
    text = CONTEXT.read_text(encoding="utf-8")
    notes.append(f"Contesto.md presente ({len(text.splitlines())} righe).")
    return text


def check_sections(text: str) -> None:
    if not text:
        return
    found = 0
    for index, title in enumerate(EXPECTED_SECTIONS, start=1):
        pattern = re.compile(rf"^#{{1,3}}\s*{index}[\.\)]?\s+.*{re.escape(title)}", re.IGNORECASE | re.MULTILINE)
        if pattern.search(text):
            found += 1
        else:
            errors.append(f"Sezione {index} mancante o con titolo diverso: «{title}».")
    notes.append(f"Sezioni trovate: {found}/{len(EXPECTED_SECTIONS)}.")


def check_git() -> None:
    gitignore = ROOT / ".gitignore"
    if not gitignore.exists():
        errors.append(".gitignore assente: impossibile escludere Contesto.md.")
        return
    entries = [line.strip() for line in gitignore.read_text(encoding="utf-8").splitlines()]
    if "/Contesto.md" not in entries and "Contesto.md" not in entries:
        errors.append(".gitignore non contiene /Contesto.md.")

    if not (ROOT / ".git").exists():
        notes.append("Repository Git non inizializzato: controllo del tracciamento saltato.")
        return
    try:
        tracked = subprocess.run(
            ["git", "ls-files", "--error-unmatch", "Contesto.md"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
    except OSError:
        notes.append("git non disponibile: controllo del tracciamento saltato.")
        return
    if tracked.returncode == 0:
        errors.append("Contesto.md risulta tracciato da Git: rimuoverlo con `git rm --cached Contesto.md`.")
    else:
        notes.append("Contesto.md non è tracciato da Git.")


def check_agent_files() -> None:
    for name in ("AGENTS.md", "CLAUDE.md"):
        path = ROOT / name
        if not path.exists():
            errors.append(f"{name} assente: gli agenti non saprebbero di Contesto.md.")
            continue
        text = path.read_text(encoding="utf-8")
        if "Contesto.md" not in text:
            errors.append(f"{name} non cita Contesto.md.")
        if len(text) > 8000:
            errors.append(f"{name} sembra contenere una copia estesa: deve rimandare a Contesto.md.")


def check_status(text: str) -> None:
    if not text:
        return
    status = re.search(
        r"^#{1,3}\s*19[\.\)]?\s+.*Stato effettivo.*$", text, re.IGNORECASE | re.MULTILINE
    )
    if not status:
        return
    tail = text[status.end() :]
    next_heading = re.search(r"^#{1,3}\s", tail, re.MULTILINE)
    block = tail[: next_heading.start()] if next_heading else tail
    lowered = block.lower()
    if "complet" not in lowered:
        errors.append("La sezione «Stato effettivo» non elenca attività completate.")
    if "aperte" not in lowered and "da fare" not in lowered:
        errors.append("La sezione «Stato effettivo» non elenca attività aperte.")


def main() -> int:
    text = check_presence()
    check_sections(text)
    check_git()
    check_agent_files()
    check_status(text)

    for note in notes:
        print(f"  ok  {note}")
    for error in errors:
        print(f"  KO  {error}")
    if errors:
        print(f"\n{len(errors)} controllo/i fallito/i.")
        return 1
    print("\nContesto.md conforme.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
