"""Fetch the plugin's logs of the running Ally boot and print the new `cpu_event` / diag lines since the last call.

    ally_log.py <label> [regex]        label s2-NN / s5-NN; prints lines of practice-diagnostics.log after the ones already shown
    ally_log.py <label> --all [regex]  everything (kept under /tmp/tr-<stream>/live/<label>.diag.log)
"""
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ally_conf as conf

KEY = conf.KEY
REMOTE = f"{conf.HOST}:{conf.plugin_logs()}"


def live(label):
    return Path(conf.local(conf.stream(label))) / "live"


def fetch(label, names=("practice-diagnostics.log", "practice-diagnostics.previous.log", "events.log")):
    folder = live(label)
    folder.mkdir(parents=True, exist_ok=True)
    for name in names:
        subprocess.run(["/usr/bin/scp", "-q", *KEY, REMOTE + name, str(folder / f"{label}.{name}")], capture_output=True)
    return folder / f"{label}.practice-diagnostics.log"


def main():
    label = sys.argv[1]
    rest = sys.argv[2:]
    show_all = "--all" in rest
    pattern = re.compile(next((a for a in rest if not a.startswith("--")), "."))
    log = fetch(label)
    lines = log.read_text(encoding="utf-8", errors="replace").splitlines() if log.exists() else []
    seen_file = live(label) / f"{label}.seen"
    seen = 0 if show_all or not seen_file.exists() else int(seen_file.read_text())
    if seen > len(lines):  # the ring rotated into the previous file: start again
        seen = 0
    for line in lines[seen:]:
        if pattern.search(line):
            print(line[:260])
    seen_file.write_text(str(len(lines)))
    print(f"-- {len(lines)} lines, {len(lines) - seen} new")


if __name__ == "__main__":
    main()
