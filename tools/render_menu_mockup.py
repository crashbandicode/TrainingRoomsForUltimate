#!/usr/bin/python3 -I
"""Render the proposed TrainingRooms menu (MENU-DRAFT.md §2, v2 minus the CPU-pilot rows the standalone
plugin now owns) as PNG and SVG screens in the in-game overlay look, 1280x720 (Switch handheld).

Output: docs/menu-mockup/NN-<page>.png and .svg (five pages + three sub-screens) and an index.md.
Pillow only (no external converter on this host); DejaVu Sans Mono for the modpack-style monospace text.
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

W, H = 1280, 720
OUT = Path(__file__).resolve().parent.parent / "docs" / "menu-mockup"
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
FONT_B = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"

# colours (the modpack overlay: near-black translucent panel, white text, red selection)
BG0, BG1 = (18, 22, 30), (40, 46, 58)
PANEL = (10, 12, 16, 226)
LINE = (70, 76, 90)
FG = (236, 236, 232)
MUTED = (150, 156, 170)
SEL = (176, 52, 42)
OURS = (111, 195, 154)
UP = (168, 155, 240)
ADV = (210, 170, 80)
HILITE = (38, 42, 52)  # selected row: a solid lift over the panel (Pillow's RGBA draw does not blend alpha fills)

# kind: toggle | drop | multi | slider | number | editor | grid | list | info ; value = text shown in the control
PAGES = [
    ("CPU", [
        ("Reactions", False, [
            ("Mash", "multi", "0 of 13 selected", "up"),
            ("Followup", "multi", "0 of 13 selected", "up"),
            ("Mash Triggers", "multi", "6 of 17 selected", "up"),
            ("Situation overrides", "editor", "0 of 13 set", "up"),
        ]),
        ("Defence", False, [
            ("Airdodge Direction", "multi", "0 of 11 selected", "up"),
            ("DI Direction", "multi", "0 of 11 selected", "up"),
            ("SDI Direction", "multi", "0 of 11 selected", "up"),
            ("SDI Strength", "drop", "None", "up"),
            ("Clatter Strength", "drop", "None", "up"),
            ("Dodge Staling", "toggle", "ON", "up"),
        ]),
        ("Ledge & Tech", False, [
            ("Ledge Options", "multi", "4 of 10 selected", "up"),
            ("Ledge Delay", "multi", "0 of 31 selected  (0-300 by 10)", "up"),
            ("Tech Options", "multi", "4 of 4 selected", "up"),
            ("Mistech Options", "multi", "4 of 4 selected", "up"),
            ("Hide Tech Animations", "toggle", "OFF", "up"),
        ]),
        ("Shield", False, [
            ("Shield", "drop", "None", "up"),
            ("Shield Tilt", "drop", "Neutral", "up"),
            ("OoS Offset", "multi", "0 of 31 selected  (0-30 hits)", "up"),
            ("Reaction Time", "multi", "0 of 31 selected  (0-30 frames)", "up"),
        ]),
        ("Attacks & Movement", False, [
            ("Attack Angle", "multi", "0 of 3 selected", "up"),
            ("Throw", "multi", "0 of 5 selected", "up"),
            ("Throw Delay", "multi", "0 of 31 selected  (0-150 by 5)", "up"),
            ("Pummel Delay", "multi", "0 of 31 selected  (0-150 by 5)", "up"),
            ("Falling Aerials", "drop", "No", "up"),
            ("Aerial Delay", "multi", "0 of 31 selected  (0-30)", "up"),
            ("Full Hop", "drop", "Yes", "up"),
            ("Fast Fall", "drop", "No", "up"),
            ("Fast Fall Delay", "multi", "0 of 31 selected  (0-30)", "up"),
            ("Crouch", "toggle", "OFF", "up"),
        ]),
    ]),
    ("Saves", [
        ("Save states", False, [
            ("Save States", "toggle", "ON", "up"),
            ("Slot", "drop", "1", "up"),
            ("Randomize Slots", "multi", "0 of 5 selected", "up"),
            ("Mirroring", "drop", "None", "up"),
            ("Auto-load on death", "toggle", "OFF", "up"),
            ("Player damage on load", "drop", "Default", "up"),
            ("Player damage range", "slider", "0 - 150 %", "up"),
            ("CPU damage on load", "drop", "Default", "up"),
            ("CPU damage range", "slider", "0 - 150 %", "up"),
            ("Buffs on load", "multi", "0 of 12 selected", "up"),
            ("Item on load (older saves)", "drop", "None", "up"),
        ]),
        ("Setup library", False, [
            ("Save setup to library...", "editor", "name entry", "ours"),
            ("Setup library", "list", "saved setups", "ours"),
        ]),
    ]),
    ("Recording", [
        ("Recording", False, [
            ("Record Slot", "drop", "1", "up"),
            ("Trigger", "drop", "Button Combination", "up"),
            ("Duration", "drop", "150 frames", "up"),
            ("Crop trailing neutral", "toggle", "ON", "up"),
        ]),
        ("Playback", False, [
            ("Button slots", "multi", "1 of 5 selected", "up"),
            ("Hitstun timing", "drop", "As Hitstun Ends", "up"),
            ("Save-state slots", "multi", "0 of 5 selected", "up"),
            ("Mash interrupts playback", "toggle", "ON", "up"),
            ("Loop", "toggle", "OFF", "up"),
        ]),
        ("Named macros", False, [
            ("Source", "drop", "Numbered Slots", "ours"),
            ("Ask before switching fighter", "toggle", "ON", "ours"),
            ("Named library", "list", "library", "ours"),
            ("Save slot to library...", "editor", "name entry", "ours"),
            ("Edit placement", "editor", "placement editor", "ours"),
            ("Replay window", "editor", "captures", "ours"),
            ("Replay capture", "toggle", "ON", "ours"),
        ]),
    ]),
    ("Fighters & Stages", [
        ("Player 1 and CPU", False, [
            ("Player Swapping", "toggle", "OFF", "ours"),
            ("Player Character", "grid", "Mr. Game & Watch", "ours"),
            ("CPU Character", "grid", "Pikachu", "ours"),
            ("Character Cycles", "grid", "two-role ordered grid", "ours"),
            ("Solo Popo (original Ice Climbers)", "toggle", "OFF", "ours"),
        ]),
        ("Extras", False, [
            ("Kirby Hats", "grid", "two-role hat order", "ours"),
            ("Give item", "grid", "none", "ours"),
            ("Mii Brawler", "list", "role's saved pick", "ours"),
            ("Mii Swordfighter", "list", "role's saved pick", "ours"),
            ("Mii Gunner", "list", "role's saved pick", "ours"),
        ]),
        ("Stages", False, [
            ("Lobby Practice", "toggle", "OFF", "ours"),
            ("Lobby Stages", "grid", "ordered legal stages + All", "ours"),
            ("Training Stage Rotation", "toggle", "OFF", "ours"),
            ("Stage Hazards", "toggle", "OFF", "up"),
        ]),
        ("Loading", True, [
            ("Cycle Preload", "drop", "5 (built-in max)", "ours"),
            ("Preload", "drop", "Both", "ours"),
            ("Ready Fighters", "number", "8", "ours"),
            ("Ready Fighter Bodies", "drop", "3", "ours"),
            ("Group Swap", "toggle", "ON", "ours"),
            ("Swap groups", "multi", "7 of 7 selected", "ours"),
            ("Preload Samples", "toggle", "OFF", "ours"),
        ]),
    ]),
    ("Display & Controls", [
        ("On screen", False, [
            ("HUD", "toggle", "ON", "up"),
            ("Hitbox Visualization", "toggle", "OFF", "up"),
            ("Frame Advantage", "toggle", "OFF", "up"),
            ("Input Display", "drop", "Smash Inputs", "up"),
            ("Group inputs by status", "toggle", "OFF", "up"),
            ("Input Delay", "number", "0 frames", "up"),
        ]),
        ("Controls", False, [
            ("Button chords", "editor", "6 chords", "mixed"),
            ("L+R+A Reset", "toggle", "ON", "up"),
            ("Menu shortcut 1", "info", "Minus  (button_config.ini)", "ours"),
            ("Menu shortcut 2", "info", "disabled  (button_config.ini)", "ours"),
            ("Permanent shortcut", "info", "B + D-pad Up", "ours"),
        ]),
    ]),
]

HELP = {
    "Mash": "Actions the CPU mashes out of hitstun, shieldstun, tumble and landing. Random among the selected.",
    "Situation overrides": "Per-situation action sets (ledge getups, tech, clatter, hitstun...). Opens a sub-screen.",
    "Save States": "Save and load positions and damage with the chord set under Display & Controls.",
    "Record Slot": "Slot the next recording is written to.",
    "Player Swapping": "Swap Player 1's fighter in Training without reloading (cycle chord: ZL + D-pad Right).",
    "HUD": "Show the Training HUD overlay (damage, state, frame data).",
}

SUBSCREENS = [
    ("Situation overrides", "CPU > Reactions", [
        ("Situation", "drop", "Ledge roll", None),
        ("Use Mash set", "toggle", "OFF", None),
        ("Airdodge", "check", "x", None), ("Jump", "check", "x", None), ("Shield", "check", " ", None),
        ("Spotdodge", "check", " ", None), ("Roll In", "check", " ", None), ("Roll Out", "check", " ", None),
        ("Aerial", "check", "x", None), ("Jab", "check", " ", None), ("Tilts", "check", " ", None),
        ("Smash Attacks", "check", " ", None), ("Grab", "check", " ", None), ("Dash", "check", " ", None),
        ("Dash Attack", "check", " ", None),
    ], "Pick the situation with the dropdown, then tick the actions. The page row shows how many situations are set."),
    ("Button chords", "Display & Controls > Controls", [
        ("Save State Save", "chord", "ZL + D-pad Down", None),
        ("Save State Load", "chord", "ZL + D-pad Up", None),
        ("Input Record", "chord", "ZR + D-pad Down", None),
        ("Input Playback", "chord", "ZR + D-pad Up", None),
        ("Cycle Player Character", "chord", "ZL + D-pad Right", None),
        ("Cycle CPU Character", "chord", "ZL + D-pad Left", None),
    ], "Hold one button and press the others to set a chord (16 buttons). A chord of one button is allowed."),
    ("Swap groups", "Fighters & Stages > Loading (advanced)", [
        ("Ice Climbers pair bank", "check", "x", None),
        ("Original fighter bank", "check", "x", None),
        ("Solo Popo groups", "check", "x", None),
        ("Ice Climbers pair groups", "check", "x", None),
        ("PT/Aegis groups", "check", "x", None),
        ("Ice Climbers pair group", "check", "x", None),
        ("Mii groups", "check", "x", None),
    ], "Which special groups may be prefetched and swapped warm. An entry greys out, with the reason, when a dependency is off."),
]


def fonts():
    return {"tab": ImageFont.truetype(FONT_B, 22), "h": ImageFont.truetype(FONT_B, 20), "t": ImageFont.truetype(FONT, 20),
            "s": ImageFont.truetype(FONT, 16), "xs": ImageFont.truetype(FONT, 14)}


def background():
    img = Image.new("RGB", (W, H), BG0)
    px = img.load()
    for y in range(H):  # vertical gradient, a hint of the stage behind the overlay
        t = y / H
        c = tuple(int(BG0[i] * (1 - t) + BG1[i] * t) for i in range(3))
        for x in range(W):
            px[x, y] = c
    d = ImageDraw.Draw(img)
    d.rectangle([0, 560, W, 600], fill=(52, 58, 72))  # a platform line, dimmed
    return img.convert("RGBA")


def control_text(kind, value):
    if kind == "toggle":
        return f"[{value}]", (OURS if value == "ON" else MUTED)
    if kind == "drop":
        return f"<  {value}  >", FG
    if kind == "multi":
        return f"{value}  >", FG
    if kind == "slider":
        return f"|=========---|  {value}", FG
    if kind == "number":
        return f"<  {value}  >", FG
    if kind in ("editor", "grid", "list"):
        return f"{value}  >", FG
    if kind == "info":
        return value, MUTED
    return value, FG


def draw_frame(d, f, title, tabs, active_tab, footer_help):
    # panel
    d.rectangle([28, 24, W - 28, H - 24], fill=PANEL, outline=LINE)
    # tab rail
    x = 48
    for i, name in enumerate(tabs):
        w = d.textlength(name, font=f["tab"]) + 28
        if i == active_tab:
            d.rectangle([x, 40, x + w, 76], fill=SEL)
            d.text((x + 14, 47), name, font=f["tab"], fill=FG)
        else:
            d.text((x + 14, 47), name, font=f["tab"], fill=MUTED)
        x += w + 10
    d.text((W - 60 - d.textlength(title, font=f["s"]), 52), title, font=f["s"], fill=MUTED)
    d.line([48, 88, W - 48, 88], fill=LINE)
    # help bar
    d.line([48, H - 92, W - 48, H - 92], fill=LINE)
    d.text((56, H - 82), footer_help, font=f["s"], fill=FG)
    d.text((56, H - 56), "L/R page   D-pad move   A change / open   B back   X reset page", font=f["xs"], fill=MUTED)


def render_page(pi, selected_group=0):
    name, groups = PAGES[pi]
    img = background()
    d = ImageDraw.Draw(img, "RGBA")
    f = fonts()
    tabs = [p[0] for p in PAGES]
    first_item = groups[selected_group][2][0][0]
    draw_frame(d, f, "TrainingRooms", tabs, pi, HELP.get(first_item, f"{first_item}: {groups[selected_group][2][0][2]}"))
    # groups column
    y = 104
    for gi, (gname, adv, items) in enumerate(groups):
        label = f"{gname}  ({len(items)})"
        if gi == selected_group:
            d.rectangle([48, y - 6, 372, y + 26], fill=HILITE)
            d.rectangle([48, y - 6, 52, y + 26], fill=SEL)
        d.text((62, y), label, font=f["h"] if gi == selected_group else f["t"], fill=FG if gi == selected_group else MUTED)
        if adv:
            d.text((62, y + 24), "advanced - collapsed by default", font=f["xs"], fill=ADV)
            y += 20
        y += 40
    d.line([388, 96, 388, H - 100], fill=LINE)
    # items
    gname, adv, items = groups[selected_group]
    y = 104
    for ii, (label, kind, value, origin) in enumerate(items):
        if ii == 0:
            d.rectangle([404, y - 6, W - 48, y + 26], fill=HILITE)
            d.rectangle([404, y - 6, 408, y + 26], fill=SEL)
        d.text((418, y), label, font=f["t"], fill=FG)
        tag = {"ours": ("ours", OURS), "up": ("upstream", UP), "mixed": ("upstream+ours", MUTED)}[origin]
        d.text((418 + d.textlength(label, font=f["t"]) + 14, y + 4), tag[0], font=f["xs"], fill=tag[1])
        text, col = control_text(kind, value)
        d.text((W - 60 - d.textlength(text, font=f["t"]), y), text, font=f["t"], fill=col)
        y += 38
        if y > H - 120:
            d.text((418, y), "...", font=f["t"], fill=MUTED)
            break
    return img


def render_subscreen(si):
    title, crumb, rows, help_text = SUBSCREENS[si]
    img = background()
    d = ImageDraw.Draw(img, "RGBA")
    f = fonts()
    draw_frame(d, f, "TrainingRooms", [crumb.split(" > ")[0]], 0, help_text)
    d.text((48, 100), f"{crumb}  >  {title}", font=f["h"], fill=FG)
    y = 146
    cols = 2 if any(k == "check" for _, k, _, _ in rows) and len(rows) > 8 else 1
    colw = (W - 96) // cols
    for i, (label, kind, value, _) in enumerate(rows):
        cx = 48 + (i % cols) * colw if kind == "check" else 48
        cy = y if kind != "check" else y
        if kind == "check":
            d.rectangle([cx + 10, cy + 2, cx + 30, cy + 22], outline=FG, width=2)
            if value == "x":
                d.rectangle([cx + 14, cy + 6, cx + 26, cy + 18], fill=SEL)
            d.text((cx + 44, cy), label, font=f["t"], fill=FG)
            if cols == 1 or i % cols == cols - 1:
                y += 36
        else:
            d.text((cx + 10, cy), label, font=f["t"], fill=FG)
            text, col = control_text("drop" if kind == "drop" else "toggle" if kind == "toggle" else "info", value)
            if kind == "chord":
                text, col = f"[ {value} ]   press A to re-bind", FG
            d.text((W - 60 - d.textlength(text, font=f["t"]), cy), text, font=f["t"], fill=col)
            y += 40
    return img


def svg_from(img_name, pi=None, si=None):
    """A text SVG with the same layout (rects + text), for scaling and editing."""
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" font-family="DejaVu Sans Mono, monospace">',
           f'<defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="rgb{BG0}"/><stop offset="1" stop-color="rgb{BG1}"/></linearGradient></defs>',
           f'<rect width="{W}" height="{H}" fill="url(#g)"/><rect y="560" width="{W}" height="40" fill="rgb(52,58,72)"/>',
           f'<rect x="28" y="24" width="{W-56}" height="{H-48}" fill="rgb(10,12,16)" fill-opacity="0.89" stroke="rgb{LINE}"/>']
    esc = lambda s: s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    def text(x, y, s, size=20, fill=FG, bold=False, anchor="start"):
        out.append(f'<text x="{x}" y="{y + size * 0.8:.0f}" font-size="{size}" fill="rgb{fill}" font-weight="{"bold" if bold else "normal"}" text-anchor="{anchor}">{esc(s)}</text>')
    tabs = [p[0] for p in PAGES] if pi is not None else [SUBSCREENS[si][1].split(" > ")[0]]
    x = 48
    for i, name in enumerate(tabs):
        w = len(name) * 13.3 + 28
        if (pi is not None and i == pi) or (si is not None and i == 0):
            out.append(f'<rect x="{x}" y="40" width="{w:.0f}" height="36" fill="rgb{SEL}"/>')
            text(x + 14, 47, name, 22, FG, True)
        else:
            text(x + 14, 47, name, 22, MUTED, True)
        x += w + 10
    text(W - 60, 52, "TrainingRooms", 16, MUTED, anchor="end")
    out.append(f'<line x1="48" y1="88" x2="{W-48}" y2="88" stroke="rgb{LINE}"/><line x1="48" y1="{H-92}" x2="{W-48}" y2="{H-92}" stroke="rgb{LINE}"/>')
    if pi is not None:
        name, groups = PAGES[pi]
        y = 104
        for gi, (gname, adv, items) in enumerate(groups):
            if gi == 0:
                out.append(f'<rect x="48" y="{y-6}" width="324" height="32" fill="white" fill-opacity="0.09"/><rect x="48" y="{y-6}" width="4" height="32" fill="rgb{SEL}"/>')
            text(62, y, f"{gname}  ({len(items)})", 20, FG if gi == 0 else MUTED, gi == 0)
            if adv:
                text(62, y + 24, "advanced - collapsed by default", 14, ADV); y += 20
            y += 40
        out.append(f'<line x1="388" y1="96" x2="388" y2="{H-100}" stroke="rgb{LINE}"/>')
        y = 104
        for ii, (label, kind, value, origin) in enumerate(groups[0][2]):
            if ii == 0:
                out.append(f'<rect x="404" y="{y-6}" width="{W-452}" height="32" fill="white" fill-opacity="0.09"/><rect x="404" y="{y-6}" width="4" height="32" fill="rgb{SEL}"/>')
            text(418, y, label, 20, FG)
            tag = {"ours": ("ours", OURS), "up": ("upstream", UP), "mixed": ("upstream+ours", MUTED)}[origin]
            text(418 + len(label) * 12.1 + 14, y + 4, tag[0], 14, tag[1])
            t, col = control_text(kind, value)
            text(W - 60, y, t, 20, col, anchor="end")
            y += 38
            if y > H - 120:
                text(418, y, "...", 20, MUTED); break
        first = groups[0][2][0]
        text(56, H - 82, HELP.get(first[0], f"{first[0]}: {first[2]}"), 16, FG)
    else:
        title, crumb, rows, help_text = SUBSCREENS[si]
        text(48, 100, f"{crumb}  >  {title}", 20, FG, True)
        y = 146
        cols = 2 if sum(1 for r in rows if r[1] == "check") > 8 else 1
        colw = (W - 96) // cols
        for i, (label, kind, value, _) in enumerate(rows):
            if kind == "check":
                cx = 48 + (i % cols) * colw
                out.append(f'<rect x="{cx+10}" y="{y+2}" width="20" height="20" fill="none" stroke="rgb{FG}" stroke-width="2"/>')
                if value == "x":
                    out.append(f'<rect x="{cx+14}" y="{y+6}" width="12" height="12" fill="rgb{SEL}"/>')
                text(cx + 44, y, label, 20, FG)
                if cols == 1 or i % cols == cols - 1:
                    y += 36
            else:
                text(58, y, label, 20, FG)
                t, col = (f"[ {value} ]   press A to re-bind", FG) if kind == "chord" else control_text(kind, value)
                text(W - 60, y, t, 20, col, anchor="end")
                y += 40
        text(56, H - 82, help_text, 16, FG)
    text(56, H - 56, "L/R page   D-pad move   A change / open   B back   X reset page", 14, MUTED)
    out.append("</svg>")
    return "\n".join(out)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    index = ["# Menu mockup screens (proposal v2, in-game look, 1280x720)", "",
             "Rendered by `tools/render_menu_mockup.py` from MENU-DRAFT.md §2 (the CPU-pilot rows are omitted: the standalone pilot has its own page). Each screen shows the first group selected; the help bar shows the focused item's text.", ""]
    n = 1
    for pi, (name, _) in enumerate(PAGES):
        slug = f"{n:02d}-{name.lower().replace(' & ', '-').replace(' ', '-')}"
        render_page(pi).convert("RGB").save(OUT / f"{slug}.png")
        (OUT / f"{slug}.svg").write_text(svg_from(slug, pi=pi), encoding="utf-8")
        index.append(f"- `{slug}.png` / `.svg` — page {name}")
        n += 1
    for si, (title, crumb, _, _) in enumerate(SUBSCREENS):
        slug = f"{n:02d}-{title.lower().replace(' ', '-')}"
        render_subscreen(si).convert("RGB").save(OUT / f"{slug}.png")
        (OUT / f"{slug}.svg").write_text(svg_from(slug, si=si), encoding="utf-8")
        index.append(f"- `{slug}.png` / `.svg` — sub-screen {title} ({crumb})")
        n += 1
    (OUT / "index.md").write_text("\n".join(index) + "\n", encoding="utf-8")
    print(f"wrote {2 * (n - 1)} files to {OUT}")


if __name__ == "__main__":
    main()
