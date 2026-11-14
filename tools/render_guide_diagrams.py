#!/usr/bin/python3 -I
"""Draw the guide diagrams in docs/diagrams/ for the readability pass (cycle numbers, frame strip timeline, ...).

Same look as tools/render_recording_diagrams.py and tools/render_decisions.py (their canvas, colours and helpers are
reused). Every picture is drawn from the facts in the page it sits on; no game-derived file is used, so these are
drawings, not Eden footage.

usage: /home/intpa/smash-training-lobby/tmp-integration/venv-pillow/bin/python -I tools/render_guide_diagrams.py [name ...]
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import render_decisions as d  # noqa: E402
from render_recording_diagrams import (  # noqa: E402,F401
    ADV, BAD, CPU, FG, GREEN, LINE, MACRO_C, MUTED, P1, PANEL, TILE, TILE_HI, W, Canvas, bracket, title,
)


def cycle_numbers():
    c = Canvas(W, 360)
    title(c, "The numbers in Character Cycles", "each press of a cycle chord loads the next number; the fighters are an example")
    rows = [("Player 1", P1, "Cycle Player: ZL + D-pad Right",
             [("0", "Mario", "started as"), ("1", "Link", ""), ("2", "Kirby", ""), ("-", "Mii Gunner", "greyed: skipped"),
              ("3", "Samus", "")]),
            ("CPU", CPU, "Cycle CPU: ZL + D-pad Left",
             [("0", "Pikachu", "started as"), ("1", "Bowser", ""), ("2", "Peach", "")])]
    y = 76
    for who, col, chord, tiles in rows:
        c.text(24, y + 4, who, "h", col)
        c.text(24, y + 32, chord, "xxs", MUTED)
        x = 280
        for n, name, note in tiles:
            greyed = n == "-"
            c.rect(x, y, x + 140, y + 78, fill=TILE, outline=LINE if greyed else col, width=2, radius=8)
            if not greyed:
                c.text(x + 14, y + 6, n, "hh", col)
            c.text(x + 14, y + 42, name, "s", MUTED if greyed else FG)
            if note:
                c.text(x + 14, y + 60, note, "xxs", MUTED)
            x += 170
        xs = [280 + 170 * i for i, tile in enumerate(tiles) if tile[0] != "-"]
        for a, b in zip(xs, xs[1:]):
            if b - a == 170:
                d.arrow(c, (a + 142, y + 39), (b - 2, y + 39), col)
            else:
                c.line([(a + 70, y - 2), (a + 70, y - 12), (b + 70, y - 12)], col, 2)
                d.arrow(c, (b + 70, y - 12), (b + 70, y - 2), col)
        first_mid, last_mid = 280 + 70, 280 + (len(tiles) - 1) * 170 + 70
        c.line([(last_mid, y + 80), (last_mid, y + 104), (first_mid, y + 104)], col, 2, dash=(6, 4))
        d.arrow(c, (first_mid, y + 104), (first_mid, y + 82), col)
        c.text(last_mid - 10, y + 108, "after the last, back to 0", "xxs", MUTED, "r")
        y += 150
    c.save("cycle-numbers")


def strip_timeline():
    c = Canvas(W, 470)
    title(c, "One exchange on the frame strip", "frames run left to right; the numbers are an example")
    k = 12  # pixels per frame
    ax, ay = 60, 214
    hit0, you, free = 100, 300, 560
    lim = free + 20 * k
    c.line([(ax, ay), (1150, ay)], LINE, 3)
    # the CPU's hitstun, the 20-frame band, and what comes after
    c.rect(hit0, ay - 30, free, ay - 6, fill=(60, 44, 96), outline=None)
    c.text((hit0 + free) / 2, ay - 24, "the CPU is in hitstun", "xs", FG, "m")
    c.rect(free, ay - 30, lim, ay - 6, fill=(86, 70, 30), outline=None)
    c.text((free + lim) / 2, ay - 24, "still the same exchange", "xs", FG, "m")
    c.rect(lim, ay - 30, 1140, ay - 6, fill=(40, 44, 54), outline=None)
    c.text((lim + 1140) / 2, ay - 24, "later: a new exchange", "xs", MUTED, "m")
    # the marks on the axis
    marks = [(hit0, "last hit", "frame 0", FG), (you, "you can act", "your first free frame", P1),
             (free, "the CPU can act", "its first free frame", CPU), (lim, "+20 frames", "after the CPU can act", ADV)]
    for x, name, sub, col in marks:
        c.line([(x, ay - 40), (x, ay + 12)], col, 3)
        c.text(x, ay + 18, name, "s", col, "m")
        c.text(x, ay + 40, sub, "xxs", MUTED, "m")
    # ADVANTAGE bracket
    bracket(c, you, free, ay + 76, GREEN, up=False)
    c.text((you + free) / 2, ay + 88, "ADVANTAGE: the CPU's first free frame minus yours (here plus)", "xs", GREEN, "m")
    # the three follow-up hits
    hits = [(free - 4 * k, "TRUE by 4", "the CPU never got a free frame before the hit (shown as -4)", (111, 195, 154)),
            (free + 9 * k, "DROP +9", "the CPU could act for 9 frames first", BAD),
            (lim + 8 * k, "not a drop", "a new exchange; ADVANTAGE waits", MUTED)]
    for x, name, sub, col in hits:
        d.arrow(c, (x, 74), (x, ay - 42), col, 3)
        c.text(x, 50, name, "s", col, "m")
    y = 330
    for x, name, sub, col in hits:
        c.text(24, y, name + ": " + sub, "xs", col)
        y += 22
    c.text(24, y + 8, "A follow-up is a later hit from a different action, on hit, within 20 frames of the CPU being free.", "xs", MUTED)
    c.text(24, y + 30, "Hits inside one multi-hit move are not follow-ups, and neither is a hit on shield.", "xs", MUTED)
    c.save("frame-strip-timeline")

def cpu_action_flow():
    c = Canvas(W, 416)
    title(c, "How the CPU picks its action", "when a situation starts")
    d.box(c, 24, 70, 220, 140, ["A situation starts", "(a hit, a tech,", "a landing ...)"], col=LINE, font="xs")
    d.box(c, 270, 70, 480, 140, ["Does it have an", "override with", "actions?"], col=ADV, font="xs")
    d.box(c, 540, 70, 800, 140, ["Pick one from the override", "(it works even with its", "trigger unticked)"], col=GREEN, font="xs")
    d.box(c, 270, 210, 480, 280, ["Is its trigger ticked", "in Mash Triggers?"], col=ADV, font="xs")
    d.box(c, 540, 210, 800, 280, ["Pick one from Mash"], col=GREEN, font="xs")
    d.box(c, 270, 330, 480, 380, ["Nothing: no reaction"], col=LINE, font="xs")
    d.box(c, 880, 70, 1176, 140, ["The first action", "pressed as soon as", "the game lets it"], col=MACRO_C, font="xs")
    d.box(c, 880, 210, 1176, 280, ["The Followup", "picked at the same moment;", "comes next, as soon as it can"], col=MACRO_C, font="xs")
    d.arrow(c, (220, 105), (270, 105), MUTED)
    d.arrow(c, (480, 105), (540, 105), GREEN)
    c.text(510, 82, "yes", "xxs", GREEN, "m")
    d.arrow(c, (375, 140), (375, 210), MUTED)
    c.text(388, 168, "no", "xxs", BAD)
    d.arrow(c, (480, 245), (540, 245), GREEN)
    c.text(510, 222, "yes", "xxs", GREEN, "m")
    d.arrow(c, (375, 280), (375, 330), MUTED)
    c.text(388, 300, "no", "xxs", BAD)
    d.arrow(c, (800, 105), (880, 105), MUTED)
    c.line([(800, 245), (840, 245), (840, 118)], MUTED, 2)
    d.arrow(c, (840, 118), (880, 118), MUTED)
    d.arrow(c, (1028, 140), (1028, 210), MUTED)
    c.text(520, 310, "With several actions ticked, the pick is random and each has the same chance.", "xs", MUTED)
    c.text(520, 334, "A new situation replaces anything still waiting.", "xs", MUTED)
    c.text(520, 358, "After a throw the CPU always does its Mash action.", "xs", MUTED)
    c.text(520, 382, "Shieldstun is the one override that needs its trigger ticked.", "xs", MUTED)
    c.save("cpu-action-flow")


def chip_weights():
    c = Canvas(W, 250)
    title(c, "Weights on the direction chips", "each A adds one weight, from x1 to x5; a sixth press clears the chip")
    c.text(24, 62, "Up x3, In x1", "h", FG)
    x = 24
    for i, (name, col) in enumerate((("Up", (232, 120, 96)), ("Up", (232, 120, 96)), ("Up", (232, 120, 96)), ("In", CPU))):
        c.rect(x, 100, x + 130, 150, fill=TILE, outline=col, width=3, radius=6)
        c.text(x + 65, 114, name, "h", col, "m")
        x += 140
    c.text(600, 108, "The CPU picks Up three times out of four,", "s", FG)
    c.text(600, 132, "In once out of four.", "s", FG)
    c.text(24, 176, "Y clears the focused chip. ZL selects every chip or clears them all.", "xs", MUTED)
    c.text(24, 198, "The help line lists the picks, such as \"Selected: Up x3, In x1\". Shield Tilt takes one chip only.", "xs", MUTED)
    c.save("chip-weights")

def mii_waiting_room():
    c = Canvas(W, 250)
    title(c, "A Mii edit and a match in a waiting room")
    d.box(c, 24, 70, 360, 170, ["In the Mii editor", "(Add New...) in a", "waiting room"], col=MACRO_C, font="xs")
    d.box(c, 430, 70, 790, 170, ["A match starts", "The editor closes. Your", "unfinished Mii is kept, not saved."], col=ADV, font="xs")
    d.box(c, 860, 70, 1176, 170, ["Back in the same room", "The editor comes back", "as you left it."], col=GREEN, font="xs")
    d.arrow(c, (360, 120), (430, 120), MUTED)
    d.arrow(c, (790, 120), (860, 120), MUTED)
    c.text(24, 196, "Your practice Mii changes in the room. The match uses the fighter you picked for it.", "xs", MUTED)
    c.save("mii-waiting-room")

def waiting_room_match():
    c = Canvas(W, 330)
    title(c, "Practice in a waiting room, and the match", "nothing of the practice is carried into the match")
    cols = [("Before", MUTED, ["You pick a fighter", "for the match."]),
            ("In the room", MACRO_C, ["Practice: the two practice", "fighters, your tools,", "game speed, the CPU's level."]),
            ("The match starts", ADV, ["Speed is back to x1.", "The level isn't applied.", "You play the fighter you", "picked for it. An open", "menu closes; its changes", "are not kept."]),
            ("Same room again", GREEN, ["Your practice fighters are", "put back (Player 1's only", "with Player Swapping On),", "and Kirby's hat."])]
    x = 24
    w = 276
    for i, (head, col, lines) in enumerate(cols):
        c.rect(x, 64, x + w, 250, fill=TILE, outline=col, width=2, radius=8)
        c.text(x + 14, 76, head, "h", col)
        for j, ln in enumerate(lines):
            c.text(x + 14, 116 + j * 22, ln, "xs", FG)
        if i < len(cols) - 1:
            d.arrow(c, (x + w + 2, 157), (x + w + 18, 157), MUTED)
        x += w + 20
    c.text(24, 280, "A new room starts from the fighters you picked for the match.", "xs", MUTED)
    c.save("waiting-room-match")

ALL = {"waiting_room_match": waiting_room_match, "mii_waiting_room": mii_waiting_room, "cpu_action_flow": cpu_action_flow, "chip_weights": chip_weights, "cycle_numbers": cycle_numbers, "strip_timeline": strip_timeline}

if __name__ == "__main__":
    for name in (sys.argv[1:] or list(ALL)):
        ALL[name]()
        print("drew", name)
