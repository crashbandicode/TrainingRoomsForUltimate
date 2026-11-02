"""Launch Eden on the Ally for a staged label (run `ally-stage.ps1` first): a scheduled task starts `wa_s5.py launch`, which
survives the ssh session. Prints the run state. Stop it with `ally_ops.py ps` and `wa_s5.py kill live-<label>`.

    ally_launch.py s5-03
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ally_ops as ao

SNIPPET = r"""
$O = 'C:\Eden\trainingrooms-pkg5'; $PYW = 'C:\Program Files\Python312\pythonw.exe'; $phase = 'live-LABEL'
if (Get-Process eden -ErrorAction SilentlyContinue) { throw 'eden.exe is running' }
$tr = '"' + $PYW + '" "' + $O + '\wa_s5.py" launch ' + $phase
schtasks /create /tn "trp5-$phase" /sc once /st 00:00 /tr $tr /f | Out-Null
schtasks /run /tn "trp5-$phase" | Out-Null
$deadline = (Get-Date).AddSeconds(90)
while ((Get-Date) -lt $deadline -and -not (Test-Path "$O\runs\$phase\run.json")) { Start-Sleep -Milliseconds 500 }
if (Test-Path "$O\runs\$phase\run.json") { (Get-Content -Raw "$O\runs\$phase\run.json" | ConvertFrom-Json | % { "state=" + $_.state + " pid=" + $_.pid }) } else { 'NO run.json' }
"""

if __name__ == "__main__":
    print(ao.ps(SNIPPET.replace("LABEL", sys.argv[1]), 200))