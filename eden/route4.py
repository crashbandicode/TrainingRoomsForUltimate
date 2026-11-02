"""Pkg4 adapter to the lead-authorized shared harness225d16b.

The shared route supplies screen/log gates and macro receipts. This adapter keeps
pkg4's profile and evidence private and explicitly launches remote PowerShell7.
No claim, launch, or profile mutation occurs on import.
"""
import base64
import hashlib
import os
import subprocess
import sys
from functools import lru_cache
from pathlib import Path

HERE = Path(__file__).resolve().parent
HARNESS = Path(os.environ.get("TR_SHARED_HARNESS",
    "/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-harness/eden/harness"))
PINNED = {
    "route.py": "ee83f4d8e7c5faf428f591df9d6cf80a01dbd6fb5dba7a4fbaad18192387f74c",
    "params.py": "4305e2244a09c0440a2060b36b50d4c495b1cda9098e13c7aecd7fcb59b19fdc",
}
for name, expected in PINNED.items():
    if hashlib.sha256((HARNESS / name).read_bytes()).hexdigest() != expected:
        raise RuntimeError(f"shared harness {name} differs from authorized225d16b")
sys.path.insert(0, str(HARNESS))
import params
import route as shared

WORK = "/home/intpa/trainingrooms-eden-p4"
YOGA = "memento-claw-yoga-ubuntu"
PROFILE_SD = f"{WORK}/profiles/t4/user/sdmc/ultimate"
REFS = HERE / "refs"
SSH = ["/usr/bin/ssh", "-o", "BatchMode=yes", "-o", "ControlMaster=auto",
       "-o", "ControlPath=/tmp/trssh-s4-%C", "-o", "ControlPersist=900", YOGA]
PARAMS = params.load(env={}, host="yoga",
    owner="Codex 01a19c9f-04ad-7f72-8d2f-440ee0a6a04f/pkg4",
    prefix="s4", stream="pkg4", root=WORK, profile="t4", display=":94",
    refs=str(REFS), install_gate="result=installed")


def remote(command, timeout=120):
    encoded = base64.b64encode(command.encode("utf-16le")).decode()
    result = subprocess.run([*SSH, "/usr/bin/pwsh", "-NoLogo", "-NoProfile",
        "-NonInteractive", "-EncodedCommand", encoded],
        capture_output=True, text=True, timeout=timeout)
    return result.returncode, result.stdout.strip(), result.stderr.strip()


class Transport(shared.Ssh):
    def gv(self, args, timeout=120):
        rc, out, err = remote(
            f"/usr/bin/python3 -I {WORK}/harness/gv.py {args}", timeout)
        if rc:
            raise RuntimeError(f"gv {args} failed: {out} {err}")
        return out

    def shot(self, name):
        out = HERE / "runs" / self.label / "shots"
        out.mkdir(parents=True, exist_ok=True)
        path = self.gv(f"shot {self.label} {name}").splitlines()[-1].strip()
        subprocess.run(["/usr/bin/scp", "-q", "-o", f"ControlPath={self.control}",
            f"{YOGA}:{path}", str(out)], check=True, timeout=60)
        return out / Path(path).name


@lru_cache
def session(label):
    params.label_ok(PARAMS, label)
    return shared.Session(Transport(PARAMS, label), REFS)


def call(label, method, params=None):
    return session(label).t.call(method, params)


def press(label, buttons, hold=150):
    return session(label).press(buttons, hold)


def snap(label, name):
    return session(label).t.shot(name)


def main(argv=None):
    parser = shared.build_parser()
    parser.set_defaults(stage_steps="down:3,right:8", tile_gate=41,
                        stage_ref="sss-t41.png", install_gate=PARAMS.install_gate)
    args = parser.parse_args(argv)
    shared.to_training(session(args.label), args)


if __name__ == "__main__":
    main()
