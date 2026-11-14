#!/usr/bin/env python3
"""Draw Eggs-ercise's target (rc.14): assets/egg-target/target.png, the picture of the layout pane `tr_egg`.

Our own design, computed here from circles; no picture is read, traced or sampled. A generic bullseye, 128 x 128 RGBA:
a solid red centre inside four rings, red and white alternating outwards (white, red, white, red), and an opaque dark
outline, so the target reads on Training's white stage and on dark stages. Outside the disc it is transparent; those
pixels carry the outline's colour so a filtered edge does not fringe. 4 x 4 samples per pixel give smooth ring edges.

The break's first frames (pane `tr_egg_cracked`, assets/egg-target/cracked.png): the same bullseye at 0.88 of the size,
cut into six wedges along jagged lines from the centre and each wedge pushed outward, so transparent cracks open
between the shards. Also computed here; nothing is read.

The placing cursor (pane `tr_egg_cursor`, assets/egg-target/cursor.png): a white crosshair (a ring, four ticks outside
it and four inside, a centre dot) with the same dark outline. Also computed here.

Usage: python3 -I tools/draw_egg_target.py [--check]
Without --check, writes the PNGs; with it, verifies the committed PNGs hold exactly these pixels.
"""
import argparse
import math
import struct
import sys
import zlib
from pathlib import Path

SIZE = 128
RED = (214, 40, 48)
WHITE = (248, 248, 248)
OUTLINE = (28, 26, 30)
# (outer radius as a fraction of the disc's, colour), from the outside in; the last is the solid centre.
BANDS = [(1.00, OUTLINE), (0.91, RED), (0.73, WHITE), (0.55, RED), (0.37, WHITE), (0.19, RED)]
SAMPLES = 4
ASSETS = Path(__file__).resolve().parents[1] / "assets/egg-target"
TARGET = ASSETS / "target.png"
CRACKED = ASSETS / "cracked.png"
CURSOR = ASSETS / "cursor.png"
# The cursor, in disc radii: the ring (inner, outer), the ticks' half width and their spans along an axis (inner, outer),
# the centre dot's radius and the outline's width around every shape.
RING, TICK, TICKS, DOT, EDGE = (0.50, 0.62), 0.05, ((0.16, 0.36), (0.72, 0.93)), 0.07, 0.05
# The cracked frame: the crack lines' angles (radians, uneven on purpose), their zigzag (radians either side, period in
# disc radii), the shards' outward push and the bullseye's size within the disc, so a pushed shard stays inside it.
CRACKS = [0.30, 1.25, 2.20, 3.35, 4.30, 5.30]
ZIG, ZIG_PERIOD = 0.10, 0.22
PUSH, SHRUNK = 0.11, 0.88


def colour_at(d):
    """The colour at `d` disc radii from the centre, None outside the disc."""
    found = None
    for outer, colour in BANDS:
        if d < outer:
            found = colour
    return found


def whole_at(px, py):
    """The target's colour at (`px`, `py`) disc radii from the centre."""
    return colour_at(math.hypot(px, py))


def crack(k, r):
    """Crack line `k`'s angle `r` disc radii out: its base angle zigzagging along it."""
    t = r / ZIG_PERIOD + 0.37 * k
    return CRACKS[k % len(CRACKS)] + 2 * math.pi * (k // len(CRACKS)) + ZIG * (4 * abs(t - math.floor(t) - 0.5) - 1)


def cracked_at(px, py):
    """The cracked frame's colour: the shard whose pushed wedge holds the point, sampled where it came from."""
    for k in range(len(CRACKS)):
        lo, hi = CRACKS[k], CRACKS[k + 1] if k + 1 < len(CRACKS) else CRACKS[0] + 2 * math.pi
        middle = (lo + hi) / 2
        qx, qy = px - PUSH * math.cos(middle), py - PUSH * math.sin(middle)
        r = math.hypot(qx, qy)
        if r >= SHRUNK:
            continue
        start = crack(k, r)
        if (math.atan2(qy, qx) - start) % (2 * math.pi) < crack(k + 1, r) - start:
            return colour_at(r / SHRUNK)
    return None


def in_cursor(px, py, grow):
    """Whether (`px`, `py`) is within `grow` disc radii of the crosshair's shapes."""
    r = math.hypot(px, py)
    if RING[0] - grow < r < RING[1] + grow or r < DOT + grow:
        return True
    across, along = sorted((abs(px), abs(py)))
    return across < TICK + grow and any(lo - grow < along < hi + grow for lo, hi in TICKS)


def cursor_at(px, py):
    """The cursor's colour: white on its shapes, the outline around them, None elsewhere."""
    if in_cursor(px, py, 0.0):
        return WHITE
    return OUTLINE if in_cursor(px, py, EDGE) else None


def pixels(at=whole_at):
    half = SIZE / 2
    out = bytearray()
    for y in range(SIZE):
        for x in range(SIZE):
            total, alpha = [0, 0, 0], 0
            for sy in range(SAMPLES):
                for sx in range(SAMPLES):
                    dx = x + (sx + 0.5) / SAMPLES - half
                    dy = y + (sy + 0.5) / SAMPLES - half
                    colour = at(dx / half, dy / half)
                    if colour is not None:
                        alpha += 1
                        total = [t + c for t, c in zip(total, colour)]
            rgb = [round(t / alpha) for t in total] if alpha else list(OUTLINE)
            out += bytes(rgb + [round(255 * alpha / SAMPLES ** 2)])
    return bytes(out)


def png(rgba):
    rows = b"".join(b"\0" + rgba[y * SIZE * 4:(y + 1) * SIZE * 4] for y in range(SIZE))

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    header = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")


def decoded(data):
    """The RGBA of a PNG this script wrote (8-bit RGBA, every row filter 0)."""
    at, idat, header = 8, b"", None
    while at < len(data):
        (length,), kind = struct.unpack(">I", data[at:at + 4]), data[at + 4:at + 8]
        body = data[at + 8:at + 8 + length]
        if kind == b"IHDR":
            header = body
        elif kind == b"IDAT":
            idat += body
        at += 12 + length
    assert header == struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0), "not a 128 x 128 RGBA PNG"
    rows = zlib.decompress(idat)
    stride = SIZE * 4 + 1
    assert all(rows[y * stride] == 0 for y in range(SIZE)), "a row filter other than 0"
    return b"".join(rows[y * stride + 1:(y + 1) * stride] for y in range(SIZE))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, at in ((TARGET, whole_at), (CRACKED, cracked_at), (CURSOR, cursor_at)):
        rgba = pixels(at)
        if args.check:
            if decoded(path.read_bytes()) != rgba:
                sys.exit(f"{path} does not hold the pixels this script draws")
            print(f"{path.name}: the drawn pixels")
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(png(rgba))
        print(f"wrote {path}")


if __name__ == "__main__":
    main()
