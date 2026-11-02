"""Launch Eden on the Ally for a staged label (stage the rig first): a scheduled task starts the stream's adapter (`wa_s2.py` /
`wa_s5.py` in <root>\\tr-<stream>, `ally_conf`), which survives the ssh session. Prints the run state. Stop it with `ally_ops.py ps` and
`<adapter> kill live-<label>`.

    ally_launch.py s2-03 | s5-03
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ally_conf as conf
import ally_ops as ao

SNIPPET = r"""
$O = 'ROOT'; $PYW = 'PYTHONW'; $phase = 'live-LABEL'
if (Get-Process eden -ErrorAction SilentlyContinue) { throw 'eden.exe is running' }
$tr = '"' + $PYW + '" "ADAPTER" launch ' + $phase
schtasks /create /tn "TASK-$phase" /sc once /st 00:00 /tr $tr /f | Out-Null
schtasks /run /tn "TASK-$phase" | Out-Null
$deadline = (Get-Date).AddSeconds(90)
while ((Get-Date) -lt $deadline -and -not (Test-Path "$O\runs\$phase\run.json")) { Start-Sleep -Milliseconds 500 }
if (Test-Path "$O\runs\$phase\run.json") { (Get-Content -Raw "$O\runs\$phase\run.json" | ConvertFrom-Json | % { "state=" + $_.state + " pid=" + $_.pid }) } else { 'NO run.json' }
"""


def snippet(label):
    name = conf.stream(label)
    return (SNIPPET.replace("ROOT", conf.win(conf.ROOT)).replace("PYTHONW", conf.PYTHONW).replace("ADAPTER", conf.adapter(name))
            .replace("TASK", conf.task_prefix(name)).replace("LABEL", label))


if __name__ == "__main__":
    print(ao.ps(snippet(sys.argv[1]), 200))
