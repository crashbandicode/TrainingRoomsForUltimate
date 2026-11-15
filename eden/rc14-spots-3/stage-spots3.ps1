param(
    [Parameter(Mandatory)][string]$Nro,
    [Parameter(Mandatory)][ValidateSet('yoga', 'xps')][string]$OnHost,
    [Parameter(Mandatory)][string]$Commit,
    [string]$Label = '',
    [string]$Display = '86',
    [string]$Port = '6586',
    [switch]$Wait,
    [switch]$Launch
)
# spots-3: stages the one-boot plan (rows/, built by build_spots3_plan.py) on the Yoga or the XPS with integration-3's card set (inputs-base),
# base config (tr2526-base.json) and up template (up-final.sh), read-only; owner `Claude spots-3`. Adapted from integration-3's stage-final.ps1.
# Without -Launch it builds the stage tarball, dry-runs the rows and renders the up script; -Launch copies and starts the detached chain (up,
# run, hold <= 25 min or RELEASE, down). Run -Launch only after the primary named the host.
$ErrorActionPreference = 'Stop'
$e = '/home/intpa/smash-training-lobby/artifacts/phase23/rc14-int3/eden'
$me = '/home/intpa/smash-training-lobby/artifacts/rc14-spots-3/eden'
$hosts = @{
    yoga = @{ ssh = 'memento-claw-yoga-ubuntu'; key = 'yoga'; name = 'yoga-ubuntu'; ledger = "c.pop('ledger_ssh', None)" }
    xps  = @{ ssh = 'intpa@ubuntu-xps.lan'; key = 'xps'; name = 'ubuntu-xps'; ledger = "c['ledger_ssh'] = 'memento-claw-yoga-ubuntu'" }
}[$OnHost]
if (-not $Label) { $Label = "spots3-$Commit-1" }
$s = "$me/stage-$Label"
$sha = (Get-FileHash -Algorithm SHA256 $Nro).Hash.ToLower()
Remove-Item -Recurse -Force $s -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $s | Out-Null
Copy-Item -Recurse "$e/inputs-base" "$s/inputs"
Copy-Item -Recurse "$me/rows" "$s/rows-hang"
Get-ChildItem "$s/rows-hang" -Recurse -Directory -Filter __pycache__ | Remove-Item -Recurse -Force
Copy-Item $Nro "$s/inputs/plugins/libtraining_rooms.nro" -Force
$m = [IO.File]::ReadAllText("$s/inputs/manifest.json") | ConvertFrom-Json -AsHashtable
$m['variant'] = "spots3-$Commit"
foreach ($p in @($m['plugins'] | Where-Object { $_['name'] -eq 'libtraining_rooms.nro' })) { $p['sha256'] = $sha; $p['from'] = $Nro }
[IO.File]::WriteAllText("$s/inputs/manifest.json", ($m | ConvertTo-Json -Depth 8))
if ((Get-FileHash -Algorithm SHA256 "$s/inputs/plugins/libtraining_rooms.nro").Hash.ToLower() -ne $sha) { throw 'the staged TrainingRooms NRO is not the given one' }
& /usr/bin/python3 -B -I /home/intpa/ssbu-mod-playbook/tools/eden-runner/runner.py dry-run "$s/rows-hang/rows.json" --out "$me/dry-out-$Label" 2>&1 | Select-String -Pattern '"result"|"errors"' | ForEach-Object { $_.Line.Trim() }
$auth = "rc.14 spots-3 (Claude spots-3, granted by the primary): one instance on the spots-3 diag $($sha.Substring(0, 8)) = rc14/spots-3 $Commit, host $OnHost, Training only, muted (volume 0), the GDB stub on before game.load. Nothing pushed, no card write."
& tar -cf "$me/$Label.tar" -C $s .
$u = [IO.File]::ReadAllText("$e/up-final.sh").Replace("integration-3-__PLAN__", 'spots-3').Replace('/tmp/int3-edens.txt', '/tmp/spots3-edens.txt').Replace('__L__', $Label).Replace('__PORT__', $Port).Replace('__DISP__', $Display).Replace('__AUTH__', $auth).Replace('__HOSTKEY__', $hosts.key).Replace('__HOSTNAME__', $hosts.name).Replace('__LEDGER__', $hosts.ledger).Replace('__PLAN__', 'spots3').Replace('__WAITQ__', $(if ($Wait) { '1' } else { '0' })).Replace('__THEN__', '')
[IO.File]::WriteAllText("$me/up-$Label.sh", $u)
"staged label=$Label host=$OnHost display=:$Display port=$Port nro=$($sha.Substring(0, 12)) tar=$((Get-Item "$me/$Label.tar").Length) bytes"
if (-not $Launch) { return }
$Y = $hosts.ssh
& scp -q -o BatchMode=yes "$me/$Label.tar" "${Y}:eden-runs/$Label.tar"
& scp -q -o BatchMode=yes "$e/tr2526-base.json" "${Y}:eden-runs/$Label-base.json"
& scp -q -o BatchMode=yes "$me/up-$Label.sh" "${Y}:/tmp/spots3-up-$Label.sh"
& ssh -o BatchMode=yes $Y "bash -c 'bash /tmp/spots3-up-$Label.sh 2>&1 | tail -12'"
