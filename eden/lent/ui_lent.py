"""ui-screens: Surface-side driver of the LENT Ally rig C:\\Eden\\c1162-w5-ally (restart rules 2026-11-02). Every remote step is one MSI PowerShell 7
snippet (`eden/ally/ally_ops.ps`); the rig's own files are never edited: this stream's files live under uiq\\ (stage_ui.ps1, wui.py, payload\\,
macros\\, set-aside\\) and the rig's CLAIM is set and put back by stage_ui.ps1.

    ui_lent.py deploy                         scp stage_ui.ps1 + wui.py + the route macro to uiq\\
    ui_lent.py stage <set> <local nro>        payload uiq\\payload\\<set>: our NRO + the pilot, tile cursor and the two providers (sha256.txt)
    ui_lent.py setaside | restore             stage_ui.ps1 modes (restore verifies against the pre manifest)
    ui_lent.py boot <set> [<prev phase>]      stage_ui.ps1 boot
    ui_lent.py launch <phase> | kill <phase>  wui.py launch (scheduled task) / kill (Job object)
    ui_lent.py route <phase>                  mute, load, title -> main menu -> Training (Final Destination, Steve vs the CPU)
Importable: `import ui_lent as u; u.ops(phase, [...]); u.shot(phase, name); u.plug(phase, rel, regex)`.
"""
import base64
import hashlib
import json
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ally"))
import ally_ops as ao  # noqa: E402  (the MSI pwsh transport; root-agnostic)

HERE = Path(__file__).resolve().parent
O = r"C:\Eden\c1162-w5-ally"
W = O + r"\uiq"
PY = r"C:\Program Files\Python312\python.exe"
PYW = r"C:\Program Files\Python312\pythonw.exe"
KEY = ["-o", "BatchMode=yes", "-o", "UserKnownHostsFile=/tmp/opencode/ally-known-hosts", "-i", "/home/intpa/.ssh/id_rsa"]
HOST = "intpa@dreamland-ally"
AID = r"C:\Eden\trainingrooms-pkg5\nro\diag"   # read only: the four aid NROs (sha-checked by the stage)
AIDS = ["libnro_hook.nro", "libparam_hook.nro", "libssbu_cpu_pilot.nro", "libssbu_tile_cursor.nro"]
MACRO = "/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-pkg5/eden/macros/tr-main-to-stage-select.edenmacro.json"
SHOTS = Path("/tmp/tr-ui/shots")
TILE = "ssbu-tile-cursor/events.log"
DIAG = "TrainingRooms/practice-diagnostics.log"


def ps(snippet, timeout=180):
    return ao.ps(snippet, timeout)


def scp(local, remote):
    subprocess.run(["/usr/bin/scp", "-q", *KEY, str(local), f"{HOST}:{remote}"], check=True)


def ops(phase, items, timeout=300):
    b64 = base64.b64encode(json.dumps(items).encode()).decode()
    out = ps(f"& '{PY}' '{W}\\wui.py' ops {phase} {b64}", timeout)
    try:
        return json.loads(out[out.index("["):])
    except ValueError:
        raise SystemExit(f"ops failed: {out[-800:]}")


def get(phase, rel, dest):
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    subprocess.run(["/usr/bin/scp", "-q", *KEY, f"{HOST}:C:/Eden/c1162-w5-ally/runs/{phase}/{rel}", str(dest)], check=True)
    return dest / Path(rel).name


def shot(phase, name, width=1280):
    from PIL import Image
    ops(phase, [["gshot", name, 2, width, 700]])
    gif = get(phase, f"shots/{name}.gif", SHOTS / phase)
    image = Image.open(gif)
    image.seek(image.n_frames - 1)
    png = SHOTS / phase / f"{name}.png"
    image.convert("RGB").save(png)
    return png


def plug(phase, rel, regex, n=5):
    return ops(phase, [["plug", rel, regex, n]])[0]


def count(phase, rel, regex):
    return plug(phase, rel, regex, 1)["count"]


def wait_count(phase, rel, regex, at_least, cap, what):
    end = time.time() + cap
    while time.time() < end:
        if count(phase, rel, regex) >= at_least:
            print(f"GATE ok {what}", flush=True)
            return
        time.sleep(2)
    raise SystemExit(f"GATE FAIL {what}: {regex!r} x{at_least} in {rel}")


def keys(phase, tokens, timeout=300):
    items = []
    for token in tokens.split():
        if re.fullmatch(r"w\d+", token):
            items.append(["sleep", int(token[1:]) / 10])
        elif "+" in token:
            items.append(["hold", token.split("+"), 150, 0.35])
        else:
            items.append(["button", token, 120, 0.3])
    return ops(phase, items, timeout)


def deploy():
    print(ps(f"New-Item -ItemType Directory -Force '{W}', '{W}\\macros', '{W}\\payload' | Out-Null; 'ok'", 60))
    scp(HERE / "stage_ui.ps1", "C:/Eden/c1162-w5-ally/uiq/stage_ui.ps1")
    scp(HERE / "wui.py", "C:/Eden/c1162-w5-ally/uiq/wui.py")
    scp(MACRO, "C:/Eden/c1162-w5-ally/uiq/macros/tr-main-to-stage-select.edenmacro.json")
    print(ps(f"Get-ChildItem '{W}' -Recurse -File | ForEach-Object {{ '{{0}} {{1}}' -f (Get-FileHash $_.FullName).Hash.Substring(0,16).ToLower(), $_.FullName }}", 60))


def stage(name, nro):
    sha = hashlib.sha256(Path(nro).read_bytes()).hexdigest()
    d = f"{W}\\payload\\{name}"
    print(ps(f"New-Item -ItemType Directory -Force '{d}' | Out-Null; foreach ($n in '{','.join(AIDS)}'.Split(',')) {{ Copy-Item '{AID}\\'+$n '{d}\\'+$n -Force }}; 'aids ok'", 90))
    scp(nro, f"C:/Eden/c1162-w5-ally/uiq/payload/{name}/libtraining_rooms.nro")
    print(ps(f"$d='{d}'; $lines = foreach ($f in Get-ChildItem $d -Filter *.nro) {{ '{{0}}  {{1}}' -f (Get-FileHash $f.FullName -Algorithm SHA256).Hash.ToLower(), $f.Name }}; "
             f"[IO.File]::WriteAllText($d + '\\sha256.txt', ($lines -join [Environment]::NewLine) + [Environment]::NewLine); Get-Content $d\\sha256.txt", 90))
    print("local", sha)


def stage_mode(*args):
    print(ps(f"& 'C:\\Program Files\\PowerShell\\7\\pwsh.exe' -NoLogo -NoProfile -NonInteractive -File '{W}\\stage_ui.ps1' " + " ".join(args), 300))


def launch(phase):
    snippet = f"""
if (Get-Process eden -ErrorAction SilentlyContinue) {{ throw 'eden.exe is running' }}
$task = 'trui-{phase}'; $tr = '"{PYW}" "{W}\\wui.py" launch {phase}'
schtasks /create /tn $task /sc once /st 00:00 /tr $tr /f | Out-Null; schtasks /run /tn $task | Out-Null
$deadline = (Get-Date).AddSeconds(90)
while ((Get-Date) -lt $deadline -and -not (Test-Path '{O}\\runs\\{phase}\\run.json')) {{ Start-Sleep -Milliseconds 500 }}
if (Test-Path '{O}\\runs\\{phase}\\run.json') {{ (Get-Content -Raw '{O}\\runs\\{phase}\\run.json' | ConvertFrom-Json | % {{ 'state=' + $_.state + ' pid=' + $_.pid }}) }} else {{ 'NO run.json'; Get-Content '{O}\\{phase}-launcher.log' -ErrorAction SilentlyContinue }}
schtasks /delete /tn $task /f | Out-Null
"""
    print(ps(snippet, 200))


def kill(phase):
    print(ps(f"& '{PY}' '{W}\\wui.py' kill {phase}; 'eden.exe: ' + ((Get-Process eden -ErrorAction SilentlyContinue | Measure-Object).Count)", 120))


def route(phase):
    """Mute, load, title -> main menu (A until it shows), the stage-select macro, Final Destination, Steve, Training. Gated on the tile
    cursor's log and the plugin's lines; a miss stops."""
    ops(phase, [["mute"], ["load"]], 200)
    ops(phase, [["waitstatus", "loading_complete", True, 180]], 260)
    wait_count(phase, "TrainingRooms/events.log", "result=installed", 1, 60, "install line")
    ops(phase, [["sleep", 8]])
    for attempt in range(6):
        keys(phase, "a w50")
        png = shot(phase, f"title-{attempt}")
        from PIL import Image
        if Image.open(png).convert("RGB").getpixel((60, 250))[0] > 150 and Image.open(png).convert("RGB").getpixel((60, 250))[1] < 80:
            print("GATE ok main menu (the red Smash panel)", flush=True)
            break
    else:
        raise SystemExit("GATE FAIL main menu")
    ops(phase, [["macro", W + r"\macros\tr-main-to-stage-select.edenmacro.json", 120], ["sleep", 2]], 240)
    wait_count(phase, TILE, "screen=sss", 1, 20, "stage select")
    ops(phase, [["hold", ["lstick", "right"], 150, 0.6]] * 4 + [["sleep", 1]])
    wait_count(phase, TILE, "settled screen=sss at=4 ", 1, 10, "Final Destination (tile 4)")
    ops(phase, [["button", "a", 150, 1]])
    wait_count(phase, TILE, "screen=css", 1, 40, "character select")
    ops(phase, [["sleep", 2], ["hold", ["lstick", "up"], 150, 1.0]])
    for _ in range(4):
        last = plug(phase, TILE, "settled screen=css", 1)["lines"]
        at = int(re.search(r"at=(\d+)", last[-1]).group(1)) if last else -1
        if at == 78:
            break
        step = "left" if at > 78 else "right"
        if at >= 0 and abs(at - 78) >= 9:
            step = "down" if at < 78 else "up"
        ops(phase, [["hold", ["lstick", step], 150, 0.8]])
    wait_count(phase, TILE, "settled screen=css at=78 ", 1, 10, "Steve (tile 78)")
    ops(phase, [["button", "a", 150, 1.5], ["button", "a", 150, 1]])
    wait_count(phase, DIAG, "cpu_hook_counts frames=", 1, 150, "Training: the match runs (cpu hook counts)")
    print("ROUTE DONE", flush=True)


def main():
    op = sys.argv[1]
    if op == "deploy":
        deploy()
    elif op == "stage":
        stage(sys.argv[2], sys.argv[3])
    elif op in ("setaside", "restore"):
        stage_mode("-Mode", op)
    elif op == "boot":
        stage_mode("-Mode", "boot", "-Set", sys.argv[2], *(["-Prev", sys.argv[3]] if len(sys.argv) > 3 else []))
    elif op == "launch":
        launch(sys.argv[2])
    elif op == "kill":
        kill(sys.argv[2])
    elif op == "route":
        route(sys.argv[2])


if __name__ == "__main__":
    main()
