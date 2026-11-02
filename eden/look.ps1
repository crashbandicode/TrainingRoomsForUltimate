param([Parameter(Mandatory)][string]$Label, [Parameter(Mandatory)][string]$Name)
# Take a screenshot of the Eden render window on the Yoga and fetch it to eden/runs/<label>/shots/.
$ErrorActionPreference = 'Stop'
$yoga = 'memento-claw-yoga-ubuntu'
$work = '/home/intpa/trainingrooms-eden-p1'
$remote = (ssh -o BatchMode=yes $yoga "cd $work; python3 -I tr_eden.py shot $Label $Name" | Select-Object -Last 1).Trim()
$dest = Join-Path (Join-Path $PSScriptRoot "runs/$Label/shots") ''
New-Item -ItemType Directory -Force $dest | Out-Null
scp -q "${yoga}:$remote" $dest
Join-Path $dest (Split-Path -Leaf $remote)
