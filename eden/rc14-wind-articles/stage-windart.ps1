param(
    [Parameter(Mandatory)][string]$Nro,
    [Parameter(Mandatory)][ValidateSet('yoga', 'xps')][string]$OnHost,
    [string]$Label = 'windart-1',
    [string]$Commit = '',
    [string]$Display = '87',
    [string]$Port = '6587',
    [switch]$Launch
)
# wind-articles (11-15): stage ONE boot of this lane's plan (built into artifacts/rc14-wind-audit/stage/plan/rows), copied from rc14-hide-cpu-tag/stage-hidetag.ps1, itself from integration-3's stage-final.ps1 (Yoga/XPS
# branch only; the base inputs, base config and card set are integration-3's, read-only). Without -Launch it builds the stage tarball,
# dry-runs the rows and renders the up script; -Launch copies them to the host and starts the detached chain (up, run, hold <= 25 min or
# RELEASE, down). Launch only after the primary granted the slot.
$ErrorActionPreference = 'Stop'
$e = '/home/intpa/smash-training-lobby/artifacts/phase23/rc14-int3/eden'
$d = '/home/intpa/tr-fixes-20261113/wind-articles/eden/rc14-wind-articles'
$t = '/home/intpa/smash-training-lobby/artifacts/rc14-wind-audit/stage'
$hosts = @{
    yoga = @{ ssh = 'memento-claw-yoga-ubuntu'; key = 'yoga'; name = 'yoga-ubuntu'; ledger = "c.pop('ledger_ssh', None)" }
    xps  = @{ ssh = 'intpa@ubuntu-xps.lan'; key = 'xps'; name = 'ubuntu-xps'; ledger = "c['ledger_ssh'] = 'memento-claw-yoga-ubuntu'" }
}[$OnHost]
$s = "$t/stage-$Label"
$sha = (Get-FileHash -Algorithm SHA256 $Nro).Hash.ToLower()
if (-not $Commit) { $Commit = (git -C /home/intpa/tr-fixes-20261113/wind-articles rev-parse --short=8 HEAD) }
& /usr/bin/python3 -I "$d/build_plan.py" "$t/plan/rows"
Remove-Item -Recurse -Force $s -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $s | Out-Null
Copy-Item -Recurse "$e/inputs-base" "$s/inputs"
Copy-Item -Recurse "$t/plan/rows" "$s/rows-hang"
Get-ChildItem "$s/rows-hang" -Recurse -Directory -Filter __pycache__ | Remove-Item -Recurse -Force
Copy-Item $Nro "$s/inputs/plugins/libtraining_rooms.nro" -Force
$m = [IO.File]::ReadAllText("$s/inputs/manifest.json") | ConvertFrom-Json -AsHashtable
$m['variant'] = "wind-articles-$Commit"
foreach ($p in @($m['plugins'] | Where-Object { $_['name'] -eq 'libtraining_rooms.nro' })) { $p['sha256'] = $sha; $p['from'] = $Nro }
[IO.File]::WriteAllText("$s/inputs/manifest.json", ($m | ConvertTo-Json -Depth 8))
if ((Get-FileHash -Algorithm SHA256 "$s/inputs/plugins/libtraining_rooms.nro").Hash.ToLower() -ne $sha) { throw 'the staged TrainingRooms NRO is not the given one' }
& /usr/bin/python3 -B -I /home/intpa/ssbu-mod-playbook/tools/eden-runner/runner.py dry-run "$s/rows-hang/rows.json" --out "$t/dry-out-$Label" 2>&1 | Select-String -Pattern '"result"|"errors"' | ForEach-Object { $_.Line.Trim() }
$auth = "rc.14 wind-articles (Claude Opus lane, granted by the primary): one instance, TR diag $($sha.Substring(0, 8)) = rc14/wind-articles $Commit, host $OnHost, Training only, muted (volume 0), the GDB stub on before game.load. Nothing pushed, no card write.".Replace("'", '')
& tar -cf "$t/$Label.tar" -C $s .
$u = [IO.File]::ReadAllText("$d/up-windart.sh").Replace('__L__', $Label).Replace('__PORT__', $Port).Replace('__DISP__', $Display).Replace('__AUTH__', $auth).Replace('__HOSTKEY__', $hosts.key).Replace('__HOSTNAME__', $hosts.name).Replace('__LEDGER__', $hosts.ledger).Replace('__PLAN__', 'windart').Replace('__WAITQ__', '0').Replace('__THEN__', '')
[IO.File]::WriteAllText("$t/up-$Label.sh", $u)
"staged label=$Label host=$OnHost display=:$Display port=$Port nro=$($sha.Substring(0, 12)) tar=$((Get-Item "$t/$Label.tar").Length) bytes"
if (-not $Launch) { return }
$Y = $hosts.ssh
& scp -q -o BatchMode=yes "$t/$Label.tar" "${Y}:eden-runs/$Label.tar"
& scp -q -o BatchMode=yes "$e/tr2526-base.json" "${Y}:eden-runs/$Label-base.json"
& scp -q -o BatchMode=yes "$t/up-$Label.sh" "${Y}:/tmp/windart-up-$Label.sh"
& ssh -o BatchMode=yes $Y "bash -c 'bash /tmp/windart-up-$Label.sh 2>&1 | tail -12'"
