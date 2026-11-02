"""The route to a Training room: boot -> title -> main menu -> Training stage select -> a stage by tile cursor ->
character select -> Training. One module for every stream (and for the Ally).

Playbook origin: docs/MACROS.md "Hybrid: one macro per segment, a check between" and docs/TILE-CURSOR.md (L3 +
D-pad steps, the `settled screen=sss at=N` gate), with tools/eden-solo/examples/css_check.py's idea of a cheap
screenshot marker. Replaces route4.py, route_local.py, route_r2.py, route_s3.py, route_s5.py, route_w.py,
route_s2.py, route_s6.py, u4/route_u4.py, route_hb.py, route_f6.py, route_dx.py and ally/ally_route.py (they
differ in the work root, profile, harness file name, ssh control path, a tile or two, and the two gates below).

Every wait is gated on a log line or a screenshot match against a reference screenshot of the same screen
(`refs/`, gitignored), with a cap; on a miss the route stops with exit 2 and leaves the last screenshot, it never
presses on onto a screen it has not seen. The macro segment is played by `gv.py macro`, which keeps the file's
sha256 with the run. Where the instance runs decides the transport: `local` (this host runs Eden, in-process),
`ssh` (another host does; every command is one `gv.py` call over a multiplexed ssh) or `ally` (the Windows rig,
ally/client.py).

    route.py <label> [--stage-steps right:6] [--tile-gate 6] [--p1-steps up] [--stage-ref sss-t41.png]
                     [--install-gate TEXT] [--training-gate shot|events:TEXT|diag:TEXT] [--from-menu]
                     [--transport local|ssh|ally]
Importable by row scripts: `s = route.session(label)` gives press, stick, shot, wait_line, wait_match, macro.
"""
import argparse
import base64
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import params  # noqa: E402

# screen -> (reference screenshot, crop box, mean-difference limit, cap in seconds); the screens of the route
GATES = {
    "main menu": ("menu.png", (40, 60, 620, 380), 30, 0),
    "stage select": ("sss-t0.png", (440, 50, 1240, 660), 14, 40),
    "stage name": ("sss-t41.png", (45, 205, 435, 515), 8, 15),
    "character select": ("css.png", (55, 50, 1225, 420), 12, 40),
    "ready to fight": ("ready.png", (40, 40, 1240, 330), 25, 25),
    "training scene": ("training.png", (100, 200, 430, 500), 30, 90),
}
MACRO = "tr-main-to-stage-select.edenmacro.json"


class Transport:
    """What the route needs from the instance: input, a screenshot as a local PNG, plugin log lines, a macro."""

    def press(self, buttons, hold=150):
        raise NotImplementedError

    def stick(self, x, y, ms):
        raise NotImplementedError

    def shot(self, name):
        raise NotImplementedError

    def lines(self, name, after=0):
        raise NotImplementedError

    def macro(self, name, cap=120):
        raise NotImplementedError

    def call(self, method, params_=None):
        raise NotImplementedError


class Local(Transport):
    """This host runs Eden: gv.py's functions in-process."""

    def __init__(self, p, label):
        import gv
        self.gv, self.p, self.label = gv, gv.configure(p), label

    def press(self, buttons, hold=150):
        for b in buttons.split(","):
            self.gv.client_call(self.label, "input.button", {"button": b, "state": "press", "lease_ms": hold + 500})
        time.sleep(hold / 1000)
        for b in buttons.split(","):
            self.gv.client_call(self.label, "input.button", {"button": b, "state": "release"})

    def stick(self, x, y, ms):
        self.gv.client_call(self.label, "input.stick", {"stick": "left", "x": x, "y": y, "lease_ms": ms})
        time.sleep(ms / 1000)

    def shot(self, name):
        return self.gv.take_shot(self.label, name)

    def lines(self, name, after=0):
        return self.gv.read_lines(name)[after:]

    def macro(self, name, cap=120):
        return self.gv.play_macro(self.label, self.p.macros_dir / name, cap)

    def call(self, method, params_=None):
        return self.gv.client_call(self.label, method, params_)


class Ssh(Transport):
    """Another host runs Eden (the Yoga, driven from the Surface): one gv.py call per command over a multiplexed
    ssh. The harness folder, its config (deploy.py) and the macros are in <root> on that host."""

    def __init__(self, p, label):
        self.p, self.label = p, label
        self.control = f"/tmp/trssh-{p.prefix}-%C"
        self.ssh = ["/usr/bin/ssh", "-o", "BatchMode=yes", "-o", "ControlMaster=auto", "-o", f"ControlPath={self.control}",
                    "-o", "ControlPersist=900", p.info["ssh"]]

    def gv(self, args, timeout=120):
        cmd = f"/usr/bin/python3 -I {self.p.root}/harness/gv.py {args}"
        r = subprocess.run([*self.ssh, cmd], capture_output=True, text=True, timeout=timeout)
        if r.returncode != 0:
            raise RuntimeError(f"gv {args} failed: {r.stdout.strip()} {r.stderr.strip()}")
        return r.stdout.strip()

    def press(self, buttons, hold=150):
        self.gv(f"press {self.label} {buttons} {hold}")

    def stick(self, x, y, ms):
        self.gv(f"stick {self.label} {x} {y} {ms}")

    def shot(self, name):
        out = HERE.parent / "runs" / self.label / "shots"
        out.mkdir(parents=True, exist_ok=True)
        remote = self.gv(f"shot {self.label} {name}").splitlines()[-1].strip()
        subprocess.run(["/usr/bin/scp", "-q", "-o", f"ControlPath={self.control}", f"{self.p.info['ssh']}:{remote}", str(out)],
                       check=True)
        return out / Path(remote).name

    def lines(self, name, after=0):
        text = self.gv(f"lines {name} --after {after}")
        return text.splitlines() if text else []

    def macro(self, name, cap=120):
        return json.loads(self.gv(f"macro {self.label} {self.p.root}/macros/{name} --timeout {cap}", cap + 30).splitlines()[-1])

    def call(self, method, params_=None):
        b64 = base64.b64encode(json.dumps(params_ or {}).encode()).decode()
        return json.loads(self.gv(f"call {self.label} {method} b64:{b64}").splitlines()[-1])


def dist(path, ref, box):
    """Mean grey difference of the box between a screenshot and the reference (Pillow, imported late: the Eden
    host may have none, the host that routes does)."""
    from PIL import Image, ImageChops, ImageStat
    a = Image.open(path).convert("L").crop(box)
    b = Image.open(ref).convert("L").crop(box)
    return ImageStat.Stat(ImageChops.difference(a, b)).mean[0]


class Session:
    """The gated primitives, shared by the route and by row scripts."""

    def __init__(self, t, refs, sleep=time.sleep, clock=time.time):
        self.t, self.refs, self.sleep, self.clock = t, Path(refs), sleep, clock

    def fail(self, what, detail):
        print(f"GATE FAIL {what} {detail}", flush=True)
        raise SystemExit(2)

    def wait_line(self, name, needle, cap, what, after=0):
        end = self.clock() + cap
        while self.clock() < end:
            for line in self.t.lines(name, after):
                if needle in line:
                    print(f"GATE ok {what}: {line[:170]}", flush=True)
                    return line
            self.sleep(1)
        self.fail(what, f"no line with {needle!r} in {name}")

    def snap(self, name):
        """A screenshot, or None: the first one after a load can find no render window yet."""
        try:
            return self.t.shot(name)
        except (RuntimeError, SystemExit, subprocess.CalledProcessError, StopIteration):
            return None

    def wait_match(self, screen, cap=None, ref=None):
        name, box, limit, default_cap = GATES[screen]
        end, best = self.clock() + (cap or default_cap), 999.0
        while self.clock() < end:
            png = self.snap(screen.replace(" ", "-"))
            if png is None:
                self.sleep(1.5)
                continue
            d = dist(png, self.refs / (ref or name), box)
            best = min(best, d)
            if d <= limit:
                print(f"GATE ok {screen} dist={d:.1f} (limit {limit}) {Path(png).name}", flush=True)
                return png
            self.sleep(1.0)
        self.fail(screen, f"best_dist={best:.1f} limit={limit}")

    def press(self, buttons, hold=150):
        self.t.press(buttons, hold)

    def stick(self, x, y, ms):
        self.t.stick(x, y, ms)

    def macro(self, name, cap=120):
        return self.t.macro(name, cap)


def parse_steps(text):
    """`right:6,down:3` -> six rights then three downs; a bare `up` is one step."""
    steps = []
    for part in filter(None, text.split(",")):
        direction, _, count = part.partition(":")
        steps += [direction] * int(count or 1)
    return steps


def plan(args):
    """The route as data: what it presses and which gates it waits on, in order (host-tested). Tile steps carry
    their pause: 0.5 s on the stage select, 0.6 s on the character select (the hand eases over about 8 frames,
    docs/TILE-CURSOR.md)."""
    out = [("wait_line", "events", args.install_gate, 240, False)]
    if not args.from_menu:
        out.append(("main menu",))
    out += [("macro", MACRO), ("wait_match", "stage select")]
    out += [("tile", d, 0.5) for d in parse_steps(args.stage_steps)]
    if args.tile_gate is not None:
        out.append(("wait_line", "tile", f"settled screen=sss at={args.tile_gate}", 10, False))
    if args.stage_ref:
        out.append(("wait_match", "stage name", args.stage_ref))
    out += [("press", "a"), ("wait_match", "character select")]
    out += [("tile", d, 0.6) for d in parse_steps(args.p1_steps)]
    out += [("press", "a"), ("wait_match", "ready to fight"), ("press", "a")]
    kind, _, needle = args.training_gate.partition(":")
    # A plugin line instead of the screenshot: the stage on another host's profile need not be the reference's.
    out.append(("wait_line", kind, needle, 90, True) if needle else ("wait_match", "training scene"))
    return out


def to_training(s, args):
    """Walk `plan(args)`; the tile gate reads only the log lines written after the stage select appeared."""
    mark = 0
    for step in plan(args):
        kind = step[0]
        if kind == "wait_line":
            s.wait_line(step[1], step[2], step[3], f"{step[1]}: {step[2]}", mark if step[1] == "tile" else 0)
            if step[4]:
                s.sleep(2)
                s.snap("training-scene")
        elif kind == "main menu":
            to_main_menu(s)
        elif kind == "macro":
            print("macro", json.dumps(s.macro(step[1], 90)), flush=True)
        elif kind == "wait_match":
            s.wait_match(step[1], ref=step[2] if len(step) > 2 else None)
            if step[1] == "stage select":
                mark = len(s.t.lines("tile"))
        elif kind == "tile":
            s.press(f"lstick,{step[1]}")
            s.sleep(step[2])
        elif kind == "press":
            s.press(step[1])
    print("ROUTE DONE", flush=True)


def to_main_menu(s):
    """Title -> main menu: A until the menu's reference matches (up to 10 presses), then a pause."""
    menu, box, limit, _ = GATES["main menu"]
    for attempt in range(10):
        s.press("a")
        s.sleep(3)
        png = s.snap("after-a")
        if png is None:
            s.sleep(3)
            continue
        d = dist(png, s.refs / menu, box)
        print(f"after A#{attempt + 1}: menu dist {d:.1f}", flush=True)
        if d <= limit:
            print("GATE ok main menu", flush=True)
            s.sleep(1.5)
            return
    s.fail("main menu", "never matched")

def session(label, p=None, transport=None):
    p = p or params.load()
    params.label_ok(p, label)
    kind = transport or ("ally" if p.host == "ally" else "local" if _here(p) else "ssh")
    if kind == "ally":
        sys.path.insert(0, str(HERE / "ally"))
        import client
        t = client.AllyTransport(p, label)
    else:
        t = {"local": Local, "ssh": Ssh}[kind](p, label)
    return Session(t, p.refs_dir)


def _here(p):
    return params.machine() == p.host


def build_parser():
    ap = argparse.ArgumentParser()
    ap.add_argument("label")
    ap.add_argument("--stage-steps", default="right:6", help="direction[:count][,...] from the first tile")
    ap.add_argument("--tile-gate", type=int, default=6, help="tile the stage pick must settle on (log gate)")
    ap.add_argument("--p1-steps", default="up", help="directions after the first L3 step (the hand starts below the grid)")
    ap.add_argument("--stage-ref", help="reference screenshot of the chosen stage's name (e.g. sss-t41.png)")
    ap.add_argument("--install-gate", help="text of the plugin's install line (default: TR_INSTALL_GATE)")
    ap.add_argument("--training-gate", default="shot", help="shot, or events:TEXT / diag:TEXT: a plugin log line")
    ap.add_argument("--from-menu", action="store_true", help="already on the main menu")
    ap.add_argument("--transport", choices=("local", "ssh", "ally"))
    return ap


def main(argv=None):
    args = build_parser().parse_args(argv)
    p = params.load()
    args.install_gate = args.install_gate or p.install_gate
    to_training(session(args.label, p, args.transport), args)


if __name__ == "__main__":
    main()

