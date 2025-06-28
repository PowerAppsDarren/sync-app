$ErrorActionPreference = 'Stop';
$packageName = $env:ChocolateyPackageName

Write-Host "Uninstalling Sync App..." -ForegroundColor Yellow

# Stop and uninstall service if it exists
try {
    $service = Get-Service -Name "sync-daemon" -ErrorAction SilentlyContinue
    if ($service) {
        Write-Host "Stopping sync-daemon service..." -ForegroundColor Yellow
        & sync-daemon stop 2>$null
        & sync-daemon uninstall 2>$null
        Write-Host "Service uninstalled successfully." -ForegroundColor Green
    }
} catch {
    Write-Host "Note: Could not uninstall service (may not have been installed)" -ForegroundColor Gray
}

# Remove shims
$exes = @('sync', 'sync-server', 'sync-daemon', 'pocketbase')
foreach ($exe in $exes) {
    try {
        Uninstall-BinFile -Name $exe
    } catch {
        # Ignore errors if shim doesn't exist
    }
}

# Note about configuration preservation
$appDataPath = Join-Path $env:APPDATA "sync-app"
if (Test-Path $appDataPath) {
    Write-Host ""
    Write-Host "Configuration and data files preserved in:" -ForegroundColor Cyan
    Write-Host "  $appDataPath" -ForegroundColor Gray
    Write-Host ""
    Write-Host "To completely remove all data, manually delete the above directory." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Sync App has been uninstalled successfully!" -ForegroundColor Green
