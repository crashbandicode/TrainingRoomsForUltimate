param(
    [Parameter(Mandatory)][string]$Nro,
    [string]$Label = 'btt-1',
    [ValidateSet('yoga', 'surface', 'xps')][string]$HostName = 'yoga',
    [string]$Display = '',
    [int]$Port = 0,
    [string]$Owner = 'Claude btt-3',
    [switch]$Render,
    [string]$Rows = ''
)
# btt-3: stage and start the Break the Targets Eden boot on a host (DRAFT, never run: the lane does not boot Eden before the
# diag build and the primary's go). With -Render it only writes up_btt.rendered.sh next to this script and runs the offline
# dry run; without it, it copies the inputs to the host and runs up_btt.sh there (which starts a detached chain: up, run, down).
# The host choice is in EDEN-ROWS.md: check the Yoga ledger first (a free host, never another lane's instance).
$d = '/home/intpa/tr-fixes-20261113/d2-btt-eden'
$hosts = @{
    yoga    = @{ ssh = 'memento-claw-yoga-ubuntu'; base = '~/eden-runs/wa-eggsfly-9c16babe-1/config.json'; display = '77'; port = 6590 }
    surface = @{ ssh = 'intpa@dreamland-surface.lan'; base = '~/eden-runs/rc14-egg-target-2/config.json'; display = '181'; port = 6591 }
    xps     = @{ ssh = 'intpa@ubuntu-xps.lan'; base = '~/eden-runs/BASE-CONFIG-ON-THE-XPS/config.json'; display = '183'; port = 6592 }
}
$h = $hosts[$HostName]
if (-not $Display) { $Display = $h.display }
if (-not $Port) { $Port = $h.port }
$nroSha = (Get-FileHash -Algorithm SHA256 $Nro).Hash.ToLower()
$fix = "$d/rows/fixtures/btt/seeded-big-battlefield.json"
$fixSha = (Get-FileHash -Algorithm SHA256 $fix).Hash.ToLower()
# Rows folder tarball (no caches), the offline checks first: segments, fixture, rows.json, the dry run, the hook simulation.
Push-Location "$d/rows"
& /usr/bin/python3 -I make_btt_segments.py
& /usr/bin/python3 -I make_btt_fixture.py
$env:BTT_ROWS = $Rows
& /usr/bin/python3 -I make_btt_rows.py
Pop-Location
& /usr/bin/python3 -B -I /home/intpa/ssbu-mod-playbook/tools/eden-runner/runner.py dry-run "$d/rows/rows.json" --out "$d/out" *> "$d/dry-run.txt"
"dry-run rc=$LASTEXITCODE"
& /usr/bin/python3 -B -I "$d/test_btt_rows.py" | Select-Object -Last 1
$t = (Get-Content -Raw "$d/up_btt.sh").Replace('__LABEL__', $Label).Replace('__NRO_SHA__', $nroSha).Replace('__FIX_SHA__', $fixSha).Replace('__BASE__', $h.base).Replace('__PORT__', "$Port").Replace('__DISP__', $Display).Replace('__OWNER__', $Owner)
[IO.File]::WriteAllText("$d/up_btt.rendered.sh", $t)
"rendered $Label host=$HostName display=:$Display port=$Port nro=$($nroSha.Substring(0, 12)) fixture=$($fixSha.Substring(0, 12))"
if ($Render) { return }
$tgz = "$d/rows.tgz"
& /usr/bin/tar --exclude=__pycache__ -C "$d/rows" -czf $tgz .
& /usr/bin/ssh -o BatchMode=yes $h.ssh 'mkdir -p ~/eden-runs/_btt3-inputs/fixtures'
& /usr/bin/scp -q -o BatchMode=yes $Nro "$($h.ssh):eden-runs/_btt3-inputs/libtraining_rooms.nro"
& /usr/bin/scp -q -o BatchMode=yes $fix "$($h.ssh):eden-runs/_btt3-inputs/fixtures/seeded-big-battlefield.json"
& /usr/bin/scp -q -o BatchMode=yes $tgz "$($h.ssh):eden-runs/_btt3-inputs/rows.tgz"
Get-Content -Raw "$d/up_btt.rendered.sh" | & /usr/bin/ssh -o BatchMode=yes $h.ssh 'bash -s'
"rc=$LASTEXITCODE"
