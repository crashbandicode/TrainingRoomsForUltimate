param(
    [Parameter(Mandatory)][string]$Nro,
    [Parameter(Mandatory)][string]$Rows,
    [string]$Label = '',
    [Parameter(Mandatory)][string]$Commit,
    [ValidateSet('yoga', 'xps')][string]$HostKey = 'yoga',
    [string]$Display = '83',
    [string]$Port = '6583',
    [switch]$Launch
)
# spots-2 (the primary's order 2026-11-14): stages one Eden boot of the spots rig on the XPS, from integration-3's stage-final.ps1 (same card set,
# settings and up script; the owner, rows folder, label, display and port are this lane's). Without -Launch it builds and checks the stage
# tarball and dry-runs the rows; with -Launch it copies them to the XPS and starts the detached chain (up, run, hold <= 25 min or RELEASE, down).
$ErrorActionPreference = 'Stop'
$e = '/home/intpa/smash-training-lobby/artifacts/phase23/rc14-int3/eden'
$t = '/home/intpa/smash-training-lobby/artifacts/rc14-spots-2/eden/stage'
New-Item -ItemType Directory -Force $t | Out-Null
$cfg = @{ host = $HostKey; display = $Display; port = $Port; fixture = $false }
$hosts = @{
    yoga = @{ ssh = 'memento-claw-yoga-ubuntu'; key = 'yoga'; name = 'yoga-ubuntu'; ledger = "c.pop('ledger_ssh', None)" }
    xps  = @{ ssh = 'intpa@ubuntu-xps.lan'; key = 'xps'; name = 'ubuntu-xps'; ledger = "c['ledger_ssh'] = 'memento-claw-yoga-ubuntu'" }
}[$HostKey]
if (-not $Label) { $Label = "spots2-$Commit-1" }
$s = "$t/stage-$Label"
$base = "$e/inputs-base"
$sha = (Get-FileHash -Algorithm SHA256 $Nro).Hash.ToLower()
$rowsDir = $Rows
if (-not (Test-Path "$rowsDir/rows.json")) { throw "no rows.json in $Rows" }
Remove-Item -Recurse -Force $s -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $s | Out-Null
Copy-Item -Recurse $base "$s/inputs"
Copy-Item -Recurse $rowsDir "$s/rows-hang"
Get-ChildItem "$s/rows-hang" -Recurse -Directory -Filter __pycache__ | Remove-Item -Recurse -Force
Copy-Item $Nro "$s/inputs/plugins/libtraining_rooms.nro" -Force
$m = [IO.File]::ReadAllText("$s/inputs/manifest.json") | ConvertFrom-Json -AsHashtable
$fixSha = ''
if ($cfg.fixture) {
    # btt-5: the seeded layout of btt-E42 (Big Battlefield, 3 targets) is a file in layouts/ the boot did not write.
    New-Item -ItemType Directory -Force "$s/inputs/files/TrainingRooms/layouts" | Out-Null
    Copy-Item "$e/rows/fixtures/btt/seeded-big-battlefield.json" "$s/inputs/files/TrainingRooms/layouts/seeded-big-battlefield.json"
    $fixSha = (Get-FileHash -Algorithm SHA256 "$s/inputs/files/TrainingRooms/layouts/seeded-big-battlefield.json").Hash.ToLower()
    $m['files'] = @($m['files'] | Where-Object { $_['rel'] -ne 'TrainingRooms/layouts/seeded-big-battlefield.json' }) + @(@{ rel = 'TrainingRooms/layouts/seeded-big-battlefield.json'; sha256 = $fixSha })
}
$m['variant'] = "spots2-$Commit"
foreach ($p in @($m['plugins'] | Where-Object { $_['name'] -eq 'libtraining_rooms.nro' })) { $p['sha256'] = $sha; $p['from'] = $Nro }
[IO.File]::WriteAllText("$s/inputs/manifest.json", ($m | ConvertTo-Json -Depth 8))
if ((Get-FileHash -Algorithm SHA256 "$s/inputs/plugins/libtraining_rooms.nro").Hash.ToLower() -ne $sha) { throw 'the staged TrainingRooms NRO is not the given one' }
& /usr/bin/python3 -B -I /home/intpa/ssbu-mod-playbook/tools/eden-runner/runner.py dry-run "$s/rows-hang/rows.json" --out "$t/dry-out-$Label" 2>&1 | Select-String -Pattern '"result"|"errors"' | ForEach-Object { $_.Line.Trim() }
$auth = "rc.14 spots-2 (Claude spots-2, the primary's order of 2026-11-14): the charge multiplier of the sweet/sour attribution, the diag build $($sha.Substring(0, 8)) = rc14/spots-2 $Commit, one instance on the $($cfg.host), Training only, muted (volume 0), the GDB stub on before game.load, Eden 2f315fc4; the Up is staggered after the other XPS instance's load. Nothing pushed, no card write."
$u = [IO.File]::ReadAllText("$e/up-final.sh").Replace('__L__', $Label).Replace('__PORT__', $cfg.port).Replace('__DISP__', $cfg.display).Replace('__AUTH__', $auth.Replace("'", '')).Replace('__HOSTKEY__', $hosts.key).Replace('__HOSTNAME__', $hosts.name).Replace('__LEDGER__', $hosts.ledger).Replace('Claude integration-3 __PLAN__', 'Claude spots-2')
[IO.File]::WriteAllText("$t/up-$Label.sh", $u)
& tar -cf "$t/$Label.tar" -C $s .
"staged label=$Label host=$($cfg.host) nro=$($sha.Substring(0, 12)) fixture=$(if ($fixSha) { $fixSha.Substring(0, 12) } else { 'none' }) tar=$((Get-Item "$t/$Label.tar").Length) bytes"
if (-not $Launch) { return }
$Y = $hosts.ssh
& scp -q -o BatchMode=yes "$t/$Label.tar" "${Y}:eden-runs/$Label.tar"
& scp -q -o BatchMode=yes $e/tr2526-base.json "${Y}:eden-runs/$Label-base.json"
& scp -q -o BatchMode=yes "$t/up-$Label.sh" "${Y}:/tmp/int3-up-$Label.sh"
& ssh -o BatchMode=yes $Y "bash -c 'bash /tmp/int3-up-$Label.sh 2>&1 | tail -12'"
