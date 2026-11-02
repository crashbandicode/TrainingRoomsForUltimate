#!/usr/bin/python3 -I
"""Crop the fork's real grid-picker captures into per-cell images for the menu mockup (proposal v4),
copy the fork's 38 item icons byte for byte (proposal v5), crop the game's DI streak from a real
capture for the DI direction chips (proposal v8), and crop the game's SDI spark from a real capture for
the SDI direction chips, plus one captured In spark as a visual check (proposal v9); v10 re-renders the SDI Up
still with a taller burst through tools/render_sdi_stretch.py (venv Python, W9 renderer imported read-only).

Review-mockup material only, never shipped artwork: the crops show game assets (stock fighter icons,
stage thumbnails) exactly as the fork's own grid pickers draw them in Training; the item icons are the
fork's own `scripts/item-icons/assets/icon-NN.png` files, unchanged; the DI streak and the SDI spark are
the game's own DI / SDI visuals (`sys_vector`, `sys_hitstop_spark`) in single frames of Eden captures.
The clean room reads those visuals from the player's game files at run time (SPEC-core-07 §4.6) and
never ships a picture of them. Every source
capture, the item manifest and every item icon are pinned by sha256; every crop box and copy is
written to docs/menu-mockup/captures/crops.json, which the renderer copies into
docs/menu-mockup/index.md.

Usage: /usr/bin/python3 -I tools/crop_menu_captures.py [ARTIFACTS_REFACTOR_ROOT]
The default root is the fork campaign's `parked-reuse175/refactor` directory on the review host.
"""
import hashlib
import importlib.util
import json
import math
import subprocess
import sys
from pathlib import Path
from PIL import Image

sys.dont_write_bytecode = True  # importing the renderer must not leave tools/__pycache__ behind

REPO = Path(__file__).resolve().parent.parent
OUT = REPO / "docs" / "menu-mockup" / "captures"
ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else
            "/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor")

# The four source captures, pinned. Two are Eden window grabs (1920x1080, emulated frame at 361,181,
# 1198x674); two are the fork's own docs/feature-walkthroughs images (1280x720 game frame).
SOURCES = {
    "change-player": ("gs-panic-pool14/evidence-m5f/surface/captures/072638-gsp9-p1grid.png",
                      "c0a2e5b48e3a80ba1a6244b1c617fbebfa385935e9a2baac1243242be5e878fb", "eden"),
    "change-cpu": ("gs-panic-pool14/evidence-m5h/surface/captures/121311-s40-cpugrid.png",
                   "50869bfcfa1f05e502ab7648a1049d9b9513149ebea88c446862b9b7bd39df53", "eden"),
    "kirby-grid": ("stages/m5k/stage/docs/feature-walkthroughs/kirby-grid.png",
                   "dda1ba4b0112ba92eb4106a0a51beac4a9efd0aa771428b5852fc8f3d363cba8", "frame"),
    "stage-rotation": ("stages/m5k/stage/docs/feature-walkthroughs/stage-rotation.png",
                       "84dbcd0e8da2e657c26251170843ff54bdb53e5f7b369e4977d1bdfa8092d412", "frame"),
}
# Cells that carry a cursor, an order badge or the original-CPU tint in the main source come from a
# capture where the same cell is clean.
PORTRAIT_SOURCE = {"Fox": "change-cpu", "Sora": "change-cpu", "ALL": "kirby-grid"}
# The fork's item-grid pictures (38 PNG, 64x64 RGBA) and their manifest, in the frozen M5k stage copy
# (byte-identical to the fork worktree's scripts/item-icons/assets/). Each icon's sha256 is checked
# against the pinned manifest's entry before it is copied unchanged.
ITEM_ICONS = ("stages/m5k/stage/scripts/item-icons/assets",
              "8fbed7ab430130d87bf6761994f6b06e911e038148d2d9e9378033ce6f870887")
# The game's DI streak (proposal v8): run A of the c1162 W5 Ally Eden capture (DI Direction = Up, Hitbox
# Visualization off; Eden's GIF recorder, 1280x720 game frame, 256-colour palette), frame +13 after first
# contact, where the streak is longest and clearest. The streak is a rotated strip of that frame: the
# frame is rotated about CENTRE by ANGLE (the streak's measured axis, degrees counter-clockwise from +x)
# so the streak points along +x, then cut from ALONG[0] to ALONG[1] along the streak and +-HALF across
# it. Pixels are kept as captured; only an alpha feather (FEATHER along / across, smoothstep) softens
# the strip's edges. The frame's background and the CPU's keyblade show through, as in the game.
DI_STREAK = {
    "source": ("c1162/captures-ally/di-sdi/runA/frames/runA_f+13.png",
               "4c7efe013c29de1a7ff6c49c5db769523c886355695366ee73fa61cb40515300"),
    "centre": (738, 332),
    "angle_deg": 44.8,
    "along": (-45, 92),
    "half": 22,
    "feather": (20, 10),
    "file": "di/di-streak.png",
}
# The game's SDI spark (proposal v9): the c1162 W8 Ally Eden capture (SDI Direction = Up, Strength High, DI none,
# Hitbox Visualization off; same recorder and frame as W5), frame +12 after the hit flash, where the spark is one tall
# spike above a ring around the CPU's head. Cut as the DI streak, with the spark's axis (straight up, 90 degrees) along
# +x and CENTRE on the ring's centre (the head), which sits on the chip centre. The spark is pure white (W8 §1, W7
# §3.3), so besides the edge feather the alpha also carries a whiteness key, smoothstep((min(R,G,B) - KEY[0]) /
# (KEY[1] - KEY[0])): the grey smoke, the stage and both fighters behind it drop out. Pixels are kept as captured.
SDI_SPARK = {
    "source": ("c1162/captures-ally-sdi/sdi-up/frames/sdiup_f+12.png",
               "34084656526eec369da73ca3eaf49a355a5a4758561663450cbdd23f2e343c37"),
    "centre": (595, 262),
    "angle_deg": 90.0,
    "along": (-64, 142),
    "half": 62,
    "feather": (10, 10),
    "key": (200, 245),
    "file": "sdi/sdi-spark.png",
}
# A visual check only (proposal v9): the spark in the SDI In run (the CPU stood right of the player, so In points
# left), frame +12, cut the same way along its own axis (180 degrees). The chip sheet shows it beside the Up crop
# turned to In, so a reviewer can compare the game's own In with the mock's rotation. It is not used on any chip.
SDI_SPARK_CHECK = {
    "source": ("c1162/captures-ally-sdi/sdi-in/frames/sdiin_f+12.png",
               "c43059336aa8823978b13602ead400d39347b9fd9179ac4ae719a3cf3c22d124"),
    "centre": (463, 352),
    "angle_deg": 180.0,
    "along": (-80, 86),
    "half": 66,
    "feather": (10, 10),
    "key": (200, 245),
    "file": "sdi/sdi-spark-in-check.png",
}
# The chips themselves (proposal v9): game-derived renders of the DI and SDI effects by the c1162 W9 extraction
# (`di-sdi-extract/RESULT.md` §4: the emitters' own textures, meshes and colours from the stock 13.0.5
# `ef_common.eff`, one still at particle life t = 0.3, effect origin on the chip centre, reach 0.46 x chip size; a
# 432 px RGBA light layer, alpha = max channel). Only the two Up layers are copied, byte for byte; the renderer turns
# each to every direction, as the game turns the one effect. W9's own manifest and the two files are pinned by sha256.
# Nothing else from W9 (no .eff, texture, mesh or parameter file) is copied.
W9_RENDERS = ("c1162/di-sdi-extract/out/chips",
              "e6ac05dded1b811ab43987d3a57c5d2c15d68afd55ad528ee890b3de59362661",
              {"di-up-4x-layer.png": "b87f8d8122ed5c752f623a95f4832f34fea1f7242cf82851105524910c494ccb",
               "sdi-up-4x-layer.png": "9d665a9051a1f54030ed53529f2eba1c18bf1a3196f5fdffe75b1b54fa061254"})


# The SDI chip's still (proposal v10): the same W9 render with the burst's `impactring1` scale-Y read from its own key
# table at particle life 0.6 instead of 0.3 (tools/render_sdi_stretch.py; it needs numpy, so it runs under the
# ssbu-recomp venv Python, imports W9's own renderer read-only and self-checks that life 0.3 reproduces W9's file).
# The result is the one file the SDI chips draw; the byte copy of W9's own Up layer above stays beside it.
W9_DIR = "c1162/di-sdi-extract"
VENV_PYTHON = "/home/intpa/ssbu-recomp/deps/venv/bin/python3"


def load_renderer():
    spec = importlib.util.spec_from_file_location("render_menu_mockup", REPO / "tools" / "render_menu_mockup.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def to_capture(kind, x, y):
    """Game-frame 1280x720 coordinates to capture pixels."""
    if kind == "eden":
        return 361 + x * 1198 / 1280, 181 + y * 674 / 720
    return x, y


def box(kind, cx, cy, hw, hh):
    (x0, y0), (x1, y1) = to_capture(kind, cx - hw, cy - hh), to_capture(kind, cx + hw, cy + hh)
    return [round(x0), round(y0), round(x1), round(y1)]


def smoothstep(t):
    t = max(0.0, min(1.0, t))
    return t * t * (3 - 2 * t)


def strip_crop(d):
    """Cut a rotated strip of a pinned frame (DI_STREAK, SDI_SPARK, SDI_SPARK_CHECK) and return its crops.json
    record. Pixels stay as captured; only the alpha changes (edge feather, and the whiteness key if `key` is set)."""
    rel, sha = d["source"]
    path = ROOT / rel
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != sha:
        sys.exit(f"{path}: sha256 {digest} != pinned {sha}")
    (cx, cy), ang, (a0, a1), half = d["centre"], d["angle_deg"], d["along"], d["half"]
    frame = Image.open(path).convert("RGB")
    strip = frame.rotate(-ang, resample=Image.BICUBIC, center=(cx, cy)).crop((cx + a0, cy - half, cx + a1, cy + half))
    strip = strip.convert("RGBA")
    w, h = strip.size
    fa, fh = d["feather"]
    key = d.get("key")
    px = strip.load()
    for y in range(h):
        for x in range(w):
            r, g, b, _ = px[x, y]
            k = smoothstep(min(x, w - 1 - x) / fa) * smoothstep(min(y, h - 1 - y) / fh)
            if key:
                k *= smoothstep((min(r, g, b) - key[0]) / (key[1] - key[0]))
            px[x, y] = (r, g, b, round(255 * k))
    (OUT / d["file"]).parent.mkdir(parents=True, exist_ok=True)
    strip.save(OUT / d["file"], optimize=True)
    t = math.radians(ang)
    corners = [[round(cx + a * math.cos(t) + b * math.sin(t), 1), round(cy - a * math.sin(t) + b * math.cos(t), 1)]
               for a, b in ((a0, -half), (a1, -half), (a1, half), (a0, half))]
    rec = {"file": d["file"], "source": str(path), "sha256": sha, "frame_geometry": "1280x720 game frame",
           "centre": list(d["centre"]), "angle_deg": ang, "along": list(d["along"]), "half_width": half,
           "feather": list(d["feather"])}
    if key:
        rec["whiteness_key"] = list(key)
    return rec | {"size": [w, h], "corners_in_source": corners,
                  "bbox_in_source": [math.floor(min(p[0] for p in corners)), math.floor(min(p[1] for p in corners)),
                                     math.ceil(max(p[0] for p in corners)), math.ceil(max(p[1] for p in corners))],
                  "pivot_in_crop": [-a0, half]}


def main():
    r = load_renderer()
    images = {}
    for key, (rel, sha, kind) in SOURCES.items():
        path = ROOT / rel
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != sha:
            sys.exit(f"{path}: sha256 {digest} != pinned {sha}")
        images[key] = Image.open(path).convert("RGB")
    for sub in ("portraits", "stages", "items", "di", "sdi", "renders", "renders/variants"):
        (OUT / sub).mkdir(parents=True, exist_ok=True)
        for old in (OUT / sub).glob("*.png"):
            old.unlink()
    cells = []
    # Portrait roster: fork README (scripts/roster-grid) cell centre x=(col-6)*132, y=345-row*105 on the
    # 1920x1080 canvas = (640+(col-6)*88, 130+row*70) at 720p; the 96 px portrait = 64 px; crop +-30.
    for row, names in enumerate(r.ROSTER):
        start = 2 if row == 6 else 0
        for i, name in enumerate(names):
            col = start + i
            src = PORTRAIT_SOURCE.get(name, "change-player")
            kind = SOURCES[src][2]
            b = box(kind, 640 + (col - 6) * 88, 130 + row * 70, 30, 30)
            out = f"portraits/{r.slug(name)}.png"
            images[src].crop(b).save(OUT / out, optimize=True)
            cells.append({"cell": name, "file": out, "source": src, "box": b})
    # Stage page 1: fork README (scripts/stage-grid) centres x=(col-1.5)*460, y=300-row*200 on 1920x1080;
    # the picture sits at (x, y+15), aspect-kept 108x90 in 340x130 = 104x87 at 720p; All is 87x87.
    for i, name in enumerate(["ALL"] + r.STAGES):
        if name == "Waiting Room":
            continue  # the fork draws Waiting Room as a text tile without a picture
        col, row = i % 4, i // 4
        cx, cy = 640 + (col - 1.5) * 306.667, 160 + row * 133.333 - 10
        b = box("frame", cx, cy, 43.5 if name == "ALL" else 52, 43.5)
        out = f"stages/{r.slug(name)}.png"
        images["stage-rotation"].crop(b).save(OUT / out, optimize=True)
        cells.append({"cell": name, "file": out, "source": "stage-rotation", "box": b})
    # Item grid: the fork's own icon files, copied unchanged (no crop, no re-encode), in ITEM_NAMES order.
    item_dir = ROOT / ITEM_ICONS[0]
    raw = (item_dir / "sources.json").read_bytes()
    if hashlib.sha256(raw).hexdigest() != ITEM_ICONS[1]:
        sys.exit(f"{item_dir / 'sources.json'}: sha256 differs from the pinned {ITEM_ICONS[1]}")
    icons = json.loads(raw)["icons"]
    if [i["label"] for i in icons] != r.ITEMS:
        sys.exit("item manifest labels differ from the renderer's ITEMS (character_items.rs ITEM_NAMES order)")
    items = []
    for icon in icons:
        data = (item_dir / icon["filename"]).read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        if digest != icon["sha256"]:
            sys.exit(f"{icon['filename']}: sha256 {digest} != manifest {icon['sha256']}")
        out = f"items/{icon['filename']}"
        (OUT / out).write_bytes(data)
        items.append({"index": icon["index"], "cell": icon["label"], "file": out, "sha256": digest})
    streak = strip_crop(DI_STREAK)
    spark = strip_crop(SDI_SPARK)
    spark_check = strip_crop(SDI_SPARK_CHECK)
    render_dir, render_manifest_sha, render_files = W9_RENDERS
    render_dir = ROOT / render_dir
    digest = hashlib.sha256((render_dir / "manifest.json").read_bytes()).hexdigest()
    if digest != render_manifest_sha:
        sys.exit(f"{render_dir / 'manifest.json'}: sha256 {digest} != pinned {render_manifest_sha}")
    renders = []
    for name, sha in render_files.items():
        data = (render_dir / name).read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        if digest != sha:
            sys.exit(f"{render_dir / name}: sha256 {digest} != pinned {sha}")
        (OUT / "renders" / name).write_bytes(data)
        renders.append({"file": f"renders/{name}", "source": str(render_dir / name), "sha256": sha,
                        "size": list(Image.open(render_dir / name).size)})
    run = subprocess.run([VENV_PYTHON, "-I", str(REPO / "tools" / "render_sdi_stretch.py"), str(ROOT / W9_DIR),
                          str(OUT / "renders")], capture_output=True, text=True)
    if run.returncode:
        sys.exit(f"render_sdi_stretch.py failed ({run.returncode}): {run.stderr.strip() or run.stdout.strip()}")
    stretch = json.loads(run.stdout)
    manifest = {
        "purpose": "Review mockup only; game-asset captures of the fork's grid pickers, byte copies of the fork's "
                   "item icons, crops of the game's DI streak and SDI spark from Eden captures and two game-derived DI / "
                   "SDI chip renders (c1162 W9); not shipped artwork.",
        "sources": {k: {"path": str(ROOT / rel), "sha256": sha, "geometry": kind} for k, (rel, sha, kind) in SOURCES.items()},
        "item_icons": {"path": str(item_dir), "manifest_sha256": ITEM_ICONS[1]},
        "cells": cells,
        "items": items,
        "di_streak": streak,
        "sdi_spark": spark,
        "sdi_spark_check": spark_check,
        "w9_renders": {"path": str(render_dir), "manifest_sha256": render_manifest_sha, "files": renders},
        "sdi_stretch": stretch,
    }
    (OUT / "crops.json").write_text(json.dumps(manifest, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote {len(cells)} crops, {len(items)} item icons, the DI streak and the SDI spark (+ In check) to {OUT}")


if __name__ == "__main__":
    main()
