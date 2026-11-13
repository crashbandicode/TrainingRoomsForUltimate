#!/usr/bin/python3 -I
"""Draw docs/decisions/TR*.png and OP*.png: one picture per operator decision opened on 2026-11-13
(docs/DECISIONS.md, "Open (2026-11-13)").

Same look as tools/render_decisions.py (its canvas and helpers are reused); the recommended option is outlined in
green. Every picture is drawn from the facts in the section's text; no game-derived file is used.

usage: /usr/bin/python3 -I tools/render_decisions_1113.py [function name ...]
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import render_decisions as d  # noqa: E402

FG, MUTED, OURS, ADV, BAD, LINE = d.FG, d.MUTED, d.OURS, d.ADV, d.BAD, d.LINE
TILE, TILE_HI, PANEL, DARK, P1, CPU = d.TILE, d.TILE_HI, d.PANEL, d.DARK, d.P1, d.CPU
W = d.WIDE
MID = W // 2

# TR2: what option (b) frees, in sloc4 lines, once integration has measured it (None: still measuring).
B_FREES = None


# ---------------------------------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------------------------------
def frame(c, x0, y0, x1, y1, title, sub=None):
    c.rect(x0, y0, x1, y1, fill=PANEL, outline=LINE, radius=8)
    c.text(x0 + 14, y0 + 10, title, "h", FG)
    if sub:
        c.text(x0 + 24 + c.textlength(title, "h"), y0 + 14, sub, "xs", MUTED)
    return y0 + 44


def options(c, y0, y1, a, b, rec):
    """Two option panels side by side; a and b are (title, lines), a line is text or (text, colour)."""
    for x0, x1, (title, items), key in ((12, MID - 6, a, "a"), (MID + 6, W - 12, b, "b")):
        y = d.panel(c, x0, y0, x1, y1, title, rec == key)
        d.lines(c, x0 + 16, y, items, font="xs", step=21)


def said(c, y0, label, kind, value, old_help, new_help):
    """The row as rc.11 shows it, its two help lines, and the held fix's wording."""
    frame(c, 12, y0, W - 12, y0 + 168, "What the row says", "rc.11, focused, with its help panel")
    d.row(c, 26, 600, y0 + 54, label, kind, value, focus=True)
    c.rect(26, y0 + 88, W - 26, y0 + 136, fill=TILE, outline=LINE, radius=4)
    y = y0 + 92
    for ln in old_help:
        c.text(38, y, ln, "xs", FG)
        y += 21
    c.text(26, y0 + 144, "the held fix c49681c9 says: " + new_help, "xxs", OURS)
    return y0 + 168


def lane(c, y, label, segs, x_label=40):
    c.text(x_label, y + 10, label, "xs", FG)
    for xa, xb, text, col in segs:
        d.box(c, xa, y, xb, y + 36, [text], col=col, fill=TILE_HI, font="xxs")


def bug(name, label, kind, value, old_help, new_help, draw, a, b, rec):
    c = d.DCanvas(W, 560)
    said(c, 12, label, kind, value, old_help, new_help)
    y = frame(c, 12, 190, W - 12, 372, "What happens")
    draw(c, y)
    options(c, 382, 548, a, b, rec)
    c.save(name)


def mini_list(c, x0, y0, x1, rows):
    """A short list screen: rows of (text, colour, ticked)."""
    y = y0
    for text, col, ticked in rows:
        c.rect(x0, y, x1, y + 26, fill=TILE, outline=LINE, radius=3)
        c.text(x0 + 10, y + 5, ("[x] " if ticked else "[ ] ") + text, "xxs", col)
        y += 30
    return y


def table(c, x0, y0, cols, rows, step=30, head_col=MUTED):
    """Columns are (title, x offset); a row is a list of cells, a cell is text or (text, colour)."""
    for title, dx in cols:
        c.text(x0 + dx, y0, title, "b", head_col)
    y = y0 + 24
    for cells in rows:
        c.line([(x0, y - 4), (W - 30, y - 4)], (52, 58, 72), 1)
        for (title, dx), cell in zip(cols, cells):
            text, col = cell if isinstance(cell, tuple) else (cell, FG)
            c.text(x0 + dx, y + 2, text, "xxs", col)
        y += step
    return y


# ---------------------------------------------------------------------------------------------------
# TrainingRooms
# ---------------------------------------------------------------------------------------------------
def tr1_slots():
    c = d.DCanvas(W, 540)
    y = frame(c, 12, 12, W - 12, 184, "Today (rc.11)", "offered as working, never filled")
    mini_list(c, 26, y, 330, [("Jump", FG, False), ("Shield", FG, False), ("Playback Slot 1", ADV, True),
                              ("Playback Slot 2", ADV, False)])
    c.rect(350, y + 30, 640, y + 70, fill=(60, 30, 26), outline=BAD, width=2, radius=6)
    c.text(495, y + 42, "Record into this slot first", "xs", FG, "m")
    d.row(c, 660, W - 26, y + 6, "Hitstun timing", "drop", "As Hitstun Ends")
    c.text(W - 36, y + 36, "has no effect", "xxs", BAD, "r")
    d.lines(c, 660, y + 62, ["Slots 1-5: 5 of 30 options in Mash, Followup and the",
                             "overrides; 5 of 10 Ledge Options. Nothing fills a slot."], font="xxs", step=18)
    w = (W - 48) / 3
    titles = ["A · hide them", "B · label them", "C · implement"]
    bodies = [
        ([("Jump", FG, False), ("Shield", FG, False), ("Spot Dodge", FG, False)],
         ["Lists skip the five slot entries;", "Hitstun timing is hidden. Saved",
          "settings keep their shape.", ("Cost: 30-60 code lines against 4", ADV),
          ("lines of size headroom; tests and", ADV), ("one Eden pass.", ADV)]),
        ([("Playback Slot 1 (unused)", MUTED, False), ("Playback Slot 2 (unused)", MUTED, False)],
         ["Text only: five help lines and the", "refusal say the slots do nothing.",
          ("Cost: the dead options stay; those", BAD), ("help lines lose what they say now;", BAD),
          ("the refusal change needs approval.", BAD)]),
        ([("Copy macro to Playback Slot...  >", OURS, False), ("Playback Slot 1 · Combo A", FG, True)],
         ["A new row copies a library macro", "into a slot for the session.",
          ("Cost: about a day: wiring, Eden", BAD), ("re-validation of slot playback,", BAD),
          ("a budget raise, two playback systems.", BAD)]),
    ]
    for i, (title, (rows, text)) in enumerate(zip(titles, bodies)):
        x0 = 12 + i * (w + 12)
        yi = d.panel(c, x0, 196, x0 + w, 528, title, i == 0, tag_below=True)
        yy = mini_list(c, x0 + 14, yi, x0 + w - 14, rows)
        d.lines(c, x0 + 16, max(yy, yi + 96) + 8, text, font="xs", step=21)
    c.save("TR1-playback-slots")


def tr2_size_gate():
    c = d.DCanvas(W, 460)
    y = frame(c, 12, 12, W - 12, 236, "TrainingRooms' size gate", "sloc4, tools/size_gate.py; ceiling in size-budget.toml")
    lo, hi, x0, x1 = 147700, 148000, 420, W - 240

    def X(v):
        return x0 + (v - lo) / (hi - lo) * (x1 - x0)

    ceiling = 147921
    bars = [("main today", ceiling, LINE, "147,921 (headroom 0)"),
            ("+ integration's #23 and #15 fixes", ceiling + 46, BAD, "147,967 (+46 over)")]
    yy = y + 10
    for label, v, col, note in bars:
        c.text(40, yy + 6, label, "xs", FG)
        c.rect(X(lo), yy, X(v), yy + 28, fill=TILE_HI, outline=col, width=2, radius=3)
        c.text(X(v) + 8, yy + 6, note, "xxs", col)
        yy += 44
    c.text(40, yy + 6, "(b) minus the dead slot code", "xs", FG)
    if B_FREES is None:
        c.rect(X(lo), yy, X(ceiling + 46), yy + 28, outline=MUTED, width=1, radius=3)
        c.text(X(ceiling + 46) + 8, yy + 6, "integration is measuring", "xxs", MUTED)
    else:
        v = ceiling + 46 - B_FREES
        c.rect(X(lo), yy, X(v), yy + 28, fill=TILE_HI, outline=OURS, width=2, radius=3)
        c.text(X(v) + 8, yy + 6, f"{v:,} ({B_FREES} lines freed)", "xxs", OURS if v <= ceiling else BAD)
    c.line([(X(ceiling), y), (X(ceiling), yy + 36)], ADV, 2, dash=(6, 4))
    c.text(X(ceiling), yy + 40, "ceiling 147,921", "xxs", ADV, "m")
    options(c, 248, 448,
            ("a · raise the ceiling by 46", ["A new [[budget]] entry at 147,967, with the",
                                            "lead's approval in approved_by.",
                                            "Cost to the player: none. The limit grows",
                                            "by exactly what the two fixes need."]),
            ("b · delete the unreachable code", ["The old slot recorder and named capture",
                                                "(#14) can't be reached; deleting them",
                                                "makes room. It goes with TR1 A (hide).",
                                                "Cost to the player: none. With TR1 C the",
                                                ("slot code is needed: then take (a).", ADV)]),
            "b")
    c.save("TR2-size-gate")


def tr3_hud():
    def draw(c, y):
        for i, (x0, hud, cap) in enumerate(((40, True, "On: the game's own HUD, unchanged; no plugin HUD is drawn"),
                                            (MID + 40, False, "Off: damage panels, item pointers, off-screen bubbles hidden"))):
            x1 = x0 + 500
            c.rect(x0, y, x1, y + 100, fill=(30, 36, 48), outline=LINE, radius=4)
            c.rect(x0 + 60, y + 58, x1 - 60, y + 66, fill=(70, 78, 96))
            c.circle(x0 + 170, y + 48, 9, fill=P1)
            c.circle(x1 - 170, y + 48, 9, fill=CPU)
            if hud:
                for j, (lab, col) in enumerate((("P1   0.0%", P1), ("CPU  0.0%", CPU))):
                    bx = x0 + 120 + j * 150
                    c.rect(bx, y + 72, bx + 110, y + 94, fill=TILE, outline=col, width=2, radius=3)
                    c.text(bx + 55, y + 76, lab, "xxs", FG, "m")
                c.circle(x1 - 30, y + 18, 11, outline=FG, width=2)
                c.polygon([(x0 + 8, y + 30), (x0 + 20, y + 24), (x0 + 20, y + 36)], fill=ADV)
            c.text(x0, y + 108, cap, "xxs", OURS if hud else ADV)
    bug("TR3-hud", "HUD", "toggle", "OFF",
        ["On: the training HUD is drawn.", "Off: the game's own HUD only."],
        "On: the game's HUD shows as usual. Off: Training hides damage, pointers and off-screen bubbles.",
        draw,
        ("A · keep it (upstream's HUD row)", ["Off hides the game's HUD, as Training",
                                              "Modpack's HUD row did ('Show/hide",
                                              "elements of the UI').",
                                              "Cost to the player: none; the held fix",
                                              "says it."]),
        ("B · build what rc.11 described", ["There is no plugin HUD for On to draw;",
                                           "it would have to be designed and built.",
                                           ("Cost: new code, and Off would stop giving", BAD),
                                           ("a clean screen for recordings.", BAD)]),
        "a")


def tr4_pummel():
    def draw(c, y):
        lane(c, y, "Throw: None (default)", [(330, W - 40, "holds you; no throw and no pummel, whatever Pummel Delay says", BAD)])
        lane(c, y + 48, "Throw: Forward, delay 60", [(330, 560, "Pummel Delay 15: holds", LINE),
                                                     (560, 960, "pummels on each hold frame", OURS),
                                                     (960, W - 40, "throws at frame 60", LINE)])
        c.text(40, y + 96, "Training Modpack's throw code does the same: Throw None returns before the pummel.",
               "xxs", MUTED)
    bug("TR4-pummel", "Pummel Delay", "multi", "15",
        ["Frames after a grab before the CPU pummels; any value",
         "pummels, none does not. Tick values or set a range."],
        "It pummels only during Throw Delay, with a Throw other than None.",
        draw,
        ("A · keep it (as upstream)", ["Set a Throw and a Throw Delay to see",
                                       "pummels.",
                                       "Cost: a Pummel Delay alone does nothing;",
                                       "the held help line says so."]),
        ("B · pummel with Throw None too", ["The CPU holds and pummels until the grab",
                                           "ends.",
                                           ("Cost: behaviour upstream never had; what", BAD),
                                           ("ends the hold has to be chosen; an Eden", BAD),
                                           ("check.", BAD)]),
        "a")


def tr5_respawn():
    def draw(c, y):
        lane(c, y, "Save States On", [(330, 540, "KO", LINE), (540, W - 40, "instant respawn: Respawn Off works", OURS)])
        lane(c, y + 48, "Save States Off", [(330, 540, "KO", LINE),
                                            (540, W - 40, "revival platform: the row is ignored", BAD)])
        c.text(40, y + 96, "TrainingRooms answers the respawn parameters only with Save States On "
               "(state/hooks.rs); Training Modpack answers them in any Training.", "xxs", MUTED)
    bug("TR5-respawn", "Respawn on platform", "toggle", "OFF",
        ["On: after a death, back on the revival platform with the game's invincibility.",
         "Off: you respawn instantly. Greyed while Auto-load on death is On (Saves)."],
        "Off: instant respawn, only while Save States is On (Saves).",
        draw,
        ("A · keep; approve a restriction", ["Respawn Off needs Save States On. The help",
                                            "line keeps saying so and needs an approved",
                                            "restriction ID (compliance).",
                                            ("Cost: Off does nothing with Save States Off.", BAD)]),
        ("B · work with Save States Off", ["Answer the respawn parameters in practice",
                                          "whatever Save States says, as upstream does.",
                                          "Cost: a small change in state/hooks.rs, a",
                                          "test, one Eden KO. Settles the compliance row."]),
        "b")


def tr6_ask_fighter():
    def draw(c, y):
        c.text(W - 40, y - 30, "with the row Off", "xs", ADV, "r")
        cells = [((40, y), "Replay window", "keeps the CPU", OURS),
                 ((MID + 20, y), "Play chord: a macro", "asks anyway", BAD),
                 ((40, y + 48), "Combo drill", "asks anyway", BAD),
                 ((MID + 20, y + 48), "Play chord: a chain", "asks anyway", BAD)]
        for (x, yy), src, result, col in cells:
            d.box(c, x, yy, x + 250, yy + 36, [src], col=LINE, fill=TILE_HI, font="xs")
            d.arrow(c, (x + 256, yy + 18), (x + 316, yy + 18), col=col)
            d.box(c, x + 322, yy, x + 540, yy + 36, [result], col=col, fill=TILE_HI, font="xs")
        c.text(40, y + 96, "Only the Replay window reads the row; Play chords and drills ask in "
               "rr_host::mismatch_for and ask_drill.", "xxs", MUTED)
    bug("TR6-ask-fighter", "Ask before switching fighter", "toggle", "OFF",
        ["On: a replay or named recording asks before it",
         "switches the CPU, a combo before it switches P1."],
        "Replay window: On asks, Off keeps it. A macro, a chain or a combo drill asks either way.",
        draw,
        ("A · keep (Replay window only)", ["Off affects the Replay window only.",
                                           ("Cost: a macro, a chain or a drill still", BAD),
                                           ("asks every time with the row Off.", BAD)]),
        ("B · every question follows it", ["Off keeps the current fighter everywhere:",
                                          "the Replay window, Play chords and drills.",
                                          "Cost: a small change in rr_host, tests, one",
                                          "Eden check of a Play chord with Off."]),
        "b")


def tr7_crop():
    def draw(c, y):
        lane(c, y, "Record, row On (rc.11)", [(330, 760, "inputs", LINE), (760, 1060, "idle frames: kept", BAD)])
        c.line([(1066, y - 4), (1066, y + 40)], ADV, 3)
        c.text(1074, y + 10, "stop", "xxs", ADV)
        lane(c, y + 48, "Upstream's Recording Crop", [(330, 760, "inputs", LINE)])
        c.line([(766, y + 48), (1060, y + 48), (1060, y + 84), (766, y + 84), (766, y + 48)], MUTED, 1, dash=(5, 5))
        c.text(913, y + 58, "idle frames: trimmed", "xxs", OURS, "m")
        c.text(40, y + 96, "Record never reads the row (rr_runtime::finish_recording); its only readers, the old "
               "slot recorder and named capture, can't be reached.", "xxs", MUTED)
    bug("TR7-crop", "Crop trailing neutral", "toggle", "ON",
        ["On: idle frames at the end of a recording are dropped.",
         "Off: the recording keeps its full length."],
        "Has no effect on Record: a recording keeps every frame up to the stop.",
        draw,
        ("A · remove the row", ["A row that does nothing goes; Crop on a",
                                "macro's screen stays. Pairs with TR2 (b).",
                                ("Cost: upstream's trim is gone for good.", BAD)]),
        ("B · make Record trim", ["Record drops trailing neutral frames when",
                                  "On, as upstream's Recording Crop did.",
                                  "Cost: a few lines in finish_recording, a",
                                  "test, an Eden take. An idle end on purpose",
                                  "needs the row Off."]),
        "b")


def tr8_player_character():
    def draw(c, y):
        lane(c, y, "Player Swapping Off", [(330, 540, "pick Mario", LINE), (540, 900, "nothing loads, no notice", BAD),
                                           (900, W - 40, "Swapping On: Mario loads", LINE)])
        lane(c, y + 48, "A combo's Switch", [(330, 540, "Switch", LINE),
                                             (540, W - 40, "Player 1 loads now; Player Swapping isn't asked", OURS)])
        c.text(40, y + 96, "A pick goes through the cycle's queue, which drops it while the cycle is off "
               "(character_roster.rs); a Switch passes `once`.", "xxs", MUTED)
    bug("TR8-player-character", "Player Character", "editor", "Mario",
        ["Pick Player 1's fighter from the portrait grid."],
        "...With Player Swapping Off, it loads when you turn it On.",
        draw,
        ("A · keep; approve a restriction", ["A pick waits for Player Swapping On. The",
                                            "help line keeps saying so and needs an",
                                            "approved restriction ID (compliance).",
                                            ("Cost: the pick looks broken with it Off.", BAD)]),
        ("B · a pick loads now", ["A portrait pick is an explicit load, like a",
                                  "combo's Switch; the cycle chord still obeys",
                                  "Player Swapping. Cost: one argument in",
                                  "character_roster.rs, a test, one Eden pick.",
                                  "Settles the compliance row."]),
        "b")


# ---------------------------------------------------------------------------------------------------
# Other projects
# ---------------------------------------------------------------------------------------------------
def op1_push():
    c = d.DCanvas(W, 560)
    y = frame(c, 12, 12, W - 12, 352, "Held doc commits", "local only; checked 2026-11-13 09:40Z")
    cols = [("lane", 28), ("repo", 160), ("commits", 520), ("on Forgejo main", 860)]
    rows = [["worker A", "13 tool repos (list in the text)", "17", ("fast-forward", OURS)],
            ["worker A", "TrainingRooms (in-game text)", "c49681c9", ("needs a rebase; after TR5, TR8", ADV)],
            ["integration", "ssbu-mod-playbook", "cab8d43 84bf626 292f6c1 13b2e83", ("fast-forward", OURS)],
            ["Deluxe lane", "ssbu-online-deluxe-rewrite", "5f9759f", ("fast-forward", OURS)],
            ["worker B", "ssbu-stall-press-carry", "71dcb0f", ("fast-forward", OURS)]]
    yy = table(c, 14, y, cols, rows)
    c.rect(28, yy + 6, W - 28, yy + 96, fill=TILE, outline=ADV, width=2, radius=6)
    d.lines(c, 40, yy + 14, [("Reconcile first (other copies of the same repos):", ADV),
                             "Surface: ~/smash-replay-to-mp4 has uncommitted 10-15 edits in 6 of the same files.",
                             "Yoga: ~/instant_back has 3 commits not on Forgejo; ~/elite-rematch and ~/ssbu-fx-control",
                             "have uncommitted work."], font="xxs", step=19)
    options(c, 364, 548,
            ("A · push the docs now", ["Push the 23 docs-only commits once the four",
                                       "copies are reconciled; c49681c9 after the",
                                       "TR5 and TR8 answers (two of its help lines",
                                       "follow them).",
                                       "Cost: none; the published docs get right."]),
            ("B · hold everything", [("About 300 stale claims stay published:", BAD),
                                     ("wrong Switch status in 7 repos, a dead", BAD),
                                     ("install link, a config example that breaks", BAD),
                                     ("enabled=false.", BAD)]),
            "a")
    c.save("OP1-push")


def op2_imgui_nro():
    c = d.DCanvas(W, 420)
    for x0, title, present in ((12, "Now: the standalone NRO on the card", True), (MID + 6, "Removed", False)):
        x1 = x0 + MID - 18
        y = frame(c, x0, 12, x1, 220, title)
        files = ["libssbu_online_deluxe_rewrite.nro", "libtraining_rooms.nro"] + (["libimgui_smash.nro"] if present else [])
        for i, f in enumerate(files):
            col = BAD if f.startswith("libimgui") else LINE
            d.box(c, x0 + 16, y + i * 34, x0 + 300, y + i * 34 + 28, [f], col=col, fill=TILE_HI, font="xxs")
        c.text(x0 + 320, y + 4, "Deluxe 0.10.4 draws", "xs", MUTED)
        c.text(x0 + 320, y + 24, "with its font:" if present else "with its own font:", "xs", MUTED)
        c.rect(x0 + 320, y + 52, x1 - 16, y + 92, fill=TILE, outline=BAD if present else OURS, width=2, radius=6)
        c.text((x0 + 320 + x1 - 16) / 2, y + 62, "Zo? · J?r?me" if present else "Zoë · Jérôme", "s",
               BAD if present else OURS, "m")
        c.text(x0 + 16, y + 112, "ASCII-only font (1.0.2 build f72a6b8d)" if present else
               "nothing else on the card imports it", "xxs", MUTED)
        c.text(x1 - 16, y + 112, "example names", "xxs", MUTED, "r")
    options(c, 232, 408,
            ("A · remove it from the card", ["A read-only card scan: nothing imports it;",
                                             "only Deluxe looks it up, and without it",
                                             "Deluxe draws with its own font.",
                                             "Cost: none found."]),
            ("B · keep it", [("Names outside ASCII show '?' in Deluxe", BAD),
                             ("until an imgui build with a wider font", BAD),
                             ("ships (OP6).", BAD)]),
            "a")
    c.save("OP2-imgui-nro")


def op3_elite_page():
    c = d.DCanvas(W, 400)
    y = frame(c, 12, 12, W - 12, 200, "Deluxe's page cycle on the console", "the card's config.ini")
    c.text(28, y + 4, "now:", "xs", MUTED)
    d.tabs(c, 120, y, ["Settings", "CPU Pilot", "Window Options"], 0)
    c.text(620, y + 4, "hide_plugin_pages=stages_counterpick", "xs", BAD)
    c.text(28, y + 52, "shown:", "xs", MUTED)
    x = d.tabs(c, 120, y + 48, ["Settings", "CPU Pilot"], 0)
    c.rect(x, y + 44, x + 230, y + 72, outline=OURS, width=2, radius=3)
    c.text(x + 10, y + 48, "Stages + Counterpick", "s", OURS)
    x = d.tabs(c, x + 238, y + 48, ["Window Options"], -1)
    c.text(x + 8, y + 54, "(its place in the cycle may differ)", "xxs", MUTED)
    c.text(28, y + 100, "Hidden since the 0.10.0 install (2026-11-01): only CPU Pilot was set to Shown, so the new "
           "page stayed Hidden.", "xxs", MUTED)
    options(c, 212, 388,
            ("A · show it", ["On the console: Window Options, Stages +",
                             "Counterpick, Shown. No card access needed.",
                             "Cost: one more page in the cycle."]),
            ("B · keep it hidden", [("The plugin runs with no visible page, and", BAD),
                                    ("every console check must unhide it first.", BAD)]),
            "a")
    c.save("OP3-elite-page")


def op4_public_repos():
    c = d.DCanvas(W, 400)
    y = frame(c, 12, 12, W - 12, 200, "Private on Forgejo: 404 when signed out")
    for i, (repo, links) in enumerate((("elite-stages-counterpick",
                                        ["Deluxe README and INSTALL.md link it;",
                                         "its own README's Releases link too."]),
                                       ("ssbu-cpu-pilot", ["Deluxe's README lists it as a page",
                                                           "plugin; 0.2.4 is on the card."]))):
        x0 = 28 + i * (MID - 10)
        c.rect(x0, y, x0 + MID - 40, y + 130, fill=TILE, outline=BAD, width=2, radius=6)
        c.text(x0 + 14, y + 10, repo, "s", FG)
        d.tag(c, x0 + MID - 130, y + 10, "private", BAD)
        d.lines(c, x0 + 14, y + 44, links, font="xs", step=21)
        c.text(x0 + 14, y + 100, "signed out: 404", "xs", BAD)
    options(c, 212, 388,
            ("A · make both public", ["After a history check for identities, as",
                                      "imgui-smash had (0 NVIDIA identities).",
                                      "Cost: none; the public links start working."]),
            ("B · keep them private", [("Drop the links from Deluxe's public docs;", BAD),
                                       ("players can't get either plugin.", BAD)]),
            "a")
    c.save("OP4-public-repos")


def op5_elite_zip():
    c = d.DCanvas(W, 400)
    y = frame(c, 12, 12, W - 12, 200, "elite-stages-counterpick v0.1.0: the release ZIP", "Forgejo release 473")
    c.rect(28, y, 560, y + 112, fill=TILE, outline=LINE, radius=6)
    d.box(c, 40, y + 10, 548, y + 44, ["libelite_stages_counterpick.nro"], col=LINE, fill=TILE_HI)
    d.box(c, 40, y + 56, 548, y + 102, ["libssbu_online_deluxe_rewrite.nro",
                                         "older Deluxe: commit 0d28adc (before 0.8.0), 1,445,888 B"],
          col=BAD, fill=TILE_HI, font="xxs")
    d.arrow(c, (570, y + 56), (660, y + 56), col=BAD)
    d.lines(c, 676, y + 22, [("Unzipped over a card with Deluxe 0.10.4,", BAD),
                             ("Deluxe goes back to the old build.", BAD),
                             "The README (b5a08da) warns: with Deluxe",
                             "0.8.0 or later, copy only the plugin NRO."], font="xs", step=22)
    options(c, 212, 388,
            ("A · drop it from the ZIP", ["Re-upload the ZIP with the plugin NRO only,",
                                          "and link Deluxe's own release.",
                                          "Cost: none; Deluxe comes from its release."]),
            ("B · keep it", [("Anyone who unzips the whole archive", BAD),
                             ("downgrades Deluxe; only the README warns.", BAD)]),
            "a")
    c.save("OP5-elite-zip")


def op6_releases():
    c = d.DCanvas(W, 560)
    y = frame(c, 12, 12, W - 12, 352, "Release pages that lag the card")
    cols = [("repo", 28), ("what the card or main has", 300), ("missing", 820)]
    rows = [["ARCropolis", "card: ac3e05a3 (main + the reject.log cap), since 11-08", ("no release", BAD)],
            ["ssbu-desync-detector", "0.5.1 on the card", ("0.5.0, 0.5.1: no tag (after console OK)", BAD)],
            ["ssbu-cpu-pilot", "0.2.4 (abd305e6) on the card", ("0.2.3, 0.2.4: no tag, no release", BAD)],
            ["ssbu-quiet-elite-switch", "0.2.0", ("no release", BAD)],
            ["quickplay-cancel-fix", "0.3.0 on the card since 11-10", ("still a pre-release", ADV)],
            ["imgui-smash", "main pins 1.0.2's archive from before the font merge", ("re-pin, then release", ADV)],
            ["GitHub mirrors", "overlay shows v0.1.2 (v0.3.1); replay-to-mp4 v0.1.0", ("'Latest' is stale", ADV)]]
    table(c, 14, y, cols, rows, step=36)
    options(c, 364, 548,
            ("A · tag and release what runs", ["Each from the commit that rebuilds the",
                                               "card's bytes exactly (checked); the",
                                               "detector after its console confirmation;",
                                               "fix GitHub 'Latest'.",
                                               "Cost: none; downloads match the card."]),
            ("B · leave them", [("The card runs builds nobody can download", BAD),
                                ("or rebuild from a tag.", BAD)]),
            "a")
    c.save("OP6-releases")


def op7_license():
    c = d.DCanvas(W, 420)
    y = frame(c, 12, 12, W - 12, 220, "LICENSE files")
    none = ["smash-replay-to-mp4", "instant_back", "ssbu-fx-control", "ssbu-quiet-elite-switch", "elite-rematch",
            "ssbu-refresh-clock"]
    mit = ["overlay", "memory-guard", "detector", "QCF", "cpu-pilot", "gsp", "tile-cursor", "TrainingRooms"]
    c.text(28, y, "none:", "xs", BAD)
    for i, r in enumerate(none):
        x = 110 + (i % 3) * 270
        d.box(c, x, y + (i // 3) * 36 - 4, x + 256, y + (i // 3) * 36 + 26, [r], col=BAD, fill=TILE_HI, font="xxs")
    c.text(28, y + 84, "MIT:", "xs", OURS)
    x = 110
    for r in mit:
        x = d.tag(c, x, y + 84, r, OURS) + 8
    c.text(28, y + 124, "Deluxe is AGPL; code taken from a GPL-family project would carry that licence.", "xxs", MUTED)
    options(c, 232, 408,
            ("A · MIT, like the others", ["Add MIT to all six after checking that none",
                                          "carries GPL-family code.",
                                          "Cost: none."]),
            ("B · leave them unlicensed", [("Nobody may reuse or redistribute the code;", BAD),
                                           ("the default is all rights reserved.", BAD)]),
            "a")
    c.save("OP7-license")


def op8_quotes():
    c = d.DCanvas(W, 400)
    y = frame(c, 12, 12, W - 12, 200, "instant_back: DESIGN-v031.md and BRIEF-0.3.1.md", "repo private today")
    c.rect(28, y, 560, y + 110, fill=TILE, outline=BAD, width=2, radius=6)
    d.lines(c, 40, y + 12, ["## Requirement", ('> "...the operator\'s own words..."', BAD), "", "(quoted as written)"],
            font="xs", step=22)
    d.arrow(c, (572, y + 55), (640, y + 55), col=OURS)
    c.rect(652, y, W - 28, y + 110, fill=TILE, outline=OURS, width=2, radius=6)
    d.lines(c, 664, y + 12, ["## Requirement", ("The operator asked for ... (paraphrased)", OURS), "",
                             "(as the detector's and B's were)"], font="xs", step=22)
    options(c, 212, 388,
            ("A · paraphrase them", ["Like the detector's design files and B's",
                                     "HANDOFF (71dcb0f).",
                                     "Cost: none; one docs-only commit."]),
            ("B · keep them", ["The playbook allows quotes in design",
                               ("working files; they'd be published if the", BAD),
                               ("repo goes public.", BAD)]),
            "a")
    c.save("OP8-quotes")


ALL = [tr1_slots, tr2_size_gate, tr3_hud, tr4_pummel, tr5_respawn, tr6_ask_fighter, tr7_crop, tr8_player_character,
       op1_push, op2_imgui_nro, op3_elite_page, op4_public_repos, op5_elite_zip, op6_releases, op7_license, op8_quotes]


def main():
    only = set(sys.argv[1:])
    for fn in ALL:
        if not only or fn.__name__ in only:
            fn()
            print("drew", fn.__name__)


if __name__ == "__main__":
    main()
