$ErrorActionPreference = 'Stop'

$packageName = 'brat'
$version = '0.1.0'
$url64 = "https://github.com/neul-labs/brat/releases/download/v$version/brat-windows-x86_64.zip"
$checksum64 = 'PLACEHOLDER_SHA256'

$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

$packageArgs = @{
    packageName    = $packageName
    unzipLocation  = $toolsDir
    url64bit       = $url64
    checksum64     = $checksum64
    checksumType64 = 'sha256'
}

Install-ChocolateyZipPackage @packageArgs

# Add to PATH
$binPath = Join-Path $toolsDir 'brat.exe'
Write-Host "Brat installed to: $binPath"
