#!/usr/bin/python3 -I
"""Crop the fork's real grid-picker captures into per-cell images for the menu mockup (proposal v4),
and copy the fork's 38 item icons byte for byte (proposal v5).

Review-mockup material only, never shipped artwork: the crops show game assets (stock fighter icons,
stage thumbnails) exactly as the fork's own grid pickers draw them in Training; the item icons are the
fork's own `scripts/item-icons/assets/icon-NN.png` files, unchanged. Every source capture, the item
manifest and every item icon are pinned by sha256; every crop box and copy is written to
docs/menu-mockup/captures/crops.json, which the renderer copies into docs/menu-mockup/index.md.

Usage: /usr/bin/python3 -I tools/crop_menu_captures.py [ARTIFACTS_REFACTOR_ROOT]
The default root is the fork campaign's `parked-reuse175/refactor` directory on the review host.
"""
import hashlib
import importlib.util
import json
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


def main():
    r = load_renderer()
    images = {}
    for key, (rel, sha, kind) in SOURCES.items():
        path = ROOT / rel
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != sha:
            sys.exit(f"{path}: sha256 {digest} != pinned {sha}")
        images[key] = Image.open(path).convert("RGB")
    for sub in ("portraits", "stages", "items"):
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
    manifest = {
        "purpose": "Review mockup only; game-asset captures of the fork's grid pickers and byte copies of the fork's "
                   "item icons; not shipped artwork.",
        "sources": {k: {"path": str(ROOT / rel), "sha256": sha, "geometry": kind} for k, (rel, sha, kind) in SOURCES.items()},
        "item_icons": {"path": str(item_dir), "manifest_sha256": ITEM_ICONS[1]},
        "cells": cells,
        "items": items,
    }
    (OUT / "crops.json").write_text(json.dumps(manifest, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote {len(cells)} crops and {len(items)} item icons to {OUT}")


if __name__ == "__main__":
    main()
