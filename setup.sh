#!/bin/bash

# Dattavani ASR Rust Setup Script
# This script helps you set up and run the Rust version of Dattavani ASR

set -e

echo "🚀 Dattavani ASR Rust Setup"
echo "=========================="

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✅ Rust is installed: $(rustc --version)"

# Check if FFmpeg is installed
if ! command -v ffmpeg &> /dev/null; then
    echo "⚠️  FFmpeg is not installed. Installing..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        if command -v brew &> /dev/null; then
            brew install ffmpeg
        else
            echo "❌ Please install Homebrew first: https://brew.sh/"
            exit 1
        fi
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux
        sudo apt-get update && sudo apt-get install -y ffmpeg
    else
        echo "❌ Please install FFmpeg manually for your system"
        exit 1
    fi
fi

echo "✅ FFmpeg is installed: $(ffmpeg -version | head -n1)"

# Check if Whisper is installed
if ! command -v whisper &> /dev/null; then
    echo "⚠️  Whisper is not installed. Installing..."
    pip install openai-whisper
fi

echo "✅ Whisper is installed: $(whisper --help | head -n1)"

# Build the project
echo "🔨 Building Dattavani ASR..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
else
    echo "❌ Build failed. Please check the errors above."
    exit 1
fi

# Create necessary directories
mkdir -p logs
mkdir -p examples

echo "📁 Created necessary directories"

# Check if configuration exists
if [ ! -f "dattavani-asr.toml" ]; then
    echo "📝 Generating default configuration..."
    ./target/release/dattavani-asr generate-config --output dattavani-asr.toml
fi

echo "✅ Configuration file ready: dattavani-asr.toml"

# Check if models configuration exists
if [ ! -f "models.toml" ]; then
    echo "📝 Models configuration already exists: models.toml"
else
    echo "✅ Models configuration ready: models.toml"
fi

echo ""
echo "🎉 Setup Complete!"
echo "=================="
echo ""
echo "📋 Next Steps:"
echo "1. Set up Google Cloud credentials:"
echo "   export GOOGLE_APPLICATION_CREDENTIALS=/path/to/your/service-account-key.json"
echo ""
echo "2. Test the installation:"
echo "   ./target/release/dattavani-asr health-check"
echo ""
echo "3. List available models:"
echo "   ./target/release/dattavani-asr models list"
echo ""
echo "4. Process an audio file:"
echo "   ./target/release/dattavani-asr stream-process your-audio.mp3 --language kn"
echo ""
echo "5. View all available commands:"
echo "   ./target/release/dattavani-asr --help"
echo ""
echo "📚 For more information, see:"
echo "   - README.md"
echo "   - PLUGGABLE_MODELS.md"
echo ""
