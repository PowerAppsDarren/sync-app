$ErrorActionPreference = 'Stop';
$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$packageName = $env:ChocolateyPackageName
$packageVersion = $env:ChocolateyPackageVersion

# Package parameters
$packageArgs = @{
  packageName   = $packageName
  unzipLocation = $toolsDir
  url           = "https://github.com/yourusername/sync-app/releases/download/v$packageVersion/sync-app-$packageVersion-x86_64-pc-windows-gnu.zip"
  checksum      = 'SHA256_PLACEHOLDER'
  checksumType  = 'sha256'
}

# Download and extract the package
Install-ChocolateyZipPackage @packageArgs

# Add to PATH
$binPath = Join-Path $toolsDir "x86_64-pc-windows-gnu"
Install-ChocolateyPath $binPath 'Machine'

# Create shims for executables
$exes = @('sync.exe', 'sync-server.exe', 'sync-daemon.exe', 'pocketbase.exe')
foreach ($exe in $exes) {
    $exePath = Join-Path $binPath $exe
    if (Test-Path $exePath) {
        Install-BinFile -Name ([System.IO.Path]::GetFileNameWithoutExtension($exe)) -Path $exePath
    }
}

# Create application data directory
$appDataPath = Join-Path $env:APPDATA "sync-app"
if (!(Test-Path $appDataPath)) {
    New-Item -ItemType Directory -Path $appDataPath -Force | Out-Null
}

# Create default configuration if it doesn't exist
$configPath = Join-Path $appDataPath "config.yaml"
if (!(Test-Path $configPath)) {
    $defaultConfig = @"
server:
  host: "127.0.0.1"
  port: 8080
  
database:
  path: "$($appDataPath -replace '\\', '/')/sync.db"
  
logging:
  level: "info"
  file: "$($appDataPath -replace '\\', '/')/sync.log"
  
sync:
  interval: "30s"
  auto_start: false
"@
    Set-Content -Path $configPath -Value $defaultConfig -Encoding UTF8
}

Write-Host ""
Write-Host "Sync App has been installed successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Available commands:" -ForegroundColor Yellow
Write-Host "  sync          - Command-line interface"
Write-Host "  sync-server   - Server component" 
Write-Host "  sync-daemon   - Background daemon"
Write-Host "  pocketbase    - PocketBase database (optional)"
Write-Host ""
Write-Host "Configuration file: $configPath" -ForegroundColor Cyan
Write-Host ""
Write-Host "To install as a Windows service:" -ForegroundColor Yellow
Write-Host "  sync-daemon install"
Write-Host "  sync-daemon start"
Write-Host ""
Write-Host "Get started with: sync --help" -ForegroundColor Green
