# Dattavani ASR Windows Uninstaller

function Write-Info { param($msg) Write-Host "[INFO] $msg" -ForegroundColor Green }
function Write-Warn { param($msg) Write-Host "[WARN] $msg" -ForegroundColor Yellow }

Write-Info "Uninstalling Dattavani ASR from Windows..."

# Remove build artifacts
Write-Info "Removing build artifacts..."
if (Test-Path "target") { Remove-Item -Recurse -Force "target" }
if (Test-Path "whisper_simple") { Remove-Item -Recurse -Force "whisper_simple" }
if (Test-Path "dattavani-asr.bat") { Remove-Item -Force "dattavani-asr.bat" }

# Remove desktop shortcut
$desktopPath = [Environment]::GetFolderPath("Desktop")
$shortcutPath = "$desktopPath\Dattavani ASR.lnk"
if (Test-Path $shortcutPath) {
    Remove-Item -Force $shortcutPath
    Write-Info "Removed desktop shortcut"
}

# Ask about removing dependencies
$removeDeps = Read-Host "Remove system dependencies (ffmpeg, whisper)? [y/N]"
if ($removeDeps -match "^[Yy]") {
    # Remove whisper
    python -m pip uninstall -y openai-whisper 2>$null
    
    # Remove chocolatey packages (requires admin)
    $isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")
    if ($isAdmin -and (Get-Command choco -ErrorAction SilentlyContinue)) {
        choco uninstall -y ffmpeg python3
    } else {
        Write-Warn "Administrator privileges required to remove system packages"
        Write-Info "Manually uninstall FFmpeg and Python3 if desired"
    }
}

Write-Info "Uninstall complete!"
Write-Info "Note: Rust and config files (.env, service-account-key.json) were preserved"
Write-Host "Press any key to exit..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
