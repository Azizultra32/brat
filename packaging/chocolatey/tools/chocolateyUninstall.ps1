$ErrorActionPreference = 'Stop'

$packageName = 'brat'
$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

$binPath = Join-Path $toolsDir 'brat.exe'
if (Test-Path $binPath) {
    Remove-Item $binPath -Force
    Write-Host "Removed: $binPath"
}
