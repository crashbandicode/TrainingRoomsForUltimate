"""The one parameter set of the TrainingRooms Eden harness (gv.py, solo.ps1, ledger.py, route.py, ally/).

Playbook origin: the hard-coded constants block of tools/eden-solo/gv.py (ROOT, SOURCE, PROFILE, OWNER, OUTPUT,
FORCED, PLUGINS, ...) and the `$work/$coord/$owner/$display/$game` lines of tools/eden-solo/solo.ps1
(playbook ae87443). Those are gsp-visualizer's values; each TrainingRooms stream copied the files and edited them,
nine times. Here every such value is a parameter with a default for the Yoga and the Surface.

A value comes from, in order: the caller (tests), an environment variable `TR_<NAME>`, the JSON file named by
`TR_CONFIG` (else `harness.config.json` beside this file), then the default. `deploy.py` writes that JSON on the
host that runs Eden, so a host-side run sees the same parameters as the driver.

    TR_OWNER     claim owner: "<Agent> <session>/<stream>-eden" (ledger key; the first two words)
    TR_PREFIX    run label prefix: labels are "<prefix>-NN" (st-03, f6-01, hb-02)
    TR_ROOT      work root on the Eden host (profiles/, runs/, PREP/LAUNCH receipts; own folder per stream)
    TR_PROFILE   profile tag under <root>/profiles (default: y on the Yoga, g on the Surface)
    TR_SOURCE    the profile it is copied from (default: the host's read-only source, Yoga b, Surface g; a
                 second instance of a pair sets the first's profile)
    TR_DISPLAY   the stream's X display, e.g. :99 (never :61)
    TR_HOST      yoga | surface | ally (default: from the machine's host name)
    TR_RUNS      run folder (default <root>/runs; every file a run writes is named after its label)
    TR_NRO       NRO set, comma separated (path or path:installed-name); the CLI's --nro adds to it
    TR_PILOT     the CPU pilot NRO for coexistence rows (path on the Eden host)
    TR_TILE      the tile-cursor NRO (path on the Eden host)
    TR_OUTPUT    the plugin's data folder under sdmc:/ultimate (TrainingRooms)
    TR_EXTRAS    other data folders archived before each boot, comma separated
    TR_UNIT      systemd unit prefix (default tr<prefix>)
    TR_REFS      reference screenshots of the route's screens (gitignored; copy them to the host that routes)
    TR_MACROS    folder of the route's macro segments
    TR_INSTALL_GATE   text the plugin's install line shows (default result=installed; the integrated tree's line
                      is cut before it: plugin=training_rooms/)
    TR_STALE_HOURS    an open claim older than this without a RELEASED is ledger noise (default 12)
    TR_FAIR_SECONDS   wait after your own RELEASED before claiming the same host again (default 180)
    TR_LEDGER, TR_LEDGER_SSH, TR_QUEUE, TR_STREAM   claim ledger path, the host that holds it, queue file, queue name
    TR_ALLY_ROOT      the Ally rig (default: the lent rig C:/Eden/c1162-w5-ally); TR_ALLY_CLAIM the rig's CLAIM text
                      while this stream holds it; TR_ALLY_MEM_GIB the rig's Job limit (11); TR_ALLY_EDEN_SHA
"""
import json
import os
import re
import socket
from dataclasses import dataclass, fields
from pathlib import Path

HERE = Path(__file__).resolve().parent
PROFILES = Path("/home/intpa/ssbu-desync-1029/profiles")
YOGA_GAME = "/home/intpa/Games/Roms/Switch/"
SURFACE_GAME = "/home/intpa/eden-assets/games/01006A800016E000/base/"
GAME_NAME = "Super Smash Bros. Ultimate [01006A800016E000]+[v13.0.3+ 9DLC][US](nsw2u.com).xci"
LENT_RIG = "C:/Eden/c1162-w5-ally"

# What differs per host: source profile (read only), the game file, forced config keys, the Yoga's GPU lock, and
# the ssh alias that reaches the host from another one. The Surface runs offline (a solo boot has no peer).
HOSTS = {
    "yoga": {"title": "Yoga", "name": "yoga", "ssh": "memento-claw-yoga-ubuntu", "source": PROFILES / "b",
             "profile": "y", "game": YOGA_GAME + GAME_NAME, "forced": {}, "gpu_lock": True},
    "surface": {"title": "Surface", "name": "surface", "ssh": "intpa@dreamland-surface.lan", "source": PROFILES / "g",
                "profile": "g", "game": SURFACE_GAME + GAME_NAME,
                "forced": {"lan_online": "false", "airplane_mode": "true"}, "gpu_lock": False},
    "ally": {"title": "Ally", "name": "ally", "ssh": "intpa@dreamland-ally", "source": None, "profile": "ally",
             "game": None, "forced": {}, "gpu_lock": False},
}
# Keys every profile copy gets: Eden keeps running behind another window, and is muted (operator rule 2026-10-09).
FORCED = {"pauseWhenInBackground": "false", "volume": "0", "volume\\default": "false"}
LISTS = ("nro", "extras")
NUMBERS = ("stale_hours", "fair_seconds", "ally_mem_gib")


@dataclass(frozen=True)
class Params:
    owner: str = ""
    prefix: str = ""
    root: str = ""
    profile: str = ""
    source: str = ""
    display: str = ""
    host: str = ""
    runs: str = ""
    nro: tuple = ()
    pilot: str = ""
    tile: str = "/home/intpa/ssbu-tile-cursor/dist/libssbu_tile_cursor.nro"
    output: str = "TrainingRooms"
    extras: tuple = ("ssbu-tile-cursor", "ssbu-cpu-pilot")
    unit: str = ""
    refs: str = ""
    macros: str = ""
    install_gate: str = "result=installed"
    stale_hours: float = 12
    fair_seconds: float = 180
    ledger: str = "/tmp/deluxe-surface-1007/OWNER-COORDINATION.md"
    ledger_ssh: str = "memento-claw-yoga-ubuntu"
    queue: str = "/home/intpa/smash-training-lobby/TRAININGROOMS-EDEN-QUEUE.md"
    stream: str = ""
    ally_root: str = LENT_RIG
    ally_claim: str = ""  # the rig's CLAIM text while this stream holds it (stage.ps1 sets it, restore puts the original back)
    ally_mem_gib: float = 11  # the lent rig's Job memory limit
    ally_eden_sha: str = "45af318c93cbc00a2794421ee884ffbcb9dbec75bbdbdeef27af30d61cd2cbad"

    @property
    def info(self):
        return HOSTS[self.host]

    @property
    def label_re(self):
        return rf"{re.escape(self.prefix)}-\d{{2,}}"

    @property
    def profile_dir(self):
        return Path(self.root) / "profiles" / self.profile

    @property
    def runs_dir(self):
        return Path(self.runs or Path(self.root) / "runs")

    @property
    def unit_prefix(self):
        return self.unit or f"tr{self.prefix}"

    @property
    def aside(self):
        return f"plugins-set-aside-{self.unit_prefix}"

    @property
    def sdmc(self):
        return self.profile_dir / "user/sdmc"

    @property
    def data_dir(self):
        return self.sdmc / "ultimate" / self.output

    @property
    def refs_dir(self):
        return Path(self.refs) if self.refs else HERE.parent / "refs"

    @property
    def macros_dir(self):
        return Path(self.macros) if self.macros else HERE.parent / "macros"

    @property
    def stream_name(self):
        return self.stream or self.prefix


def detect_host(name=None):
    name = (name or socket.gethostname()).lower()
    for host in ("surface", "yoga"):
        if host in name:
            return host
    raise SystemExit(f"unknown host {name!r}: set TR_HOST to one of {sorted(HOSTS)}")


def machine():
    """The host this code runs on (yoga, surface), or None."""
    try:
        return detect_host()
    except SystemExit:
        return None


def _config(env):
    path = env.get("TR_CONFIG")
    path = Path(path) if path else HERE / "harness.config.json"
    return json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}


def load(env=None, **given):
    """Resolve the parameter set. `given` wins over `TR_*` variables, which win over the config file."""
    env = os.environ if env is None else env
    raw = {**_config(env)}
    for f in fields(Params):
        value = env.get(f"TR_{f.name.upper()}")
        if value is not None:
            raw[f.name] = value
    raw.update({k: v for k, v in given.items() if v is not None})
    values = {}
    for f in fields(Params):
        if f.name not in raw:
            continue
        value = raw[f.name]
        if f.name in LISTS and isinstance(value, str):
            value = tuple(x for x in value.split(",") if x)
        elif f.name in LISTS:
            value = tuple(value)
        elif f.name in NUMBERS:
            value = float(value)
        values[f.name] = value
    values["host"] = (values.get("host") or detect_host()).lower()
    if values["host"] not in HOSTS:
        raise SystemExit(f"unknown host {values['host']!r}: one of {sorted(HOSTS)}")
    values["ally_root"] = values.get("ally_root", LENT_RIG).replace("\\", "/").rstrip("/")
    info = HOSTS[values["host"]]
    values.setdefault("profile", info["profile"])
    p = Params(**values)
    problems = check(p)
    if problems:
        raise SystemExit("bad harness parameters: " + "; ".join(problems))
    return p


def check(p):
    """Syntax checks of the values that are set (a value that is not set is `need`'s business)."""
    bad = []
    if p.prefix and not re.fullmatch(r"[a-z][a-z0-9]*", p.prefix):
        bad.append(f"prefix {p.prefix!r} must be lower-case letters and digits")
    if p.display and (not re.fullmatch(r":\d{1,3}", p.display) or p.display == ":61"):
        bad.append(f"display {p.display!r} not allowed (:N, never :61)")
    if p.profile == "a":
        bad.append("profile 'a' is never ours")
    if p.root and not os.path.isabs(p.root) and not re.match(r"[A-Za-z]:", p.root):
        bad.append(f"root {p.root!r} must be absolute")
    return bad


def need(p, *names):
    missing = [n for n in names if not getattr(p, n)]
    if missing:
        raise SystemExit("missing harness parameters (set TR_<NAME> or the config file): " + ", ".join(missing))
    return p


def label_ok(p, label):
    need(p, "prefix")
    if not re.fullmatch(p.label_re, label):
        raise SystemExit(f"bad label {label!r}: must match {p.label_re}")
    return label


def split_spec(spec):
    """`path` or `path:installed-name` (load order follows the file name) -> (Path, name)."""
    path, _, name = spec.partition(":")
    return Path(path), name or Path(path).name


def dump(p):
    """The parameters as the JSON `deploy.py` puts beside gv.py on the Eden host."""
    here_only = ("nro", "refs", "macros")  # paths on the driver's machine
    return json.dumps({f.name: list(getattr(p, f.name)) if f.name in LISTS else getattr(p, f.name)
                       for f in fields(Params) if f.name not in here_only}, indent=1) + "\n"
