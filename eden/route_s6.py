"""Pkg6 configuration adapter for the lead-approved shared harness at225d16bd.

Use TR_CONFIG=eden/harness-pkg6.json for solo.ps1 as well. Admission remains
subject to the raw ledger, queue, interleave and live process checks.
"""
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
HARNESS = Path("/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-harness/eden/harness")
os.environ.setdefault("TR_CONFIG", str(HERE / "harness-pkg6.json"))
sys.path.insert(0, str(HARNESS))
import params
import route

P = params.load()
SD = P.sdmc / "ultimate"


def press(label, buttons, hold=150):
    route.session(label, P, "local").press(buttons, hold)


def stick(label, x, y, ms):
    route.session(label, P, "local").stick(x, y, ms)


def shot(label, name):
    return route.session(label, P, "local").t.shot(name)


if __name__ == "__main__":
    route.main()
