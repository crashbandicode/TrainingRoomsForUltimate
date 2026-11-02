"""Package-5 Eden harness for TrainingRoomsForUltimate (runs ON the Yoga): `eden/tr_eden.py` (package 1) with the\nstream-5 work root, profile `t5`, display :95 and labels `s5-NN`.

Adapted from the playbook's tools/eden-solo/gv.py + solo.ps1 (playbook 53fd1f2): the private profile
is a hard-linked copy of the Yoga source profile b (read-only game content hard-linked, everything
else copied; never a profile named a); only the copy is modified. Differences from gv.py:
  * own work root, profile name, owner id and display (never profile a, never display :61);
  * the installed set is exactly the NROs given (anything else in plugins/ is set aside);
  * `settings.json` is written into the copy's sd:/ultimate/TrainingRooms/ before boot;
  * core.volume=0 is set through Eden's control API and saved before the game boots (operator
    rule 2026-10-09) and is also kept in the profile's qt-config.ini;
  * claim before Up and release after Down (also after a failed Up) in the Yoga claim ledger.

Usage: tr_eden.py up <label> [--nro p ...] [--settings json-file] [--diag-txt file]
       tr_eden.py down <label>
       tr_eden.py shot|press|stick|call|tree <label> ...
       tr_eden.py scan <label>        (console.log: panic/abort/crash/Unmapped)
"""
import argparse
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path("/home/intpa/trainingrooms-eden-p5")
SOURCE = Path("/home/intpa/ssbu-desync-1029/profiles/b")
PROFILE = ROOT / "profiles" / "t5"
APP = Path("/home/intpa/Applications/Eden-GIF/current/Eden.AppImage")
RUNTIME = Path("/home/intpa/elite-rematch-work/eden-lan-runtime")
RUNTIME_SHA = "ee3c2e953a0b87541c29eb8918ce0354489035296ac5eaf060b3444d06a8bbcb"
COORD = Path("/tmp/deluxe-surface-1007/OWNER-COORDINATION.md")
HELPER = "/home/intpa/eden-nso-autolan/live-tools/display-rootful.ps1"
GAME = ("/home/intpa/Games/Roms/Switch/Super Smash Bros. Ultimate "
        "[01006A800016E000]+[v13.0.3+ 9DLC][US](nsw2u.com).xci")
OWNER = "Claude 0caa72b8/pkg5-eden"
DISPLAY = ":95"
READ_ONLY = ["user/nand/user/Contents", "user/nand/system/Contents", "user/firmware"]
PLUGINS = Path("user/sdmc/atmosphere/contents/01006A800016E000/romfs/skyline/plugins")
OUTPUT = Path("user/sdmc/ultimate/TrainingRooms")
CONFIG = Path("user/config/qt-config.ini")
FORCED = {"pauseWhenInBackground": "false", "volume": "0", "volume\\default": "false"}
SHOTS = ROOT / "shots"
sys.path.insert(0, str(APP.parent / "tools"))


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def label_ok(label):
    assert re.fullmatch(r"s5-\d{2}", label), f"bad label: {label}"
    return label


def run_dir(label):
    return ROOT / "runs" / label


def stamp():
    return time.strftime("%Y-%m-%d %H:%M", time.gmtime()) + "Z"


def ledger(line):
    with COORD.open("a", encoding="utf-8") as f:
        f.write(f"\n{stamp()} {line}\n")


HOST = "yoga"          # this harness runs ON the Yoga; the ledger names the host a claim is on
STALE_CLAIM_HOURS = 12


def open_claims():
    """Open claims of other owners that concern this host (pkg6's host-aware rule, stream/pkg6 9360dfa). The host a claim is
    ON comes from its "on <host>" phrase; a line that only mentions another host in passing does not claim it; a line with
    no such phrase falls back to every host it names (conservative). A claim older than STALE_CLAIM_HOURS is stale noise."""
    import calendar
    state = {}
    now = time.time()
    for line in COORD.read_text(encoding="utf-8", errors="replace").splitlines():
        m = re.search(r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2})Z (CLAIMED|RELEASED):? (\S+ \S+)", line)
        if not m:
            continue
        stamped = calendar.timegm(time.strptime(m.group(1), "%Y-%m-%d %H:%M"))
        names = re.findall(r"\bon (?:the )?(surface|yoga|ally)\b", line, re.I) or re.findall(r"\b(surface|yoga|ally)\b", line, re.I)
        concerns = not names or HOST in {h.lower() for h in names}
        state[m.group(3)] = m.group(2) == "CLAIMED" and (now - stamped) / 3600 < STALE_CLAIM_HOURS and concerns
    return [k for k, v in state.items() if v and k != OWNER]

def mem_available_kib():
    return next(int(l.split()[1]) for l in Path("/proc/meminfo").read_text().splitlines()
                if l.startswith("MemAvailable:"))


def eden_processes():
    out = subprocess.run(["/usr/bin/pgrep", "-af", "Eden.AppImage|/eden( |$)|eden-run.py"],
                         capture_output=True, text=True).stdout
    return [l for l in out.splitlines() if "pgrep" not in l]


def prep(label, nros, settings, diag_txt, extras=()):
    receipt = {"label": label, "time": time.time(), "source": str(SOURCE), "profile": str(PROFILE),
               "set_aside": [], "archived": []}
    assert SOURCE.name != "a" and SOURCE.is_dir()
    if not PROFILE.exists():
        temp = PROFILE.with_name(f".{PROFILE.name}-{label}-tmp")
        assert not temp.exists(), f"temporary path exists: {temp}"
        temp.parent.mkdir(parents=True, exist_ok=True)
        skip = {SOURCE / rel for rel in READ_ONLY}

        def ignore(directory, names):
            return [n for n in names if n in ("profile.lock", "crash_dumps", "log")
                    or Path(directory) / n in skip]

        shutil.copytree(SOURCE, temp, symlinks=True, ignore=ignore)
        for rel in READ_ONLY:
            if (SOURCE / rel).is_dir():
                shutil.copytree(SOURCE / rel, temp / rel, symlinks=True, copy_function=os.link)
        text = (temp / CONFIG).read_text(encoding="utf-8").replace(str(SOURCE), str(PROFILE))
        (temp / CONFIG).write_text(text, encoding="utf-8")
        temp.rename(PROFILE)
        receipt["created"] = True
    lines = []
    for line in (PROFILE / CONFIG).read_text(encoding="utf-8").splitlines():
        key = line.split("=", 1)[0]
        if key == "nand_directory":
            line = f"nand_directory={PROFILE / 'user/nand'}"
        elif key == "sdmc_directory":
            line = f"sdmc_directory={PROFILE / 'user/sdmc'}"
        elif key in FORCED:
            line = f"{key}={FORCED[key]}"
        lines.append(line)
    (PROFILE / CONFIG).write_text("\n".join(lines) + "\n", encoding="utf-8")
    config = (PROFILE / CONFIG).read_text(encoding="utf-8")
    assert str(SOURCE) not in config and "/profiles/a/" not in config, "config points outside"
    assert "volume=0" in config.splitlines(), "config lacks volume=0"
    plugins = PROFILE / PLUGINS
    plugins.mkdir(parents=True, exist_ok=True)
    aside = plugins.parent / "plugins-set-aside-trainingrooms-p5"
    aside.mkdir(exist_ok=True)
    for path in sorted(plugins.glob("*.nro")):
        target = aside / f"{path.stem}.{label}-{int(time.time())}.nro"
        path.replace(target)
        receipt["set_aside"].append(target.name)
    for spec in nros:  # <path> or <path>:<installed name> (load order follows the file name)
        src, _, name = spec.partition(":")
        nro = Path(src)
        assert nro.suffix == ".nro" and nro.is_file(), f"missing NRO: {nro}"
        shutil.copyfile(nro, plugins / (name or nro.name))
    receipt["plugins"] = {p.name: digest(p) for p in sorted(plugins.glob("*.nro"))}
    assert not any("training_modpack" in p.name for p in plugins.glob("*.nro")), "fork NRO present"
    for rel in (OUTPUT, Path("user/sdmc/ultimate/ssbu-tile-cursor"), Path("user/sdmc/ultimate/ssbu-cpu-pilot")):
        previous = PROFILE / rel
        if previous.exists():
            archived = previous.with_name(f"{previous.name}.before-{label}")
            assert not archived.exists(), f"archive exists: {archived}"
            previous.replace(archived)
            receipt["archived"].append(str(archived))
    out = PROFILE / OUTPUT
    out.mkdir(parents=True)
    if settings:
        shutil.copyfile(settings, out / "settings.json")  # a real file, never a link
        receipt["settings"] = (out / "settings.json").read_text(encoding="utf-8")
    for spec in extras:  # NAME=PATH: a real file in the plugin's folder before boot (developer.ini, ...)
        name, _, path = spec.partition("=")
        shutil.copyfile(path, out / name)
        receipt.setdefault("extras", {})[name] = (out / name).read_text(encoding="utf-8")
    if diag_txt:
        shutil.copyfile(diag_txt, out / "diag.txt")
        receipt["diag_txt"] = Path(diag_txt).read_text(encoding="utf-8")
    ROOT.mkdir(parents=True, exist_ok=True)
    (ROOT / f"PREP-{label}.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def launch(label):
    run = run_dir(label)
    assert (PROFILE / "user").is_dir(), f"profile missing: {PROFILE}"
    assert not run.exists(), f"run exists: {run}"
    assert re.fullmatch(r":\d{1,3}", DISPLAY) and DISPLAY != ":61"
    assert Path(f"/tmp/.X11-unix/X{DISPLAY[1:]}").exists(), f"display not up: {DISPLAY}"
    mem = mem_available_kib()
    assert mem >= 10 * 1024 ** 2, f"need 10 GiB available, have {mem} KiB"
    assert digest(RUNTIME) == RUNTIME_SHA, "runtime hash mismatch"
    env = dict(os.environ, DISPLAY=DISPLAY, WAYLAND_DISPLAY="", XAUTHORITY="", QT_QPA_PLATFORM="xcb",
               XDG_DATA_HOME=str(PROFILE / "xdg-data"), XDG_CONFIG_HOME=str(PROFILE / "xdg-config"),
               XDG_CACHE_HOME=str(PROFILE / "xdg-cache"),
               EDEN_LOG_FILTER="*:Warning Frontend:Info Service.Sockets:Info Network:Info")
    for key in ("EDEN_NSO_LAN_CONFIG", "EDEN_LAN_RUNTIME", "EDEN_LAN_ASSETS", "EDEN_LAN_PROFILE_DIR",
                "EDEN_LAN_FIGHTER_SEEDS"):
        env.pop(key, None)
    (PROFILE / "ratings").mkdir(mode=0o700, exist_ok=True)
    child = ["/usr/bin/env", f"EDEN_LAN_RUNTIME={RUNTIME}", f"EDEN_LAN_PROFILE_DIR={PROFILE / 'ratings'}",
             str(APP)]
    (ROOT / "runs").mkdir(parents=True, exist_ok=True)
    command = ["/usr/bin/python3", str(APP.parent / "tools/eden-run.py"), "--memory-gib", "8",
               "--runtime", "3600", "--startup-timeout", "180", "--hang-timeout", "60",
               "--stall-timeout", "0", "--background", "--output", str(run), "--", *child]
    receipt_path = ROOT / f"LAUNCH-{label}.json"
    assert not receipt_path.exists(), f"receipt exists: {receipt_path}"
    with (ROOT / f"launch-{label}.log").open("x", encoding="utf-8") as log:
        process = subprocess.Popen(command, env=env, cwd=PROFILE, stdin=subprocess.DEVNULL,
                                   stdout=log, stderr=subprocess.STDOUT, start_new_session=True,
                                   close_fds=True)
    receipt = {"owner": OWNER, "label": label, "display": DISPLAY, "pid": process.pid,
               "app_sha256": digest(APP), "runtime": str(RUNTIME), "mem_available_kib": mem,
               "time": time.time(), "profile": str(PROFILE), "run": str(run)}
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    return receipt


def client_call(label, method, params=None, timeout=15):
    from edenctl import Client
    response = Client(str(run_dir(label) / "control.sock"), timeout=timeout).request(method, params or {})
    if not response.get("ok"):
        raise SystemExit(json.dumps(response))
    return response["result"]


def nodes(tree):
    if isinstance(tree, dict):
        if "class" in tree and "id" in tree:
            yield tree
        for key, value in tree.items():
            if key != "properties":
                yield from nodes(value)
    elif isinstance(tree, list):
        for value in tree:
            yield from nodes(value)


def shot(label, name, target=None):
    t = {"id": target} if target else next(
        n for n in nodes(client_call(label, "ui.tree", {"brief": True, "visible_only": True}))
        if n["class"] == "GRenderWindow")
    out = SHOTS / label
    out.mkdir(parents=True, exist_ok=True)
    path = out / f'{time.strftime("%H%M%S", time.gmtime())}-{name}.png'
    client_call(label, "ui.screenshot", {"path": str(path), "target": t["id"]}, timeout=30)
    return path


def up(args):
    label = args.label
    assert not eden_processes(), f"this host already runs Eden: {eden_processes()}"
    assert not Path(f"/tmp/.X11-unix/X{DISPLAY[1:]}").exists(), f"display {DISPLAY} in use"
    mem = mem_available_kib()
    assert mem >= 10 * 1024 ** 2, f"MemAvailable {mem} KiB < 10 GiB"
    free = shutil.disk_usage("/").free
    assert free >= 40 * 10 ** 9, f"df free {free} < 40 GB"
    busy = open_claims()
    if busy:
        print(json.dumps({"active_claims": busy}))
        sys.exit(3)
    nro_hashes = {n.partition(":")[2] or Path(n).name: digest(n.partition(":")[0]) for n in args.nro}
    ledger(f"CLAIMED: {OWNER} TrainingRooms package-5 Eden rows (menu rendering and assets), "
           f"solo run {label} on Yoga, display {DISPLAY}, own profile {PROFILE} (copy of "
           f"ssbu-desync-1029/profiles/b), plugins {nro_hashes}. No LAN. No physical Switch writes.")
    try:
        print("prep", json.dumps({k: v for k, v in prep(label, args.nro, args.settings, args.diag_txt, args.extra).items()
                                  if k in ("plugins", "set_aside", "archived", "settings", "diag_txt", "extras")}))
        subprocess.run(["/usr/bin/pwsh", "-NoLogo", "-NoProfile", "-NonInteractive", "-File", HELPER,
                        "-Action", "Start", "-Unit", f"trp5-{label}-display", "-Socket", f"wayland-trp5-{label}",
                        "-XDisplay", DISPLAY, "-MaxRuntimeSeconds", "3600"],
                       check=True, stdout=subprocess.DEVNULL)
        subprocess.run(["/usr/bin/systemd-run", "--user", "--quiet", f"--unit=trp5-{label}-gpu-lock",
                        "--property=RuntimeMaxSec=3600", "--property=TimeoutStopSec=5", "/usr/bin/flock",
                        "--nonblock", "/tmp/opencode/yoga-gpu-reservation.lock", "/usr/bin/sleep", "3600"],
                       check=True)
        time.sleep(1)
        active = subprocess.run(["/usr/bin/systemctl", "--user", "is-active", f"trp5-{label}-gpu-lock"],
                                capture_output=True, text=True).stdout.strip()
        assert active == "active", "Yoga GPU lock is held by someone else"
        print("launch", json.dumps(launch(label)))
        deadline = time.time() + 90
        while not (run_dir(label) / "control.sock").exists() and time.time() < deadline:
            time.sleep(2)
        # Operator rule 2026-10-09: muted, set through the CLI and persisted before the game boots.
        print("volume.set", json.dumps(client_call(label, "settings.set", {"key": "core.volume", "value": 0})))
        print("settings.save", json.dumps(client_call(label, "settings.save")))
        print("volume.get", json.dumps(client_call(label, "settings.get", {"key": "core.volume"})))
        print("game.load", json.dumps(client_call(label, "game.load", {"path": GAME}, timeout=60)))
    except BaseException as exc:
        print(f"UP FAILED: {exc!r}", file=sys.stderr)
        down(args, failed=True)
        raise


def down(args, failed=False):
    label = args.label
    receipt = ROOT / f"LAUNCH-{label}.json"
    note = {}
    if receipt.exists():
        r = json.loads(receipt.read_text(encoding="utf-8"))
        pid, run = int(r["pid"]), Path(r["run"])

        def alive():
            try:
                os.kill(pid, 0)
                return True
            except ProcessLookupError:
                return False

        def state():
            try:
                return json.loads((run / "run.json").read_text(encoding="utf-8"))
            except (FileNotFoundError, json.JSONDecodeError):
                return {}

        if alive():
            os.kill(pid, signal.SIGTERM)
        deadline = time.monotonic() + 35
        while time.monotonic() < deadline and state().get("state") != "stopped" and alive():
            time.sleep(1)
        if alive() and state().get("unit"):
            subprocess.run(["/usr/bin/systemctl", "--user", "stop", state()["unit"]], check=False, timeout=15)
            time.sleep(2)
        s = state()
        note = {"pid": pid, "alive": alive(), "state": s.get("state"), "reason": s.get("reason"),
                "exit_code": s.get("exit_code")}
    subprocess.run(["/usr/bin/pwsh", "-NoLogo", "-NoProfile", "-NonInteractive", "-File", HELPER,
                    "-Action", "Stop", "-Unit", f"trp5-{label}-display", "-Socket", f"wayland-trp5-{label}",
                    "-XDisplay", DISPLAY, "-MaxRuntimeSeconds", "3600"], check=False,
                   stdout=subprocess.DEVNULL)
    subprocess.run(["/usr/bin/systemctl", "--user", "stop", f"trp5-{label}-gpu-lock"], check=False,
                   stderr=subprocess.DEVNULL)
    left = eden_processes()
    ledger(f"RELEASED: {OWNER} TrainingRooms package-5 Eden run {label} stopped on the Yoga (display {DISPLAY} "
           f"and GPU lock stopped; Eden processes left: {len(left)}); profile {PROFILE} retained. "
           f"No physical Switch writes.")
    print("down", json.dumps({"label": label, "failed_up": failed, **note, "left": left}))


def scan_lines(text):
    """(line number, line) of every panic/abort/crash/Unmapped line (case-insensitive)."""
    return [(i + 1, l) for i, l in enumerate(text) if re.search(r"panic|abort|crash|Unmapped", l, re.I)]


def scan(args):
    """Search console.log for panic/abort/crash and Unmapped lines (EDEN.md "Read the results")."""
    log = run_dir(args.label) / "console.log"
    text = log.read_text(encoding="utf-8", errors="replace").splitlines()
    hits = scan_lines(text)
    for n, l in hits[:200]:
        print(f"{n}: {l[:300]}")
    print(f"-- {len(text)} lines, {len(hits)} matching")


def restore(args):
    """Put the profile copy back to source b's plugin set: originals come back from the set-aside folder
    (matched by sha256), this tool's NROs are removed, nothing is deleted until the set matches."""
    sk = PROFILE / PLUGINS.parent
    plugins, aside = PROFILE / PLUGINS, sk / "plugins-set-aside-trainingrooms-p5"
    src_plugins = SOURCE / PLUGINS
    want = {p.name: digest(p) for p in src_plugins.glob("*.nro")}
    have = {p.name: digest(p) for p in plugins.glob("*.nro")}
    pool = {digest(p): p for p in sorted(aside.glob("*.nro"))} if aside.is_dir() else {}
    for name, h in want.items():
        if have.get(name) == h:
            continue
        if name in have:
            (plugins / name).unlink()
        assert h in pool, f"original {name} not found in the set-aside folder"
        shutil.copyfile(pool[h], plugins / name)
    for name in list(have):
        if name not in want:
            (plugins / name).unlink()
    final = {p.name: digest(p) for p in plugins.glob("*.nro")}
    assert final == want, f"plugin set differs from source: {final} vs {want}"
    if aside.is_dir():
        for p in aside.glob("*.nro"):
            p.unlink()
        aside.rmdir()
    for rel in list((PROFILE / "user/sdmc/ultimate").glob("TrainingRooms*")) + \
            list((PROFILE / "user/sdmc/ultimate").glob("ssbu-tile-cursor*")) + \
            list((PROFILE / "user/sdmc/ultimate").glob("ssbu-cpu-pilot*")):
        shutil.rmtree(rel)
    print(json.dumps({"restored": {k: v[:12] for k, v in final.items()}}))


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="op", required=True)
    p = sub.add_parser("up")
    p.add_argument("label", type=label_ok)
    p.add_argument("--nro", action="append", default=[])
    p.add_argument("--settings")
    p.add_argument("--diag-txt")
    p.add_argument("--extra", action="append", default=[])
    p.set_defaults(fn=up)
    p = sub.add_parser("prep")  # profile copy only: touches nothing but this tool's own work root
    p.add_argument("label", type=label_ok)
    p.add_argument("--nro", action="append", default=[])
    p.add_argument("--settings")
    p.add_argument("--diag-txt")
    p.add_argument("--extra", action="append", default=[])
    p.set_defaults(fn=lambda a: print(json.dumps(prep(a.label, a.nro, a.settings, a.diag_txt, a.extra))))
    p = sub.add_parser("down")
    p.add_argument("label", type=label_ok)
    p.set_defaults(fn=down)
    p = sub.add_parser("restore")
    p.set_defaults(fn=restore)
    p = sub.add_parser("scan")
    p.add_argument("label", type=label_ok)
    p.set_defaults(fn=scan)
    p = sub.add_parser("shot")
    p.add_argument("label", type=label_ok)
    p.add_argument("name")
    p.add_argument("target", nargs="?")
    p.set_defaults(fn=lambda a: print(shot(a.label, a.name, a.target)))
    p = sub.add_parser("press")
    p.add_argument("label", type=label_ok)
    p.add_argument("buttons")
    p.add_argument("hold", nargs="?", type=int, default=120)

    def press(a):
        buttons = a.buttons.split(",")
        for b in buttons:
            client_call(a.label, "input.button", {"button": b, "state": "press", "lease_ms": a.hold + 500})
        time.sleep(a.hold / 1000)
        for b in buttons:
            client_call(a.label, "input.button", {"button": b, "state": "release"})
        print("ok")

    p.set_defaults(fn=press)
    p = sub.add_parser("stick")
    p.add_argument("label", type=label_ok)
    p.add_argument("x", type=float)
    p.add_argument("y", type=float)
    p.add_argument("ms", type=int)

    def stick(a):
        client_call(a.label, "input.stick", {"stick": "left", "x": a.x, "y": a.y, "lease_ms": a.ms})
        time.sleep(a.ms / 1000)
        print("ok")

    p.set_defaults(fn=stick)
    p = sub.add_parser("call")
    p.add_argument("label", type=label_ok)
    p.add_argument("method")
    p.add_argument("json", nargs="?")  # inline JSON, or @file to read the params from a file

    def call(a):
        raw = a.json
        if raw and raw.startswith("@"):
            raw = Path(raw[1:]).read_text(encoding="utf-8")
        elif raw and raw.startswith("b64:"):
            import base64
            raw = base64.b64decode(raw[4:]).decode("utf-8")
        print(json.dumps(client_call(a.label, a.method, json.loads(raw) if raw else {})))

    p.set_defaults(fn=call)
    p = sub.add_parser("macro")  # play an explicit macro file, poll until it finishes
    p.add_argument("label", type=label_ok)
    p.add_argument("path")
    p.add_argument("--timeout", type=float, default=120)

    def macro(a):
        res = client_call(a.label, "macro.play", {"path": a.path})
        print("play", json.dumps(res))
        deadline = time.time() + a.timeout
        last = None
        while time.time() < deadline:
            st = client_call(a.label, "macro.status")
            last = st
            if str(st.get("state")) not in ("playing", "starting", "running"):
                break
            time.sleep(0.5)
        print("status", json.dumps(last))
        print("input.reset", json.dumps(client_call(a.label, "input.reset")))

    p.set_defaults(fn=macro)
    args = parser.parse_args()
    args.fn(args)


if __name__ == "__main__":
    main()
