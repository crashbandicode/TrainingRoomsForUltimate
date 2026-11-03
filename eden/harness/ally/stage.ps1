#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('setaside', 'boot', 'restore')][string]$Mode,
    # The rig and this stream's folder on it (TR_ALLY_ROOT; the lent rig is the only Ally rig TrainingRooms may run).
    [string]$Rig = 'C:\Eden\c1162-w5-ally',
    [string]$Work = 'tr',
    # The text the rig's CLAIM holds while this stream has it (TR_ALLY_CLAIM) and the claim it had before.
    [Parameter(Mandatory)][string]$Claim,
    [string]$Lent = 'c1162-w5-ally-f50948e9-captures-ally',
    [string]$Output = 'TrainingRooms',
    [string]$Set = '', [string]$Prev = '',
    [switch]$DeveloperPage,
    [string[]]$Keep = @('arcropolis', 'mods')
)
# Stage, boot and restore the Ally rig for a TrainingRooms stream: the Windows variant of gv.py prep / restore
# (the profile copy's set-aside folder, matched by sha256 against a manifest taken first).
# Origin: eden/lent/stage_ui.ps1 (ui-screens), itself derived from the rig's own m5qa\stage_a.ps1 (tm-m5q-eden-a): same
# set-aside / restore / verify discipline, with the rig, claim, folder, data folder and kept trees as parameters.
# Runs ON the Ally as <rig>\<Work>\stage.ps1.
#   setaside            claim (CLAIM -> this stream, the original kept) + pre manifest + move the profile's plugins and ultimate data
#                       aside, back up qt-config.ini; exefs is not touched (it must be the stock Skyline subsdk9, checked)
#   boot -Set <name> [-Prev <phase>]
#                       plugin set <Work>\payload\<name>\*.nro (every file sha-checked against its sha256.txt) and a fresh ultimate
#                       tree (-DeveloperPage: <Output>\developer.ini with developer_page=on). A previous boot's ultimate tree
#                       is kept as <Work>\<Prev>-ultimate.
#   restore             everything back from the set-aside, verified against the pre manifest; the original CLAIM put back
$ErrorActionPreference = 'Stop'
$O = $Rig; $W = "$O\$Work"; $MINE = "$W\payload"; $SA = "$W\set-aside"
$SD = "$O\profile\user\sdmc"; $C = "$SD\atmosphere\contents\01006A800016E000"
$PL = "$C\romfs\skyline\plugins"; $EX = "$C\exefs"; $U = "$SD\ultimate"; $QT = "$O\profile\user\config\qt-config.ini"
$SKY_STOCK = '9959ecb13eab6db81a07cee04a789b1905fd605409fc84efeb7ec43da7e29280'
$NPDM = '09538851176328215eb5fd959fb887511b21b31f3d713f0e14caffc75a95c34a'
function Sha256Of($p) { (Get-FileHash -Algorithm SHA256 -LiteralPath $p).Hash.ToLower() }
function Tree($root) {
    $m = [ordered]@{}
    if (Test-Path -LiteralPath $root) {
        Get-ChildItem -LiteralPath $root -Recurse -File -Force | Sort-Object FullName | ForEach-Object { $m[$_.FullName.Substring($root.Length + 1)] = Sha256Of $_.FullName }
    }
    $m
}
function Others { Get-ChildItem -LiteralPath $U -Directory -Force | Where-Object { $Keep -notcontains $_.Name } }
if (Get-Process eden -ErrorAction SilentlyContinue) { throw 'eden.exe is running' }

switch ($Mode) {
    'setaside' {
        if (Test-Path $SA) { throw 'set-aside exists already' }
        $previousClaim = (Get-Content "$O\CLAIM" -Raw).Trim()
        if ($previousClaim -ne $Lent) { throw "unexpected claim: $previousClaim" }
        if ((Sha256Of "$EX\subsdk9") -ne $SKY_STOCK -or (Sha256Of "$EX\main.npdm") -ne $NPDM) { throw 'exefs is not the stock Skyline pair' }
        New-Item -ItemType Directory -Force $W | Out-Null
        Set-Content "$W\CLAIM.orig" $previousClaim -NoNewline; Set-Content "$O\CLAIM" $Claim -NoNewline
        $pre = [ordered]@{ plugins = Tree $PL; exefs = Tree $EX; ultimate = [ordered]@{}; keep = $null; qt = Sha256Of $QT }
        foreach ($d in (Others)) { $pre.ultimate[$d.Name] = Tree $d.FullName }
        $pre.keep = [ordered]@{}; foreach ($k in $Keep) { $pre.keep[$k] = Tree "$U\$k" }
        $pre | ConvertTo-Json -Depth 6 | Set-Content "$W\pre-manifest.json"
        New-Item -ItemType Directory -Force "$SA\plugins", "$SA\ultimate", "$SA\config" | Out-Null
        Get-ChildItem -LiteralPath $PL -File | ForEach-Object { Move-Item -LiteralPath $_.FullName -Destination "$SA\plugins\$($_.Name)" }
        Copy-Item $QT "$SA\config\qt-config.ini"
        foreach ($d in (Others)) { Move-Item -LiteralPath $d.FullName -Destination "$SA\ultimate\$($d.Name)" }
        "set aside: plugins $($pre.plugins.Count), exefs $($pre.exefs.Count) (stock, untouched), ultimate dirs $($pre.ultimate.Keys -join ','); claim $Claim"
    }
    'boot' {
        if ((Get-Content "$O\CLAIM" -Raw).Trim() -ne $Claim) { throw 'not my claim' }
        if ($Prev) {
            foreach ($d in (Others)) {
                New-Item -ItemType Directory -Force "$W\$Prev-ultimate" | Out-Null
                Move-Item -LiteralPath $d.FullName -Destination "$W\$Prev-ultimate\$($d.Name)"
            }
        }
        foreach ($d in (Others)) { throw "unexpected dir in ultimate: $($d.Name)" }
        $tr = New-Item -ItemType Directory -Force "$U\$Output"
        if ($DeveloperPage) { [IO.File]::WriteAllText("$tr\developer.ini", "developer_page=on`n") }
        $src = "$MINE\$Set"; $want = @{}
        foreach ($line in Get-Content "$src\sha256.txt") { if ($line.Trim()) { $p = $line -split '\s+', 2; $want[$p[1].Trim()] = $p[0].ToLower() } }
        Get-ChildItem -LiteralPath $PL -File | ForEach-Object { Move-Item -LiteralPath $_.FullName -Destination "$W\used-$Set-prev-$($_.Name)" -Force }
        foreach ($name in $want.Keys) {
            if ((Sha256Of "$src\$name") -ne $want[$name]) { throw "payload hash $name" }
            Copy-Item "$src\$name" "$PL\$name"
            if ((Sha256Of "$PL\$name") -ne $want[$name]) { throw "copy hash $name" }
        }
        if ((Sha256Of "$EX\subsdk9") -ne $SKY_STOCK) { throw 'exefs subsdk9 changed' }
        $inst = [ordered]@{}; Get-ChildItem -LiteralPath $PL -File | Sort-Object Name | ForEach-Object { $inst[$_.Name] = Sha256Of $_.FullName }
        $inst | ConvertTo-Json | Set-Content "$W\installed-$Set.json"
        "installed $Set`: $($inst.Count) NROs"; $inst.GetEnumerator() | ForEach-Object { "  $($_.Value.Substring(0,16)) $($_.Key)" }
    }
    'restore' {
        if ((Get-Content "$O\CLAIM" -Raw).Trim() -ne $Claim) { throw 'not my claim' }
        $pre = Get-Content "$W\pre-manifest.json" -Raw | ConvertFrom-Json -AsHashtable
        $stamp = (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss'); $LU = "$W\last-ultimate-$stamp"; $UPF = "$W\used-plugins-final-$stamp"; New-Item -ItemType Directory -Force $LU, $UPF | Out-Null
        foreach ($d in (Others)) { Move-Item -LiteralPath $d.FullName -Destination "$LU\$($d.Name)" }
        Get-ChildItem -LiteralPath $PL -File | ForEach-Object { Move-Item -LiteralPath $_.FullName -Destination "$UPF\$($_.Name)" }
        Get-ChildItem -LiteralPath "$SA\plugins" -File | ForEach-Object { Move-Item -LiteralPath $_.FullName -Destination "$PL\$($_.Name)" }
        foreach ($d in (Get-ChildItem -LiteralPath "$SA\ultimate" -Directory -Force)) { Move-Item -LiteralPath $d.FullName -Destination "$U\$($d.Name)" }
        Copy-Item "$SA\config\qt-config.ini" $QT -Force
        $bad = @()
        $now = Tree $PL; foreach ($k in $pre.plugins.Keys) { if ($now[$k] -ne $pre.plugins[$k]) { $bad += "plugins\$k" } }; if ($now.Count -ne $pre.plugins.Count) { $bad += 'plugins count' }
        $now = Tree $EX; foreach ($k in $pre.exefs.Keys) { if ($now[$k] -ne $pre.exefs[$k]) { $bad += "exefs\$k" } }
        $n = 0; foreach ($d in $pre.ultimate.Keys) { $now = Tree "$U\$d"; foreach ($k in $pre.ultimate[$d].Keys) { $n++; if ($now[$k] -ne $pre.ultimate[$d][$k]) { $bad += "ultimate\$d\$k" } }; if ($now.Count -ne $pre.ultimate[$d].Count) { $bad += "ultimate\$d count" } }
        if ((Sha256Of $QT) -ne $pre.qt) { $bad += 'qt-config.ini' }
        $dirs = (Get-ChildItem -LiteralPath $U -Directory -Force).Name | Sort-Object
        $exp = @($pre.ultimate.Keys) + $Keep | Sort-Object
        if (($dirs -join ',') -ne ($exp -join ',')) { $bad += "ultimate dirs: $($dirs -join ',')" }
        $keepdiff = @(); foreach ($k in $Keep) { $now = Tree "$U\$k"; foreach ($f in $now.Keys) { if ($pre.keep[$k][$f] -ne $now[$f]) { $keepdiff += "$k\$f" } } }
        Set-Content "$O\CLAIM" (Get-Content "$W\CLAIM.orig" -Raw) -NoNewline
        if (-not $bad) { Rename-Item $SA "set-aside-done-$stamp" }
        $res = [ordered]@{ at = (Get-Date).ToUniversalTime().ToString('o'); plugins = $pre.plugins.Count; exefs = $pre.exefs.Count; ultimate_files = $n; qt = 'checked'; mismatches = $bad; changed_in_kept_trees = $keepdiff; claim = (Get-Content "$O\CLAIM" -Raw) }
        $res | ConvertTo-Json -Depth 4 | Set-Content "$W\restore-verify-$stamp.json"; $res | ConvertTo-Json -Depth 4
    }
}
