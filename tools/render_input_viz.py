#!/usr/bin/env python3
"""INPUT-VIZ decision mockups: three compact input/timing displays drawn over a real Eden Training frame.

Usage: render_input_viz.py <background.png> <out_dir>

The background is an Eden capture (1280x674 with side bars); its 16:9 game area is cropped and scaled to
1280x720. Every overlay element is drawn here from rectangles, lines and text in the plugin's own style; no game or
fork artwork is used. All frame numbers are illustrative (a plausible Steve jab -> up tilt link on Donkey Kong).
"""
import os
import sys

from PIL import Image, ImageDraw, ImageFont

W, H = 1280, 720


def font(size, bold=False, mono=False):
    # DejaVu carries the minus sign, the arrows and the check marks the readouts use.
    names = (["DejaVuSansMono-Bold.ttf"] if mono else []) + (["DejaVuSans-Bold.ttf"] if bold else ["DejaVuSans.ttf"])
    for name in names:
        for folder in ("/usr/share/fonts/truetype/noto", "/usr/share/fonts/truetype/dejavu"):
            path = os.path.join(folder, name)
            if os.path.exists(path):
                return ImageFont.truetype(path, size)
    return ImageFont.load_default()


ARROWS = font(14, bold=True)  # DejaVu carries the arrow glyphs
for _folder in ("/usr/share/fonts/truetype/dejavu",):
    if os.path.exists(os.path.join(_folder, "DejaVuSans-Bold.ttf")):
        ARROWS = ImageFont.truetype(os.path.join(_folder, "DejaVuSans-Bold.ttf"), 14)

# The plugin's own palette for this mockup.
PANEL = (14, 18, 27, 222)
EDGE = (255, 255, 255, 46)
TEXT = (236, 240, 246, 255)
MUTED = (150, 160, 176, 255)
GOOD = (72, 208, 118, 255)
BAD = (240, 86, 78, 255)
WARN = (244, 186, 64, 255)
STATE = {
    "idle": (64, 72, 88, 255),       # actionable
    "startup": (244, 180, 52, 255),
    "active": (238, 70, 84, 255),
    "recovery": (82, 136, 232, 255),
    "hitlag": (246, 246, 246, 255),
    "hitstun": (170, 98, 234, 255),
    "shieldstun": (48, 196, 188, 255),
}
LABEL = {
    "startup": "startup", "active": "active", "recovery": "recovery", "idle": "actionable",
    "hitlag": "hitlag (freeze)", "hitstun": "hitstun", "shieldstun": "shieldstun",
}
P1_TINT = (228, 64, 64, 255)
CPU_TINT = (150, 150, 158, 255)
ATTACK_CHIP = (66, 182, 106, 255)
JUMP_CHIP = (64, 170, 220, 255)

# ---- The illustrative exchange (frame 0 = the jab input) ------------------------------------------------------
# Steve: jab startup 4, hit on frame 4, hitlag 4, 1 more active, recovery to 16, actionable 17.
# Up tilt input on frame 18 (first actionable 17: 1 frame late), startup 5, hit on frame 23, hitlag 5, active 2,
# recovery to 43, actionable 44. Donkey Kong: hitlag with each hit, hitstun 8..25 (actionable 26), then 28..49
# (actionable 50). Link gap = hit 23 - DK actionable 26 = -3 (true). Advantage +9 after the jab, +6 after the tilt.
P1_RUNS = [(-7, -1, "idle"), (0, 3, "startup"), (4, 7, "hitlag"), (8, 8, "active"), (9, 16, "recovery"),
           (17, 17, "idle"), (18, 22, "startup"), (23, 27, "hitlag"), (28, 29, "active"), (30, 43, "recovery"),
           (44, 52, "idle")]
CPU_RUNS = [(-7, 3, "idle"), (4, 7, "hitlag"), (8, 22, "hitstun"), (23, 27, "hitlag"), (28, 49, "hitstun"),
            (50, 52, "idle")]
STUN_GIVEN = [(8, 25, 18), (28, 49, 22)]   # the hitstun each hit gave (the first one cut short by the next hit)
INPUTS = [(0, None, "A"), (18, "up", "A")]
WINDOW = (17, 21)       # up tilt inputs on these frames land while DK is still in hitstun
FIRST = -7
LAST = 52


def overlay():
    return Image.new("RGBA", (W, H), (0, 0, 0, 0))


def background(path):
    im = Image.open(path).convert("RGB")
    w, h = im.size
    gw = round(h * 16 / 9)
    left = (w - gw) // 2
    return im.crop((left, 0, left + gw, h)).resize((W, H), Image.LANCZOS).convert("RGBA")


def panel(d, box, radius=10, fill=PANEL, outline=EDGE, width=1):
    d.rounded_rectangle(box, radius=radius, fill=fill, outline=outline, width=width)


def text(d, xy, s, f, fill=TEXT, anchor="la"):
    d.text(xy, s, font=f, fill=fill, anchor=anchor)


def arrow(d, cx, cy, direction, size, fill):
    """The plugin's own stick arrow: a shaft and a triangular head."""
    vec = {"up": (0, -1), "down": (0, 1), "left": (-1, 0), "right": (1, 0)}[direction]
    sx, sy = vec
    tip = (cx + sx * size, cy + sy * size)
    base = (cx - sx * size * 0.8, cy - sy * size * 0.8)
    d.line([base, (cx + sx * size * 0.2, cy + sy * size * 0.2)], fill=fill, width=max(2, size // 3))
    px, py = -sy, sx
    head = size * 0.7
    d.polygon([tip, (cx + sx * size * 0.1 + px * head, cy + sy * size * 0.1 + py * head),
               (cx + sx * size * 0.1 - px * head, cy + sy * size * 0.1 - py * head)], fill=fill)


def chip(d, x, y, stick, button, size=20, colour=ATTACK_CHIP):
    """An input chip: an optional stick arrow, then a rounded button square with its letter."""
    f = font(int(size * 0.72), bold=True)
    width = size if stick is None else size * 2 - 2
    d.rounded_rectangle((x, y, x + width, y + size), radius=5, fill=(30, 36, 48, 255), outline=(255, 255, 255, 70))
    bx = x if stick is None else x + size - 2
    if stick is not None:
        arrow(d, x + size / 2, y + size / 2, stick, int(size * 0.32), TEXT)
    d.rounded_rectangle((bx + 2, y + 2, bx + size - 2, y + size - 2), radius=4,
                        fill=JUMP_CHIP if button == "X" else colour)
    text(d, (bx + size / 2, y + size / 2 + 1), button, f, fill=(10, 14, 20, 255), anchor="mm")
    return width


def hatch(d, box, colour):
    x0, y0, x1, y1 = box
    d.rectangle(box, fill=colour)
    for k in range(int(x0) - int(y1 - y0), int(x1), 4):
        d.line([(max(k, x0), y0 + (max(k, x0) - k)), (min(k + (y1 - y0), x1), y0 + (min(k + (y1 - y0), x1) - k))],
               fill=(150, 156, 168, 255), width=1)


def tag(d, box, s, fill, f, fg=(10, 14, 20, 255)):
    d.rounded_rectangle(box, radius=(box[3] - box[1]) // 2, fill=fill)
    text(d, ((box[0] + box[2]) / 2, (box[1] + box[3]) / 2 + 1), s, f, fill=fg, anchor="mm")


def note_box(d, s_title, lines, recommended):
    """The mockup annotation (not part of the HUD), bottom right over the empty floor."""
    box = (842, 588, 1268, 710)
    panel(d, box, radius=8, fill=(250, 250, 252, 236),
          outline=(40, 170, 80, 255) if recommended else (90, 96, 110, 255), width=4 if recommended else 2)
    text(d, (box[0] + 14, box[1] + 10), s_title, font(17, bold=True), fill=(20, 24, 32, 255))
    y = box[1] + 36
    for line in lines:
        text(d, (box[0] + 14, y), line, font(13), fill=(46, 52, 64, 255))
        y += 18


def legend(d, x, y, states, f):
    for s in states:
        box = (x, y + 2, x + 14, y + 14)
        if s == "hitlag":
            hatch(d, box, STATE[s])
        else:
            d.rectangle(box, fill=STATE[s])
        text(d, (x + 19, y), LABEL[s], f, fill=TEXT)
        x += 24 + d.textlength(LABEL[s], font=f) + 10


# ---- Option A: the frame strip ---------------------------------------------------------------------------------
def option_a(bg):
    img = bg.copy()
    o = overlay()
    d = ImageDraw.Draw(o)
    pitch, cw = 9, 8
    x0, y0 = 196, 8
    gx = x0 + 56                                   # first cell
    col = lambda f: gx + (f - FIRST) * pitch       # left edge of frame f's cell
    nx = col(LAST) + cw + 3                        # the "now" edge
    rx = nx + 18                                   # readouts
    x1, y1 = rx + 248, y0 + 144
    panel(d, (x0, y0, x1, y1))
    lane_in, lane_p1, lane_cpu = y0 + 10, y0 + 48, y0 + 72
    lh = 19
    small = font(11, bold=True)
    tiny = font(10)
    tag(d, (x0 + 8, lane_in + 3, x0 + 48, lane_in + 21), "IN", (60, 68, 84, 255), small, fg=TEXT)
    tag(d, (x0 + 8, lane_p1 + 1, x0 + 48, lane_p1 + 18), "P1", P1_TINT, small, fg=(255, 255, 255, 255))
    tag(d, (x0 + 8, lane_cpu + 1, x0 + 48, lane_cpu + 18), "CPU", CPU_TINT, small, fg=(20, 22, 28, 255))
    # The window in which the follow-up input still makes a true combo, on the input row.
    d.rectangle((col(WINDOW[0]), lane_p1 - 8, col(WINDOW[1]) + cw, lane_p1 - 3), fill=GOOD)
    text(d, (col(WINDOW[1]) + cw + 5, lane_p1 - 6), "combo window 5f", tiny, fill=GOOD, anchor="lm")
    # Lanes.
    for runs, top in ((P1_RUNS, lane_p1), (CPU_RUNS, lane_cpu)):
        for a, b, s in runs:
            for f in range(a, b + 1):
                box = (col(f), top, col(f) + cw, top + lh)
                if s == "hitlag":
                    hatch(d, box, STATE[s])
                else:
                    d.rectangle(box, fill=STATE[s])
    # The hitstun the jab gave, still running when the up tilt landed: a violet frame over the hitlag cells.
    d.rectangle((col(23) - 1, lane_cpu - 2, col(25) + cw + 1, lane_cpu + lh + 2), outline=STATE["hitstun"], width=2)
    for a, b, n in STUN_GIVEN:
        end = min(b, 22) if a == 8 else b
        text(d, ((col(a) + col(end) + cw) / 2, lane_cpu + lh / 2), str(n), small, fill=(255, 255, 255, 255),
             anchor="mm")
    for a, b, s in P1_RUNS:
        if s == "recovery" and b - a >= 6:
            text(d, ((col(a) + col(b) + cw) / 2, lane_p1 + lh / 2), str(b - a + 1), small,
                 fill=(255, 255, 255, 255), anchor="mm")
    # Inputs: a chip above the frame they landed on, with a tick down to the P1 lane.
    for f, stick, button in INPUTS:
        cx = col(f) + cw / 2
        d.line([(cx, lane_in + 22), (cx, lane_p1)], fill=(255, 255, 255, 170), width=1)
        width = 20 if stick is None else 38
        chip(d, cx - width / 2, lane_in + 3, stick, button)
    # Link bracket under the CPU lane: hit on 23, DK free on 26. Advantage bracket: P1 free 44, DK free 50.
    by = lane_cpu + lh + 4
    for (a, b, label) in ((col(23), col(26), "link −3"), (col(44), col(50), "adv +6")):
        d.line([(a, by), (a, by + 6), (b, by + 6), (b, by)], fill=GOOD, width=2)
        text(d, ((a + b) / 2, by + 8), label, small, fill=GOOD, anchor="ma")
    # Now marker.
    d.line([(nx, lane_in), (nx, lane_cpu + lh)], fill=(255, 255, 255, 120), width=1)
    text(d, (nx - 3, lane_in), "now", tiny, fill=MUTED, anchor="ra")
    # Readouts.
    d.line([(rx - 9, y0 + 10), (rx - 9, y0 + 108)], fill=EDGE, width=1)
    big = font(21, bold=True, mono=True)
    lab = font(11, bold=True)
    rows = [("ADVANTAGE", "+6", GOOD, "after the up tilt"), ("LINK", "−3", GOOD, "TRUE combo ✓"),
            ("INPUT", "+1", WARN, "1f late · 3 spare")]
    for i, (name, value, colour, sub) in enumerate(rows):
        yy = y0 + 6 + i * 37
        text(d, (rx, yy), name, lab, fill=MUTED)
        text(d, (rx, yy + 12), value, big, fill=colour)
        text(d, (rx + 50, yy + 17), sub, font(12, bold=True), fill=TEXT)
    # The last few links, newest first (the same exchanges as option B's cards).
    hy = y0 + 122
    d.line([(x0 + 10, hy - 3), (x1 - 10, hy - 3)], fill=EDGE, width=1)
    text(d, (x0 + 12, hy + 8), "LAST", small, fill=MUTED, anchor="lm")
    hx = gx
    for label, ok, move in (("TRUE −3", True, "Jab → Up tilt"), ("DROP +2", False, "Up tilt → Up tilt"),
                            ("TRUE −1", True, "Jab → Down tilt"), ("TRUE −6", True, "Jab 1 → Jab 2")):
        tag(d, (hx, hy, hx + 64, hy + 16), label, GOOD if ok else BAD, font(10, bold=True))
        text(d, (hx + 70, hy + 8), move, tiny, fill=TEXT, anchor="lm")
        hx += 70 + d.textlength(move, font=tiny) + 18
    img.alpha_composite(o)
    d3 = ImageDraw.Draw(img)
    note_box(d3, "Option A · Frame strip (recommended)", [
        "Mockup over a real Eden Training frame; frame numbers are",
        "illustrative (Steve jab → up tilt on Donkey Kong). The last",
        "60 frames: P1 inputs above both fighters' states; the",
        "readouts and LAST row sum up the most recent links.",
    ], True)
    legend_box = (842, 548, 1268, 584)
    lo = overlay()
    ld = ImageDraw.Draw(lo)
    panel(ld, legend_box, radius=6, fill=(14, 18, 27, 225), outline=(0, 0, 0, 0))
    legend(ld, legend_box[0] + 8, legend_box[1] + 3, ["startup", "active", "recovery", "idle"], font(11))
    legend(ld, legend_box[0] + 8, legend_box[1] + 19, ["hitlag", "hitstun", "shieldstun"], font(11))
    img.alpha_composite(lo)
    return img


# ---- Option B: link cards ----------------------------------------------------------------------------------------
CARDS = [
    # (chips, names, verdict, gap, window, input offset, advantage, faded)
    ([(None, "A"), ("up", "A")], "Jab → Up tilt", True, -3, 5, 1, "+6", 1.0),
    ([("up", "A"), ("up", "A")], "Up tilt → Up tilt", False, 2, 2, 3, "−8 on shield", 0.85),
    ([(None, "A"), ("down", "A")], "Jab → Down tilt", True, -1, 3, 1, "+5", 0.7),
    ([(None, "A"), (None, "A")], "Jab 1 → Jab 2", True, -6, 7, 0, "+9", 0.55),
]


def option_b(bg):
    img = bg.copy()
    o = overlay()
    d = ImageDraw.Draw(o)
    x0, y0, x1 = 12, 112, 332
    ch = 96
    y1 = y0 + 32 + ch * len(CARDS) + 4
    panel(d, (x0, y0, x1, y1))
    text(d, (x0 + 12, y0 + 9), "LINKS", font(15, bold=True))
    text(d, (x0 + 68, y0 + 11), "last 4 exchanges · P1", font(12), fill=MUTED)
    for i, (chips, names, true, gap, window, offset, adv, fade) in enumerate(CARDS):
        cy = y0 + 32 + i * ch
        a = int(255 * fade)
        card = Image.new("RGBA", (x1 - x0 - 16, ch - 8), (0, 0, 0, 0))
        c = ImageDraw.Draw(card)
        c.rounded_rectangle((0, 0, card.width - 1, card.height - 1), radius=7, fill=(32, 38, 52, 255),
                            outline=(GOOD if true else BAD)[:3] + (200,), width=2)
        x = 8
        for k, (stick, button) in enumerate(chips):
            x += chip(c, x, 8, stick, button) + 4
            if k == 0:
                text(c, (x + 2, 18), "→", ARROWS, fill=MUTED, anchor="lm")
                x += 18
        verdict = ("TRUE " if true else "DROP ") + ("−" if gap < 0 else "+" if gap > 0 else "") + str(abs(gap))
        tag(c, (card.width - 86, 7, card.width - 8, 29), verdict, GOOD if true else BAD, font(13, bold=True))
        text(c, (10, 33), names, font(12, bold=True), fill=TEXT)
        # The input-timing bar: frames from the first actionable frame (0) to +9; green = the true-combo window.
        bx, by, step = 10, 54, 12
        for f in range(-1, 10):
            fill = (72, 208, 118, 255) if 0 <= f < window else (70, 78, 94, 255)
            c.rectangle((bx + (f + 1) * step, by, bx + (f + 1) * step + step - 2, by + 10), fill=fill)
        text(c, (bx + step + 4, by + 13), "0", font(10), fill=MUTED, anchor="ma")
        mx = bx + (offset + 1) * step + (step - 2) / 2
        c.polygon([(mx - 6, by - 7), (mx + 6, by - 7), (mx, by)], fill=(255, 255, 255, 255))
        late = "on time" if offset == 0 else f"{offset}f late"
        spare = window - 1 - offset
        sub = late + (f" · {spare} spare" if spare >= 0 else f" · {-spare}f past")
        tx = bx + 11 * step + 10
        text(c, (tx, by - 8), sub, font(12, bold=True), fill=(WARN if spare >= 0 else BAD))
        text(c, (tx, by + 10), "adv " + adv, font(12, bold=True), fill=(GOOD if adv.startswith("+") else BAD))
        if a < 255:
            alpha = card.getchannel("A").point(lambda v: v * a // 255)
            card.putalpha(alpha)
        o.alpha_composite(card, (x0 + 8, cy))
    img.alpha_composite(o)
    d3 = ImageDraw.Draw(img)
    note_box(d3, "Option B · Link cards", [
        "Mockup over a real Eden Training frame; numbers illustrative.",
        "One card per exchange: inputs, move names, TRUE/DROP",
        "with the link gap, your press (▼) against the combo",
        "window (green) and the advantage after. Older cards fade.",
    ], False)
    return img


# ---- Option C: the link ruler ------------------------------------------------------------------------------------
def ruler(d, x, y, lo, hi, step, zones, markers, label, value, value_colour, caption):
    lab = font(13, bold=True)
    text(d, (x, y + 4), label, lab, fill=MUTED)
    rx = x + 66
    for f in range(lo, hi + 1):
        fill = (70, 78, 94, 255)
        for a, b, colour in zones:
            if a <= f <= b:
                fill = colour
        d.rectangle((rx + (f - lo) * step, y + 6, rx + (f - lo) * step + step - 2, y + 18), fill=fill)
        if f % 2 == 0:
            text(d, (rx + (f - lo) * step + (step - 2) / 2, y + 21), ("−" if f < 0 else "+" if f > 0 else "") +
                 str(abs(f)), font(10), fill=MUTED, anchor="ma")
    for f, alpha, big in markers:
        mx = rx + (f - lo) * step + (step - 2) / 2
        if big:
            d.polygon([(mx - 7, y - 4), (mx + 7, y - 4), (mx, y + 6)], fill=(255, 255, 255, 255))
            d.line([(mx, y + 4), (mx, y + 20)], fill=(255, 255, 255, 255), width=2)
        else:
            d.ellipse((mx - 4, y - 3, mx + 4, y + 5), fill=(255, 255, 255, alpha))
    vx = rx + (hi - lo + 1) * step + 14
    text(d, (vx, y - 3), value, font(22, bold=True, mono=True), fill=value_colour)
    text(d, (vx + 50, y + 4), caption, font(12, bold=True), fill=value_colour)


def option_c(bg):
    img = bg.copy()
    o = overlay()
    d = ImageDraw.Draw(o)
    x0, y0, x1, y1 = 330, 8, 950, 132
    panel(d, (x0, y0, x1, y1))
    # The input sequence with the frames between inputs.
    x = x0 + 14
    text(d, (x, y0 + 13), "INPUTS", font(13, bold=True), fill=MUTED)
    x += 66
    seq = [((None, "A"), "18f"), (("up", "A"), None)]
    for (stick, button), gap in seq:
        x += chip(d, x, y0 + 9, stick, button, size=22) + 6
        if gap:
            d.line([(x, y0 + 20), (x + 44, y0 + 20)], fill=MUTED, width=2)
            text(d, (x + 22, y0 + 16), gap, font(11, bold=True), fill=TEXT, anchor="mb")
            x += 50
    text(d, (x + 4, y0 + 20), "Jab → Up tilt", font(13, bold=True), fill=TEXT, anchor="lm")
    text(d, (x1 - 14, y0 + 20), "adv +9 → +6", font(13, bold=True), fill=GOOD, anchor="rm")
    green = (72, 208, 118, 255)
    red = (200, 70, 66, 255)
    # LINK: where the follow-up hit landed against the moment the opponent could act (0). ≤ 0 = true combo.
    ruler(d, x0 + 14, y0 + 48, -8, 6, 20, [(-8, 0, green), (1, 6, red)],
          [(-1, 120, False), (2, 140, False), (-4, 170, False), (0, 200, False), (-3, 255, True)],
          "LINK", "−3", GOOD, "TRUE · 3 to spare")
    # INPUT: the follow-up input against your first actionable frame (0); green = inputs that still combo.
    ruler(d, x0 + 14, y0 + 92, -3, 11, 20, [(0, 4, green)],
          [(3, 120, False), (6, 140, False), (0, 170, False), (4, 200, False), (1, 255, True)],
          "INPUT", "+1", WARN, "1f late · window 0…+4")
    img.alpha_composite(o)
    d3 = ImageDraw.Draw(img)
    note_box(d3, "Option C · Link ruler", [
        "Mockup over a real Eden Training frame; numbers illustrative.",
        "LINK: hit vs the opponent's first free frame (≤ 0 = true).",
        "INPUT: your press vs your first actionable frame, with the",
        "combo window in green. Dots = the last attempts, fading.",
    ], False)
    return img


def main():
    bg = background(sys.argv[1])
    out = sys.argv[2]
    os.makedirs(out, exist_ok=True)
    for name, fn in (("INPUT-VIZ-A-frame-strip", option_a), ("INPUT-VIZ-B-link-cards", option_b),
                     ("INPUT-VIZ-C-link-ruler", option_c)):
        fn(bg).convert("RGB").save(os.path.join(out, name + ".png"), optimize=True)
        print(os.path.join(out, name + ".png"))


if __name__ == "__main__":
    main()
