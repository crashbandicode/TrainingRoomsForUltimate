"""One-picture summary of package 1 from the existing Eden screenshots and logs (Pillow only).

Run: /usr/bin/python3 -I eden/make_summary.py   (writes eden/package-1-summary.png, 1280 wide)
Sources (all under eden/runs/): tr105 events.log + jump-scan, tr101 hazard/LRA shots, tr102 hazards-on shot,
tr104 jump-scan (delay 0 baseline). NRO sha256 (first 8): release 6d644384 (tr101, tr102, tr104),
release 9bad6541 (tr105, after the D1-D4 fixes).
"""
import textwrap
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
RUNS = HERE / "runs"
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
BOLD = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
MONO = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
W, PW, PH = 1280, 640, 410
BG, PANEL, INK, DIM, ACC = (18, 20, 26), (30, 33, 42), (235, 238, 245), (160, 168, 184), (255, 207, 90)


def font(path, size):
    return ImageFont.truetype(path, size)


def crop(path, box, width, label=None):
    im = Image.open(path).convert("RGB").crop(box)
    im = im.resize((width, round(im.height * width / im.width)), Image.LANCZOS)
    if label:
        d = ImageDraw.Draw(im)
        f = font(BOLD, 14)
        tw = d.textlength(label, font=f)
        d.rectangle((0, 0, tw + 12, 22), fill=(0, 0, 0))
        d.text((6, 3), label, font=f, fill=ACC)
    return im


def panel(canvas, col, row, title, caption):
    x, y = col * PW, 56 + row * PH
    d = ImageDraw.Draw(canvas)
    d.rectangle((x + 6, y + 6, x + PW - 6, y + PH - 6), fill=PANEL)
    d.text((x + 16, y + 12), title, font=font(BOLD, 18), fill=INK)
    for i, part in enumerate(textwrap.wrap(" ".join(caption), 92)[:3]):
        d.text((x + 16, y + PH - 58 + 15 * i), part, font=font(FONT, 11), fill=DIM)
    return x + 16, y + 44


def main():
    canvas = Image.new("RGB", (W, 56 + 2 * PH), BG)
    d = ImageDraw.Draw(canvas)
    d.text((16, 10), "TrainingRoomsForUltimate, package 1: Eden results", font=font(BOLD, 24), fill=INK)
    d.text((16, 38), "Smash Ultimate 13.0.5, Eden on the Yoga, 2026-11-02. Frames are real screenshots from the runs named in each caption.",
           font=font(FONT, 12), fill=DIM)

    # 1. install line (tr105)
    x, y = panel(canvas, 0, 0, "1  Install line (release)",
                 ("tr105, release NRO 9bad6541. Tick, per-feature state and the settings read are separate fields.",
                  "Every one of the ten sites reports stock; heap_allocations=0."))
    line = (RUNS / "tr105/events.log").read_text().strip()
    tick, rest = line.split(" ", 1)
    sites = [t for t in rest.split() if "=stock/" in t]
    feats = [t for t in rest.split() if t.endswith("=installed") and not t.startswith("result")]
    settings = rest.split("settings: ")[1].split(" result=")[0]
    head = rest.split(" final_input_mapping")[0]
    tail = rest[rest.index(" result="):].strip()
    mono = font(MONO, 12)
    rows = [("tick", tick, ACC), ("identity", head, INK), ("sites", f"{len(sites)} x <site>=stock/<far|near|checked>", INK),
            ("features", " ".join(feats), (140, 220, 150)), ("settings", settings, (120, 190, 255)), ("result", tail, INK)]
    yy = y
    for key, val, color in rows:
        d.text((x, yy), key, font=font(BOLD, 13), fill=DIM)
        for part in textwrap.wrap(val, 52):
            d.text((x + 80, yy), part, font=mono, fill=color)
            yy += 16
        yy += 12
    d.text((x, yy + 4), "was: tick 0, input_delay=on, no settings field, empty diagnostics file", font=font(FONT, 12), fill=DIM)

    # 2. hazards off vs on
    x, y = panel(canvas, 1, 0, "2  Stage Hazards on Pokemon Stadium 2",
                 ("Off: tr101 (release 6d644384), still Stadium form 6 min 40 s in. On: tr102 (release 6d644384),",
                  "the stage changed form within 45 s and repeatedly after (ice form shown)."))
    box = (40, 40, 1240, 674)
    a = crop(RUNS / "tr101/shots/013330-hazard-check-3.png", box, 300, "Hazards Off (tr101)")
    b = crop(RUNS / "tr102/shots/014853-now.png", box, 300, "Hazards On (tr102)")
    canvas.paste(a, (x, y))
    canvas.paste(b, (x + 310, y))
    d.text((x, y + a.height + 8), "Same stage and setup, only the setting differs; in Off no form change in all screenshots.",
           font=font(FONT, 12), fill=INK)

    # 3. input delay
    x, y = panel(canvas, 0, 1, "3  Input Delay 5: jump pressed at macro frame 60",
                 ("delay 0: tr104 (release 6d644384). delay 5: tr105 (release 9bad6541). Continuous runs, paused at the shown frame;",
                  "Delay 5 also gave onset 74 in tr101 (6d644384) and tr103 (diag 82f60e24)."))
    box = (110, 140, 400, 480)
    frames = [("tr104 delay 0, frame 69", RUNS / "tr104/probes/jump-scan/09-f069.png"),
              ("tr105 delay 5, frame 69", RUNS / "tr105/probes/jump-scan/07-f069.png"),
              ("tr105 delay 5, frame 74", RUNS / "tr105/probes/jump-scan/12-f074.png")]
    xx = x
    for label, p in frames:
        im = crop(p, box, 196, label)
        canvas.paste(im, (xx, y))
        xx += 204
    ty = y + 196 * 340 // 290 + 12
    d.line((x, ty + 22, x + 600, ty + 22), fill=DIM, width=2)
    for f, c, name in ((60, DIM, "press"), (69, (140, 220, 150), "onset, delay 0"), (74, ACC, "onset, delay 5")):
        px = x + round((f - 56) / (78 - 56) * 600)
        d.line((px, ty + 14, px, ty + 30), fill=c, width=3)
        d.text((px - 30, ty + 33), f"{f} {name}", font=font(FONT, 11), fill=c)
    d.text((x, ty - 4), "onset shifts by exactly 5 frames", font=font(BOLD, 12), fill=INK)

    # 4. L+R+A
    x, y = panel(canvas, 1, 1, "4  L+R+A Reset (setting On)",
                 ("tr101 (release 6d644384): walk right, hold L+R+A, back at spawn. Same result in tr103 (diag).",
                  "With the setting Off (tr102) Steve stayed where he was. tr106 diag: hub_run input_delay_cleared=5 on each reset."))
    box = (40, 200, 740, 520)
    frames = [("start", "tr101/shots/012903-lra0-start.png"), ("walked right", "tr101/shots/012906-lra1-moved.png"),
              ("after L+R+A", "tr101/shots/012913-lra2-after-chord.png")]
    xx = x
    for label, p in frames:
        im = crop(RUNS / p, box, 196, label)
        canvas.paste(im, (xx, y))
        xx += 204
    d.text((x, y + 196 * 320 // 700 + 14), "Third frame matches the first: the game's reset ran.", font=font(FONT, 12), fill=INK)

    out = HERE / "package-1-summary.png"
    canvas.convert("P", palette=Image.ADAPTIVE, colors=256).save(out, optimize=True)
    print(out, canvas.size, out.stat().st_size)


main()
