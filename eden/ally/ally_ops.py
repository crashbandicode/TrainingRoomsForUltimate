"""Surface-side client of the Ally adapter (`wa_s2.py` on the Ally): send a list of ops, print the replies, fetch files.

    ally_ops.py ops <phase> '<json list of ops>' [timeout_s]
    ally_ops.py ops <phase> @file.json [timeout_s]
    ally_ops.py ps '<pwsh snippet>' [timeout_s]           one MSI PowerShell 7 snippet on the Ally (EncodedCommand)
    ally_ops.py get <phase> <relative path under runs\\<phase>> [dest dir]      scp one file back

Importable: `import ally_ops as ao; ao.ops(phase, [["status"]])`. Remote Windows commands run through the MSI PowerShell 7
(`C:\\Program Files\\PowerShell\\7\\pwsh.exe`, never cmd.exe or Windows PowerShell 5.1); the ssh options are the Ally rig's.
"""
import base64
import json
import subprocess
import sys
from pathlib import Path

SSH = ["-o", "BatchMode=yes", "-o", "ConnectTimeout=15", "-o", "UserKnownHostsFile=/tmp/opencode/ally-known-hosts",
       "-i", "/home/intpa/.ssh/id_rsa"]
HOST = "intpa@dreamland-ally"
ROOT = "C:/Eden/trainingrooms-pkg2"
PYTHON = r"C:\Program Files\Python312\python.exe"
WA = r"C:\Eden\trainingrooms-pkg2\wa_s2.py"


def ps(snippet, timeout=120):
    enc = base64.b64encode(snippet.encode("utf-16-le")).decode()
    cmd = f'"C:\\Program Files\\PowerShell\\7\\pwsh.exe" -NoLogo -NoProfile -NonInteractive -EncodedCommand {enc}'
    r = subprocess.run(["/usr/bin/timeout", str(timeout), "/usr/bin/ssh", *SSH, HOST, cmd], capture_output=True, text=True)
    return (r.stdout + r.stderr).strip()


def ops(phase, items, timeout=300):
    b64 = base64.b64encode(json.dumps(items).encode()).decode()
    return ps(f"& '{PYTHON}' '{WA}' ops {phase} {b64}", timeout)


def get(phase, rel, dest="."):
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    subprocess.run(["/usr/bin/scp", "-q", *SSH, f"{HOST}:{ROOT}/runs/{phase}/{rel}", str(dest)], check=True)
    return dest / Path(rel).name


def main():
    op = sys.argv[1]
    if op == "ops":
        raw = sys.argv[3]
        items = json.loads(Path(raw[1:]).read_text() if raw.startswith("@") else raw)
        print(ops(sys.argv[2], items, int(sys.argv[4]) if len(sys.argv) > 4 else 300))
    elif op == "ps":
        print(ps(sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 120))
    elif op == "get":
        print(get(sys.argv[2], sys.argv[3], sys.argv[4] if len(sys.argv) > 4 else "."))


if __name__ == "__main__":
    main()
