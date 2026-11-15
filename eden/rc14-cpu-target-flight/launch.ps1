# rc.14 cpu-target-flight: ship a staged boot to the Yoga or the XPS and start it (worker A's launch-yoga-eggs.ps1, with a wait):
# every 60 s until the up script admits it (exit 0) or the deadline passes. Exit codes 3 (an Eden on the host), 4/5 (port or
# display busy) and 7 (another Eden in its load phase) wait; anything else stops.
#   /usr/bin/pwsh -NoLogo -NoProfile -NonInteractive -File launch.ps1 -L <label> -Target yoga|xps [-WaitMinutes 90]
param([Parameter(Mandatory)][string]$L, [Parameter(Mandatory)][ValidateSet('yoga', 'xps')][string]$Target, [int]$WaitMinutes = 90)
$ErrorActionPreference = 'Stop'
$Y = @{ yoga = 'memento-claw-yoga-ubuntu'; xps = 'intpa@ubuntu-xps.lan' }[$Target]
$out = '/home/intpa/smash-training-lobby/artifacts/rc14-cpu-target-flight/eden'
$up = "$out/$Target-up-$L.sh"
foreach ($f in "$out/$L.tar", $up) { if (-not (Test-Path $f)) { throw "missing $f" } }
$deadline = (Get-Date).AddMinutes($WaitMinutes)
while ($true) {
    $now = (Get-Date).ToUniversalTime().ToString('HH:mm:ssZ')
    & scp -q -o BatchMode=yes "$out/$L.tar" "${Y}:eden-runs/$L.tar"
    & scp -q -o BatchMode=yes /tmp/opencode/wa-look/tr2526-base.json "${Y}:eden-runs/$L-base.json"
    & scp -q -o BatchMode=yes $up "${Y}:/tmp/cf-up-$L.sh"
    $text = & ssh -o BatchMode=yes $Y "bash /tmp/cf-up-$L.sh 2>&1 | tail -12; exit `${PIPESTATUS[0]}"
    $rc = $LASTEXITCODE
    "$now up rc=$rc"
    $text | ForEach-Object { "  $_" }
    if ($rc -eq 0) { "$now STARTED $L"; exit 0 }
    if ($rc -notin 3, 4, 5, 7) { "$now STOP: up failed with rc=$rc"; exit $rc }
    & ssh -o BatchMode=yes $Y "rm -f eden-runs/$L.tar eden-runs/$L-base.json"
    if ((Get-Date) -gt $deadline) { "$now GAVE UP after $WaitMinutes min"; exit 9 }
    Start-Sleep -Seconds 60
}
