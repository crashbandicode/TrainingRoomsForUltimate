"""After `ally-build.sh gates` on the Ally: fetch the two NROs and the gate logs, hash and size them, build the two plugin sets
(`release`, `diag`: the providers, the CPU pilot, the tile cursor and ours) with their sha256.txt, push them to the Ally root and
print the gate summary.

    ally_fetch.py <tag>        e.g. TAG (a folder under /home/intpa/trainingrooms-eden-pkg5/nro/)
"""
import hashlib
import shutil
import subprocess
import sys
from pathlib import Path

KEY = ["-o", "BatchMode=yes", "-o", "UserKnownHostsFile=/tmp/opencode/ally-known-hosts", "-i", "/home/intpa/.ssh/id_rsa"]
ALLY = "intpa@dreamland-ally"
OUT = "C:/Users/intpa/trainingrooms-pkg5/out"
BASE = Path("/home/intpa/trainingrooms-eden-pkg5/nro")
AID = ["libnro_hook.nro", "libparam_hook.nro", "libssbu_cpu_pilot.nro", "libssbu_tile_cursor.nro"]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def scp(*args):
    subprocess.run(["/usr/bin/scp", "-q", *KEY, *args], check=True)


def main():
    tag = sys.argv[1]
    d = BASE / tag
    for sub in ("release", "diag", "logs"):
        (d / sub).mkdir(parents=True, exist_ok=True)
    for mode in ("release", "diag"):
        scp(f"{ALLY}:{OUT}/libtraining_rooms.{mode}.nro", str(d / mode / "libtraining_rooms.nro"))
    for log in ("switch-release.log", "switch-diag.log", "host-test.log", "host-clippy.log", "host-nodefault.log", "GATES.txt"):
        scp(f"{ALLY}:{OUT}/{log}", str(d / "logs" / log))
    print(Path(d / "logs/GATES.txt").read_text().strip())
    warnings = sum(1 for log in ("switch-release.log", "switch-diag.log") for line in (d / "logs" / log).read_text().splitlines() if line.startswith("warning"))
    passed = failed = 0
    for line in (d / "logs/host-test.log").read_text().splitlines():
        if line.startswith("test result:"):
            parts = line.split()
            passed += int(parts[3])
            failed += int(parts[5])
    print(f"switch warnings={warnings} host tests passed={passed} failed={failed}")
    for mode in ("release", "diag"):
        nro = d / mode / "libtraining_rooms.nro"
        size = subprocess.run(["/usr/bin/python3", "-I", "/home/intpa/ssbu-mod-playbook/tools/build/nro_size.py", str(nro)], capture_output=True, text=True).stdout.strip()
        print(mode, sha(nro))
        print(size)
        target = BASE / f"set-{tag}" / mode
        target.mkdir(parents=True, exist_ok=True)
        for name in AID:
            shutil.copyfile(BASE / "aid" / name, target / name)
        shutil.copyfile(nro, target / "libtraining_rooms.nro")
        lines = [f"{sha(p)}  {p.name}" for p in sorted(target.glob("*.nro"))]
        (target / "sha256.txt").write_text("\n".join(lines) + "\n")
        files = [str(p) for p in sorted(target.iterdir())]
        scp(*files, f"{ALLY}:C:/Eden/trainingrooms-pkg5/nro/{mode}/")


if __name__ == "__main__":
    main()
