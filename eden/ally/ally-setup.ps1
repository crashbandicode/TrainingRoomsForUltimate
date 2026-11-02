#requires -Version 7.0
# TrainingRooms Ally Eden rig: build an OWNED root with its own profile copy from the portable Eden profile of another
# owned rig (read-only source: nothing in it is written). Runs ON the Ally with MSI PowerShell 7:
#   pwsh.exe -NoLogo -NoProfile -NonInteractive -File ally-setup.ps1 [-Root C:\Eden\trainingrooms-pkg5]
# Result: <Root>\runner (copy of the Windows port's helper modules), <Root>\profile (portable Eden: the exe and Qt files
# are copies; user\nand\user\Contents and user\nand\system are NTFS hard links, byte-identical and never written; saves,
# keys, config, cache are copies), <Root>\profile\user\sdmc (Skyline's stock exefs and an empty plugins folder),
# <Root>\CLAIM, <Root>\macros. The qt-config points at this root and is muted (volume=0, persisted) and offline.
param([string]$Root = 'C:\Eden\trainingrooms-pkg5', [string]$SourceRoot = 'C:\Eden\c1162-w5-ally',
      [string]$Owner = 'trainingrooms-pkg5-0caa72b8-sonnet55')
$ErrorActionPreference = 'Stop'
$EDEN_SHA = '45af318c93cbc00a2794421ee884ffbcb9dbec75bbdbdeef27af30d61cd2cbad'
$SUBSDK9 = '9959ecb13eab6db81a07cee04a789b1905fd605409fc84efeb7ec43da7e29280'
$NPDM = '09538851176328215eb5fd959fb887511b21b31f3d713f0e14caffc75a95c34a'
if (Get-Process eden -ErrorAction SilentlyContinue) { throw 'eden.exe is running' }
if (Test-Path $Root) { throw "$Root exists: this script only creates a new root" }
$S = "$SourceRoot\profile"; $SU = "$S\user"
$P = "$Root\profile"; $U = "$P\user"
function Sha([string]$p) { (Get-FileHash -Algorithm SHA256 -LiteralPath $p).Hash.ToLower() }
function LinkTree([string]$src, [string]$dst) {
    New-Item -ItemType Directory -Force $dst | Out-Null
    $n = 0
    foreach ($item in Get-ChildItem -LiteralPath $src -Recurse -Force) {
        $rel = $item.FullName.Substring($src.Length + 1); $target = Join-Path $dst $rel
        if ($item.PSIsContainer) { New-Item -ItemType Directory -Force $target | Out-Null }
        else { New-Item -ItemType HardLink -Path $target -Target $item.FullName | Out-Null; $n++ }
    }
    "linked $n files: ${src}"
}
New-Item -ItemType Directory -Force $Root, $P, $U | Out-Null
Copy-Item "$SourceRoot\runner" "$Root\runner" -Recurse -Exclude '__pycache__'
Get-ChildItem $S -File | ForEach-Object { Copy-Item $_.FullName "$P\$($_.Name)" }
foreach ($d in 'docs', 'plugins', 'tools') { Copy-Item "$S\$d" "$P\$d" -Recurse }
if ((Sha "$P\eden.exe") -ne $EDEN_SHA) { throw 'eden.exe hash differs from the pinned build' }
"eden.exe $EDEN_SHA"
foreach ($d in 'amiibo', 'cache', 'config', 'dump', 'icons', 'keys', 'load', 'lossless', 'play_time', 'post_presets', 'post_shaders', 'tas') {
    if (Test-Path "$SU\$d") { Copy-Item "$SU\$d" "$U\$d" -Recurse } else { New-Item -ItemType Directory -Force "$U\$d" | Out-Null }
}
foreach ($d in 'crash_dumps', 'log', 'screenshots') { New-Item -ItemType Directory -Force "$U\$d" | Out-Null }
LinkTree "$SU\nand\user\Contents" "$U\nand\user\Contents"
LinkTree "$SU\nand\system" "$U\nand\system"
Copy-Item "$SU\nand\user\save" "$U\nand\user\save" -Recurse
$ex = "$U\sdmc\atmosphere\contents\01006A800016E000"
New-Item -ItemType Directory -Force "$ex\exefs", "$ex\romfs\skyline\plugins", "$U\sdmc\ultimate" | Out-Null
$src = "$SU\sdmc\atmosphere\contents\01006A800016E000\exefs"
Copy-Item "$src\subsdk9" "$ex\exefs\subsdk9"; Copy-Item "$src\main.npdm" "$ex\exefs\main.npdm"
if ((Sha "$ex\exefs\subsdk9") -ne $SUBSDK9 -or (Sha "$ex\exefs\main.npdm") -ne $NPDM) { throw 'exefs hash' }
# qt-config: this root, muted (volume 0 persisted), offline, running in the background.
$qt = "$U\config\qt-config.ini"
$text = [IO.File]::ReadAllText($qt).Replace($SourceRoot.Replace('\', '/'), $Root.Replace('\', '/'))
$lines = foreach ($line in $text -split "`r?`n") {
    switch -Regex ($line) {
        '^volume=' { 'volume=0'; break }
        '^volume\\default=' { 'volume\default=false'; break }
        '^airplane_mode=' { 'airplane_mode=true'; break }
        '^lan_online=' { 'lan_online=false'; break }
        '^pauseWhenInBackground=' { 'pauseWhenInBackground=false'; break }
        default { $line }
    }
}
[IO.File]::WriteAllText($qt, ($lines -join "`n") + "`n")
$cfg = [IO.File]::ReadAllText($qt)
if ($cfg.Contains($SourceRoot.Replace('\', '/'))) { throw 'the config still points at the source root' }
if ($cfg -notmatch '(?m)^volume=0$') { throw 'config lacks volume=0' }
New-Item -ItemType Directory -Force "$Root\runs", "$Root\macros", "$Root\shots", "$Root\nro" | Out-Null
Set-Content "$Root\CLAIM" $Owner -NoNewline
$files = (Get-ChildItem $U -Recurse -File).Count
"owned root ${Root}: $files files under user; CLAIM $Owner"
