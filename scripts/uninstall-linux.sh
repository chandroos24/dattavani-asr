#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

print_info() { echo -e "${GREEN}[INFO]${NC} $1"; }

# Detect distribution
if [[ -f /etc/os-release ]]; then
    . /etc/os-release
    DISTRO="$ID"
fi

print_info "Uninstalling Dattavani ASR from $PRETTY_NAME..."

# Remove build artifacts
rm -rf target/
rm -rf whisper_simple/

# Stop and remove systemd service
if [[ -f /etc/systemd/system/dattavani-asr.service ]]; then
    sudo systemctl stop dattavani-asr 2>/dev/null || true
    sudo systemctl disable dattavani-asr 2>/dev/null || true
    sudo rm -f /etc/systemd/system/dattavani-asr.service
    sudo systemctl daemon-reload 2>/dev/null || true
fi

# Ask about removing dependencies
read -p "Remove system dependencies (ffmpeg, whisper)? [y/N]: " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    # Remove whisper
    case "$DISTRO" in
        "arch"|"manjaro"|"alpine")
            pip uninstall -y openai-whisper 2>/dev/null || true
            ;;
        *)
            pip3 uninstall -y openai-whisper 2>/dev/null || true
            ;;
    esac
    
    # Remove ffmpeg based on package manager
    case "$DISTRO" in
        "ubuntu"|"debian")
            sudo apt-get remove -y ffmpeg
            ;;
        "fedora")
            sudo dnf remove -y ffmpeg
            ;;
        "centos"|"rhel"|"rocky"|"almalinux"|"amzn")
            sudo yum remove -y ffmpeg
            ;;
        "opensuse"|"opensuse-leap"|"opensuse-tumbleweed")
            sudo zypper remove -y ffmpeg
            ;;
        "arch"|"manjaro")
            sudo pacman -R --noconfirm ffmpeg
            ;;
        "alpine")
            sudo apk del ffmpeg
            ;;
    esac
fi

print_info "Uninstall complete!"
print_info "Note: Rust and config files (.env, service-account-key.json) were preserved"
