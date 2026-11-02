"""Package-5 menu walker: navigation in the open in-game menu computed from `src/model/menu/table.rs` (the one table the
model reads), so a script names a page and a row title instead of counting presses.

    w = Walk(phase, pilot_present=True)
    w.open()                          # B + D-pad Up (the page the player last used shows)
    w.page("Display & Controls")      # L / R presses along the ring
    w.focus("HUD")                    # Up / Down presses to the row (the cursor is tracked here and checked by the shots)
    w.keys("a")                       # raw tokens (see s5_ally.keys)
    w.shot("name")

The cursor of a page starts on its first focusable entry (a row, or the header of an advanced group); a header of an advanced
group is focusable and A opens or closes it (the page's entries then change; `expand` tracks that).
"""
import re
from pathlib import Path

import s5_ally as s

TABLE = Path(__file__).resolve().parents[2] / "src/model/menu/table.rs"
PAGES = ["CPU", "Saves", "Recording", "Fighters & Stages", "Display & Controls", "Developer"]


def parse():
    text = TABLE.read_text(encoding="utf-8")
    groups = []
    for m in re.finditer(r'(?:g\(Page::(\w+), "([^"]+)"\)|Group \{ page: Page::(\w+), title: "([^"]+)", advanced: (true|false) \})', text):
        page = m.group(1) or m.group(3)
        title = m.group(2) or m.group(4)
        groups.append({"page": page, "title": title, "advanced": m.group(5) == "true"})
    consts = {}
    for m in re.finditer(r"const (\w+): u8 = (\d+);", text):
        consts[m.group(1)] = int(m.group(2))
    rows = []
    start = text.index("pub static ROWS")
    for line in text[start:].splitlines():
        m = re.search(r'(?:^|[\s.])(?:r|delay)\((\w+), "([^"]+)", "([^"]*)"', line)
        if not m:
            continue
        vis = "always"
        if "Vis::PilotPresent" in line:
            vis = "present"
        elif "Vis::PilotAbsent" in line:
            vis = "absent"
        rows.append({"group": consts[m.group(1)], "title": m.group(2), "id": m.group(3), "vis": vis})
    return groups, rows


GROUPS, ROWS = parse()
PAGE_NAME = {"Cpu": "CPU", "Saves": "Saves", "Recording": "Recording", "FightersStages": "Fighters & Stages",
             "DisplayControls": "Display & Controls", "Developer": "Developer"}


class Walk:
    def __init__(self, phase, pilot_present=True, developer=False):
        self.phase = phase
        self.pilot = pilot_present
        self.ring = PAGES if developer else PAGES[:-1]
        self.cur_page = 0
        self.cursor = {}
        self.expanded = set()

    # -- the model's entry list -------------------------------------------------------------
    def entries(self, page):
        out = []
        for g, group in enumerate(GROUPS):
            if PAGE_NAME[group["page"]] != page:
                continue
            rows = [r for r in ROWS if r["group"] == g and (r["vis"] == "always" or (r["vis"] == "present") == self.pilot)]
            if not rows:
                continue
            out.append(("G", g, group["title"], group["advanced"]))
            if not group["advanced"] or g in self.expanded:
                out.extend(("R", g, r["title"], False) for r in rows)
        return out

    def focusable(self, entry):
        return entry[0] == "R" or entry[3]

    def start_cursor(self, page):
        entries = self.entries(page)
        return next(i for i, e in enumerate(entries) if self.focusable(e))

    # -- actions ----------------------------------------------------------------------------
    def keys(self, tokens):
        return s.keys(self.phase, tokens)

    def shot(self, name):
        return s.shot(self.phase, name)

    def open(self):
        self.keys("b+up w8")

    def page(self, title):
        target = self.ring.index(title)
        n = len(self.ring)
        right = (target - self.cur_page) % n
        left = (self.cur_page - target) % n
        if right <= left:
            self.keys(" ".join(["r"] * right))
        else:
            self.keys(" ".join(["l"] * left))
        self.cur_page = target

    def focus(self, title):
        page = self.ring[self.cur_page]
        entries = self.entries(page)
        at = self.cursor.get(page, self.start_cursor(page))
        want = next(i for i, e in enumerate(entries) if e[2] == title and self.focusable(e))
        count = len(entries)
        steps = 0
        path = []
        i = at
        # Down only (the focusable entries wrap); the cheapest direction
        down = []
        j = at
        while j != want:
            j = (j + 1) % count
            if self.focusable(entries[j]):
                down.append("down")
        up = []
        j = at
        while j != want:
            j = (j - 1) % count
            if self.focusable(entries[j]):
                up.append("up")
        path = down if len(down) <= len(up) else up
        if path:
            self.keys(" ".join(path))
        self.cursor[page] = want
        return len(path)

    def toggle_group(self, title):
        """A on an advanced group header (the header must be focused)."""
        g = next(i for i, grp in enumerate(GROUPS) if grp["title"] == title)
        self.keys("a")
        self.expanded ^= {g}
