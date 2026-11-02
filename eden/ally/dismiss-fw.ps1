# Copy of the Ally rig's dismiss-fw.ps1 (m5q worker) with this root's log path.
# Runs in the Ally's interactive session (scheduled task). Dismisses ONLY the Windows Security firewall prompt that
# the first launch of the owned eden.exe path raised, by pressing its Cancel button (keeps the default block; no allow
# rule is created). Touches nothing else. Result goes to the log file in the owned root.
param([string]$Log = 'C:\Eden\trainingrooms-pkg2\dismiss-fw.log')
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$ae = [System.Windows.Automation.AutomationElement]
$root = $ae::RootElement
$result = @()
$windows = $root.FindAll('Children', [System.Windows.Automation.Condition]::TrueCondition)
foreach ($w in $windows) {
    $name = $w.Current.Name
    if ($name -ne 'Windows Security') { continue }
    $text = ($w.FindAll('Descendants', [System.Windows.Automation.Condition]::TrueCondition) | ForEach-Object { $_.Current.Name } | Where-Object { $_ }) -join ' | '
    $result += "found window '$name': $text"
    if ($text -notmatch 'eden\.exe') { $result += 'not the eden.exe prompt; left alone'; continue }
    $cancel = $w.FindAll('Descendants', (New-Object System.Windows.Automation.PropertyCondition($ae::NameProperty, 'Cancel'))) | Select-Object -First 1
    if ($cancel) {
        $cancel.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
        $result += 'pressed Cancel'
    } else { $result += 'no Cancel button found' }
}
if (-not $result) { $result = @('no Windows Security window present') }
$result | Set-Content -Path $Log -Encoding utf8

