#!/usr/bin/env python3
"""Driver minimale di mouse e tastiera per macOS.

Mouse: CGEvent via ctypes (nessuna dipendenza), con CGPoint passato per valore.
Tastiera: System Events, che gestisce correttamente i caratteri accentati.

    python3 tutorial/uia.py move 400 300
    python3 tutorial/uia.py click 400 300 [--double] [--right]
    python3 tutorial/uia.py drag 400 300 700 500
    python3 tutorial/uia.py type "testo da scrivere"
    python3 tutorial/uia.py key 36            # codici virtuali HID (36 = invio)
    python3 tutorial/uia.py scroll 400 300 -6
    python3 tutorial/uia.py where             # posizione corrente del puntatore
"""
import ctypes
import ctypes.util
import subprocess
import sys
import time

CF = ctypes.CDLL(ctypes.util.find_library("CoreFoundation"))
CG = ctypes.CDLL("/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices")


class CGPoint(ctypes.Structure):
    _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double)]


CG.CGEventCreate.restype = ctypes.c_void_p
CG.CGEventCreate.argtypes = [ctypes.c_void_p]
CG.CGEventCreateMouseEvent.restype = ctypes.c_void_p
CG.CGEventCreateMouseEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint32, CGPoint, ctypes.c_uint32]
CG.CGEventCreateScrollWheelEvent.restype = ctypes.c_void_p
CG.CGEventCreateScrollWheelEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_int32]
CG.CGEventPost.argtypes = [ctypes.c_uint32, ctypes.c_void_p]
CG.CGEventSetIntegerValueField.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_int64]
CG.CGEventGetLocation.restype = CGPoint
CG.CGEventGetLocation.argtypes = [ctypes.c_void_p]
CG.CGWarpMouseCursorPosition.argtypes = [CGPoint]
CF.CFRelease.argtypes = [ctypes.c_void_p]

LEFT_DOWN, LEFT_UP, RIGHT_DOWN, RIGHT_UP = 1, 2, 3, 4
MOVE, DRAG = 5, 6
HID = 0


def post(event):
    CG.CGEventPost(HID, event)
    CF.CFRelease(event)


def move(x, y):
    post(CG.CGEventCreateMouseEvent(None, MOVE, CGPoint(float(x), float(y)), 0))


def click(x, y, double=False, right=False):
    move(x, y)
    time.sleep(0.1)
    down, up = (RIGHT_DOWN, RIGHT_UP) if right else (LEFT_DOWN, LEFT_UP)
    for _ in range(2 if double else 1):
        d = CG.CGEventCreateMouseEvent(None, down, CGPoint(float(x), float(y)), 0)
        CG.CGEventSetIntegerValueField(d, 1, 1)  # kCGMouseEventClickState
        post(d)
        time.sleep(0.05)
        u = CG.CGEventCreateMouseEvent(None, up, CGPoint(float(x), float(y)), 0)
        CG.CGEventSetIntegerValueField(u, 1, 1)
        post(u)
        time.sleep(0.1)


def drag(x1, y1, x2, y2, steps=24):
    move(x1, y1)
    time.sleep(0.1)
    post(CG.CGEventCreateMouseEvent(None, LEFT_DOWN, CGPoint(float(x1), float(y1)), 0))
    for i in range(1, steps + 1):
        x = x1 + (x2 - x1) * i / steps
        y = y1 + (y2 - y1) * i / steps
        post(CG.CGEventCreateMouseEvent(None, DRAG, CGPoint(float(x), float(y)), 0))
        time.sleep(0.02)
    post(CG.CGEventCreateMouseEvent(None, LEFT_UP, CGPoint(float(x2), float(y2)), 0))


def scroll(x, y, ticks):
    move(x, y)
    time.sleep(0.05)
    post(CG.CGEventCreateScrollWheelEvent(None, HID, 1, int(ticks)))


def where():
    ev = CG.CGEventCreate(None)
    p = CG.CGEventGetLocation(ev)
    CF.CFRelease(ev)
    print(f"{p.x:.1f} {p.y:.1f}")


def type_text(text):
    escaped = text.replace("\\", "\\\\").replace('"', '\\"')
    subprocess.run(
        ["osascript", "-e", f'tell application "System Events" to keystroke "{escaped}"'],
        check=True,
    )


MODIFIERS = {"cmd": "command", "shift": "shift", "alt": "option", "ctrl": "control"}


def key(code, mods=()):
    mods = [MODIFIERS.get(m, m) for m in mods]
    lines = ['tell application "System Events"']
    lines += [f"key down {m}" for m in mods]
    lines.append(f"key code {code}")
    lines += [f"key up {m}" for m in mods]
    lines.append("end tell")
    subprocess.run(["osascript", "-e", "\n".join(lines)], check=True)


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    cmd, args = argv[0], argv[1:]
    if cmd == "move":
        move(int(args[0]), int(args[1]))
    elif cmd == "click":
        click(int(args[0]), int(args[1]), "--double" in args, "--right" in args)
    elif cmd == "drag":
        drag(int(args[0]), int(args[1]), int(args[2]), int(args[3]))
    elif cmd == "type":
        type_text(args[0])
    elif cmd == "key":
        key(int(args[0]), [a for a in args[1:]])
    elif cmd == "scroll":
        scroll(int(args[0]), int(args[1]), int(args[2]))
    elif cmd == "where":
        where()
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
