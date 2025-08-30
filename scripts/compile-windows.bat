@echo off
REM Dattavani ASR Windows Compilation Script

echo [INFO] Compiling Dattavani ASR for Windows...

REM Check if Rust is installed
where rustc >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Rust not found. Please install Rust first.
    echo [INFO] Run: .\install-windows.ps1
    pause
    exit /b 1
)

REM Check if we're in the right directory
if not exist "Cargo.toml" (
    echo [ERROR] Cargo.toml not found. Please run from project root.
    pause
    exit /b 1
)

REM Clean previous builds
echo [INFO] Cleaning previous builds...
cargo clean

REM Build release version
echo [INFO] Building release version...
cargo build --release

if %errorlevel% neq 0 (
    echo [ERROR] Build failed!
    pause
    exit /b 1
)

REM Check if binary was created
if not exist "target\release\dattavani-asr.exe" (
    echo [ERROR] Binary not found after build!
    pause
    exit /b 1
)

echo [INFO] ✅ Compilation successful!
echo [INFO] Binary location: target\release\dattavani-asr.exe
echo [INFO] File size: 
dir "target\release\dattavani-asr.exe" | findstr "dattavani-asr.exe"

REM Test the binary
echo [INFO] Testing binary...
target\release\dattavani-asr.exe --help >nul 2>nul
if %errorlevel% equ 0 (
    echo [INFO] ✅ Binary test successful!
) else (
    echo [WARN] ⚠️  Binary test failed - may need dependencies
)

echo.
echo [INFO] Compilation complete! Press any key to exit.
pause >nul
