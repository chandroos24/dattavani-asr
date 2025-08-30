@echo off
REM Manual Windows Installer (requires pre-installed dependencies)

echo [INFO] Dattavani ASR Manual Windows Installer
echo [INFO] This installer requires you to manually install dependencies first
echo.

REM Check for Rust
where rustc >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Rust not found!
    echo [INFO] Please install Rust from: https://rustup.rs/
    echo [INFO] Then run this script again.
    pause
    exit /b 1
)

REM Check for Python
where python >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Python not found!
    echo [INFO] Please install Python from: https://python.org/downloads/
    pause
    exit /b 1
)

REM Check for FFmpeg
where ffmpeg >nul 2>nul
if %errorlevel% neq 0 (
    echo [WARN] FFmpeg not found!
    echo [INFO] Please install FFmpeg from: https://ffmpeg.org/download.html
    echo [INFO] Or use Chocolatey: choco install ffmpeg
)

echo [INFO] Installing OpenAI Whisper...
python -m pip install --upgrade pip
python -m pip install --user openai-whisper

echo [INFO] Setting up Whisper CLI...
mkdir whisper_simple\bin 2>nul

echo [INFO] Building Dattavani ASR...
cargo build --release

if %errorlevel% neq 0 (
    echo [ERROR] Build failed!
    pause
    exit /b 1
)

REM Create config files
if not exist "service-account-key.json" (
    echo [INFO] Creating service account key template...
    echo {> service-account-key.json
    echo   "type": "service_account",>> service-account-key.json
    echo   "project_id": "YOUR_PROJECT_ID",>> service-account-key.json
    echo   "private_key_id": "YOUR_PRIVATE_KEY_ID",>> service-account-key.json
    echo   "private_key": "YOUR_PRIVATE_KEY",>> service-account-key.json
    echo   "client_email": "YOUR_CLIENT_EMAIL",>> service-account-key.json
    echo   "client_id": "YOUR_CLIENT_ID",>> service-account-key.json
    echo   "auth_uri": "https://accounts.google.com/o/oauth2/auth",>> service-account-key.json
    echo   "token_uri": "https://oauth2.googleapis.com/token">> service-account-key.json
    echo }>> service-account-key.json
)

if not exist ".env" (
    echo [INFO] Creating .env configuration...
    echo GOOGLE_APPLICATION_CREDENTIALS=./service-account-key.json> .env
    echo WHISPER_MODEL_SIZE=large-v3>> .env
    echo MAX_WORKERS=4>> .env
    echo LOG_LEVEL=info>> .env
)

REM Create batch wrapper
echo [INFO] Creating batch wrapper...
echo @echo off> dattavani-asr.bat
echo cd /d "%%~dp0">> dattavani-asr.bat
echo target\release\dattavani-asr.exe %%*>> dattavani-asr.bat

echo.
echo [INFO] ✅ Installation complete!
echo [INFO] Binary: target\release\dattavani-asr.exe
echo [INFO] Wrapper: dattavani-asr.bat
echo [WARN] Don't forget to edit service-account-key.json
echo.
pause
