"""Where the Surface-side Ally scripts point, in one place for every stream (packages 2 and 5 wrote the same scripts with their own
roots; merged 2026-11-02).

Restart rules 2026-11-02: on the Ally, TrainingRooms runs ONLY the lent rig C:\\Eden\\c1162-w5-ally (it has an inbound Block rule).
The streams' own roots (C:\\Eden\\trainingrooms-pkg2, ...-pkg5) have no firewall rule, so Windows prompts: never run them, never answer
the prompt, never create firewall rules. Stage and restore the lent rig with its own `stage_a.ps1` discipline (restore and verify after
every run).

    TR_ALLY_ROOT     the rig root on the Ally (default C:/Eden/c1162-w5-ally); the adapter's runs are under <root>/runs/
    TR_ALLY_STREAM   pkg2 | pkg5 when a script has no label to read it from (labels s2-NN / s5-NN name their stream)

A stream's own files on the rig (its adapter `wa_s2.py` / `wa_s5.py`, plugin sets, settings) go to <root>/tr-<stream>/, never into the
rig's own folders.
"""
import os
import re

HOST = "intpa@dreamland-ally"
KEY = ["-o", "BatchMode=yes", "-o", "UserKnownHostsFile=/tmp/opencode/ally-known-hosts", "-i", "/home/intpa/.ssh/id_rsa"]
ROOT = os.environ.get("TR_ALLY_ROOT", "C:/Eden/c1162-w5-ally").replace("\\", "/").rstrip("/")
PYTHON = r"C:\Program Files\Python312\python.exe"
PYTHONW = r"C:\Program Files\Python312\pythonw.exe"
# The stream's label prefix, adapter and scheduled-task prefix.
STREAMS = {"pkg2": ("s2", "wa_s2.py", "trp2"), "pkg5": ("s5", "wa_s5.py", "trp5")}


def stream(label=None):
    """`pkg2` / `pkg5`: from a label or phase (`s2-03`, `live-s5-02`), else from TR_ALLY_STREAM."""
    if label:
        m = re.search(r"(?:^|-)s(\d)-\d{2}$", label)
        if m:
            for name, (prefix, _, _) in STREAMS.items():
                if prefix == "s" + m.group(1):
                    return name
    name = os.environ.get("TR_ALLY_STREAM", "")
    if name not in STREAMS:
        raise SystemExit(f"no stream for {label!r}: use a label s2-NN / s5-NN or set TR_ALLY_STREAM to one of {sorted(STREAMS)}")
    return name


def win(path):
    """A rig path in Windows form (for MSI PowerShell 7 snippets and the adapter's arguments)."""
    return path.replace("/", "\\")


def work(name):
    """The stream's own folder on the rig (forward slashes, for scp)."""
    return f"{ROOT}/tr-{name}"


def adapter(name):
    return win(f"{work(name)}/{STREAMS[name][1]}")


def task_prefix(name):
    return STREAMS[name][2]


def plugin_logs():
    """The plugin's output folder in the rig's profile (scp form)."""
    return f"{ROOT}/profile/user/sdmc/ultimate/TrainingRooms/"


def build_out(name):
    """Where `ally-build.sh gates` leaves the NROs and logs on the Ally (the stream's WSL pack folder)."""
    return f"C:/Users/intpa/trainingrooms-{name}/out"


def local(name):
    """The Surface's scratch folder of the stream (live logs, shots, generated settings)."""
    return f"/tmp/tr-{name}"


def nro_base(name):
    return f"/home/intpa/trainingrooms-eden-{name}/nro"
