param([Parameter(Mandatory)][string]$Nro, [Parameter(Mandatory)][string]$L, [Parameter(Mandatory)][string]$Commit)
# save-here lane: stage one XPS boot (slot 1) for Save setup here, from worker A's card stage (/tmp/opencode/wa-hbp-stage:
# the card's plugins, the boot rows g-gdb-on, g-load, g-gdb-attached, g-training, z-boot and their hooks) plus
# setuphere.py and the cov-replay fixture. Writes eden/$L.tar and eden/up-$L.sh.
$ErrorActionPreference = 'Stop'
$here = '/home/intpa/smash-training-lobby/artifacts/setup-here/eden'
$s = "$here/stage-$L"
$sha = (Get-FileHash -Algorithm SHA256 $Nro).Hash.ToLower()
if (Test-Path $s) { Remove-Item -Recurse -Force $s }
Copy-Item -Recurse /tmp/opencode/wa-hbp-stage $s
Remove-Item "$s/rows-hang/hbprobe.py"
Copy-Item "$here/setuphere.py" "$s/rows-hang/"
New-Item -ItemType Directory -Force "$s/rows-hang/fixtures" | Out-Null
Copy-Item -Recurse /home/intpa/smash-training-lobby/artifacts/phase23/rc14-int3/eden/rows/fixtures/cov-replay "$s/rows-hang/fixtures/"
& /usr/bin/python3 -I "$here/mk-rows.py" "$s/rows-hang/rows.json" $sha.Substring(0, 8) $Commit
if ($LASTEXITCODE -ne 0) { throw 'mk-rows failed' }
Copy-Item $Nro "$s/inputs/plugins/libtraining_rooms.nro" -Force
$m = Get-Content "$s/inputs/manifest.json" -Raw | ConvertFrom-Json
$tr = $m.plugins | Where-Object { $_.name -eq 'libtraining_rooms.nro' }
$tr.sha256 = $sha; $tr.from = $Nro
$m.variant = "card+setup-here-$Commit"
[IO.File]::WriteAllText("$s/inputs/manifest.json", ($m | ConvertTo-Json -Depth 10))
$u = [IO.File]::ReadAllText("$here/xps-up-template.sh")
$u = $u.Replace('L=wa-cardb-7a6ba197-1', "L=$L").Replace('PORT=6577', 'PORT=6583').Replace('DISPNUM=77', 'DISPNUM=83')
$u = $u.Replace("inst['display'] = ':77'", "inst['display'] = ':83'").Replace('eden-runs/wa-base', 'eden-runs/sh-base')
$u = $u.Replace("grep -q 'eden-runs/wa-'", "grep -q 'eden-runs/sh-'").Replace('/tmp/wa-cardb-edens.txt', '/tmp/sh-edens.txt')
$u = $u.Replace("--owner 'Claude worker-A-3'", "--owner 'Claude save-here'")
$i = $u.IndexOf("c['authorization'] = (")
$j = $u.IndexOf("p.write_text(json.dumps(c, indent=1) + '\n')", $i)
$auth = "c['authorization'] = ('rc.14 SL4 Save setup here (Claude save-here lane; the primary 2026-11-14: XPS slot 1): one row " +
  "t-setup-here on the branch diag $($sha.Substring(0, 8)) of $Commit (rc14/setup-here on setup-b 3e06a8cf), one instance, " +
  "Training only, muted (volume 0), the GDB stub on before game.load, Eden 2f315fc4. Nothing pushed.')`n"
$u = $u.Substring(0, $i) + $auth + $u.Substring($j)
$u = $u.Replace('# Worker A (tr-worker-a-2), rc.13 final pass card B', '# save-here lane, rc.14 SL4 Save setup here (from worker A''s XPS up script)')
[IO.File]::WriteAllText("$here/up-$L.sh", $u)
Select-String -Path "$here/up-$L.sh" -Pattern '^L=|^PORT|^DISPNUM|LOAD PHASE|authorization|wa-|cardb|7a6ba197' | ForEach-Object { 'CHECK ' + $_.Line.Substring(0, [Math]::Min(150, $_.Line.Length)) }
& tar -cf "$here/$L.tar" -C $s .
"staged $L sha=$sha tar=$here/$L.tar"
