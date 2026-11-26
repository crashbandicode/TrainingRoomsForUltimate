#!/usr/bin/python3 -I
"""Draw the five scrubbing-moves decision pictures for docs/DECISIONS.md.

Every picture is drawn from the design text; no game-derived file is used.

usage: /home/intpa/smash-training-lobby/tmp-integration/venv-pillow/bin/python -I tools/render_scrub_moves.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import render_decisions as d  # noqa: E402

OUT = Path(__file__).resolve().parent.parent / "docs" / "decisions" / "scrub-moves"
W = 1200


class Canvas(d.DCanvas):
    def save(self, name, height):
        OUT.mkdir(parents=True, exist_ok=True)
        self.img.crop((0, 0, self.img.width, height)).convert("RGB").save(OUT / f"{name}.png", optimize=True)


def wrap(c, text, width, font="xs"):
    lines, line = [], ""
    for word in text.split():
        candidate = f"{line} {word}" if line else word
        if c.textlength(candidate, font) <= width:
            line = candidate
        else:
            if not line or c.textlength(word, font) > width:
                raise ValueError(f"text does not fit: {word}")
            lines.append(line)
            line = word
    if line:
        lines.append(line)
    return lines


def paragraph(c, x, y, text, width, font="xs", col=d.MUTED, step=None):
    lines = wrap(c, text, width, font)
    step = step or d.m.FONTS[font][1] + 6
    d.lines(c, x, y, lines, font=font, col=col, step=step)
    return y + len(lines) * step


def boxed(c, x0, y0, x1, y1, text, font="xs", col=d.LINE, fill=d.TILE):
    lines = wrap(c, text, x1 - x0 - 18, font)
    if (d.m.FONTS[font][1] + 5) * len(lines) > y1 - y0 - 8:
        raise ValueError(f"box too short: {text}")
    d.box(c, x0, y0, x1, y1, lines, col=col, fill=fill, font=font)


def vertical_flow(c, x0, x1, y, texts, height=66, gap=24):
    for i, text in enumerate(texts):
        boxed(c, x0, y, x1, y + height, text, font="s")
        if i + 1 < len(texts):
            d.arrow(c, ((x0 + x1) / 2, y + height + 3), ((x0 + x1) / 2, y + height + gap - 3))
        y += height + gap
    return y - gap


def title(c, text):
    c.text(20, 16, text, "hh", d.FG)


def sm1():
    c = Canvas(W, 990)
    title(c, "SM1 · What a shown frame draws")
    c.rect(16, 58, 1184, 128, fill=d.PANEL, outline=d.LINE, radius=6)
    paragraph(c, 30, 72, "Install 6: the editor stops both fighters whole, so each keeps the pose it had when the editor took it. The scrub only moves that frozen body.", 1135)
    ay = d.panel(c, 16, 145, 594, 735, "A · The animation and its frame", rec=True, tag_below=True)
    by = d.panel(c, 606, 145, 1184, 735, "B · Force the status too", tag_below=True)
    aend = vertical_flow(c, 32, 578, ay + 8, [
        "the row for this frame: point · facing · animation · frame",
        "the fighter is held, its move scripts asleep",
        "set the animation at that frame, speed 0",
        "the fighter stands in the take's pose",
    ])
    y = aend + 24
    for text in [
        "No status change: no script runs, so no hitbox, effect, sound or article.",
        "Kept by reading the animation and frame back every frame.",
        "Only on the recorded fighter, and only an animation that fighter has.",
    ]:
        y = paragraph(c, 34, y, text, 540) + 12
    bend = vertical_flow(c, 622, 1168, by + 8, [
        "set the status as well",
        "the game runs that status's own code",
        "articles, effects and movement start",
    ])
    y = bend + 24
    for text in ["Shows more (a thrown item, a drawn weapon).", "Each side effect has to be found and stopped."]:
        y = paragraph(c, 624, y, text, 540) + 12
    paragraph(c, 24, 765, "On release: a plain animation back first (wait or fall), then the hold's own restore, then the scripts wake.", 1150)
    c.save("scrub-moves-SM1-shown-frame", 805)


def row_cells(c, y, fields):
    x = 30
    for field in fields:
        width = c.textlength(field, "tiny") + 4
        highlighted = field in ("animation", "frame")
        c.rect(x, y, x + width, y + 42, fill=d.TILE_HI if highlighted else d.TILE,
               outline=d.OURS if highlighted else d.LINE, width=1)
        c.text(x + width / 2, y + 13, field, "tiny", d.FG, "m")
        x += width
    if x > 425:
        raise ValueError("file row overflows its panel")


def sm2():
    c = Canvas(W, 1190)
    title(c, "SM2 · How a positions file gets its moves")
    d.panel(c, 16, 65, 432, 605, "A file's row")
    c.text(30, 138, "version 1 · 36 bytes a row", "s", d.FG)
    fields = ["fighter", "costume", "facing", "situation", "status", "speed", "point"]
    row_cells(c, 174, fields)
    c.text(30, 262, "version 2 · 48 bytes a row", "s", d.FG)
    row_cells(c, 298, fields + ["animation", "frame"])
    d.lines(c, 30, 388, ["Cells, left to right:", "fighter · costume · facing", "situation · status · speed",
                        "point · animation · frame"], font="s", col=d.FG, step=26)
    paragraph(c, 30, 520, "Version 1 still loads: positions only, and the line: Play it once to see the moves while scrubbing.", 380, "s")
    edges = [444, 636, 828, 1184]
    headers = ["The file today", "One eligible playback gives", "Afterwards"]
    for i, text in enumerate(headers):
        boxed(c, edges[i], 78, edges[i + 1], 136, text, font="xs", fill=d.TILE_HI)
    rows = [
        ("none, or one that does not load", "any rows", "version 2, generated", 90),
        ("version 1, generated", "a run at least as long", "replaced: version 2, generated", 90),
        ("version 1, generated", "a shorter run", "kept", 80),
        ("version 1, recorded with the take", "a run at least as long", None, 250),
        ("version 1, recorded with the take", "a shorter run, a stopped preview, a failed write", "unchanged", 110),
        ("version 2 with moves", "any run", "recorded: never replaced · generated: only by a longer run", 110),
    ]
    y = 144
    for left, middle, right, height in rows:
        boxed(c, edges[0], y, edges[1], y + height, left)
        boxed(c, edges[1], y, edges[2], y + height, middle)
        if right is None:
            d.box(c, edges[2], y, edges[3], y + height, [])
            x0, x1 = edges[2] + 8, edges[3] - 8
            boxed(c, x0, y + 8, x1, y + 102, "A · its own rows kept; the run's animations added where fighter and status agree", col=d.OURS)
            d.tag(c, x1 - 104, y + 110, "recommended", d.OURS)
            boxed(c, x0, y + 136, x1, y + 185, "B · replaced by the playback's rows")
            boxed(c, x0, y + 192, x1, y + 241, "C · left as it is")
        else:
            boxed(c, edges[2], y, edges[3], y + height, right)
        y += height + 7
    paragraph(c, 24, 940, "The file says where its moves came from: none, the take, or a playback. A playback's moves are never called recorded.", 1150)
    c.save("scrub-moves-SM2-file-upgrade", 980)


def sm3():
    c = Canvas(W, 845)
    title(c, "SM3 · Recorded fighter or the one you are on")
    paragraph(c, 24, 54, "Default: the fighter you are on. On a saved macro and on a capture alike.", 1145)
    for x0, x1, role, yours, take, notice in [
        (16, 594, "Player 1", "Mario (yours)", "Link (the take's)",
         "on another fighter: positions only, and the strip reads: Moves show on Link. ZL + L switches Player 1."),
        (606, 1184, "CPU", "Fox (yours)", "Marth (the take's)",
         "on another fighter: positions only, and the strip reads: Moves show on Marth. ZL + R switches the CPU."),
    ]:
        d.panel(c, x0, 92, x1, 320, role)
        boxed(c, x0 + 16, 150, x0 + 234, 210, yours, font="s")
        boxed(c, x1 - 234, 150, x1 - 16, 210, take, font="s")
        d.arrow(c, (x0 + 246, 180), (x1 - 246, 180), d.OURS, both=True)
        y = paragraph(c, x0 + 18, 232, "on the take's fighter: the moves are drawn", 540)
        paragraph(c, x0 + 18, y + 8, notice, 540)
    texts = [
        "the key",
        "the roster's own loader (Kirby, Ice Climbers and preload rules unchanged)",
        "Loading Link… B cancels",
        "the strip names the fighter the game now has",
    ]
    for i, text in enumerate(texts):
        x0 = 20 + i * 295
        boxed(c, x0, 349, x0 + 265, 445, text)
        if i < 3:
            d.arrow(c, (x0 + 269, 397), (x0 + 291, 397))
    paragraph(c, 24, 469, "One load at a time. A second press reads: Wait for the character swap to finish.", 1150)
    d.panel(c, 16, 532, 594, 670, "A · They stay as toggled", rec=True)
    c.text(30, 572, "when the editor closes", "h", d.FG)
    paragraph(c, 32, 630, "As Switch does today. No extra load.", 530, "s")
    d.panel(c, 606, 532, 1184, 670, "B · Both go back on close")
    paragraph(c, 622, 596, "One more load for each fighter that was switched.", 530, "s")
    c.save("scrub-moves-SM3-toggle", 695)


def sm4():
    c = Canvas(W, 850)
    title(c, "SM4 · The keys")
    ay = d.panel(c, 16, 65, 594, 705, "A · Two keys", rec=True)
    by = d.panel(c, 606, 65, 1184, 705, "B · A pop-up")
    for i, (key, text) in enumerate([
        ("ZL + L", "Player 1: the take's fighter ⇄ yours"),
        ("ZL + R", "CPU: the take's fighter ⇄ yours"),
    ]):
        y = ay + 18 + i * 66
        boxed(c, 32, y, 147, y + 48, key, font="s", col=d.OURS)
        paragraph(c, 163, y + 12, text, 410, "s", d.FG)
    y = ay + 165
    y = paragraph(c, 32, y, "ZR works as ZL does. GameCube controller: Z + L, Z + R.", 540) + 8
    y = paragraph(c, 32, y, "Left is Player 1 and right is the CPU, as on the damage display.", 540) + 24
    c.text(32, y, "Unchanged", "h", d.FG)
    y += 38
    for text in [
        "L / R  In / Out, or cut at the playhead",
        "Left / Right  move",
        "Y  snap",
        "ZL + Y  the move's start, with its setup",
        "X  preview",
        "ZL + X  the other capture",
        "ZR tap  where the preview plays",
        "A  save",
        "B  back",
    ]:
        c.text(38, y, text, "s", d.FG)
        y += 34
    d.panel(c, 630, by + 18, 1160, by + 240, "Fighters")
    d.row(c, 644, 1146, by + 82, "Player 1", "info", "‹ Mario | Link ›")
    d.row(c, 644, 1146, by + 120, "CPU", "info", "‹ Fox | Marth ›")
    paragraph(c, 644, by + 170, "Up/Down row · Left/Right change · B close", 490)
    y = by + 270
    y = paragraph(c, 624, y, "Opened with ZL + L or ZL + R.", 540, "s") + 14
    paragraph(c, 624, y, "Names both fighters before anything loads; one more step each time.", 540, "s")
    c.save("scrub-moves-SM4-keys", 730)


def sm5():
    c = Canvas(W, 860)
    title(c, "SM5 · A Replay window capture itself")
    ay = d.panel(c, 16, 65, 594, 677, "A · Not in this change", rec=True, tag_below=True)
    by = d.panel(c, 606, 65, 1184, 677, "B · Captures keep the animation too", tag_below=True)
    aend = vertical_flow(c, 32, 578, ay + 8, [
        "a capture's frame: inputs and state, no animation",
        "scrub the capture: positions, as today",
        "A saves a macro",
        "play it once",
        "scrub the macro: the moves",
    ], height=72, gap=25)
    paragraph(c, 34, aend + 24, "No change to the capture format.", 530, "s")
    bend = vertical_flow(c, 622, 1168, by + 8, [
        "every captured frame of every fighter gains animation and frame",
        "scrub the capture: the moves at once",
    ], height=82, gap=28)
    y = paragraph(c, 624, bend + 30, "The capture format changes.", 530, "s") + 12
    paragraph(c, 624, y, "More memory in every match with Replay capture On.", 530, "s")
    c.save("scrub-moves-SM5-captures", 703)


if __name__ == "__main__":
    for draw in (sm1, sm2, sm3, sm4, sm5):
        draw()
        print("drew", draw.__name__)
