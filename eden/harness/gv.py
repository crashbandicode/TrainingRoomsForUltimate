"""One TrainingRooms Eden instance on the host it runs on (the Yoga or the Surface), every stream's parameters.

Playbook origin: tools/eden-solo/gv.py (playbook ae87443; "adapted from elite-rematch/eden"), same structure and
subcommand names: prep, launch, shot, tree, press, stick, call, keepalive, stop. The values gv.py hard-coded
(ROOT, OWNER, label pattern, OUTPUT, OTHER_OUTPUTS, FORCED, aside folder) come from params.py. Added (each one was
written by a stream in its own copy of gv.py; the diff of every copy is in docs/HARNESS.md):
  mute       core.volume=0 through the control API, saved and read back before the game boots (operator rule
             2026-10-09; the profile's qt-config.ini carries volume=0 too, set by prep)
  macro      play one macro segment and wait for it; sha256 of every played file kept in runs/<label>/macros.jsonl
             (docs/MACROS.md "Receipts")
  lines      a plugin log's lines or their count (the route's log gates)
  scan       console.log: panic/abort/crash, and Unmapped reads/writes judged by docs/EDEN.md "Read the results"
  restore    the profile copy back to the source's plugin set, set-aside NROs matched by sha256
  collect    the run's logs, receipts and plugin folders next to the repo
  reload     replace the plugin's live settings (diag build) and wait for its `diag_settings_reload` line
  preflight  no Eden here, display free, MemAvailable >= 10 GiB, df >= 40 GB (solo.ps1 Up runs it)
  params     the resolved parameter set as JSON (solo.ps1 reads its values from here)
prep also installs the stream's rows: settings.json, diag.txt, developer.ini or any file (`--put PATH[:NAME]`),
a seed folder (`--seed`), files and trees under sdmc (`--sd-file`, `--tree`), and keeps the plugin's data
(`--keep-data`) when a boot reads what the last boot wrote.

Usage: gv.py prep <label> [--nro PATH[:NAME]]... [--pilot] [--tile] [--put PATH[:NAME]]... [--settings F]
                          [--diag-txt F] [--developer-ini F] [--seed DIR] [--sd-file SRC:REL]...
                          [--tree SRC:REL[:UNREADABLE]]... [--keep-data] [<nro>...]
       gv.py launch <label> | stop <label> | preflight | params | label <label>
       gv.py shot <label> <name> | tree <label> | press <label> <button>[,<button>...] [hold_ms]
       gv.py stick <label> <x> <y> <ms> | call <label> <method> [json|@file|b64:...] | keepalive <label> <stop> <s>
       gv.py mute <label> | macro <label> <path> | lines <name> [--after N] [--count]
       gv.py scan <label> [--window T0:T1] | restore | collect <label> [--to DIR] | reload <label> <settings> | unfix <rel>
The private profile is a hard-linked copy of the host's source profile (Yoga b, Surface g; never a profile named a);
only the copy is modified.
"""
import argparse
import base64
import hashlib
import json
import os
import re
import shutil
import signal
import stat
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import params  # noqa: E402

APP = Path("/home/intpa/Applications/Eden-GIF/current/Eden.AppImage")
RUNTIME = Path("/home/intpa/elite-rematch-work/eden-lan-runtime")
RUNTIME_SHA = "ee3c2e953a0b87541c29eb8918ce0354489035296ac5eaf060b3444d06a8bbcb"
READ_ONLY = ["user/nand/user/Contents", "user/nand/system/Contents", "user/firmware"]
PLUGINS = Path("user/sdmc/atmosphere/contents/01006A800016E000/romfs/skyline/plugins")
CONFIG = Path("user/config/qt-config.ini")
RUN_SECONDS = 3600
MIN_MEM_KIB = 10 * 1024 ** 2
MIN_FREE_BYTES = 40 * 10 ** 9
EDEN_PROCS = "Eden.AppImage|/eden( |$)|eden-run.py"
UNMAPPED = re.compile(r"Unmapped\s+(Read|Write)(\d*)\s+@\s+0x([0-9a-fA-F]+)")
STAMPED = re.compile(r"^\[\s*(\d+\.\d+)\]")
sys.path.insert(0, str(APP.parent / "tools"))

P = ROOT = SOURCE = PROFILE = OWNER = OUTPUT = SHOTS = None
FORCED = {}


def configure(p=None):
    """Bind the module to a parameter set (default: the environment's). The playbook's globals, from params."""
    global P, ROOT, SOURCE, PROFILE, OWNER, OUTPUT, SHOTS, FORCED
    P = p or params.load()
    ROOT, PROFILE, OWNER = Path(P.root), P.profile_dir, P.owner
    SOURCE = Path(P.source) if P.source else P.info["source"]
    OUTPUT = Path("user/sdmc/ultimate") / P.output
    SHOTS = ROOT / "shots"
    FORCED = {**params.FORCED, **P.info["forced"]}
    return P


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def label_ok(label):
    return params.label_ok(P, label)


def run_dir(label):
    return P.runs_dir / label


def mem_available_kib():
    return next(int(line.split()[1]) for line in Path("/proc/meminfo").read_text().splitlines()
                if line.startswith("MemAvailable:"))


def eden_processes():
    out = subprocess.run(["/usr/bin/pgrep", "-af", EDEN_PROCS], capture_output=True, text=True).stdout
    return [l for l in out.splitlines() if "pgrep" not in l]


def preflight(args=None):
    """Everything `Up` needs before it claims: one Eden per host, our display free, memory and disk."""
    procs = eden_processes()
    assert not procs, f"this host already runs Eden: {procs}"
    assert not Path(f"/tmp/.X11-unix/X{P.display[1:]}").exists(), f"display {P.display} in use"
    mem = mem_available_kib()
    assert mem >= MIN_MEM_KIB, f"MemAvailable {mem} KiB < 10 GiB"
    free = shutil.disk_usage("/").free
    assert free >= MIN_FREE_BYTES, f"df free {free} < 40 GB"
    print(json.dumps({"preflight": "ok", "mem_available_kib": mem, "df_free": free}))


def put_file(out, spec, receipt):
    """A real file (never a hard link) in the plugin's data folder: PATH or PATH:NAME."""
    src, name = params.split_spec(spec)
    shutil.copyfile(src, out / name)
    text = (out / name).read_bytes()
    entry = {"sha256": hashlib.sha256(text).hexdigest(), "size": len(text)}
    if len(text) <= 2000:
        entry["text"] = text.decode("utf-8", "replace")
    receipt.setdefault("put", {})[name] = entry


def prep(args):
    receipt = {"label": args.label, "time": time.time(), "source": str(SOURCE), "profile": str(PROFILE),
               "set_aside": [], "archived": []}
    assert SOURCE.name != "a" and SOURCE.is_dir(), f"bad source profile: {SOURCE}"
    assert SOURCE != PROFILE, "source and target profile are the same"
    if not PROFILE.exists():
        temp = PROFILE.with_name(f".{PROFILE.name}-{args.label}-tmp")
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
    assert f"sdmc_directory={PROFILE / 'user/sdmc'}" in config.splitlines(), "private SDMC path missing"
    for key, value in FORCED.items():
        assert f"{key}={value}" in config.splitlines(), f"config lacks {key}={value}"

    plugins = PROFILE / PLUGINS
    plugins.mkdir(parents=True, exist_ok=True)
    aside = plugins.parent / P.aside
    aside.mkdir(exist_ok=True)
    for path in sorted(plugins.glob("*.nro")):
        target = aside / f"{path.stem}.{args.label}-{int(time.time())}.nro"
        path.replace(target)
        receipt["set_aside"].append(target.name)
    specs = [*P.nro, *args.nro, *args.nros, *([P.pilot] if args.pilot else []), *([P.tile] if args.tile else [])]
    for spec in specs:  # <path> or <path>:<installed name> (load order follows the file name)
        nro, name = params.split_spec(spec)
        assert nro.suffix == ".nro" and nro.is_file(), f"missing NRO: {nro}"
        shutil.copyfile(nro, plugins / name)
    receipt["plugins"] = {p.name: digest(p) for p in sorted(plugins.glob("*.nro"))}
    assert not any("training_modpack" in p.name for p in plugins.glob("*.nro")), "fork NRO present"
    trees = [Path("user/sdmc") / s.partition(":")[2].partition(":")[0] for s in args.tree]
    for rel in [OUTPUT, *(Path("user/sdmc/ultimate") / e for e in P.extras), *trees]:
        previous = PROFILE / rel
        if args.keep_data and rel == OUTPUT:
            continue  # the next boot reads what the previous boot left (a copy is kept in the receipt)
        if previous.exists():
            archived = previous.with_name(f"{previous.name}.before-{args.label}")
            assert not archived.exists(), f"archive exists: {archived}"
            previous.replace(archived)
            receipt["archived"].append(str(archived))
    out = PROFILE / OUTPUT
    out.mkdir(parents=True, exist_ok=args.keep_data)
    if args.keep_data:
        receipt["kept_data"] = {p.name: digest(p) for p in sorted(out.iterdir()) if p.is_file()}
    if args.seed:  # a folder copied into the data folder (replay_captures/, macros/, replay_capture.ini)
        shutil.copytree(args.seed, out, dirs_exist_ok=True)
        receipt["seed"] = sorted(str(p.relative_to(args.seed)) for p in Path(args.seed).rglob("*") if p.is_file())[:40]
    puts = [*args.put, *([f"{args.settings}:settings.json"] if args.settings else []),
            *([f"{args.diag_txt}:diag.txt"] if args.diag_txt else []),
            *([f"{args.developer_ini}:developer.ini"] if args.developer_ini else [])]
    for spec in puts:
        put_file(out, spec, receipt)
    receipt["sd_files"] = {}
    for spec in args.sd_file:  # <src>:<path under sdmc>, a real copy
        src, _, rel = spec.partition(":")
        (PROFILE / "user/sdmc" / rel).parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, PROFILE / "user/sdmc" / rel)
        receipt["sd_files"][rel] = digest(PROFILE / "user/sdmc" / rel)
    for spec in args.tree:  # <folder>:<path under sdmc>[:<unreadable subfolder>]
        src, _, rest = spec.partition(":")
        rel, _, unreadable = rest.partition(":")
        shutil.copytree(src, PROFILE / "user/sdmc" / rel)
        if unreadable:
            os.chmod(PROFILE / "user/sdmc" / rel / unreadable, 0)
        receipt.setdefault("trees", {})[rel] = {"from": src, "unreadable": unreadable or None}
    ROOT.mkdir(parents=True, exist_ok=True)
    (ROOT / f"PREP-{args.label}.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({k: receipt.get(k) for k in ("label", "plugins", "set_aside", "archived", "put", "seed",
                                                  "sd_files", "trees", "kept_data")}))


def launch(args):
    display = args.display or P.display
    run = run_dir(args.label)
    assert (PROFILE / "user").is_dir(), f"profile missing: {PROFILE}"
    assert not run.exists(), f"run exists: {run}"
    assert re.fullmatch(r":\d{1,3}", display) and display != ":61", f"display not allowed: {display}"
    assert Path(f"/tmp/.X11-unix/X{display[1:]}").exists(), f"display not up: {display}"
    mem_kib = mem_available_kib()
    assert mem_kib >= MIN_MEM_KIB, f"need 10 GiB available, have {mem_kib} KiB"
    assert digest(RUNTIME) == RUNTIME_SHA, "runtime hash mismatch"
    env = dict(os.environ, DISPLAY=display, WAYLAND_DISPLAY="", XAUTHORITY="", QT_QPA_PLATFORM="xcb",
               XDG_DATA_HOME=str(PROFILE / "xdg-data"), XDG_CONFIG_HOME=str(PROFILE / "xdg-config"),
               XDG_CACHE_HOME=str(PROFILE / "xdg-cache"),
               EDEN_LOG_FILTER="*:Warning Frontend:Info Service.Sockets:Info Network:Info")
    for key in ("EDEN_NSO_LAN_CONFIG", "EDEN_LAN_RUNTIME", "EDEN_LAN_ASSETS", "EDEN_LAN_PROFILE_DIR",
                "EDEN_LAN_FIGHTER_SEEDS"):
        env.pop(key, None)
    (PROFILE / "ratings").mkdir(mode=0o700, exist_ok=True)
    child = ["/usr/bin/env", f"EDEN_LAN_RUNTIME={RUNTIME}", f"EDEN_LAN_PROFILE_DIR={PROFILE / 'ratings'}", str(APP)]
    P.runs_dir.mkdir(parents=True, exist_ok=True)
    command = ["/usr/bin/python3", str(APP.parent / "tools/eden-run.py"), "--memory-gib", "8",
               "--runtime", str(RUN_SECONDS), "--startup-timeout", "180", "--hang-timeout", "60",
               "--stall-timeout", "0", "--background", "--output", str(run), "--", *child]
    receipt_path = ROOT / f"LAUNCH-{args.label}.json"
    assert not receipt_path.exists(), f"receipt exists: {receipt_path}"
    with (ROOT / f"launch-{args.label}.log").open("x", encoding="utf-8") as log:
        process = subprocess.Popen(command, env=env, cwd=PROFILE, stdin=subprocess.DEVNULL,
                                   stdout=log, stderr=subprocess.STDOUT, start_new_session=True,
                                   close_fds=True)
    receipt = {"owner": OWNER, "label": args.label, "display": display, "pid": process.pid,
               "app_sha256": digest(APP), "runtime": str(RUNTIME), "mem_available_kib": mem_kib,
               "time": time.time(), "profile": str(PROFILE), "run": str(run)}
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(receipt))


def client_call(label, method, params_=None, timeout=15):
    from edenctl import Client
    response = Client(str(run_dir(label) / "control.sock"), timeout=timeout).request(method, params_ or {})
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


def take_shot(label, name, target=None):
    target = {"id": target} if target else next(
        n for n in nodes(client_call(label, "ui.tree", {"brief": True, "visible_only": True}))
        if n["class"] == "GRenderWindow")
    out = SHOTS / label
    out.mkdir(parents=True, exist_ok=True)
    path = out / f'{time.strftime("%H%M%S", time.gmtime())}-{name}.png'
    client_call(label, "ui.screenshot", {"path": str(path), "target": target["id"]}, timeout=30)
    return path


def shot(args):
    print(take_shot(args.label, args.name, args.target))


def tree(args):
    """Visible widgets: class, id and any short text/title, one per line."""
    for n in nodes(client_call(args.label, "ui.tree", {"brief": True, "visible_only": True})):
        words = [str(n.get(k))[:80] for k in ("text", "title", "windowTitle", "objectName") if n.get(k)]
        print(n["class"], n["id"], *words)


def press(args):
    buttons = args.buttons.split(",")
    for button in buttons:
        client_call(args.label, "input.button", {"button": button, "state": "press", "lease_ms": args.hold + 500})
    time.sleep(args.hold / 1000)
    for button in buttons:
        client_call(args.label, "input.button", {"button": button, "state": "release"})
    print("ok")


def stick(args):
    client_call(args.label, "input.stick", {"stick": "left", "x": args.x, "y": args.y, "lease_ms": args.ms})
    time.sleep(args.ms / 1000)
    print("ok")


def call(args):
    raw = args.json
    if raw and raw.startswith("@"):
        raw = Path(raw[1:]).read_text(encoding="utf-8")
    elif raw and raw.startswith("b64:"):  # the route sends its params this way over ssh (no quoting)
        raw = base64.b64decode(raw[4:]).decode("utf-8")
    print(json.dumps(client_call(args.label, args.method, json.loads(raw) if raw else {})))


def muted(reply):
    """Whether a `settings.get` reply says 0: the bare number, or a `value`/`current`/`effective` field."""
    found = []

    def walk(node):
        if isinstance(node, dict):
            for key, value in node.items():
                if key in ("value", "current", "effective"):
                    found.append(value)
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    if isinstance(reply, (int, float, str)) and not isinstance(reply, bool):
        found.append(reply)
    walk(reply)
    return bool(found) and all(str(v) in ("0", "0.0") for v in found)


def mute(args):
    """Operator rule 2026-10-09: muted through the CLI, saved before the game boots; the read-back must say 0."""
    out = {"set": client_call(args.label, "settings.set", {"key": "core.volume", "value": 0}),
           "save": client_call(args.label, "settings.save"),
           "get": client_call(args.label, "settings.get", {"key": "core.volume"})}
    print(json.dumps(out))
    if not muted(out["get"]):
        raise SystemExit(f"core.volume did not read back 0: {out['get']}")


def play_macro(label, path, timeout=120):
    """Play one macro file, wait until it has finished with no error, release the inputs, keep its sha256."""
    path = Path(path)
    assert path.is_file(), f"no macro file: {path}"
    client_call(label, "macro.play", {"path": str(path)})
    end, status = time.time() + timeout, {}
    while time.time() < end:
        status = client_call(label, "macro.status")
        if str(status.get("state")) not in ("playing", "starting", "running"):
            break
        time.sleep(0.5)
    client_call(label, "input.reset")
    receipt = {"time": time.time(), "path": str(path), "sha256": digest(path), "state": status.get("state"),
               "error": status.get("error")}
    run_dir(label).mkdir(parents=True, exist_ok=True)
    with (run_dir(label) / "macros.jsonl").open("a", encoding="utf-8") as f:
        f.write(json.dumps(receipt) + "\n")
    if status.get("error") or str(status.get("state")) in ("playing", "starting", "running"):
        raise SystemExit(f"macro did not finish cleanly: {receipt}")
    return receipt


def macro(args):
    print(json.dumps(play_macro(args.label, args.path, args.timeout)))


def log_path(name):
    """`events`, `diag` or `tile`, else a path under sdmc:/ultimate."""
    known = {"events": P.data_dir / "events.log", "diag": P.data_dir / "practice-diagnostics.log",
             "tile": P.sdmc / "ultimate/ssbu-tile-cursor/events.log"}
    return known.get(name) or P.sdmc / "ultimate" / name


def read_lines(name):
    try:
        return log_path(name).read_text(encoding="utf-8", errors="replace").splitlines()
    except FileNotFoundError:
        return []


def lines(args):
    text = read_lines(args.name)
    print(len(text) if args.count else "\n".join(text[args.after:]))


def keepalive(args):
    """Keep the fighter in place (walking off counts as a self-destruct; the game drops players after ~30 s
    without input), as elite-rematch drive.py."""
    from edenctl import Client
    socket = str(run_dir(args.label) / "control.sock")
    deadline, step, errors = time.monotonic() + args.seconds, 0, 0
    while time.monotonic() < deadline and not Path(args.stop_file).exists():
        for method, params_ in (("input.stick", {"stick": "left", "x": 0, "y": -1, "lease_ms": 300}),
                                ("input.button", {"button": "zl", "state": "tap", "duration_ms": 80})):
            try:
                Client(socket, timeout=5).request(method, params_)
            except Exception:  # a dropped call must never end the loop
                errors += 1
        step += 1
        time.sleep(0.7)
    try:
        Client(socket, timeout=5).request("input.reset", {})
    except Exception:
        errors += 1
    print(json.dumps({"steps": step, "errors": errors,
                      "reason": "stop" if Path(args.stop_file).exists() else "deadline"}))


def stop(args):
    receipt = json.loads((ROOT / f"LAUNCH-{args.label}.json").read_text(encoding="utf-8"))
    pid, run = int(receipt["pid"]), Path(receipt["run"])

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
        subprocess.run(["systemctl", "--user", "stop", state()["unit"]], check=False, timeout=15)
        time.sleep(2)
    s = state()
    print(json.dumps({"label": args.label, "pid": pid, "alive": alive(), "state": s.get("state"),
                      "reason": s.get("reason"), "exit_code": s.get("exit_code")}))
    sys.exit(1 if alive() else 0)


def scan_lines(text):
    """(line number, line) of every panic/abort/crash/Unmapped line (case-insensitive)."""
    return [(i + 1, l) for i, l in enumerate(text) if re.search(r"panic|abort|crash|Unmapped", l, re.I)]


def verdict(text, window=None):
    """docs/EDEN.md "Read the results": an Unmapped read or write below 0x1000, or inside the plugin's activity
    window (console-log seconds, `[  12.345]`), fails the lane; other Unmapped lines are attributed by timestamp
    against the plugin's own log before they are dismissed. panic/abort/crash lines are listed for review."""
    fail, other, review = [], [], []
    for n, line in scan_lines(text):
        m = UNMAPPED.search(line)
        if not m:
            review.append((n, line))
            continue
        stamp = STAMPED.match(line)
        at = float(stamp.group(1)) if stamp else None
        inside = bool(window and at is not None and window[0] <= at <= window[1])
        (fail if int(m.group(3), 16) < 0x1000 or inside else other).append((n, line))
    return {"fail": fail, "unmapped_other": other, "review": review}


def scan(args):
    """Search console.log for panic/abort/crash and Unmapped lines; exit 1 when the lane fails."""
    text = (run_dir(args.label) / "console.log").read_text(encoding="utf-8", errors="replace").splitlines()
    window = tuple(float(x) for x in args.window.split(":")) if args.window else None
    result = verdict(text, window)
    for title, rows in (("FAIL", result["fail"]), ("unmapped (attribute)", result["unmapped_other"]),
                        ("review", result["review"])):
        for n, line in rows[:50]:
            print(f"{title} {n}: {line[:300]}")
    print(f"-- {len(text)} lines, {len(result['fail'])} failing, {len(result['unmapped_other'])} other Unmapped, "
          f"{len(result['review'])} panic/abort/crash")
    sys.exit(1 if result["fail"] else 0)


def rmtree(path):
    """Remove a tree, giving back the mode of any folder a fixture made unreadable."""
    for dirpath, dirs, _ in os.walk(path):
        for d in dirs:
            os.chmod(os.path.join(dirpath, d), stat.S_IRWXU)
    shutil.rmtree(path)


def restore(args):
    """Put the profile copy back to the source's plugin set: originals come back from the set-aside folder
    (matched by sha256), this tool's NROs are removed, nothing is deleted until the set matches."""
    sk = PROFILE / PLUGINS.parent
    plugins, aside = PROFILE / PLUGINS, sk / P.aside
    want = {p.name: digest(p) for p in (SOURCE / PLUGINS).glob("*.nro")}
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
    ultimate = PROFILE / "user/sdmc/ultimate"
    for name in (P.output, *P.extras):
        for rel in ultimate.glob(f"{name}*"):
            rmtree(rel)
    print(json.dumps({"restored": {k: v[:12] for k, v in final.items()}}))


def collect(args):
    dest = Path(args.to) if args.to else HERE.parent / "runs" / args.label
    dest.mkdir(parents=True, exist_ok=True)
    for name in (P.output, *P.extras):
        if (P.sdmc / "ultimate" / name).exists():
            shutil.copytree(P.sdmc / "ultimate" / name, dest / name, dirs_exist_ok=True)
    for name in ("console.log", "run.json", "memory.peak", "macros.jsonl"):
        if (run_dir(args.label) / name).exists():
            shutil.copyfile(run_dir(args.label) / name, dest / name)
    for name in (f"PREP-{args.label}.json", f"LAUNCH-{args.label}.json"):
        if (ROOT / name).exists():
            shutil.copyfile(ROOT / name, dest / name)
    print(dest)


def reload(args):
    """Write a whole settings file as `diag-settings-next.json` (tmp file, then rename) and wait for the plugin's
    `diag_settings_reload` line."""
    diag = log_path("diag")
    count = lambda: sum("diag_settings_reload" in l for l in  # noqa: E731
                        diag.read_text(encoding="utf-8", errors="replace").splitlines()) if diag.exists() else 0
    before = count()
    out = P.data_dir
    shutil.copyfile(args.settings, out / ".diag-settings-next.tmp")
    (out / ".diag-settings-next.tmp").rename(out / "diag-settings-next.json")
    end = time.time() + 30
    while time.time() < end:
        if count() > before:
            print([l for l in diag.read_text(encoding="utf-8", errors="replace").splitlines()
                   if "diag_settings_reload" in l][-1])
            return
        time.sleep(1)
    raise SystemExit("no diag_settings_reload line within 30 s")


def unfix(args):
    os.chmod(PROFILE / "user/sdmc" / args.rel, 0o755)


def show_params(args):
    print(json.dumps({"owner": P.owner, "prefix": P.prefix, "root": P.root, "profile": str(PROFILE),
                      "display": P.display, "host": P.host, "title": P.info["title"], "unit": P.unit_prefix,
                      "game": P.info["game"], "gpu_lock": P.info["gpu_lock"], "runs": str(P.runs_dir),
                      "label_re": P.label_re, "run_seconds": RUN_SECONDS}))


def build_parser():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="op", required=True)

    def add(name, fn, *positional, label=True, **kw):
        p = sub.add_parser(name)
        if label:
            p.add_argument("label", type=label_ok)
        for item in positional:
            p.add_argument(item[0], **item[1])
        p.set_defaults(fn=fn)
        return p

    p = add("prep", prep, ("nros", {"nargs": "*", "default": []}))
    p.add_argument("--nro", action="append", default=[])
    p.add_argument("--pilot", action="store_true")
    p.add_argument("--tile", action="store_true")
    p.add_argument("--put", action="append", default=[])
    p.add_argument("--settings")
    p.add_argument("--diag-txt")
    p.add_argument("--developer-ini")
    p.add_argument("--seed")
    p.add_argument("--sd-file", action="append", default=[])
    p.add_argument("--tree", action="append", default=[])
    p.add_argument("--keep-data", action="store_true")
    add("launch", launch).add_argument("--display")
    add("shot", shot, ("name", {}), ("target", {"nargs": "?"}))
    add("tree", tree)
    add("press", press, ("buttons", {}), ("hold", {"nargs": "?", "type": int, "default": 120}))
    add("stick", stick, ("x", {"type": float}), ("y", {"type": float}), ("ms", {"type": int}))
    add("call", call, ("method", {}), ("json", {"nargs": "?"}))
    add("keepalive", keepalive, ("stop_file", {}), ("seconds", {"type": float}))
    add("stop", stop)
    add("mute", mute)
    add("macro", macro, ("path", {})).add_argument("--timeout", type=float, default=120)
    p = add("lines", lines, ("name", {}), label=False)
    p.add_argument("--after", type=int, default=0)
    p.add_argument("--count", action="store_true")
    add("scan", scan).add_argument("--window")
    add("restore", restore, label=False)
    add("collect", collect).add_argument("--to")
    add("reload", reload, ("settings", {}))
    add("unfix", unfix, ("rel", {}), label=False)
    add("preflight", preflight, label=False)
    add("params", show_params, label=False)
    add("label", lambda a: print(a.label))
    return parser


def main(argv=None):
    configure()
    args = build_parser().parse_args(argv)
    args.fn(args)


if __name__ == "__main__":
    main()
