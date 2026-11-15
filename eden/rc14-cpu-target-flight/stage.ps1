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
# worker A's approved verdict fix (11-14 12:53Z): gdb_rc11.py ca948a02 is canonical.
$g = (Get-FileHash -Algorithm SHA256 "$s/rows-hang/gdb_rc11.py").Hash.ToLower()
if (-not $g.StartsWith('ca948a02')) { throw "gdb_rc11.py is $g, not the canonical ca948a02" }
& /usr/bin/python3 -I "$here/mk-rows.py" '/home/intpa/smash-training-lobby/artifacts/phase23/option-gifs-20261113/takes/p2/p2-staling-sbs/rows/rows.json' "$s/rows-hang/rows.json" $sha.Substring(0, 8) $Commit
if ($LASTEXITCODE) { throw "mk-rows failed" }
Copy-Item $Nro "$s/inputs/plugins/libtraining_rooms.nro" -Force
$m = [IO.File]::ReadAllText("$s/inputs/manifest.json")
$m = $m.Replace('cf0fa7eefc802d1b2cabd874938a31545d68a7dae08d492bd22f12b5cb4e0dcb', $sha).Replace('/home/intpa/tr-fixes-20261113/wa-fixes-logs/hbprobe/libtraining_rooms-ae602057-diag.nro', $Nro).Replace('card+hbprobe-ae602057', "rc14-cpu-target-flight-$Commit")
[IO.File]::WriteAllText("$s/inputs/manifest.json", $m)

# The Yoga up script: worker A's yoga-up-dg.sh with this label, the load-phase guard and this authorization.
$u = [IO.File]::ReadAllText("$w/yoga-up-dg.sh")
$u = $u.Replace('L=wa-olm-49955d1d-2', "L=$L")
$guard = @'
ps -eo pid,etimes,args | grep '/usr/bin/eden --control-socket' | grep -v grep | awk '{print $1, $2}' > /tmp/cf-edens.txt || true
cat /tmp/cf-edens.txt
if awk '$2 < 600 {f=1} END {exit !f}' /tmp/cf-edens.txt; then echo "LOAD PHASE: another Eden is under 10 min old"; exit 7; fi
'@
$u = $u.Replace("if ss -ltn | grep -q "":`$PORT ""; then", $guard.Replace("`r`n", "`n") + "`nif ss -ltn | grep -q "":`$PORT ""; then")
$i = $u.IndexOf("c['authorization'] = (")
$j = $u.IndexOf("p.write_text(json.dumps(c, indent=1) + '\n')", $i)
$u = $u.Substring(0, $i) + "c['authorization'] = ('" + $Auth.Replace("'", '') + "')`n" + $u.Substring($j)
$u = $u.Replace("--owner 'Claude worker-A-3'", "--owner 'Claude cpu-target-flight'").Replace("--owner 'Claude worker-A-2'", "--owner 'Claude cpu-target-flight'")
# Own port, display and base folder; "mine running" is this lane's labels.
$u = $u.Replace('PORT=6577', 'PORT=6583').Replace('DISPNUM=77', 'DISPNUM=83').Replace("inst['display'] = ':77'", "inst['display'] = ':83'")
$u = $u.Replace("grep -q 'eden-runs/wa-'", "grep -q 'eden-runs/cf-'").Replace('eden-runs/wa-base', 'eden-runs/cf-base')
if ($u -cmatch "wa-base|PORT=6577|':77'|Claude worker-A") { throw "the up script still carries worker A's run values" }
$u = $u.Replace('# Worker A (tr-worker-a-2), hitbox probe boot on the Yoga', '# rc.14 cpu-target-flight (Opus), one boot on the Yoga')
[IO.File]::WriteAllText("$out/yoga-up-$L.sh", $u)
Select-String -Path "$out/yoga-up-$L.sh" -Pattern '^L=|LOAD PHASE|authorization|--owner|^PORT|^DISPNUM|wa-' | ForEach-Object { 'CHECK ' + $_.Line.Substring(0, [Math]::Min(170, $_.Line.Length)) }
& tar -cf "$out/$L.tar" -C $s .
"staged $L sha=$sha tar=$out/$L.tar"
