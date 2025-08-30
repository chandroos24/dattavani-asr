# Dattavani ASR Windows Installer
# Requires PowerShell 5.1+ and Administrator privileges for some components

param(
    [switch]$SkipChoco,
    [switch]$Help
)

if ($Help) {
    Write-Host @"
Dattavani ASR Windows Installer

Usage: .\install-windows.ps1 [options]

Options:
    -SkipChoco    Skip Chocolatey installation (if already installed)
    -Help         Show this help message

Requirements:
    - Windows 10/11 or Windows Server 2019+
    - PowerShell 5.1+
    - Administrator privileges (for system dependencies)

"@
    exit 0
}

# Colors for output
function Write-Info { param($msg) Write-Host "[INFO] $msg" -ForegroundColor Green }
function Write-Warn { param($msg) Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Write-Error { param($msg) Write-Host "[ERROR] $msg" -ForegroundColor Red }

Write-Info "Installing Dattavani ASR for Windows..."

# Check Windows version
$osVersion = [System.Environment]::OSVersion.Version
if ($osVersion.Major -lt 10) {
    Write-Error "Windows 10 or later required"
    exit 1
}

# Check if running as Administrator for system installs
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")

# Install Chocolatey if not present
if (-not $SkipChoco -and -not (Get-Command choco -ErrorAction SilentlyContinue)) {
    if (-not $isAdmin) {
        Write-Error "Administrator privileges required to install Chocolatey"
        Write-Info "Run PowerShell as Administrator or use -SkipChoco if Chocolatey is already installed"
        exit 1
    }
    
    Write-Info "Installing Chocolatey..."
    Set-ExecutionPolicy Bypass -Scope Process -Force
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
    iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
    refreshenv
}

# Install system dependencies via Chocolatey
if (Get-Command choco -ErrorAction SilentlyContinue) {
    if (-not $isAdmin) {
        Write-Warn "Administrator privileges recommended for system dependencies"
        Write-Info "Some dependencies may need manual installation"
    } else {
        Write-Info "Installing system dependencies..."
        choco install -y ffmpeg python3 git
    }
} else {
    Write-Warn "Chocolatey not available. Please install FFmpeg, Python3, and Git manually"
}

# Install Rust
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Info "Installing Rust..."
    $rustInstaller = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustInstaller
    & $rustInstaller -y --default-toolchain stable
    $env:PATH += ";$env:USERPROFILE\.cargo\bin"
    Remove-Item $rustInstaller
}

# Install OpenAI Whisper
Write-Info "Installing OpenAI Whisper..."
python -m pip install --upgrade pip
python -m pip install --user openai-whisper

# Create whisper_simple directory and copy whisper executable
Write-Info "Setting up Whisper CLI..."
New-Item -ItemType Directory -Force -Path "whisper_simple\bin"
$whisperPath = python -c "import whisper; import os; print(os.path.dirname(whisper.__file__))" 2>$null
if ($whisperPath) {
    $whisperScript = Get-ChildItem "$env:USERPROFILE\AppData\Roaming\Python\Python*\Scripts\whisper.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($whisperScript) {
        Copy-Item $whisperScript.FullName "whisper_simple\bin\whisper.exe"
    }
}

# Build the project
Write-Info "Building Dattavani ASR..."
cargo build --release

# Create service account key template
if (-not (Test-Path "service-account-key.json")) {
    Write-Info "Creating service account key template..."
    @'
{
  "type": "service_account",
  "project_id": "YOUR_PROJECT_ID",
  "private_key_id": "YOUR_PRIVATE_KEY_ID",
  "private_key": "YOUR_PRIVATE_KEY",
  "client_email": "YOUR_CLIENT_EMAIL",
  "client_id": "YOUR_CLIENT_ID",
  "auth_uri": "https://accounts.google.com/o/oauth2/auth",
  "token_uri": "https://oauth2.googleapis.com/token"
}
'@ | Out-File -FilePath "service-account-key.json" -Encoding UTF8
    Write-Warn "Please edit service-account-key.json with your Google Cloud credentials"
}

# Create .env file
if (-not (Test-Path ".env")) {
    Write-Info "Creating .env configuration..."
    @'
GOOGLE_APPLICATION_CREDENTIALS=./service-account-key.json
WHISPER_MODEL_SIZE=large-v3
MAX_WORKERS=4
LOG_LEVEL=info
'@ | Out-File -FilePath ".env" -Encoding UTF8
}

# Create Windows batch wrapper
Write-Info "Creating Windows batch wrapper..."
@'
@echo off
cd /d "%~dp0"
set PATH=%~dp0whisper_simple\bin;%PATH%
target\release\dattavani-asr.exe %*
'@ | Out-File -FilePath "dattavani-asr.bat" -Encoding ASCII

# Create desktop shortcut
$desktopPath = [Environment]::GetFolderPath("Desktop")
$shortcutPath = "$desktopPath\Dattavani ASR.lnk"
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = "cmd.exe"
$shortcut.Arguments = "/k cd /d `"$(Get-Location)`" && echo Dattavani ASR Environment Ready"
$shortcut.WorkingDirectory = Get-Location
$shortcut.Save()

Write-Info "Installation complete!"
Write-Info "Binary location: target\release\dattavani-asr.exe"
Write-Info "Batch wrapper: dattavani-asr.bat"
Write-Info "Desktop shortcut created: $shortcutPath"
Write-Warn "Don't forget to configure service-account-key.json"

# Test installation
Write-Info "Testing installation..."
if (Test-Path "target\release\dattavani-asr.exe") {
    try {
        & "target\release\dattavani-asr.exe" --help | Out-Null
        Write-Info "✅ Installation successful!"
    } catch {
        Write-Error "❌ Installation test failed: $_"
        exit 1
    }
} else {
    Write-Error "❌ Binary not found"
    exit 1
}
