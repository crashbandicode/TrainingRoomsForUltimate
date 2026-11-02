"""Package-5 helpers over `ally_ops.py` (run from the Surface): key sequences into the in-game menu and one PNG per call
(Eden's own recorder, last frame of a short GIF: renderer read-back, independent of the desktop).

    import s5_ally as s
    s.keys(phase, "b+up")            # hold B, press D-pad Up (the permanent opening chord)
    s.keys(phase, "down down a")     # a list of presses, one after the other
    s.shot(phase, "name")            # -> /tmp/tr-pkg5/shots/<phase>/<name>.png (1280 wide)
    s.diag(phase, regex)             # new matching lines of the plugin's diag log
Tokens of `keys`: a button name (`a b x y l r zl zr up down left right plus minus lstick`), `x+y` for a chord (all held 150 ms),
`wN` for N tenths of a second of waiting.
"""
import json
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ally_ops as ao

SHOTS = Path("/tmp/tr-pkg5/shots")
DIAG = "TrainingRooms/practice-diagnostics.log"


def run(phase, items, timeout=300):
    out = ao.ops(phase, items, timeout)
    try:
        return json.loads(out[out.index("["):])
    except ValueError:
        raise SystemExit(f"ops failed: {out[-600:]}")


def ops_for(tokens):
    items = []
    for token in tokens.split():
        if re.fullmatch(r"w\d+", token):
            items.append(["sleep", int(token[1:]) / 10])
        elif "+" in token:
            items.append(["hold", token.split("+"), 150, 0.35])
        else:
            items.append(["button", token, 120, 0.3])
    return items


def keys(phase, tokens, timeout=300):
    return run(phase, ops_for(tokens), timeout)


def shot(phase, name, width=1280):
    from PIL import Image
    run(phase, [["gshot", name, 2, width, 700]])
    out = SHOTS / phase
    out.mkdir(parents=True, exist_ok=True)
    gif = ao.get(phase, f"shots/{name}.gif", str(out))
    image = Image.open(gif)
    image.seek(image.n_frames - 1)
    png = out / f"{name}.png"
    image.convert("RGB").save(png)
    return png


def diag(phase, regex="."):
    import ally_log
    log = ally_log.fetch(phase.replace("live-", ""))
    pattern = re.compile(regex)
    lines = log.read_text(encoding="utf-8", errors="replace").splitlines() if log.exists() else []
    return [line for line in lines if pattern.search(line)]
