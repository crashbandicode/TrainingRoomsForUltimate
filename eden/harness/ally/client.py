"""Surface-side client of the Ally rig: deploy, stage, launch, ops, files, and the route's transport.

Origin: eden/ally/ally_ops.py (ops, ps, get: the MSI PowerShell 7 transport), ally_launch.py (the scheduled-task
launch), ally_conf.py (the rig root: TR_ALLY_ROOT, default the lent rig) and eden/lent/ui_lent.py (deploy, stage,
setaside/boot/restore, kill, shots through Eden's recorder), which were the same code with a stream's name in it.
The parameters are the harness's (params.py): TR_ALLY_ROOT, TR_ALLY_CLAIM, TR_PREFIX, TR_OUTPUT.
Remote Windows commands run through the MSI PowerShell 7 (`C:\\Program Files\\PowerShell\\7\\pwsh.exe`, never
cmd.exe or Windows PowerShell 5.1) with the rig's ssh options. No step here touches the firewall: the lent rig's inbound
Allow rules are the operator's (never added, changed or removed by the harness); never answer or dismiss a prompt.

    client.py deploy                       wa.py, wa.config.json, stage.ps1 and the macros -> <rig>\\tr-<prefix>\\
    client.py stage <set> <nro>...         payload <set>: the NROs and their sha256.txt
    client.py setaside | restore           stage.ps1 (restore verifies against the pre manifest)
    client.py boot <set> [<prev phase>]    stage.ps1 boot
    client.py launch <label> | kill <label> | game <label>     (game: mute, game.load, wait for loading_complete)
    client.py ops <label> '<json list>' | ps '<pwsh snippet>' | get <label> <rel path> [dest]
Importable: `import client; client.ops(p, label, [...])`, `client.AllyTransport(p, label)` for route.py.
"""
import base64
import hashlib
import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import params  # noqa: E402
import route  # noqa: E402

SSH = ["-o", "BatchMode=yes", "-o", "ConnectTimeout=15", "-o", "UserKnownHostsFile=/tmp/opencode/ally-known-hosts",
       "-i", "/home/intpa/.ssh/id_rsa"]
PWSH = r"C:\Program Files\PowerShell\7\pwsh.exe"
PYTHON = r"C:\Program Files\Python312\python.exe"
PYTHONW = r"C:\Program Files\Python312\pythonw.exe"


def win(path):
    return str(path).replace("/", "\\")


def work(p):
    """This stream's folder on the rig (forward slashes, for scp). Its own files go here, never into the rig's."""
    return f"{p.ally_root}/tr-{p.prefix}"


def host(p):
    return p.info["ssh"]


def ps(p, snippet, timeout=120):
    enc = base64.b64encode(("$ProgressPreference = 'SilentlyContinue'; " + snippet).encode("utf-16-le")).decode()
    cmd = f'"{PWSH}" -NoLogo -NoProfile -NonInteractive -EncodedCommand {enc}'
    r = subprocess.run(["/usr/bin/timeout", str(timeout), "/usr/bin/ssh", *SSH, host(p), cmd], capture_output=True, text=True)
    return (r.stdout + r.stderr).strip()


def scp(p, local, remote):
    subprocess.run(["/usr/bin/scp", "-q", *SSH, str(local), f"{host(p)}:{remote}"], check=True)


def ops(p, phase, items, timeout=300):
    b64 = base64.b64encode(json.dumps(items).encode()).decode()
    out = ps(p, f"& '{PYTHON}' '{win(work(p))}\\wa.py' ops {phase} {b64}", timeout)
    try:
        return json.loads(out[out.index("["):])
    except ValueError:
        raise SystemExit(f"ops failed: {out[-800:]}")


def get(p, phase, rel, dest="."):
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    subprocess.run(["/usr/bin/scp", "-q", *SSH, f"{host(p)}:{p.ally_root}/runs/{phase}/{rel}", str(dest)], check=True)
    return dest / Path(rel).name


def deploy(p):
    params.need(p, "prefix", "ally_claim")
    w = win(work(p))
    print(ps(p, f"New-Item -ItemType Directory -Force '{w}', '{w}\\macros', '{w}\\payload' | Out-Null; 'ok'", 60))
    with tempfile.TemporaryDirectory() as tmp:
        config = Path(tmp) / "wa.config.json"
        config.write_text(json.dumps({"ally_root": p.ally_root, "ally_claim": p.ally_claim, "ally_mem_gib": p.ally_mem_gib,
                                      "ally_eden_sha": p.ally_eden_sha, "output": p.output}), encoding="utf-8")
        for f in (HERE / "wa.py", HERE / "stage.ps1", config):
            scp(p, f, f"{work(p)}/{f.name}")
    for f in sorted(p.macros_dir.glob("*.edenmacro.json")):
        scp(p, f, f"{work(p)}/macros/{f.name}")
    print(ps(p, f"Get-ChildItem '{w}' -Recurse -File | ForEach-Object {{ '{{0}} {{1}}' -f (Get-FileHash $_.FullName).Hash.Substring(0,16).ToLower(), $_.FullName }}", 60))


def stage(p, name, nros):
    d = f"{win(work(p))}\\payload\\{name}"
    print(ps(p, f"New-Item -ItemType Directory -Force '{d}' | Out-Null; 'ok'", 60))
    lines = []
    for nro in nros:
        scp(p, nro, f"{work(p)}/payload/{name}/{Path(nro).name}")
        lines.append(f"{hashlib.sha256(Path(nro).read_bytes()).hexdigest()}  {Path(nro).name}")
    with tempfile.TemporaryDirectory() as tmp:
        sums = Path(tmp) / "sha256.txt"
        sums.write_text("\n".join(lines) + "\n", encoding="utf-8")
        scp(p, sums, f"{work(p)}/payload/{name}/sha256.txt")
    print("\n".join(lines))


def stage_mode(p, mode, *extra):
    args = f"-Mode {mode} -Rig '{win(p.ally_root)}' -Work 'tr-{p.prefix}' -Claim '{p.ally_claim}' -Output '{p.output}' " + " ".join(extra)
    print(ps(p, f"& '{PWSH}' -NoLogo -NoProfile -NonInteractive -File '{win(work(p))}\\stage.ps1' {args}", 300))


def launch(p, phase):
    snippet = f"""
$O = '{win(p.ally_root)}'; $phase = '{phase}'; $task = '{p.unit_prefix}-{phase}'
if (Get-Process eden -ErrorAction SilentlyContinue) {{ throw 'eden.exe is running' }}
schtasks /create /tn $task /sc once /st 00:00 /tr ('"{PYTHONW}" "{win(work(p))}\\wa.py" launch ' + $phase) /f | Out-Null; schtasks /run /tn $task | Out-Null
$deadline = (Get-Date).AddSeconds(90)
while ((Get-Date) -lt $deadline -and -not (Test-Path "$O\\runs\\$phase\\run.json")) {{ Start-Sleep -Milliseconds 500 }}
if (Test-Path "$O\\runs\\$phase\\run.json") {{ (Get-Content -Raw "$O\\runs\\$phase\\run.json" | ConvertFrom-Json | % {{ 'state=' + $_.state + ' pid=' + $_.pid }}) }} else {{ 'NO run.json'; Get-Content "$O\\$phase-launcher.log" -ErrorAction SilentlyContinue }}
schtasks /delete /tn $task /f | Out-Null
"""
    print(ps(p, snippet, 200))


def kill(p, phase):
    print(ps(p, f"& '{PYTHON}' '{win(work(p))}\\wa.py' kill {phase}; 'eden.exe: ' + ((Get-Process eden -ErrorAction SilentlyContinue | Measure-Object).Count)", 120))


class AllyTransport(route.Transport):
    """route.py's transport for the rig: every command is one batch of wa.py ops. Screenshots are one frame of Eden's own
    recorder (renderer read-back; the desktop does not matter)."""

    def __init__(self, p, label):
        self.p, self.label = p, label
        self.logs = {"events": f"{p.output}/events.log", "diag": f"{p.output}/practice-diagnostics.log",
                     "tile": "ssbu-tile-cursor/events.log"}

    def run(self, items, timeout=300):
        return ops(self.p, self.label, items, timeout)

    def press(self, buttons, hold=150):
        names = buttons.split(",")
        self.run([["button", names[0], hold, 0.3]] if len(names) == 1 else [["hold", names, hold, 0.35]])

    def stick(self, x, y, ms):
        self.run([["stick", "left", x, y, ms, 0.25]])

    def shot(self, name, width=1280):
        from PIL import Image
        self.run([["gshot", name, 2, width, 700]])
        dest = HERE.parent.parent / "runs" / self.label / "shots"
        gif = get(self.p, self.label, f"shots/{name}.gif", dest)
        image = Image.open(gif)
        image.seek(image.n_frames - 1)
        png = dest / f"{time.strftime('%H%M%S', time.gmtime())}-{name}.png"
        image.convert("RGB").save(png)
        return png

    def lines(self, name, after=0):
        return self.run([["lines", self.logs.get(name, name), after]])[0]["lines"]

    def macro(self, name, cap=120):
        r = self.run([["macro", name, cap]], cap + 120)[0]
        return {"sha256": r["sha256"], "state": r["status"]["state"], "name": name}

    def call(self, method, params_=None):
        return self.run([["call", method, params_ or {}]])[0]["result"]


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    p = params.load(host="ally")
    op = argv[0]
    if op == "deploy":
        deploy(p)
    elif op == "stage":
        stage(p, argv[1], argv[2:])
    elif op in ("setaside", "restore"):
        stage_mode(p, op)
    elif op == "boot":
        stage_mode(p, "boot", "-Set", argv[1], *(["-Prev", argv[2]] if len(argv) > 2 else []))
    elif op == "launch":
        launch(p, argv[1])
    elif op == "kill":
        kill(p, argv[1])
    elif op == "game":  # muted and saved before the game boots, then load and wait for it
        ops(p, argv[1], [["mute"], ["load"]], 200)
        print(json.dumps(ops(p, argv[1], [["waitstatus", "loading_complete", True, 180]], 260)))
    elif op == "ops":
        print(json.dumps(ops(p, argv[1], json.loads(Path(argv[2][1:]).read_text() if argv[2].startswith("@") else argv[2]))))
    elif op == "ps":
        print(ps(p, argv[1]))
    elif op == "get":
        print(get(p, argv[1], argv[2], argv[3] if len(argv) > 3 else "."))
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
