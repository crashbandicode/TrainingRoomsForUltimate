"""Copy the harness, its parameters, the macros and the NROs to the host that runs Eden (<root> there).

Playbook origin: none (the playbook's tools run where they are copied by hand; the streams scp'd their
copies to the work root on the Yoga, which is what this does, once, with the parameters as a config file).

    deploy.py [--nro PATH]...     TR_HOST names the target; on this machine it only creates <root>/harness
Layout on the host: <root>/harness/{*.py,*.ps1,harness.config.json}  <root>/macros/  <root>/nro/
Prints the NRO paths on the host (pass them to `solo.ps1 -Nro`). Needs a fresh copy after a harness change.
"""
import argparse
import subprocess
import sys
import tempfile
from dataclasses import replace
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import params  # noqa: E402
import route  # noqa: E402

SSH = ["/usr/bin/ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=15"]


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--nro", action="append", default=[])
    args = ap.parse_args(argv)
    p = params.need(params.load(), "owner", "prefix", "root", "display")
    here = route._here(p)
    # The tile cursor and the CPU pilot are NROs of the driver's machine: they travel with the others and the config
    # the host reads names their place in <root>/nro.
    aids = {k: getattr(p, k) for k in ("tile", "pilot") if getattr(p, k) and Path(getattr(p, k)).is_file()}
    args.nro = [*args.nro, *[v for v in aids.values() if v not in args.nro]]
    p = replace(p, **{k: f"{p.root}/nro/{Path(v).name}" for k, v in aids.items()})
    files = [*HERE.glob("*.py"), *HERE.glob("*.ps1")]
    with tempfile.TemporaryDirectory() as tmp:
        config = Path(tmp) / "harness.config.json"
        config.write_text(params.dump(p), encoding="utf-8")
        dest = f"{p.root}/harness"
        macros = sorted(p.macros_dir.glob("*.edenmacro.json"))
        if here:  # this machine runs Eden: the same layout, without ssh
            for sub in ("harness", "macros", "nro"):
                Path(p.root, sub).mkdir(parents=True, exist_ok=True)
            for f in [*files, config]:
                Path(dest, f.name).write_bytes(f.read_bytes())
            for f in macros:
                Path(p.root, "macros", f.name).write_bytes(f.read_bytes())
            for n in args.nro:
                Path(p.root, "nro", Path(n).name).write_bytes(Path(n).read_bytes())
            print(json_paths(p, args.nro))
            return
        target = p.info["ssh"]
        subprocess.run([*SSH, target, f"New-Item -ItemType Directory -Force {dest},{p.root}/macros,{p.root}/nro | Out-Null"],
                       check=True)
        subprocess.run(["/usr/bin/scp", "-q", "-o", "BatchMode=yes", *map(str, [*files, config]), f"{target}:{dest}/"], check=True)
        if macros:
            subprocess.run(["/usr/bin/scp", "-q", "-o", "BatchMode=yes", *map(str, macros), f"{target}:{p.root}/macros/"], check=True)
        if args.nro:
            subprocess.run(["/usr/bin/scp", "-q", "-o", "BatchMode=yes", *args.nro, f"{target}:{p.root}/nro/"], check=True)
    print(json_paths(p, args.nro))


def json_paths(p, nros):
    return "\n".join(f"{p.root}/nro/{Path(n).name}" for n in nros) or f"{p.root}/harness"


if __name__ == "__main__":
    main()
