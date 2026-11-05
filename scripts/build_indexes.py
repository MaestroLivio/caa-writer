#!/usr/bin/env python3
"""Verifica e ricostruzione degli indici FTS5 del database di CAA Writer.

Il database è creato e migrato dall'applicazione Rust. Questo script serve
all'amministratore del repertorio: controlla l'integrità, ricostruisce gli
indici di ricerca e stampa un riepilogo. Non modifica i dati dei simboli.

Uso:
  python3 scripts/build_indexes.py --db "$HOME/Library/Application Support/org.caawriter.desktop/caa-writer.db"
  python3 scripts/build_indexes.py --db ... --rebuild
"""

from __future__ import annotations

import argparse
import sqlite3
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Indici FTS5 del database CAA Writer")
    parser.add_argument("--db", required=True, help="percorso del database SQLite")
    parser.add_argument("--rebuild", action="store_true", help="ricostruisce gli indici FTS")
    args = parser.parse_args()

    path = Path(args.db).expanduser()
    if not path.exists():
        print(f"Database non trovato: {path}", file=sys.stderr)
        return 1

    connection = sqlite3.connect(path)
    connection.row_factory = sqlite3.Row
    try:
        tables = {
            row["name"]
            for row in connection.execute(
                "SELECT name FROM sqlite_master WHERE type IN ('table','view')"
            )
        }
        if "symbols" not in tables:
            print("Schema non riconosciuto: manca la tabella symbols.", file=sys.stderr)
            return 1

        fts_tables = sorted(name for name in tables if name.endswith("_fts"))
        print(f"Database: {path}")
        print(f"Tabelle FTS: {', '.join(fts_tables) if fts_tables else 'nessuna'}")

        if args.rebuild:
            for name in fts_tables:
                connection.execute(f"INSERT INTO {name}({name}) VALUES('rebuild')")
                print(f"  indice ricostruito: {name}")
            connection.commit()

        connection.execute(f"INSERT INTO {fts_tables[0]}({fts_tables[0]}) VALUES('integrity-check')")
        print("Integrità FTS: ok")

        for table in ("symbols", "concepts", "lemmas", "symbol_packs", "search_statistics"):
            if table in tables:
                count = connection.execute(f"SELECT COUNT(*) AS n FROM {table}").fetchone()["n"]
                print(f"  {table}: {count} righe")

        if "symbols" in tables and "hidden" in {
            row["name"] for row in connection.execute("PRAGMA table_info(symbols)")
        }:
            hidden = connection.execute("SELECT COUNT(*) AS n FROM symbols WHERE hidden = 1").fetchone()["n"]
            print(f"  simboli nascosti: {hidden}")
    finally:
        connection.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
