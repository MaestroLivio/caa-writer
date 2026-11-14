#!/usr/bin/env python3
"""Controlli di coerenza della Agent Skill.

Non inventa risultati: se il binario non è disponibile, lo dichiara e verifica
solo i file statici. Con --binary controlla che il catalogo generato contenga
davvero tutte le operazioni del registro reale.

Uso:
    python3 agent-skill/tests/check_skill.py
    python3 agent-skill/tests/check_skill.py --binary src-tauri/target/debug/caa-cli \
        --resources resources
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SKILL = ROOT / "agent-skill" / "SKILL.md"
CAPABILITIES = ROOT / "agent-skill" / "references" / "capabilities.md"
REFERENCES = [
    ROOT / "agent-skill" / "references" / "workflows.md",
    ROOT / "agent-skill" / "references" / "security.md",
]
EXAMPLES = [
    ROOT / "agent-skill" / "examples" / "cli.md",
    ROOT / "agent-skill" / "examples" / "mcp.jsonl",
]

errors: list[str] = []
notes: list[str] = []


def check_frontmatter() -> None:
    text = SKILL.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        errors.append("SKILL.md: manca il frontmatter YAML")
        return
    end = text.find("\n---", 4)
    if end < 0:
        errors.append("SKILL.md: frontmatter non chiuso")
        return
    front = text[4:end]
    for key in ("name:", "description:"):
        if key not in front:
            errors.append(f"SKILL.md: manca {key} nel frontmatter")


def check_files() -> None:
    for path in [SKILL, CAPABILITIES, *REFERENCES, *EXAMPLES]:
        if not path.exists():
            errors.append(f"file mancante: {path.relative_to(ROOT)}")
        elif path.stat().st_size == 0:
            errors.append(f"file vuoto: {path.relative_to(ROOT)}")
    text = SKILL.read_text(encoding="utf-8")
    for reference in REFERENCES + [CAPABILITIES]:
        relative = reference.relative_to(ROOT / "agent-skill").as_posix()
        if relative not in text:
            errors.append(f"SKILL.md non collega {relative}")


def generated_operations(binary: Path, resources: Path | None) -> list[str]:
    with tempfile.TemporaryDirectory(prefix="caa-skill-check-") as temp:
        command = [str(binary), "--data-dir", temp, "capabilities"]
        if resources is not None:
            command += ["--resources", str(resources)]
        completed = subprocess.run(command, capture_output=True, text=True, timeout=180)
        if completed.returncode != 0:
            raise RuntimeError(
                f"caa-cli capabilities è fallito ({completed.returncode}): {completed.stderr.strip()}"
            )
        value = json.loads(completed.stdout)
        return [item["name"] for item in value["operations"]]


def check_capabilities(binary: Path, resources: Path | None) -> None:
    text = CAPABILITIES.read_text(encoding="utf-8")
    documented = set(re.findall(r"^## `([^`]+)`", text, flags=re.MULTILINE))
    if not documented:
        errors.append("capabilities.md: nessuna operazione trovata")
        return
    operations = generated_operations(binary, resources)
    missing = sorted(set(operations) - documented)
    stale = sorted(documented - set(operations))
    if missing:
        errors.append("capabilities.md non documenta: " + ", ".join(missing))
    if stale:
        errors.append("capabilities.md documenta operazioni inesistenti: " + ", ".join(stale))
    notes.append(f"catalogo: {len(operations)} operazioni, {len(documented)} documentate")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=None)
    parser.add_argument("--resources", type=Path, default=None)
    args = parser.parse_args()

    check_frontmatter()
    check_files()

    binary = args.binary
    if binary is None:
        candidate = ROOT / "src-tauri" / "target" / "debug" / "caa-cli"
        binary = candidate if candidate.exists() else None
    if binary is None or not binary.exists():
        notes.append(
            "caa-cli non trovato: salto il confronto con il registro reale "
            "(compila con `cargo build --bin caa-cli`)"
        )
    else:
        check_capabilities(binary, args.resources)

    for note in notes:
        print(f"nota: {note}")
    if errors:
        for error in errors:
            print(f"errore: {error}", file=sys.stderr)
        return 1
    print("Agent Skill coerente con i file e con il catalogo.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
