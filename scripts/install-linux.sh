#!/bin/bash

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
print_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Detect Linux distribution
detect_distro() {
    if [[ -f /etc/os-release ]]; then
        . /etc/os-release
        DISTRO="$ID"
        VERSION="$VERSION_ID"
    elif [[ -f /etc/redhat-release ]]; then
        DISTRO="rhel"
    elif [[ -f /etc/debian_version ]]; then
        DISTRO="debian"
    else
        print_error "Cannot detect Linux distribution"
        exit 1
    fi
    
    print_info "Detected: $PRETTY_NAME"
}

# Install packages based on distribution
install_packages() {
    case "$DISTRO" in
        "ubuntu"|"debian")
            print_info "Using apt package manager..."
            sudo apt-get update
            sudo apt-get install -y build-essential cmake pkg-config libssl-dev curl git python3 python3-pip ffmpeg ca-certificates
            ;;
        "fedora")
            print_info "Using dnf package manager..."
            sudo dnf update -y
            sudo dnf groupinstall -y "Development Tools"
            sudo dnf install -y cmake pkg-config openssl-devel curl git python3 python3-pip ffmpeg
            ;;
        "centos"|"rhel"|"rocky"|"almalinux")
            print_info "Using yum package manager..."
            sudo yum update -y
            sudo yum groupinstall -y "Development Tools"
            sudo yum install -y cmake pkg-config openssl-devel curl git python3 python3-pip epel-release
            sudo yum install -y ffmpeg
            ;;
        "amzn")
            print_info "Using yum package manager (Amazon Linux)..."
            sudo yum update -y
            sudo yum groupinstall -y "Development Tools"
            sudo yum install -y cmake pkg-config openssl-devel curl git python3 python3-pip epel-release
            sudo yum install -y ffmpeg
            ;;
        "opensuse"|"opensuse-leap"|"opensuse-tumbleweed")
            print_info "Using zypper package manager..."
            sudo zypper refresh
            sudo zypper install -y -t pattern devel_basis
            sudo zypper install -y cmake pkg-config libopenssl-devel curl git python3 python3-pip ffmpeg
            ;;
        "arch"|"manjaro")
            print_info "Using pacman package manager..."
            sudo pacman -Syu --noconfirm
            sudo pacman -S --noconfirm base-devel cmake pkg-config openssl curl git python python-pip ffmpeg
            ;;
        "alpine")
            print_info "Using apk package manager..."
            sudo apk update
            sudo apk add build-base cmake pkgconfig openssl-dev curl git python3 py3-pip ffmpeg
            ;;
        *)
            print_error "Unsupported distribution: $DISTRO"
            print_info "Supported: Ubuntu, Debian, Fedora, CentOS, RHEL, Rocky, AlmaLinux, Amazon Linux, openSUSE, Arch, Manjaro, Alpine"
            exit 1
            ;;
    esac
}

# Get pip command based on distro
get_pip_cmd() {
    case "$DISTRO" in
        "arch"|"manjaro"|"alpine")
            echo "pip"
            ;;
        *)
            echo "pip3"
            ;;
    esac
}

print_info "Universal Dattavani ASR Installer for Linux"

# Detect distribution
detect_distro

# Install system packages
install_packages

# Install Rust
if ! command -v rustc &> /dev/null; then
    print_info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    echo 'source ~/.cargo/env' >> ~/.bashrc
fi

# Install OpenAI Whisper
print_info "Installing OpenAI Whisper..."
PIP_CMD=$(get_pip_cmd)
$PIP_CMD install --user --upgrade pip
$PIP_CMD install --user openai-whisper

# Create whisper_simple directory and symlink
mkdir -p whisper_simple/bin
ln -sf ~/.local/bin/whisper whisper_simple/bin/whisper 2>/dev/null || \
ln -sf $(which whisper) whisper_simple/bin/whisper 2>/dev/null || true

# Build the project
print_info "Building Dattavani ASR..."
source ~/.cargo/env
cargo build --release

# Create service account key template
if [[ ! -f "service-account-key.json" ]]; then
    print_info "Creating service account key template..."
    cat > service-account-key.json << 'EOF'
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
EOF
    print_warn "Please edit service-account-key.json with your Google Cloud credentials"
fi

# Create .env file
if [[ ! -f ".env" ]]; then
    print_info "Creating .env configuration..."
    cat > .env << 'EOF'
GOOGLE_APPLICATION_CREDENTIALS=./service-account-key.json
WHISPER_MODEL_SIZE=large-v3
MAX_WORKERS=4
LOG_LEVEL=info
EOF
fi

# Make scripts executable
chmod +x yttl.sh 2>/dev/null || true

# Create systemd service (skip for Alpine)
if [[ "$DISTRO" != "alpine" ]] && command -v systemctl &> /dev/null; then
    print_info "Creating systemd service template..."
    sudo tee /etc/systemd/system/dattavani-asr.service > /dev/null << EOF
[Unit]
Description=Dattavani ASR Service
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$(pwd)
ExecStart=$(pwd)/target/release/dattavani-asr
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
fi

print_info "Installation complete!"
print_info "Distribution: $PRETTY_NAME"
print_info "Binary location: ./target/release/dattavani-asr"
if [[ "$DISTRO" != "alpine" ]] && command -v systemctl &> /dev/null; then
    print_info "Systemd service: sudo systemctl enable dattavani-asr"
fi
print_warn "Don't forget to configure service-account-key.json"
print_warn "Run 'source ~/.bashrc' or restart terminal to update PATH"

# Test installation
print_info "Testing installation..."
if ./target/release/dattavani-asr --help &> /dev/null; then
    print_info "✅ Installation successful on $PRETTY_NAME!"
else
    print_error "❌ Installation test failed"
    exit 1
fi
