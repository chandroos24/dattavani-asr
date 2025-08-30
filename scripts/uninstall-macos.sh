#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

print_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }

print_info "Uninstalling Dattavani ASR..."

# Remove build artifacts
rm -rf target/
rm -rf whisper_simple/

# Remove desktop shortcut
rm -f "$HOME/Desktop/Dattavani ASR.command"

# Ask about removing dependencies
read -p "Remove system dependencies (ffmpeg, whisper)? [y/N]: " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    pip3 uninstall -y openai-whisper
    brew uninstall ffmpeg
fi

print_info "Uninstall complete!"
print_info "Note: Rust, Homebrew, and config files (.env, service-account-key.json) were preserved"
