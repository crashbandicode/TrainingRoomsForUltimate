# rc.14 cpu-target-flight: stage one Yoga boot of the cpuflight.py rows on worker A's Eggs-ercise harness (stage-eggs.ps1 and
# mk-yoga-up-eggs.ps1, adapted: own stage folder, own label, the lane's hooks). Run on the Surface:
#   /usr/bin/pwsh -NoLogo -NoProfile -NonInteractive -File stage.ps1 -Nro <diag nro> -L <label> -Commit <sha8> -Auth '<text>'
param([Parameter(Mandatory)][string]$Nro, [Parameter(Mandatory)][string]$L, [Parameter(Mandatory)][string]$Commit,
      [Parameter(Mandatory)][string]$Auth)
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$out = '/home/intpa/smash-training-lobby/artifacts/rc14-cpu-target-flight/eden'
$s = "$out/stage-$L"
$w = '/tmp/opencode/wa-look'
$sha = (Get-FileHash -Algorithm SHA256 $Nro).Hash.ToLower()
if (Test-Path $s) { throw "stage $s exists" }
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item -Recurse /tmp/opencode/wa-hbp-stage $s
Remove-Item "$s/rows-hang/hbprobe.py"
Remove-Item -Recurse -Force "$s/rows-hang/__pycache__" -ErrorAction SilentlyContinue
Copy-Item "$w/eggsrows.py", "$w/eggsview.py", "$w/eggsfly.py", "$w/eggsrange.py" "$s/rows-hang/"
Copy-Item "$here/cpuflight.py", "$here/cfcheck.py" "$s/rows-hang/"
# New stages copy the canonical gdb_rc11.py: 30109d07 since the windows-guest-cpu final rollout (11-14; it replaced ca948a02).
Copy-Item '/home/intpa/smash-training-lobby/artifacts/phase23/option-gifs-20261113/takes/p2/p2-staling-sbs/rows/gdb_rc11.py' "$s/rows-hang/gdb_rc11.py" -Force
$g = (Get-FileHash -Algorithm SHA256 "$s/rows-hang/gdb_rc11.py").Hash.ToLower()
if (-not $g.StartsWith('30109d07')) { throw "gdb_rc11.py is $g, not the canonical 30109d07" }
& /usr/bin/python3 -I "$here/mk-rows.py" '/home/intpa/smash-training-lobby/artifacts/phase23/option-gifs-20261113/takes/p2/p2-staling-sbs/rows/rows.json' "$s/rows-hang/rows.json" $sha.Substring(0, 8) $Commit
if ($LASTEXITCODE) { throw "mk-rows failed" }
Copy-Item $Nro "$s/inputs/plugins/libtraining_rooms.nro" -Force
$m = [IO.File]::ReadAllText("$s/inputs/manifest.json")
$m = $m.Replace('cf0fa7eefc802d1b2cabd874938a31545d68a7dae08d492bd22f12b5cb4e0dcb', $sha).Replace('/home/intpa/tr-fixes-20261113/wa-fixes-logs/hbprobe/libtraining_rooms-ae602057-diag.nro', $Nro).Replace('card+hbprobe-ae602057', "rc14-cpu-target-flight-$Commit")
[IO.File]::WriteAllText("$s/inputs/manifest.json", $m)

# The up scripts: worker A's yoga-up-dg.sh and xps-up-cardb.sh with this label, the load-phase guard, this authorization,
# the lane's own port, display and base folder, and the primary's rule (02:52Z 11-15): the timed rows run alone on the
# host, so Up only with no other Eden there.
function Up-Script([string]$template, [string]$hostName, [string]$busy, [string]$headLine) {
    $u = [IO.File]::ReadAllText("$w/$template")
    $u = [regex]::Replace($u, '(?m)^L=.*$', "L=$L")
    if (-not $u.Contains('LOAD PHASE')) {
        $guard = "ps -eo pid,etimes,args | grep '/usr/bin/eden --control-socket' | grep -v grep | awk '{print `$1, `$2}' > /tmp/cf-edens.txt || true`ncat /tmp/cf-edens.txt`nif awk '`$2 < 600 {f=1} END {exit !f}' /tmp/cf-edens.txt; then echo `"LOAD PHASE: another Eden is under 10 min old`"; exit 7; fi`n"
        $u = $u.Replace("if ss -ltn | grep -q "":`$PORT ""; then", $guard + "if ss -ltn | grep -q "":`$PORT ""; then")
    }
    $a = $u.IndexOf("c['authorization'] = (")
    $b = $u.IndexOf("p.write_text(json.dumps(c, indent=1) + '\n')", $a)
    $u = $u.Substring(0, $a) + "c['authorization'] = ('" + $Auth.Replace("'", '') + "')`n" + $u.Substring($b)
    $u = [regex]::Replace($u, "--owner 'Claude worker-A-\d'", "--owner 'Claude cpu-target-flight'")
    $u = $u.Replace('PORT=6577', 'PORT=6583').Replace('DISPNUM=77', 'DISPNUM=83').Replace("inst['display'] = ':77'", "inst['display'] = ':83'")
    $u = $u.Replace("grep -q 'eden-runs/wa-'", "grep -q 'eden-runs/cf-'").Replace('eden-runs/wa-base', 'eden-runs/cf-base')
    $u = $u.Replace('/tmp/wa-cardb-edens.txt', '/tmp/cf-edens.txt')
    if (-not $u.Contains($busy)) { throw "$template`: the busy check moved" }
    $u = $u.Replace($busy, 'if [ "$N" -ge 1 ]; then echo "HOST NOT EMPTY (the timed rows run alone)"; exit 3; fi')
    $u = [regex]::Replace($u, '(?m)\A(set -e\n)#.*\n#.*\n', "`$1# rc.14 cpu-target-flight (Opus), one boot on the $headLine`: cpuflight.py on the branch diag $($sha.Substring(0, 8)) ($Commit).`n# Adapted from worker A's $template (rc12-boot4/remote_up.sh).`n")
    if ($u -cmatch "wa-base|PORT=6577|':77'|Claude worker-A|wa-cardb") { throw "$template`: still carries worker A's run values" }
    [IO.File]::WriteAllText("$out/$hostName-up-$L.sh", $u)
    Select-String -Path "$out/$hostName-up-$L.sh" -Pattern '^#|^L=|LOAD PHASE|NOT EMPTY|authorization|--owner|^PORT|^DISPNUM|ledger_ssh|hostname' | ForEach-Object { "CHECK $hostName " + $_.Line.Substring(0, [Math]::Min(150, $_.Line.Length)) }
}
Up-Script 'yoga-up-dg.sh' 'yoga' 'if [ "$N" -ge 2 ]; then echo "YOGA BUSY"; exit 3; fi' 'Yoga'
Up-Script 'xps-up-cardb.sh' 'xps' 'if [ "$N" -ge 2 ]; then echo "XPS BUSY"; exit 3; fi' 'XPS'
& tar -cf "$out/$L.tar" -C $s .
"staged $L sha=$sha tar=$out/$L.tar"
