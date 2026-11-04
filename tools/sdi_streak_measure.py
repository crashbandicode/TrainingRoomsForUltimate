"""Measure the SDI chip still's streak length against the first render and the captured spark (N10).

The chip the menu draws stretches the spark's white streaks x2.0 along the SDI vector (SPEC-core-07
section 4.6, "Streaks (N10)"; DECISIONS.md N10). The host render of the stock 13.0.5 effect file stays
outside the repo (di-sdi-extract); the repo keeps byte copies of the two Up layers under
`docs/menu-mockup/captures/renders/` (docs/menu-mockup/index.md, "DI and SDI chip renders", pinned by
sha256). This tool measures those byte copies the way the recorded review measured them
(index.md "SDI burst, longer"): the solid burst is the largest connected run of pixels with
alpha >= 230 (crown and legs; the thin `spark1` line is a separate component and is excluded), and its
bounding box is the measurement.

Checked (recorded numbers, index.md):
- the first render's burst is 58 x 86 px (height / width 1.48) - the method still sees it;
- the drawn still (x2.0) is longer than the first render's and twice as long along the vector;
- the drawn still's tip reach, calibrated on the first render's recorded 0.19 of the chip, is within
  a bound of the captured spark's (Up tip 0.36; the drawn 0.39; bound 0.05);
- the burst keeps its width (the remaining recorded difference is weight, not length);
- the DI render is byte-identical to its pinned sha256 (N10 changes the SDI chips only).

The captured spark itself is evidence outside the repo (c1162/captures-ally-sdi); its numbers are the
recorded measurements cited above, not re-measured here.
usage: python3 -I tools/sdi_streak_measure.py [--repo DIR]
"""
import argparse
import hashlib
import pathlib
import struct
import sys
import zlib

RENDERS = "docs/menu-mockup/captures/renders"
FIRST = "sdi-up-4x-layer.png"
DRAWN = "sdi-up-4x-layer-stretch.png"
DI = "di-up-4x-layer.png"
PINNED = {
    DI: "b87f8d8122ed5c752f623a95f4832f34fea1f7242cf82851105524910c494ccb",
    FIRST: "9d665a9051a1f54030ed53529f2eba1c18bf1a3196f5fdffe75b1b54fa061254",
    DRAWN: "11d9e2d29004c918fe7dc266216140d7087b3993a0dc1e19fe48f6967dbb4d61",
}
ALPHA = 230
RECORDED_FIRST = (58, 86)  # width, height, index.md
RECORDED_FIRST_TIP = 0.19  # of the chip, index.md
RECORDED_DRAWN_TIP = 0.39  # of the chip, index.md (x2.0 drawn)
CAPTURED_UP_TIP = 0.36  # of the chip, index.md (the captured spark, sdiup frames +10..+15)
TIP_BOUND = 0.05


def fail(message):
    print(f"FAIL {message}")
    sys.exit(1)


def read_png(path):
    """(width, height, alpha channel) of an 8-bit non-interlaced RGB/RGBA PNG."""
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        fail(f"{path} is not a PNG")
    chunks, at = [], 8
    while at < len(data):
        (length, kind) = struct.unpack(">I4s", data[at:at + 8])
        chunks.append((kind, data[at + 8:at + 8 + length]))
        at += 12 + length
        if kind == b"IEND":
            break
    header = next((body for kind, body in chunks if kind == b"IHDR"), None)
    if header is None:
        fail(f"{path} has no IHDR")
    width, height, depth, color, _, _, interlace = struct.unpack(">IIBBBBB", header)
    if depth != 8 or interlace != 0 or color not in (2, 6):
        fail(f"{path}: unsupported PNG (depth={depth} color={color} interlace={interlace})")
    channels = 4 if color == 6 else 3
    raw = zlib.decompress(b"".join(body for kind, body in chunks if kind == b"IDAT"))
    stride = width * channels
    if len(raw) != (stride + 1) * height:
        fail(f"{path}: unexpected scanline count")
    pixels = bytearray(stride * height)
    prior = bytearray(stride)
    for y in range(height):
        line = raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)]
        filter = raw[y * (stride + 1)]
        for x in range(stride):
            left = pixels[y * stride + x - channels] if x >= channels else 0
            up = prior[x]
            up_left = prior[x - channels] if x >= channels else 0
            if filter == 0:
                value = line[x]
            elif filter == 1:
                value = (line[x] + left) & 0xFF
            elif filter == 2:
                value = (line[x] + up) & 0xFF
            elif filter == 3:
                value = (line[x] + (left + up) // 2) & 0xFF
            else:
                pa, pb, pc = abs(up - up_left), abs(left - up_left), abs(left + up - 2 * up_left)
                guess = left if pa <= pb and pa <= pc else (up if pb <= pc else up_left)
                value = (line[x] + guess) & 0xFF
            pixels[y * stride + x] = value
        prior = pixels[y * stride:(y + 1) * stride]
    alpha = pixels[channels - 1::channels] if channels == 4 else None
    if alpha is None:
        fail(f"{path}: expected the light layer's alpha (= max channel), found RGB")
    return width, height, alpha


def burst(width, height, alpha):
    """The solid burst's bounds: the union of every connected component of alpha >= ALPHA pixels that is
    wider than the thin `spark1` line (2 px in 13.0.5), dropping stray pixels. 8-connected, like the
    recorded review measurement (docs/menu-mockup/index.md, "SDI burst, longer")."""
    mask = [a >= ALPHA for a in alpha]
    seen = bytearray(len(mask))
    union = None
    for start in range(len(mask)):
        if not mask[start] or seen[start]:
            continue
        stack, count, bounds = [start], 0, [start % width, start // width, 0, 0]
        seen[start] = 1
        while stack:
            at = stack.pop()
            x, y = at % width, at // width
            count += 1
            bounds[0], bounds[1] = min(bounds[0], x), min(bounds[1], y)
            bounds[2], bounds[3] = max(bounds[2], x), max(bounds[3], y)
            for neighbour in (at - 1, at + 1, at - width, at + width, at - width - 1, at - width + 1, at + width - 1, at + width + 1):
                row, column = neighbour // width, neighbour % width
                if 0 <= neighbour < len(mask) and abs(row - y) <= 1 and mask[neighbour] and not seen[neighbour]:
                    seen[neighbour] = 1
                    stack.append(neighbour)
        if count >= 8 and bounds[2] - bounds[0] + 1 > 3:
            l, t, r, b = bounds
            union = (l, t, r, b) if union is None else (min(union[0], l), min(union[1], t), max(union[2], r), max(union[3], b))
    if union is None:
        fail("no burst found above the alpha threshold")
    return union


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=str(pathlib.Path(__file__).resolve().parent.parent))
    args = parser.parse_args()
    root = pathlib.Path(args.repo)
    for name, digest in PINNED.items():
        path = root / RENDERS / name
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != digest:
            fail(f"{name}: sha256 {actual} is not the pinned {digest} (docs/menu-mockup/index.md)")
    width, height, alpha = read_png(root / RENDERS / FIRST)
    (l, t, r, b) = burst(width, height, alpha)
    first_w, first_h, first_top = r - l + 1, b - t + 1, t
    if abs(first_w - RECORDED_FIRST[0]) > 1 or abs(first_h - RECORDED_FIRST[1]) > 1:
        fail(f"first render burst {first_w}x{first_h}, the recorded method saw {RECORDED_FIRST[0]}x{RECORDED_FIRST[1]}")
    (_, _, alpha) = read_png(root / RENDERS / DRAWN)
    (l, t, r, b) = burst(width, height, alpha)
    drawn_w, drawn_h = r - l + 1, b - t + 1
    if drawn_h <= first_h:
        fail(f"drawn streak {drawn_h} px is not above the first render's {first_h} px")
    if abs(drawn_h - 2 * first_h) > 2:
        fail(f"drawn streak {drawn_h} px is not twice the first render's {first_h} px (x2.0, N10)")
    if abs(drawn_w - first_w) > 2:
        fail(f"the burst kept its width: first {first_w} px, drawn {drawn_w} px")
    # The still's canvas is the chip (the review measures the render against the chip side), and the
    # effect origin sits on the canvas centre: the tip is the burst's furthest pixel up the vector.
    centre = height / 2.0
    first_tip = (centre - first_top) / height
    drawn_tip = (centre - t) / height
    if abs(first_tip - RECORDED_FIRST_TIP) > 0.01:
        fail(f"first render tip {first_tip:.3f} of the chip, recorded {RECORDED_FIRST_TIP}")
    if abs(drawn_tip - RECORDED_DRAWN_TIP) > 0.01:
        fail(f"drawn tip {drawn_tip:.3f} of the chip, recorded {RECORDED_DRAWN_TIP}")
    if abs(drawn_tip - CAPTURED_UP_TIP) > TIP_BOUND:
        fail(f"drawn tip {drawn_tip:.3f} of the chip is outside the captured spark's {CAPTURED_UP_TIP} +/- {TIP_BOUND}")
    print(f"PASS first burst {first_w}x{first_h} px (tip {first_tip:.3f}), drawn {drawn_w}x{drawn_h} px "
          f"(x{drawn_h / first_h:.2f}, tip {drawn_tip:.3f} of the chip; captured Up {CAPTURED_UP_TIP}, bound {TIP_BOUND}); "
          "DI render pinned unchanged")
    return 0


if __name__ == "__main__":
    sys.exit(main())