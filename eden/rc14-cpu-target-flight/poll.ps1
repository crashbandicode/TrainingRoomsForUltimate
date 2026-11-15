# rc.14 cpu-target-flight: copy a running boot's row receipts back and print each row's status (one look, no input).
#   /usr/bin/pwsh -NoLogo -NoProfile -NonInteractive -File poll.ps1 -L <label> [-Target yoga|xps]
param([Parameter(Mandatory)][string]$L, [ValidateSet('yoga', 'xps')][string]$Target = 'yoga')
$Y = @{ yoga = 'memento-claw-yoga-ubuntu'; xps = 'intpa@ubuntu-xps.lan' }[$Target]
$d = "/home/intpa/smash-training-lobby/artifacts/rc14-cpu-target-flight/eden/live-$L"
New-Item -ItemType Directory -Force $d | Out-Null
& scp -q -o BatchMode=yes "${Y}:eden-runs/$L/out/*.json" "${Y}:eden-runs/$L/chain.log" "$d/" 2>$null
foreach ($f in Get-ChildItem $d -Filter '*.json' | Where-Object { $_.Name -match '^(cf-|z-|g-)' } | Sort-Object LastWriteTime) {
    $r = Get-Content -Raw $f.FullName | ConvertFrom-Json
    $why = if ($r.error) { ' ' + ([string]$r.error).Substring(0, [Math]::Min(300, ([string]$r.error).Length)) } else { '' }
    '{0} {1}{2}' -f $f.BaseName, $r.status, $why
}
if (Test-Path "$d/chain.log") { Get-Content "$d/chain.log" | Select-Object -Last 2 }
