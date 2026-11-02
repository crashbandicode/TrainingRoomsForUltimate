#requires -Version 7.0
# Stage one boot on the owned Ally root (runs ON the Ally, MSI PowerShell 7): the plugin set, the settings, fresh plugin
# output folders. Refuses while eden.exe runs or without this root's CLAIM. Every NRO is checked against sha256.txt in the
# set folder; whatever was in the plugins folder and the earlier outputs are archived under <Root>\runs\<Label>-prev, nothing is deleted.
#   ally-stage.ps1 -Label s5-01 -Set C:\Eden\trainingrooms-pkg5\nro\release [-Settings <settings.json>]
param([Parameter(Mandatory)][string]$Label, [Parameter(Mandatory)][string]$Set, [string]$Settings = '',
      [string]$Root = 'C:\Eden\trainingrooms-pkg5', [string]$Owner = 'trainingrooms-pkg5-0caa72b8-sonnet55')
$ErrorActionPreference = 'Stop'
if ($Label -notmatch '^s5-\d{2}$') { throw 'label' }
if (Get-Process eden -ErrorAction SilentlyContinue) { throw 'eden.exe is running' }
if ((Get-Content "$Root\CLAIM" -Raw).Trim() -ne $Owner) { throw 'not my claim' }
$SD = "$Root\profile\user\sdmc"; $PL = "$SD\atmosphere\contents\01006A800016E000\romfs\skyline\plugins"; $U = "$SD\ultimate"
$prev = "$Root\runs\$Label-prev"; New-Item -ItemType Directory -Force $prev | Out-Null
function Sha([string]$p) { (Get-FileHash -Algorithm SHA256 -LiteralPath $p).Hash.ToLower() }
Get-ChildItem $PL -Filter *.nro -ErrorAction SilentlyContinue | ForEach-Object { Move-Item $_.FullName "$prev\$($_.Name)" -Force }
$want = @{}
foreach ($line in Get-Content "$Set\sha256.txt") { $p = $line -split '\s+', 2; $want[$p[1].Trim()] = $p[0].ToLower() }
foreach ($name in $want.Keys) {
    if ((Sha "$Set\$name") -ne $want[$name]) { throw "payload hash $name" }
    Copy-Item "$Set\$name" "$PL\$name"
    if ((Sha "$PL\$name") -ne $want[$name]) { throw "copy hash $name" }
}
foreach ($d in 'TrainingRooms', 'ssbu-cpu-pilot', 'ssbu-tile-cursor') {
    if (Test-Path "$U\$d") { Move-Item "$U\$d" "$prev\$d" -Force }
}
New-Item -ItemType Directory -Force "$U\TrainingRooms" | Out-Null
if ($Settings) {
    Copy-Item $Settings "$U\TrainingRooms\settings.json"
    "settings.json sha256 " + (Sha "$U\TrainingRooms\settings.json").Substring(0, 16)
}
Get-ChildItem $PL -Filter *.nro | Sort-Object Name | ForEach-Object { '{0}  {1}' -f (Sha $_.FullName), $_.Name }
