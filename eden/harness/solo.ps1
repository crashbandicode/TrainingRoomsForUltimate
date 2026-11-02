[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('Up', 'Down')][string]$Action,
    [Parameter(Mandatory)][string]$Label,
    [string[]]$Nro = @(),
    # Rows: files and folders prep puts into the profile copy (see gv.py: --put, --seed, --sd-file, --tree).
    [string[]]$Put = @(),
    [string]$Settings,
    [string]$Diag,
    [string]$DevIni,
    [string]$Seed,
    [string[]]$SdFile = @(),
    [string[]]$Tree = @(),
    [switch]$Pilot,
    [switch]$Tile,
    [switch]$KeepData,
    # A relaunch inside a claim this owner already holds (the file set changed): Down -KeepClaim, Up -NoClaim.
    [switch]$NoClaim,
    [switch]$KeepClaim,
    # Down: put the profile copy back to the source's plugin set, and scan console.log.
    [switch]$Restore,
    [string]$ClaimNote = 'TrainingRooms Eden rows'
)
# One TrainingRooms Eden instance on the host it runs on (Yoga or Surface), every stream's parameters.
# Playbook origin: tools/eden-solo/solo.ps1 (ae87443): same steps and order (claim check, Eden/display/memory
# checks, CLAIMED, gv.py prep, display, GPU lock, gv.py launch, game.load; Down stops, releases). Values come
# from params.py through `gv.py params` (TR_OWNER, TR_ROOT, TR_DISPLAY, ... see params.py); the claim check and
# the CLAIMED/RELEASED lines are ledger.py's. Added: mute before game.load, the stream's rows, Down after a
# failed Up, -NoClaim/-KeepClaim for a relaunch inside a claim, -Restore, host-aware (the Surface has no GPU lock
# and reaches the ledger over ssh).
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$gv = Join-Path $here 'gv.py'
$ledger = Join-Path $here 'ledger.py'
function Py([string[]]$arguments) {
    $out = /usr/bin/python3 -I @arguments 2>&1
    if ($LASTEXITCODE -ne 0) { throw "python failed: $($arguments -join ' ')`n$out" }
    "$out"
}
$p = (Py @($gv, 'params')) | ConvertFrom-Json
$null = Py @($gv, 'label', $Label)
$work = $p.root
$display = $p.display
$helper = '/home/intpa/eden-nso-autolan/live-tools/display-rootful.ps1'
New-Item -ItemType Directory -Force $work | Out-Null
$log = Join-Path $work "solo-$Label.log"
function Note([string]$text) {
    $line = "$((Get-Date).ToUniversalTime().ToString('HH:mm:ss'))Z $text"
    Add-Content -LiteralPath $log -Value $line
    $line
}
function Stop-Display {
    /usr/bin/pwsh -NoLogo -NoProfile -NonInteractive -File $helper -Action Stop -Unit "$($p.unit)-$Label-display" -Socket "wayland-$($p.unit)-$Label" -XDisplay $display -MaxRuntimeSeconds 3600 | Out-Null
}

if ($Action -eq 'Up') {
    Note ('preflight: ' + (Py @($gv, 'preflight')))
    if (-not $NoClaim) {
        $hashes = ($Nro | ForEach-Object { '{0}={1}' -f (Split-Path -Leaf ($_ -split ':')[0]), (Get-FileHash -Algorithm SHA256 ($_ -split ':')[0]).Hash.Substring(0, 12).ToLower() }) -join ', '
        # ledger.py refuses while any claim on this host is open (foreign owners included), within fairness
        # seconds of our own release, and while imgui-nvn-sol is queued and has not had the host since.
        Note ('claim: ' + (Py @($ledger, 'claim', "$ClaimNote, solo run $Label on $($p.title), display $display, own profile $($p.profile) (copy of the host's source profile), muted, plugins $hashes. No LAN. No physical Switch writes.")))
    }
    try {
        $prep = @($gv, 'prep', $Label)
        foreach ($n in $Nro) { $prep += @('--nro', $n) }
        foreach ($x in $Put) { $prep += @('--put', $x) }
        foreach ($x in $SdFile) { $prep += @('--sd-file', $x) }
        foreach ($x in $Tree) { $prep += @('--tree', $x) }
        if ($Settings) { $prep += @('--settings', $Settings) }
        if ($Diag) { $prep += @('--diag-txt', $Diag) }
        if ($DevIni) { $prep += @('--developer-ini', $DevIni) }
        if ($Seed) { $prep += @('--seed', $Seed) }
        if ($Pilot) { $prep += '--pilot' }
        if ($Tile) { $prep += '--tile' }
        if ($KeepData) { $prep += '--keep-data' }
        Note ('prep: ' + (Py $prep))
        /usr/bin/pwsh -NoLogo -NoProfile -NonInteractive -File $helper -Action Start -Unit "$($p.unit)-$Label-display" -Socket "wayland-$($p.unit)-$Label" -XDisplay $display -MaxRuntimeSeconds 3600 | Out-Null
        Note 'display started'
        if ($p.gpu_lock) {
            /usr/bin/systemd-run --user --quiet "--unit=$($p.unit)-$Label-gpu-lock" --property=RuntimeMaxSec=3600 --property=TimeoutStopSec=5 /usr/bin/flock --nonblock /tmp/opencode/yoga-gpu-reservation.lock /usr/bin/sleep 3600
            Start-Sleep -Seconds 1
            if ("$(/usr/bin/systemctl --user is-active "$($p.unit)-$Label-gpu-lock")" -ne 'active') { throw 'Yoga GPU lock is held by someone else' }
        }
        Note ('launch: ' + (Py @($gv, 'launch', $Label)))
        $deadline = (Get-Date).AddSeconds(90)
        while (-not (Test-Path "$($p.runs)/$Label/control.sock") -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 2 }
        # Operator rule 2026-10-09: muted through the CLI and persisted before the game boots; the read-back must be 0.
        Note ('mute: ' + (Py @($gv, 'mute', $Label)))
        Note ('game.load: ' + (Py @($gv, 'call', $Label, 'game.load', (@{ path = $p.game } | ConvertTo-Json -Compress))))
        Note 'up'
    }
    catch {
        Note "UP FAILED: $_"
        & $PSCommandPath -Action Down -Label $Label -KeepClaim:$NoClaim
        throw
    }
}
else {
    try { Note ('stop: ' + (Py @($gv, 'stop', $Label))) } catch { Note "stop failed: $_" }
    try { Stop-Display } catch { Note "display stop failed: $_" }
    if ($p.gpu_lock) { /usr/bin/systemctl --user stop "$($p.unit)-$Label-gpu-lock" 2>$null }
    if ($Restore) {
        try { Note ('scan: ' + (Py @($gv, 'scan', $Label))) } catch { Note "scan: $_" }
        try { Note ('restore: ' + (Py @($gv, 'restore'))) } catch { Note "restore failed: $_" }
    }
    $left = @(/usr/bin/pgrep -af 'Eden.AppImage|/eden( |$)|eden-run.py' | Where-Object { $_ -notmatch 'pgrep' }).Count
    if (-not $KeepClaim) {
        # Always released, also after a failed run.
        Note ('release: ' + (Py @($ledger, 'release', "$ClaimNote run $Label stopped on $($p.title) (display $display stopped; Eden processes left: $left); profile retained. No physical Switch writes.")))
    }
    Note "down; kept claim: $KeepClaim"
}
