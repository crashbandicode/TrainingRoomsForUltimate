#!/usr/bin/python3 -I
"""Draw docs/decisions/GTE-*.png and BRIDGE-40MS.png: one picture per operator decision opened on 2026-11-21
(docs/DECISIONS.md, "Open — 2026-11-21 morning (Guard the Edge + bridge)").

Same look as tools/render_decisions.py (its canvas, colours and panels are reused); the recommended option is
outlined in green. These pictures are 800 px wide with larger type and stacked options, so they stay readable at phone
width.

Every labelled number comes from the posts and model files the section cites (paths under artifacts/phase23/ in the
workspace). The stage is Final Destination's ring to scale, from edge-juggle/reference-crossings/MODEL.md §2 (the same
ring on Battlefield and the Waiting Room). Measured points (starts, the closest approach, Bayonetta's worked trace) are
placed at their coordinates; where the path between them is not in the evidence it is drawn as a plain curve and
carries no number.

usage: /usr/bin/python3 -I tools/render_decisions_1121.py [function name ...]
"""
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import render_decisions as d  # noqa: E402
import render_menu_mockup as m  # noqa: E402

FG, MUTED, OURS, ADV, BAD, LINE = d.FG, d.MUTED, d.OURS, d.ADV, d.BAD, d.LINE
TILE_HI, PANEL, DARK, CPU = d.TILE_HI, d.PANEL, d.DARK, d.CPU
W = 800
PAD = 12
STAGE = (78, 86, 104)
GRID = (52, 58, 72)

# Phone-width type: body text is 22 px on an 800 px picture (the "recommended" tag 21 px bold).
m.FONTS.update({"P_title": (m.FONT_B, 30), "P_h": (m.FONT_B, 25), "P_s": (m.FONT, 22), "P_b": (m.FONT_B, 22),
                "P_tag": (m.FONT_B, 21)})
STEP = 30  # line step for P_s

# Final Destination's ring (MODEL.md §2, "the common ring"); ledges at (±79.989, 0).
RING = [(41.416, -40.118), (-41.396, -40.099), (-64.395, -25.111), (-79.986, -8.319), (-79.986, 0), (-74.986, 0),
        (74.989, 0), (79.989, 0), (79.989, -8.319), (64.387, -25.103)]
LEDGE_X = 79.989


# ---------------------------------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------------------------------
def canvas():
    return d.DCanvas(W, 2000)


def finish(c, y, name):
    c.img = c.img.crop((0, 0, W, int(y) + PAD))
    c.save(name)


def wrap(c, text, font, width):
    out, cur = [], ""
    for word in text.split(" "):
        t = word if not cur else cur + " " + word
        if c.textlength(t, font) <= width or not cur:
            cur = t
        else:
            out.append(cur)
            cur = word
    if cur:
        out.append(cur)
    for ln in out:
        assert c.textlength(ln, font) <= width, f"too wide: {ln!r}"
    return out


def para(c, x, y, text, width, font="P_s", col=FG, step=STEP):
    for ln in wrap(c, text, font, width):
        c.text(x, y, ln, font, col)
        y += step
    return y


def header(c, title, sub):
    assert c.textlength(title, "P_title") <= W - 2 * PAD - 8, title
    c.text(PAD + 4, 14, title, "P_title", FG)
    return para(c, PAD + 4, 58, sub, W - 2 * PAD - 8, "P_s", MUTED) + 8


def tag(c, x_right, y, text, col):
    w = c.textlength(text, "P_tag") + 18
    c.rect(x_right - w, y, x_right, y + 30, fill=col, radius=6)
    c.text(x_right - w / 2, y + 4, text, "P_tag", DARK, "m")
    return w


def options(c, y, opts, rec):
    """Full-width option panels stacked; an option is (title, [(text, colour), ...]); rec is the recommended index."""
    inner = W - 2 * PAD - 32
    for i, (title, body) in enumerate(opts):
        is_rec = i == rec
        tw = c.textlength("recommended", "P_tag") + 18 + 16 if is_rec else 0
        assert c.textlength(title, "P_h") + tw <= inner, title
        lines = [(ln, col) for text, col in body for ln in wrap(c, text, "P_s", inner)]
        h = 14 + 40 + len(lines) * STEP + 10
        c.rect(PAD, y, W - PAD, y + h, fill=PANEL, outline=OURS if is_rec else LINE, width=4 if is_rec else 1,
               radius=8)
        c.text(PAD + 16, y + 14, title, "P_h", OURS if is_rec else FG)
        if is_rec:
            tag(c, W - PAD - 12, y + 12, "recommended", OURS)
        yy = y + 14 + 42
        for ln, col in lines:
            c.text(PAD + 16, yy, ln, "P_s", col)
            yy += STEP
        y += h + 10
    return y


class World:
    """Game coordinates (x right, y up) onto a screen box, one scale for both axes."""

    def __init__(self, sx0, sy0, sx1, wx0, wx1, wy_top, wy_bot):
        self.k = (sx1 - sx0) / (wx1 - wx0)
        self.sx0, self.sy0, self.wx0, self.wy_top = sx0, sy0, wx0, wy_top
        self.h = (wy_top - wy_bot) * self.k

    def p(self, x, y):
        return (self.sx0 + (x - self.wx0) * self.k, self.sy0 + (self.wy_top - y) * self.k)

    def ps(self, pts):
        return [self.p(x, y) for x, y in pts]


def spline(pts, n=14):
    """Catmull-Rom through the points (game coordinates), as a polyline."""
    if len(pts) < 3:
        return list(pts)
    ext = [pts[0]] + list(pts) + [pts[-1]]
    out = []
    for i in range(1, len(ext) - 2):
        p0, p1, p2, p3 = ext[i - 1], ext[i], ext[i + 1], ext[i + 2]
        for j in range(n):
            t = j / n
            out.append(tuple(0.5 * (2 * p1[k] + (-p0[k] + p2[k]) * t
                                    + (2 * p0[k] - 5 * p1[k] + 4 * p2[k] - p3[k]) * t * t
                                    + (-p0[k] + 3 * p1[k] - 3 * p2[k] + p3[k]) * t ** 3) for k in (0, 1)))
    out.append(pts[-1])
    return out


def head(c, a, b, col, size=16):
    ang = math.atan2(b[1] - a[1], b[0] - a[0])
    c.polygon([b, (b[0] - size * math.cos(ang - 0.42), b[1] - size * math.sin(ang - 0.42)),
               (b[0] - size * math.cos(ang + 0.42), b[1] - size * math.sin(ang + 0.42))], fill=col)


def path(c, wd, pts, col, width=4, dash=None, arrow=True, smooth=True):
    sp = wd.ps(spline(pts) if smooth else pts)
    c.line(sp, col, width, dash=dash)
    if arrow:
        head(c, sp[-3], sp[-1], col)
    return sp


def marker(c, xy, label, col, r=15):
    x, y = xy
    c.circle(x, y, r, fill=col, outline=DARK, width=2)
    c.text(x, y - 12, label, "P_b", DARK, "m")


def ko(c, xy, col=BAD, s=11):
    x, y = xy
    c.line([(x - s, y - s), (x + s, y + s)], col, 5)
    c.line([(x - s, y + s), (x + s, y - s)], col, 5)


def clip_right(poly, xmax):
    """The part of a polygon left of x = xmax (Sutherland-Hodgman against one vertical edge)."""
    out = []
    for i, (x0, y0) in enumerate(poly):
        x1, y1 = poly[(i + 1) % len(poly)]
        if x0 <= xmax:
            out.append((x0, y0))
        if (x0 <= xmax) != (x1 <= xmax):
            out.append((xmax, y0 + (xmax - x0) / (x1 - x0) * (y1 - y0)))
    return out


def stage(c, wd, names=True):
    c.polygon(wd.ps(RING), fill=STAGE, outline=LINE, width=2)
    for sx, name in ((-LEDGE_X, "near ledge"), (LEDGE_X, "far ledge")):
        x, y = wd.p(sx, 0)
        c.circle(x, y, 6, fill=ADV)
        if names:
            c.text(x, y - 34, name, "P_s", ADV, "m")


def panel_box(c, y0, h, title=None):
    c.rect(PAD, y0, W - PAD, y0 + h, fill=PANEL, outline=LINE, radius=8)
    if title:
        c.text(PAD + 16, y0 + 12, title, "P_h", FG)
        return y0 + 52
    return y0 + 12


def legend(c, y, rows, x=PAD + 16, width=W - 2 * PAD - 32):
    """Rows of (marker label, colour, text): a numbered dot, then the wrapped text."""
    for lab, col, text in rows:
        if lab == "x":
            ko(c, (x + 15, y + 13), col, 9)
        else:
            marker(c, (x + 15, y + 13), lab, col, 14)
        y = para(c, x + 42, y, text, width - 42, "P_s", FG) + 6
    return y


# ---------------------------------------------------------------------------------------------------
# GTE-HERO: inbox 06:36Z fusion-g4; edge-juggle/reference-crossings/hero.md §2, §3
# ---------------------------------------------------------------------------------------------------
def gte_hero():
    c = canvas()
    y = header(c, "Hero · the crossing under the stage",
               "Guard the Edge, Final Destination to scale. 23 traced attempts from off-stage starts.")
    wd = World(PAD + 14, y + 56, W - PAD - 14, -114, 106, 14, -124)
    panel_box(c, y, wd.h + 70)
    stage(c, wd)
    start = (-98.5, -27.5)  # (17 out, -27) and (20 out, -28) [hero.md §3]
    # Near side: double jump + Kaswoosh to the near ledge, as Level 9 already does [hero.md §2].
    path(c, wd, [start, (-103, -14), (-97, -3), (-86, 1.5)], MUTED, 3)
    # The crossing: the hold kills the sideways speed, the rise is held 11.8 under the flat part (top y -51.91),
    # the closest approach is (80.4, -82.0), the flights end at (77..104, -114..-117) [hero.md §1, §3].
    held = -51.91
    path(c, wd, [start, (-72, -41), (-42, -56), (-20, -62), (-8, -64), (4, -56), (14, held), (41.4, held),
                 (60, -62), (80.4, -82.0), (90, -100), (96, -115)], BAD, 4, arrow=False)
    ko(c, wd.p(96, -115.5))
    # The far ledge's grab box (drawn, not measured) and the 55.5 gap to the closest approach.
    box_bot = -82.0 + 55.5
    c.rect(*wd.p(74, 0), *wd.p(90, box_bot), outline=ADV, width=2)
    gx = 84.5
    c.line([wd.p(gx, -82.0), wd.p(gx, box_bot)], ADV, 3, dash=(8, 6))
    c.line([wd.p(80.4, -82.0), wd.p(gx, -82.0)], ADV, 2)
    c.circle(*wd.p(80.4, -82.0), 6, fill=BAD)
    c.text(wd.p(gx, 0)[0] - 10, wd.p(0, (box_bot - 82.0) / 2)[1] - 12, "55.5", "P_b", ADV, "r")
    # numbered markers
    sx, sy = wd.p(*start)
    c.circle(sx, sy, 8, fill=CPU)
    marker(c, (sx + 6, sy + 32), "1", CPU)
    marker(c, wd.p(-112, -6), "N", MUTED)
    marker(c, wd.p(-14, -74), "2", BAD)
    marker(c, wd.p(28, held - 9), "3", BAD)
    marker(c, wd.p(100, -82.0), "4", ADV)
    y = legend(c, y + wd.h + 84, [
        ("1", CPU, "Start: 17 to 20 out, 27 to 28 below the near ledge."),
        ("N", MUTED, "Near side: jump + full Kaswoosh. The game's own CPU (Level 9) already does it: 14 of 14; "
                     "Perfect 23 of 23."),
        ("2", BAD, "Crossing: double jump, then Kaswoosh under the stage."),
        ("3", BAD, "The stage's underside holds him 11.8 below it, so the rise is lost."),
        ("4", ADV, "Closest pass: 55.5 from the far ledge's box. An air dodge adds about 44: less than half of "
                   "what is missing."),
        ("x", BAD, "Leaves the far side 82 to 117 below the ledge: KO, 23 of 23."),
    ])
    y = options(c, y + 6, [
        ("1 · Ship Hero without it", [
            ("Near side = Level 9 quality; Perfect only picks the cheapest spell that reaches (saves MP). "
             "Optional: a Zoom-to-ledge mixup tied to the existing Hero Zoom drill.", MUTED)]),
        ("2 · Only from an on-stage jump", [
            ("A jump off the near lip (the public clip: about 60 more height, 20 closer). Not a recovery, so it is "
             "outside Guard the Edge.", MUTED)]),
        ("3 · You suggest a trick", [("A Hero trick we have not flown.", MUTED)]),
    ], 0)
    finish(c, y, "GTE-HERO")


# ---------------------------------------------------------------------------------------------------
# GTE-SONIC: inbox 09:02Z fusion-g2 (boots g2o-plsn-m1-xps-2, fg2-m-xps-6, fg2-n-yoga-7, fg2-r-yoga-8, fg2-s-yoga-9)
# ---------------------------------------------------------------------------------------------------
def gte_sonic():
    c = canvas()
    y = header(c, "Sonic · the crossing under the stage",
               "Guard the Edge, Final Destination to scale. 38 traced plays in 5 boots; no flight crossed.")
    wd = World(PAD + 14, y + 56, W - PAD - 14, -136, 108, 30, -104)
    panel_box(c, y, wd.h + 70)
    stage(c, wd)
    start = (-124, 19)  # Spin Dash from (44 out, +19 ... +24) [fg2-n-yoga-7, fg2-r-yoga-8]
    spring = (43.8, -80)  # Spring at x 43.4 to 44.2, 77 to 82 below [fg2-r-yoga-8]
    top = (79.9, -18.5)  # tops at (78.6 ... 81.1, -19.8 ... -17.2) [fg2-r-yoga-8]
    path(c, wd, [start, (-104, 0), (-84, -28), (-58, -56), (-26, -72), (8, -79), spring], CPU, 4, dash=(10, 7),
         arrow=False)
    # The Spring rides the far slope's underside outward, facing the ledge, and is not taken.
    path(c, wd, [spring, (43.9, -60), (45.5, -48), (55, -42), (64.4, -34.5), (72, -26.5), top], BAD, 4, arrow=False)
    path(c, wd, [top, (82, -24), (84, -45), (85, -70), (85.5, -88)], BAD, 3, dash=(8, 6), arrow=False)
    ko(c, wd.p(85.5, -90))
    c.circle(*wd.p(*top), 7, fill=BAD)
    # Facing away, outside the ledge: a Spring straight up, also not taken.
    fx = 98
    c.line([wd.p(fx, -64), wd.p(fx, -32)], BAD, 4)
    head(c, wd.p(fx, -40), wd.p(fx, -30), BAD)
    bx, by = wd.p(fx, -66)
    c.polygon([(bx + 4, by - 8), (bx + 20, by), (bx + 4, by + 8)], fill=BAD)  # facing away from the stage
    # The same spot, just inside the far ledge, takes other fighters' moves (1.2 to 2.2 inside it).
    lx, ly = wd.p(*top)
    c.circle(lx, ly, 22, outline=OURS, width=3)
    sx, sy = wd.p(*start)
    c.circle(sx, sy, 8, fill=CPU)
    marker(c, (sx, sy - 34), "1", CPU)
    marker(c, wd.p(32, -69), "2", BAD)
    marker(c, wd.p(58, -24), "3", BAD)
    marker(c, wd.p(fx + 1, -76), "4", BAD)
    marker(c, wd.p(60, -10), "5", OURS)
    c.line([wd.p(63.5, -12), (lx - 21, ly - 6)], OURS, 2)
    y = legend(c, y + wd.h + 84, [
        ("1", CPU, "Start: 44 out, 19 to 24 above the near ledge; Spin Dash and jump under the stage."),
        ("2", BAD, "Spring (his only rise that reaches ledge height), pressed under the far slope."),
        ("3", BAD, "He rides the slope up facing the far ledge and tops out 17.2 to 19.8 below it: "
                   "not caught, 0 of 8."),
        ("4", BAD, "Outside the ledge, facing away: not caught, 0 of 4."),
        ("5", OURS, "The same spot catches Lucario, PAC-MAN and Greninja (1.2 to 2.2 inside the ledge)."),
    ])
    y = options(c, y + 6, [
        ("1 · Ship Sonic without it", [("His near side stays equal to Level 9.", MUTED)]),
        ("2 · Only from a stage or ledge start", [
            ("As in your clip (it starts on the stage, with a ground jump and both jumps). Outside Guard the "
             "Edge's off-stage situation.", MUTED)]),
        ("3 · You suggest a trick", [
            ("Still untried: a Homing Attack on you near the far ledge; inputs late in the dive.", MUTED)]),
    ], 0)
    finish(c, y, "GTE-SONIC")


# ---------------------------------------------------------------------------------------------------
# GTE-BAYO: inbox 10:35Z fusion-g1, 10:37Z edge-juggle-sol, 10:39Z edge-recovery-fable;
# edge-juggle/reference-crossings/g1/bayonetta.md (worked trace bayonetta-bh1-hang-cross, attempt 1)
# ---------------------------------------------------------------------------------------------------
def gte_bayo():
    c = canvas()
    y = header(c, "Bayonetta · a start hanging on the ledge?",
               "Guard the Edge, Final Destination to scale. Her crossing works from a hang, never from the air.")
    wd = World(PAD + 14, y + 56, W - PAD - 14, -112, 100, 14, -162)
    panel_box(c, y, wd.h + 70)
    stage(c, wd)
    ko_y = -154.0  # alive at -154.00, dead by -154.36 [bayonetta.md]
    c.line([wd.p(-112, ko_y), wd.p(100, ko_y)], BAD, 2, dash=(10, 8))
    c.text(wd.p(98, ko_y)[0], wd.p(0, ko_y)[1] - 30, "KO line", "P_s", BAD, "r")
    # Worked trace waypoints [bayonetta.md, attempt 1, jump on frame 132].
    hang = (-82.35, -23.17)
    low = (-16.08, -138.43)
    route = [hang, (-60, -55), (-36, -98), (-17.10, -136.66), low, (-9, -112), (3.81, -84.94), (33, -74.8),
             (62.77, -76.10), (74, -52), (81.17, -35.04), (84.5, -14), (85.56, -7.98), (85.9, -13.0),
             (82.36, -23.17)]
    path(c, wd, route, OURS, 4)
    # A release in the air at (20 out, -28) drifts in at 1.74 of height per unit and goes under the KO line.
    air = (-99.99, -28.0)
    air_end = (air[0] + (air[1] - ko_y) / 1.74, ko_y)
    path(c, wd, [air, air_end], BAD, 3, dash=(9, 7), arrow=False, smooth=False)
    ko(c, wd.p(*air_end))
    for pt in (hang, (82.36, -23.17)):
        c.circle(*wd.p(*pt), 7, fill=OURS)
    c.circle(*wd.p(*air), 7, fill=BAD)
    marker(c, wd.p(-72, -14), "1", OURS)
    marker(c, wd.p(-2, -140), "2", OURS)
    marker(c, wd.p(33, -64), "3", OURS)
    marker(c, wd.p(70, -38), "4", OURS)
    marker(c, wd.p(93, -32), "5", OURS)
    marker(c, wd.p(-104, -16), "A", BAD)
    y = legend(c, y + wd.h + 84, [
        ("1", OURS, "Hanging on the near ledge, she lets go."),
        ("2", OURS, "Jump + Witch Twist near the bottom (133 to 142 below the ledge)."),
        ("3", OURS, "After Burner Kick across, under the stage."),
        ("4", OURS, "Jump + Witch Twist at the far ledge."),
        ("5", OURS, "The far ledge catches her: 48 of 48 (Final Destination 28, Battlefield 10, Waiting Room 10)."),
        ("A", BAD, "Released in the air instead (20 out, 28 below): she must first fall 25 under the near ledge, "
                   "and runs out of height. KO, 9 of 9."),
    ])
    y = options(c, y + 6, [
        ("1 · No hang start (as for G&W)", [
            ("Bayonetta ships without the crossing; her near side = Level 9. A substitute mixup if you name one "
             "(for Mr. Game & Watch you chose the Oil Panic stall).", MUTED)]),
        ("2 · Add the hang start", [
            ("A new wizard start: CPU hanging on the near ledge; the first result after she lets go is scored, "
             "in both modes. Proven 48 of 48, review PASS: ready if you want it.", MUTED)]),
    ], 0)
    finish(c, y, "GTE-BAYO")


# ---------------------------------------------------------------------------------------------------
# BRIDGE-40MS: bridge-menu-introspection/OPERATOR-DECISION-40MS.md; inbox 09:55Z bridge-opus and
# review-bridge-astra, 10:44Z, 10:45Z, 11:01Z, 11:02Z, 11:10Z
# ---------------------------------------------------------------------------------------------------
def bridge_40ms():
    c = canvas()
    y = header(c, "Bridge · one unreadable menu frame",
               "The hardware-test bridge must turn itself Off whenever it cannot prove the game is on an "
               "offline screen.")
    box_h = 500
    top = panel_box(c, y, box_h, "The first frame of a new menu page")
    t0x, k = 196, 13.0  # screen x of 0 ms, px per ms
    frame_ms = 1000 / 60

    def X(ms):
        return t0x + ms * k

    lanes = top + 34
    rows = {"game": lanes, "worker": lanes + 70, "today": lanes + 150, "window": lanes + 250}
    for name, ry in rows.items():
        c.text(PAD + 16, ry + 6, name, "P_b", MUTED)
    for i in range(3):
        fx = X(i * frame_ms)
        c.line([(fx, top + 26), (fx, rows["window"] + 40)], GRID, 2, dash=(6, 6))
        c.text(fx, top - 6, f"frame {i + 1}", "P_s", MUTED, "m")
    # game: the page appears just before frame 1
    d.box(c, X(-8), rows["game"], X(8), rows["game"] + 38, ["new page"], col=LINE, fill=TILE_HI, font="P_s")
    # worker: learns the page 3 to 11 ms later (review-bridge-astra 09:55Z correction; Load 12: 9 ms, Load 13: 5 ms)
    c.rect(X(3), rows["worker"] + 4, X(11), rows["worker"] + 34, fill=TILE_HI, outline=MUTED, width=2, radius=4)
    c.text(X(11) + 10, rows["worker"] + 6, "learns it 3 to 11 ms later", "P_s", MUTED)
    # today: the one refused read is Unknown, and Unknown turns the bridge Off
    c.rect(X(0) - 6, rows["today"], X(0) + 70, rows["today"] + 40, fill=(60, 30, 26), outline=BAD, width=3, radius=6)
    c.text(X(0) + 32, rows["today"] + 7, "Off", "P_h", BAD, "m")
    para(c, X(0) + 86, rows["today"] + 2, "one unreadable frame = Unknown = Off; the menu row fails",
         W - PAD - 16 - X(0) - 86, "P_s", BAD)
    # with the window: no observation for at most 40 ms (MAX_MAIN_AGE_MS), no bridge inputs; the next live read decides
    wy = rows["window"]
    c.rect(X(0), wy, X(40), wy + 40, fill=(58, 50, 30), outline=ADV, width=2, radius=6)
    c.text(X(20), wy + 8, "no observation, no inputs", "P_s", ADV, "m")
    c.line([(X(40), wy - 8), (X(40), wy + 48)], ADV, 3)
    c.text(X(40), wy - 36, "40 ms", "P_b", ADV, "m")
    f2 = X(frame_ms)
    ty = wy + 62
    c.line([(f2, wy + 40), (f2, ty - 4)], MUTED, 2)
    c.text(X(0), ty, "the next real read (frame 2):", "P_s", FG)
    c.text(X(0), ty + 30, "Offline: stays On", "P_b", OURS)
    c.text(X(0), ty + 60, "Online, Popup, Unknown: Off at once", "P_b", BAD)
    assert X(0) + c.textlength("Online, Popup, Unknown: Off at once", "P_b") < W - PAD - 12
    y = top + box_h - 52 + 12
    y = panel_box(c, y, 172, "Still Off at once, as today")
    y = d.lines(c, PAD + 16, y - 4, [
        "a bad pointer, a wrong shape or a replaced object",
        "Online, Popup or Unknown on the next real read",
        "no real read within the 40 ms",
    ], font="P_s", col=FG, step=STEP) + 20
    y = options(c, y, [
        ("1 · Approve the 40 ms window", [
            ("Safe: the bridge sends nothing in the window; 40 ms is the age limit it already allows for menu "
             "readings; real negatives still turn it Off at once.", FG),
            ("Cost: a real move to an online screen is noticed up to 40 ms late, while the game alone drives the "
             "screen.", ADV),
            ("Built on a separate branch, never loaded. Next: the reviewer's check, the gate, a counted run.",
             MUTED)]),
        ("2 · Reject", [
            ("The bridge ships without reliable menu rows, or the work goes on with another design. A faster "
             "worker narrows the gap but cannot close it.", MUTED)]),
    ], 0)
    finish(c, y, "BRIDGE-40MS")


# ---------------------------------------------------------------------------------------------------
# GTE-LUCARIO: inbox 09:58Z fusion-g2; edge-juggle/reference-crossings/g2/lucario.md
# ---------------------------------------------------------------------------------------------------
def gte_lucario():
    c = canvas()
    y = header(c, "Lucario · from what damage?",
               "Guard the Edge. His crossing under the stage (Extreme Speed) on Final Destination, Battlefield "
               "and the Waiting Room.")
    top = panel_box(c, y, 214, "His rush grows with his own damage")
    x0, k = PAD + 150, 4.8  # px per unit
    for i, (pct, length) in enumerate(((0, 67.3), (65, 83.6), (130, 100.3))):
        ry = top + 8 + i * 48
        col = OURS if pct == 130 else (ADV if pct == 65 else BAD)
        c.text(x0 - 22, ry, f"{pct} %", "P_b", FG, "r")
        c.line([(x0, ry + 14), (x0 + length * k, ry + 14)], col, 8)
        head(c, (x0, ry + 14), (x0 + length * k + 12, ry + 14), col, 20)
        c.text(x0 + length * k + 22, ry + 1, f"{length}", "P_b", col)
    y = top + 214 - 52 + 14
    # What backs the crossing at each damage; the dot says which option first offers that row.
    rows = [("1", OURS, "100, 130 %", "two tables (64 and 85 cells); flown on three stages: 108 of 108 inside starts "
                                      "reached the far ledge, outside 0 of 34"),
            ("2", ADV, "65 %", "no table; 6 starts planned live: 30 of 30 on Final Destination"),
            (None, MUTED, "20 to 50 %", "a plan from only 1 to 6 of 56 sampled starts; in no option"),
            ("3", BAD, "0 %", "one line, 2 of 2, caught on the rush's last step: no margin")]
    tx = PAD + 232
    tw = W - PAD - 16 - tx
    heights = [len(wrap(c, text, "P_s", tw)) * STEP + 16 for _, _, _, text in rows]
    top = panel_box(c, y, 52 + sum(heights) + 4, "What backs it at each damage")
    yy = top
    for (lab, col, dmg, text), h in zip(rows, heights):
        if lab:
            marker(c, (PAD + 32, yy + 13), lab, col, 14)
        c.text(PAD + 60, yy, dmg, "P_b", col)
        para(c, tx, yy, text, tw, "P_s", FG if lab else MUTED)
        yy += h
        c.line([(PAD + 16, yy - 9), (W - PAD - 16, yy - 9)], GRID, 1)
    y = yy + 18
    y = options(c, y, [
        ("1 · From 100 %, as tabled", [
            ("Only where a table backs it: the first row above.", MUTED)]),
        ("2 · Also from 65 %, planned live", [
            ("No table; the plan is solved at the live damage. Pick this if the mixup should appear earlier in a "
             "stock.", MUTED)]),
        ("3 · The 0 % line as flown", [
            ("One line with no margin: it was caught on the rush's last step.", MUTED)]),
    ], 0)
    finish(c, y, "GTE-LUCARIO")


# ---------------------------------------------------------------------------------------------------
# GTE-NESS: inbox 11:36Z fusion-g2 (boots fg2-g1-yoga-12, fg2-g1-yoga-13; the facing case fg2-m-xps-6).
# The path is one traced attempt: fg2-g1-yoga-12/frames-ness.tsv, ness-r1-4out-y-26-d80-k39, attempt 1.
# ---------------------------------------------------------------------------------------------------
def gte_ness():
    c = canvas()
    y = header(c, "Ness · the crossing under the stage",
               "Guard the Edge, Final Destination to scale. 24 traced launches from under the stage, two boots.")
    wd = World(PAD + 14, y + 56, W - PAD - 14, -110, 110, 14, -128)
    panel_box(c, y, wd.h + 70)
    stage(c, wd)
    start = (-84.0, -26.0)
    low = (-7.0, -121.6)  # the double jump
    ball = (38.4, -83.0)  # PK Thunder: he waits while the ball is steered, then it launches him
    path(c, wd, [start, (-64.4, -47.0), (-40.2, -78.4), (-16.1, -109.8), low, (11.4, -106.8), (23.6, -86.7),
                 (34.6, -76.6), (38.4, -76.6), ball], CPU, 4, dash=(10, 7), arrow=False)
    top = (81.3, -20.6)
    path(c, wd, [ball, (45.7, -71.1), (63.6, -42.1), (75.4, -25.2), top], BAD, 5, arrow=False, smooth=False)
    path(c, wd, [top, (82.0, -26.9), (82.0, -34.9), (91.5, -52.3), (106.6, -72.0)], BAD, 3, dash=(8, 6),
         arrow=False)
    ko(c, wd.p(106.6, -72.0))
    rx, ry = wd.p(80.4, -19.5)
    c.circle(rx, ry, 24, outline=BAD, width=4)
    sx, sy = wd.p(*start)
    c.circle(sx, sy, 8, fill=CPU)
    marker(c, (sx - 22, sy + 24), "1", CPU)
    marker(c, wd.p(-7.0, -112), "2", CPU)
    marker(c, wd.p(28, -97), "3", CPU)
    mx, my = wd.p(40, -50)
    c.line([(mx, my), (rx - 20, ry + 14)], BAD, 2)
    marker(c, (mx, my), "4", BAD)
    c.text(mx - 22, my - 13, "not taken: 24 of 24", "P_b", BAD, "r")
    y = legend(c, y + wd.h + 84, [
        ("1", CPU, "Start: the pocket next to the near ledge (4 to 8 out, 24 to 30 below), the only place he reaches "
                   "the launch point from."),
        ("2", CPU, "Drift under the stage, double jump."),
        ("3", CPU, "PK Thunder: the ball launches him (PK Thunder 2) up under the far slope."),
        ("4", BAD, "He passes 0.3 to 4.2 inside the far ledge, 18.1 to 22.3 below it, clear of the slope: not "
                   "taken, 24 of 24."),
        ("x", BAD, "Past the ledge the launch tops out 15 to 17 below and falls back untaken: KO."),
    ])
    y = para(c, PAD + 16, y + 2, "The lane's reading, not a measured rule: he flies outward facing away, so the "
             "ledge would have to take him from behind. Lucario's, PAC-MAN's and Greninja's moves are taken that "
             "way; PK Thunder 2 and Sonic's Spring are not.", W - 2 * PAD - 32, "P_s", MUTED) + 10
    # Inset: facing the stage, the ledge does take PK Thunder 2 (launched from outside the near ledge, fg2-m-xps-6).
    iw = World(PAD + 16, y + 52 + 30, PAD + 16 + 230, -114, -56, 6, -42)
    inset_h = 52 + 30 + iw.h + 18
    top_y = panel_box(c, y, inset_h, "Facing the stage, it is taken")
    c.polygon(iw.ps(clip_right(RING, -56)), fill=STAGE, outline=LINE, width=2)
    lx, ly = iw.p(-LEDGE_X, 0)
    c.circle(lx, ly, 6, fill=ADV)
    c.text(lx, ly - 30, "near ledge", "P_s", ADV, "m")
    a0, a1 = iw.p(-110, -38), iw.p(-86, -18.8)
    ln = math.hypot(a1[0] - a0[0], a1[1] - a0[1])
    tip = (a1[0] - (a1[0] - a0[0]) * 22 / ln, a1[1] - (a1[1] - a0[1]) * 22 / ln)
    c.line([a0, tip], OURS, 5)
    head(c, a0, tip, OURS, 20)
    c.circle(a0[0], a0[1], 7, fill=OURS)
    c.circle(*a1, 18, outline=OURS, width=4)
    para(c, PAD + 300, top_y + 6, "Launched from outside the ledge, facing the stage, PK Thunder 2 is taken 18.8 "
         "below it: 2 of 2.", W - PAD - 16 - (PAD + 300), "P_s", FG)
    y = options(c, y + inset_h + 10, [
        ("1 · Ship Ness without it", [
            ("His near side = Level 9, plus the PK Thunder path around you that the design asks for (being proven "
             "now).", MUTED)]),
        ("2 · Only from a stage or ledge start", [
            ("Two launches, the second from outside the far ledge, as in the community video. Outside Guard the "
             "Edge's off-stage situation.", MUTED)]),
        ("3 · You suggest a trick", [("A Ness trick not yet flown.", MUTED)]),
    ], 0)
    finish(c, y, "GTE-NESS")


ALL = [gte_hero, gte_sonic, gte_bayo, bridge_40ms, gte_lucario, gte_ness]


def main():
    only = set(sys.argv[1:])
    for fn in ALL:
        if not only or fn.__name__ in only:
            fn()
            print("drew", fn.__name__)


if __name__ == "__main__":
    main()
