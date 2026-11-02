"""deluxe-stream rows (Q7 / L1), run ON the host after `route_dx.py` reached Training. Each step is gated on a log line
(the diag build's `practice-diagnostics.log` and the install line in `events.log`) or on the INI on disk, with a cap;
screenshots are taken at each step for the evidence doc.

  boot <label> host|nohost     install line `deluxe_row=` and the registration / adapter lines
  page <label> <n>             Deluxe window cycle: n presses of D-pad Down while L+R are held (the k04 chord), a shot each
  select <label> <n>           n plain D-pad Down presses (row selection inside the open page), a shot
  deluxe <label> on|off        ZR + Right/Left on the selected row; gate: the writer's INI line and the file's value
  menu <label> on|off          the menu row's path (diag request file -> writer::request_replay_capture); same gates
"""
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import route_dx as rt  # noqa: E402

INI = rt.SD / "TrainingRooms/replay_capture.ini"


def boot(label, host):
    line = rt.wait_line(rt.EVENTS, "deluxe_row=", 60, "install line deluxe_row")
    want = "deluxe_row=installed" if host == "host" else "deluxe_row=not_installed reason=host_missing"
    if want not in line:
        print(f"GATE FAIL install line lacks {want!r}", flush=True)
        raise SystemExit(2)
    print(f"GATE ok {want}; heap_allocations=0: {'heap_allocations=0' in line}", flush=True)
    if host == "host":
        rt.wait_line(rt.DIAG, "deluxe_row registered handle=", 10, "registration line")
        rt.wait_line(rt.DIAG, "host_registered=true", 10, "adapter resident with the host")
    else:
        rt.wait_line(rt.DIAG, "deluxe_row not registered reason=host_missing", 10, "no-host line")


def chord(label, buttons, hold=250):
    """Hold `buttons` (comma list) together; the last one is pressed after the others are down."""
    names = buttons.split(",")
    for b in names[:-1]:
        rt.se.client_call(label, "input.button", {"button": b, "state": "press", "lease_ms": hold + 900})
    time.sleep(0.15)
    rt.press(label, names[-1], hold)
    for b in names[:-1]:
        rt.se.client_call(label, "input.button", {"button": b, "state": "release"})


def page(label, n):
    for i in range(int(n)):
        chord(label, "l,r,down")
        time.sleep(0.8)
        rt.shot(label, f"deluxe-cycle{i + 1}")


def select(label, n):
    for _ in range(int(n)):
        rt.press(label, "down", 150)
        time.sleep(0.4)
    rt.shot(label, "deluxe-select")


def ini_value():
    try:
        text = INI.read_text(encoding="utf-8")
    except FileNotFoundError:
        return None
    for line in text.splitlines():
        if line.strip().startswith("enabled="):
            return line.strip().split("=", 1)[1]
    return None


def expect(label, value, before):
    want = "enabled=true" if value == "on" else "enabled=false"
    rt.wait_line(rt.DIAG, f"replay_capture.ini written by the writer thread {want}", 10, f"INI write {value}", after=before)
    deadline = time.time() + 5
    while time.time() < deadline and ini_value() != value:
        time.sleep(0.5)
    print(f"{'GATE ok' if ini_value() == value else 'GATE FAIL'} file enabled={ini_value()}", flush=True)
    rt.shot(label, f"after-{value}")


def deluxe(label, value):
    before = len(rt.lines(rt.DIAG))
    chord(label, "zr," + ("right" if value == "on" else "left"), 200)
    expect(label, value, before)


def menu(label, value):
    before = len(rt.lines(rt.DIAG))
    req = rt.SD / "TrainingRooms/diag-capture.req"
    tmp = req.with_suffix(".tmp")
    tmp.write_text(value, encoding="utf-8")
    tmp.rename(req)
    rt.wait_line(rt.DIAG, "diag_request replay_capture enabled=", 10, "menu-path request", after=before)
    expect(label, value, before)


if __name__ == "__main__":
    op, label, *rest = sys.argv[1:]
    {"boot": boot, "page": page, "select": select, "deluxe": deluxe, "menu": menu}[op](label, *rest)
