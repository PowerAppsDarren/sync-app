# Setup script for PocketBase on Windows
# This script downloads and sets up PocketBase for development

param(
    [string]$Version = "0.22.0"
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Platform = "windows_amd64"
$Filename = "pocketbase_${Version}_${Platform}.zip"
$Url = "https://github.com/pocketbase/pocketbase/releases/download/v${Version}/${Filename}"
$ArchivePath = Join-Path $ScriptDir $Filename

Write-Host "Setting up PocketBase $Version for $Platform..." -ForegroundColor Green

# Download PocketBase if not already present
if (-not (Test-Path $ArchivePath)) {
    Write-Host "Downloading $Filename..." -ForegroundColor Yellow
    try {
        Invoke-WebRequest -Uri $Url -OutFile $ArchivePath -UseBasicParsing
        Write-Host "Download completed." -ForegroundColor Green
    }
    catch {
        Write-Error "Failed to download PocketBase: $_"
        exit 1
    }
}
else {
    Write-Host "PocketBase archive already exists, skipping download." -ForegroundColor Yellow
}

# Extract PocketBase
Write-Host "Extracting PocketBase..." -ForegroundColor Yellow
try {
    Expand-Archive -Path $ArchivePath -DestinationPath $ScriptDir -Force
    Write-Host "Extraction completed." -ForegroundColor Green
}
catch {
    Write-Error "Failed to extract PocketBase: $_"
    exit 1
}

# Create pb_data directory for PocketBase data
$DataDir = Join-Path $ScriptDir "pb_data"
if (-not (Test-Path $DataDir)) {
    New-Item -ItemType Directory -Path $DataDir -Force | Out-Null
    Write-Host "Created pb_data directory." -ForegroundColor Green
}

Write-Host "PocketBase setup complete!" -ForegroundColor Green
Write-Host "You can start PocketBase with:" -ForegroundColor Cyan
Write-Host "  cd pocketbase && ./pocketbase.exe serve" -ForegroundColor White
Write-Host ""
Write-Host "PocketBase admin UI will be available at: http://localhost:8090/_/" -ForegroundColor Cyan
