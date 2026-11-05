#!/usr/bin/env python3
"""Genera le icone di CAA Writer (PNG, ICO, ICNS) senza dipendenze esterne.

Il disegno è originale: pagina bianca su fondo blu, righe di testo e un
pittogramma stilizzato (persona). Viene rasterizzato con supersampling e
codificato a mano in PNG (zlib), poi incapsulato in ICO e ICNS.

Uso: python3 scripts/generate_icons.py
"""

from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "src-tauri" / "icons"

BLUE = (31, 91, 216)
BLUE_DARK = (20, 55, 138)
WHITE = (255, 255, 255)
INK = (27, 58, 107)
ACCENT = (245, 179, 1)


def _mix(a, b, t: float):
    return (
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    )


def _rounded_rect(px, py, cx, cy, hw, hh, r) -> bool:
    dx = abs(px - cx) - (hw - r)
    dy = abs(py - cy) - (hh - r)
    if dx <= 0 and dy <= 0:
        return True
    dx = max(dx, 0.0)
    dy = max(dy, 0.0)
    return dx * dx + dy * dy <= r * r


def _circle(px, py, cx, cy, r) -> bool:
    return (px - cx) ** 2 + (py - cy) ** 2 <= r * r


def shade(x: float, y: float):
    """Colore RGBA (0..255) nel punto, coordinate normalizzate 0..1."""
    # Sfondo: quadrato arrotondato con gradiente verticale.
    if not _rounded_rect(x, y, 0.5, 0.5, 0.5, 0.5, 0.22):
        return (0, 0, 0, 0)
    bg = _mix(BLUE, BLUE_DARK, min(max((y - 0.1) / 0.8, 0.0), 1.0))

    # Pagina
    page_hw, page_hh = 0.30, 0.36
    if _rounded_rect(x, y, 0.52, 0.50, page_hw, page_hh, 0.04):
        # Riga di testo
        if _rounded_rect(x, y, 0.56, 0.30, 0.16, 0.022, 0.02):
            return INK + (255,)
        if _rounded_rect(x, y, 0.52, 0.38, 0.20, 0.022, 0.02):
            return INK + (255,)
        # Pittogramma: persona stilizzata
        if _circle(x, y, 0.46, 0.51, 0.048):
            return INK + (255,)
        if _rounded_rect(x, y, 0.46, 0.60, 0.075, 0.045, 0.03):
            return INK + (255,)
        # Secondo pittogramma: sole/idea
        if _circle(x, y, 0.63, 0.55, 0.05):
            return ACCENT + (255,)
        if _rounded_rect(x, y, 0.63, 0.55, 0.075, 0.008, 0.004):
            return ACCENT + (255,)
        if _rounded_rect(x, y, 0.63, 0.55, 0.008, 0.075, 0.004):
            return ACCENT + (255,)
        # Riga in basso
        if _rounded_rect(x, y, 0.52, 0.73, 0.14, 0.018, 0.015):
            return INK + (210,)
        return WHITE + (255,)

    return bg + (255,)


def render(size: int, ss: int = 4):
    """Rasterizza a `size` px con supersampling ss x ss."""
    rows = []
    step = 1.0 / (size * ss)
    for py in range(size):
        row = bytearray()
        for px in range(size):
            r = g = b = a = 0.0
            for sy in range(ss):
                for sx in range(ss):
                    x = (px * ss + sx + 0.5) * step
                    y = (py * ss + sy + 0.5) * step
                    cr, cg, cb, ca = shade(x, y)
                    alpha = ca / 255.0
                    r += cr * alpha
                    g += cg * alpha
                    b += cb * alpha
                    a += alpha
            n = ss * ss
            if a <= 0.0001:
                row += bytes((0, 0, 0, 0))
            else:
                row += bytes(
                    (
                        int(min(255, r / a)),
                        int(min(255, g / a)),
                        int(min(255, b / a)),
                        int(min(255, (a / n) * 255)),
                    )
                )
        rows.append(bytes(row))
    return rows


def png_bytes(size: int, ss: int = 4) -> bytes:
    rows = render(size, ss)
    raw = b"".join(b"\x00" + row for row in rows)

    def chunk(tag: bytes, data: bytes) -> bytes:
        body = tag + data
        return (
            struct.pack(">I", len(data))
            + body
            + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
        )

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def write_png(path: Path, size: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(png_bytes(size))
    print(f"  {path.relative_to(ROOT)} ({size}x{size})")


def build_ico(path: Path, sizes=(16, 24, 32, 48, 64, 128, 256)) -> None:
    images = [(size, png_bytes(size)) for size in sizes]
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    entries = b""
    payload = b""
    for size, data in images:
        entries += struct.pack(
            "<BBBBHHII",
            0 if size >= 256 else size,
            0 if size >= 256 else size,
            0,
            0,
            1,
            32,
            len(data),
            offset,
        )
        offset += len(data)
        payload += data
    path.write_bytes(header + entries + payload)
    print(f"  {path.relative_to(ROOT)} (ICO)")


def build_icns(path: Path, entries) -> None:
    chunks = b""
    for tag, size in entries:
        data = png_bytes(size)
        chunks += tag + struct.pack(">I", len(data) + 8) + data
    path.write_bytes(b"icns" + struct.pack(">I", len(chunks) + 8) + chunks)
    print(f"  {path.relative_to(ROOT)} (ICNS)")


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    print("Generazione icone CAA Writer:")
    write_png(OUT / "32x32.png", 32)
    write_png(OUT / "128x128.png", 128)
    write_png(OUT / "128x128@2x.png", 256)
    write_png(OUT / "icon.png", 512)
    write_png(OUT / "icon-1024.png", 1024)
    for name, size in (
        ("StoreLogo.png", 50),
        ("Square30x30Logo.png", 30),
        ("Square44x44Logo.png", 44),
        ("Square71x71Logo.png", 71),
        ("Square89x89Logo.png", 89),
        ("Square107x107Logo.png", 107),
        ("Square142x142Logo.png", 142),
        ("Square150x150Logo.png", 150),
        ("Square284x284Logo.png", 284),
        ("Square310x310Logo.png", 310),
    ):
        write_png(OUT / name, size)
    build_ico(OUT / "icon.ico")
    build_icns(
        OUT / "icon.icns",
        [
            (b"ic07", 128),
            (b"ic08", 256),
            (b"ic09", 512),
            (b"ic10", 1024),
            (b"ic11", 32),
            (b"ic12", 64),
            (b"ic13", 256),
            (b"ic14", 512),
        ],
    )
    print("Fatto.")


if __name__ == "__main__":
    main()
