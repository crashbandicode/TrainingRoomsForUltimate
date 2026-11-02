"""Row `cpu-01-mash-each-action` on the Ally (one roll per situation entry, B8, so the L+R+A reset re-arms the mash after each reload): for each action, Mash Toggles = that action and Mash Triggers = Always (settings
reloaded in the running diag boot through `diag-settings-next.json`), four seconds of the CPU's own frames, then the command
bits and the statuses the CPU went through, by name from the boot's `cpu_ids` dump.

    ally_rows_mash.py <label> [action ...]
"""
import json
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import ally_log as al  # noqa: E402
import ally_ops as ao  # noqa: E402
import mk_settings as mk  # noqa: E402

DEFAULT = ["Spot Dodge", "Roll Forwards", "Roll Backwards", "Shield", "Jab", "Forward Tilt", "Up Tilt", "Down Tilt", "Forward Smash",
           "Up Smash", "Down Smash", "Grab", "Neutral Special", "Dash", "Dash Attack", "Jump", "Neutral Air", "Air Dodge"]


def names_by_id(lines):
    out = {}
    for line in lines:
        if "cpu_ids" in line:
            for pair in line.split("cpu_ids ", 1)[1].split():
                name, _, value = pair.partition("=")
                out.setdefault(int(value), []).append(name.replace("FIGHTER_STATUS_KIND_", ""))
    return out


def main():
    label, actions = sys.argv[1], sys.argv[2:] or DEFAULT
    phase = f"live-{label}"
    gen = Path("/tmp/tr-pkg5/gen")
    gen.mkdir(parents=True, exist_ok=True)
    results = []
    for action in actions:
        body = mk.settings(mash_state=mk.row("mash_state", action, names=mk.ACTIONS), mash_triggers=mk.row("mash_triggers", "Always", names=mk.TRIGGERS))
        name = "m-" + re.sub(r"[^a-z0-9]+", "-", action.lower()) + ".json"
        (gen / name).write_text(json.dumps(body, separators=(",", ":")))
        subprocess.run(["/usr/bin/scp", "-q", *al.KEY, str(gen / name), f"intpa@dreamland-ally:C:/Eden/trainingrooms-pkg5/settings/{name}"], check=True)
        al.fetch(label)
        before = len((al.LIVE / f"{label}.practice-diagnostics.log").read_text(errors="replace").splitlines())
        ao.ops(phase, [["settings", f"C:\\Eden\\trainingrooms-pkg5\\settings\\{name}"], ["waitplug", "TrainingRooms/practice-diagnostics.log", "diag_settings_reload", 20], ["hold", ["l", "r", "a"], 400, 1.0], ["sleep", 4]], 120)
        log = al.fetch(label)
        text = log.read_text(errors="replace").splitlines()
        ids = names_by_id(text)
        new = text[before:]
        bits = sorted({m.group(1) for l in new for m in [re.search(r"command_query entry=1 cat=1 .* bits=(0x[0-9a-f]+)", l)] if m and m.group(1) != "0x0"})
        statuses = []
        for l in new:
            m = re.search(r"cpu_event status entry=1 .* status=(-?\d+) ", l)
            if m and (not statuses or statuses[-1] != int(m.group(1))):
                statuses.append(int(m.group(1)))
        shown = [(ids.get(s, [str(s)])[0]) for s in statuses][:14]
        results.append((action, bits, shown))
        print(f"{action:16} bits={bits} statuses={shown}", flush=True)
    (Path(__file__).resolve().parent.parent / "runs" / label).mkdir(parents=True, exist_ok=True)
    (Path(__file__).resolve().parent.parent / "runs" / label / "mash-each-action.json").write_text(json.dumps(results, indent=1))


if __name__ == "__main__":
    main()
