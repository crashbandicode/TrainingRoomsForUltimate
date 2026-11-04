"""Draw docs/decisions/HB1-route.png: how hitbox / hurtbox spheres are drawn (route B shared pool vs route A own system).

Same style as tools/render_decisions.py (its canvas and helpers); the recommended option is outlined in green.
The left panel shows a real Eden capture of route B (run hb-03, stream `hitbox`):
  /home/intpa/smash-training-lobby/TrainingRoomsForUltimate-hitbox/eden/runs/hb-03/shots/122157-air-3.png
Pass another path as argv[1] to use a different copy of that frame.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import render_decisions as d  # noqa: E402
from PIL import Image  # noqa: E402

SHOT = Path(sys.argv[1] if len(sys.argv) > 1 else
            "/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-hitbox/eden/runs/hb-03/shots/122157-air-3.png")
HURT = (242, 201, 76)
HIT = (214, 84, 72)
W, H = d.WIDE, 490


def paste(c, im, x0, y0, x1, y1):
    k = min((x1 - x0) / im.width, (y1 - y0) / im.height)
    w, h = round(im.width * k), round(im.height * k)
    c.img.alpha_composite(im.convert("RGBA").resize((w, h), Image.LANCZOS), (x0, y0))
    c.rect(x0, y0, x0 + w, y0 + h, outline=d.LINE, width=1)
    return x0 + w, y0 + h


def main():
    c = d.DCanvas(W, H)
    mid = W // 2
    # Route B (left, recommended)
    y = d.panel(c, 12, 12, mid - 6, H - 12, "B · shared effect pool", rec=True,
                note="Proven on Eden (hb-03): 23 spheres on Luigi, 19 on Steve, 0 failed")
    shot = Image.open(SHOT)
    x1, y1 = paste(c, shot.crop((330, 70, 760, 330)), 26, y, 26 + 330, y + 210)
    paste(c, shot.crop((380, 110, 500, 250)), x1 + 12, y, mid - 22, y + 210)
    y = y1 + 16
    d.lines(c, 26, y, [
        "Each sphere is a sys_shield effect from the",
        "game's own effect pool (the fork's method).",
        ("About 56 free slots, shared with sparks, KO", d.FG),
        ("effects and trails.", d.FG),
        ("Ridley's first frame: 55 wanted, 2 refused", d.BAD),
        ("until other effects ended (hb-04).", d.BAD),
        "Move effects pause while it is On (as the fork).",
    ], font="xs", step=21)
    # Route A (right)
    y = d.panel(c, mid + 6, 12, W - 12, H - 12, "A · own visualization system",
                note="Not proven: no safe route found (study 2026-11-03)")
    ax = mid + 26
    d.box(c, ax, y + 6, ax + 170, y + 56, ["game effects"])
    d.box(c, ax + 300, y + 6, W - 32, y + 56, ["game effect pool"])
    d.arrow(c, (ax + 172, y + 31), (ax + 298, y + 31))
    for cx, cy, r, col in ((ax + 40, y + 120, 16, HURT), (ax + 78, y + 136, 12, HURT), (ax + 58, y + 172, 12, HIT)):
        c.circle(cx, cy, r, fill=col, outline=d.DARK, width=1)
    c.text(ax, y + 196, "visualization spheres", "xs", d.MUTED)
    d.box(c, ax + 300, y + 96, W - 32, y + 180, ["separate renderer", "+ authored sphere", "(neither exists yet)"],
          col=d.MUTED, fill=d.PANEL, tcol=d.MUTED)
    d.arrow(c, (ax + 110, y + 140), (ax + 298, y + 140), col=d.MUTED)
    d.lines(c, ax, y + 236, [
        "Spheres would never take the game's slots.",
        ("Missing: GPU, allocator and teardown", d.BAD),
        ("ownership for a second system, draw order,", d.BAD),
        ("bone following, and a sphere asset", d.BAD),
        ("(ef_common.eff has none).", d.BAD),
        "HB1 drawing stays Off until it is built",
        "and proven (trace boot + Eden screenshot).",
    ], font="xs", step=21)
    c.save("HB1-route")


if __name__ == "__main__":
    main()
