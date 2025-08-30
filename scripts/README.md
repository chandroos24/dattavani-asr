# Installation Scripts

This directory contains installation and uninstallation scripts for different operating systems.

## Available Scripts

### Installation Scripts

- **`install-linux.sh`** - Universal installer for all major Linux distributions
- **`install-macos.sh`** - macOS installer using Homebrew
- **`install-windows.ps1`** - Windows PowerShell installer with Chocolatey (automated)
- **`install-windows-manual.bat`** - Manual Windows installer (requires pre-installed dependencies)
- **`install-debian.sh`** - Debian/Ubuntu specific installer (legacy)
- **`install-amazon-linux.sh`** - Amazon Linux specific installer (legacy)

### Compilation Scripts

- **`compile-windows.bat`** - Windows compilation script

### Uninstallation Scripts

- **`uninstall-linux.sh`** - Universal Linux uninstaller
- **`uninstall-macos.sh`** - macOS uninstaller
- **`uninstall-windows.ps1`** - Windows PowerShell uninstaller

## Quick Start

### Linux (Recommended)
```bash
cd scripts
./install-linux.sh
```

### macOS
```bash
cd scripts
./install-macos.sh
```

### Windows

**PowerShell (Automated)**:
```powershell
cd scripts
.\install-windows.ps1
```

**Manual (Pre-install dependencies)**:
```cmd
cd scripts
install-windows-manual.bat
```

**Compile Only**:
```cmd
cd scripts
compile-windows.bat
```

### Uninstall
```bash
cd scripts
./uninstall-linux.sh    # Linux
./uninstall-macos.sh    # macOS
```
```powershell
cd scripts
.\uninstall-windows.ps1  # Windows
```

## Windows Requirements

- Windows 10/11 or Windows Server 2019+
- PowerShell 5.1+ (for PowerShell installer)
- Administrator privileges (for system dependencies)

### Windows Dependencies

The PowerShell installer automatically installs:
- Chocolatey package manager
- FFmpeg
- Python 3
- Git
- Rust toolchain
- OpenAI Whisper

The manual batch installer requires you to pre-install:
- Rust: https://rustup.rs/
- Python: https://python.org/downloads/
- FFmpeg: https://ffmpeg.org/download.html

## Supported Linux Distributions

- **Debian-based**: Ubuntu, Debian
- **Red Hat-based**: Fedora, CentOS, RHEL, Rocky Linux, AlmaLinux
- **Amazon Linux**: Amazon Linux 2/2023
- **SUSE-based**: openSUSE Leap/Tumbleweed
- **Arch-based**: Arch Linux, Manjaro
- **Alpine**: Alpine Linux

## What Gets Installed

- Rust toolchain
- FFmpeg
- OpenAI Whisper CLI
- System build dependencies
- Project binary (`target/release/dattavani-asr`)
- Configuration templates
- Systemd service (Linux only)
- Desktop shortcuts (macOS/Windows)

## Post-Installation

1. Edit `service-account-key.json` with your Google Cloud credentials
2. Optionally modify `.env` configuration
3. Run `source ~/.bashrc` or restart terminal (Linux/macOS)
4. Test with `./target/release/dattavani-asr --help` (Linux/macOS) or `dattavani-asr.bat --help` (Windows)
