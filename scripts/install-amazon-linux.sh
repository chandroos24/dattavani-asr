#!/bin/bash

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
print_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Detect Amazon Linux version
if [[ -f /etc/os-release ]]; then
    . /etc/os-release
    if [[ "$ID" != "amzn" ]]; then
        print_error "This installer is for Amazon Linux only"
        exit 1
    fi
else
    print_error "Cannot detect OS version"
    exit 1
fi

print_info "Installing Dattavani ASR for Amazon Linux..."

# Update system
print_info "Updating system packages..."
sudo yum update -y

# Install system dependencies
print_info "Installing system dependencies..."
sudo yum groupinstall -y "Development Tools"
sudo yum install -y gcc gcc-c++ make cmake pkg-config openssl-devel python3 python3-pip

# Install FFmpeg (EPEL required)
print_info "Installing FFmpeg..."
sudo yum install -y epel-release
sudo yum install -y ffmpeg ffmpeg-devel

# Install Rust
if ! command -v rustc &> /dev/null; then
    print_info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    echo 'source ~/.cargo/env' >> ~/.bashrc
fi

# Install OpenAI Whisper
print_info "Installing OpenAI Whisper..."
pip3 install --user --upgrade pip
pip3 install --user openai-whisper

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

print_info "Installation complete!"
print_info "Binary location: ./target/release/dattavani-asr"
print_warn "Don't forget to configure service-account-key.json"
print_warn "Run 'source ~/.bashrc' or restart terminal to update PATH"

# Test installation
print_info "Testing installation..."
if ./target/release/dattavani-asr --help &> /dev/null; then
    print_info "✅ Installation successful!"
else
    print_error "❌ Installation test failed"
    exit 1
fi
