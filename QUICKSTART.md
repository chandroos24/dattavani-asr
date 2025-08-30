# Quick Start Guide - Dattavani ASR Rust

This guide will help you get up and running with the Rust version of Dattavani ASR in just a few minutes.

## 🚀 Quick Setup

### 1. Run the Setup Script
```bash
./setup.sh
```

This will:
- Check for Rust, FFmpeg, and Whisper
- Install missing dependencies
- Build the project
- Generate configuration files

### 2. Set Up Google Cloud Credentials (Optional)
```bash
# If you have a service account key
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/your/service-account-key.json

# Or use application default credentials
gcloud auth application-default login
```

### 3. Test the Installation
```bash
./target/release/dattavani-asr health-check
```

## 🎵 Basic Usage

### Process a Kannada Audio File
```bash
# Using the Kannada model (default)
./target/release/dattavani-asr stream-process examples/shanishanti-live-23-aug-25.mp3 --language kn

# The transcript will be saved to examples/gen-transcript/
```

### List Available Models
```bash
./target/release/dattavani-asr models list
```

### Test a Specific Model
```bash
./target/release/dattavani-asr models test \
  --model-id whisper-kannada-small \
  --audio-file examples/shanishanti-live-23-aug-25.mp3 \
  --language kn
```

### Benchmark Multiple Models
```bash
./target/release/dattavani-asr models benchmark \
  --audio-file examples/shanishanti-live-23-aug-25.mp3 \
  --language kn \
  --max-models 3
```

## 🔧 Configuration

### Main Configuration (`dattavani-asr.toml`)
```toml
[models]
default_model = "whisper-kannada-small"
confidence_fallback_enabled = true
min_confidence_threshold = 0.6
max_model_attempts = 3
custom_models_file = "models.toml"

[whisper]
model_size = "vasista22/whisper-kannada-small"
device = "auto"
language = "kn"

[processing]
max_workers = 4
segment_duration = 300
```

### Models Configuration (`models.toml`)
The system comes pre-configured with Kannada models:
- `whisper-kannada-small` (primary)
- `whisper-kannada-base` (fallback)
- `whisper-large-v3` (multilingual fallback)

## 📋 Common Commands

### Process Single File
```bash
# Local file
./target/release/dattavani-asr stream-process audio.mp3 --language kn

# With custom output
./target/release/dattavani-asr stream-process audio.mp3 --output transcript.txt --language kn

# Google Drive file (requires credentials)
./target/release/dattavani-asr stream-process "https://drive.google.com/file/d/FILE_ID/view" --language kn
```

### Process Multiple Files
```bash
# Local folder
./target/release/dattavani-asr stream-batch /path/to/audio/folder --language kn

# With pattern matching
./target/release/dattavani-asr stream-batch /path/to/folder --pattern "*.mp3" --language kn
```

### Model Management
```bash
# List all models
./target/release/dattavani-asr models list

# List models for specific language
./target/release/dattavani-asr models list --language kn

# Add a new model
./target/release/dattavani-asr models add \
  --id "my-model" \
  --provider "HuggingFace" \
  --model-path "user/model-name" \
  --language "kn"

# Remove a model
./target/release/dattavani-asr models remove my-model
```

### System Information
```bash
# Health check
./target/release/dattavani-asr health-check

# Supported formats
./target/release/dattavani-asr supported-formats

# Test authentication (if using Google Drive)
./target/release/dattavani-asr test-auth
```

## 🎯 Kannada Transcription Example

Here's a complete example for transcribing Kannada audio:

```bash
# 1. Place your Kannada audio file in the examples folder
cp your-kannada-audio.mp3 examples/

# 2. Process with automatic model selection and fallbacks
./target/release/dattavani-asr stream-process examples/your-kannada-audio.mp3 --language kn

# 3. The system will:
#    - Try whisper-kannada-small first
#    - Fall back to whisper-kannada-base if confidence is low
#    - Use whisper-large-v3 as final fallback
#    - Save transcript to examples/gen-transcript/

# 4. View the results
cat examples/gen-transcript/your-kannada-audio_transcript.txt
```

## 🔍 Debugging

### Enable Debug Logging
```bash
RUST_LOG=debug ./target/release/dattavani-asr stream-process audio.mp3 --language kn
```

### Check Model Selection Process
```bash
# This will show which models are tried and their confidence scores
RUST_LOG=info ./target/release/dattavani-asr models benchmark \
  --audio-file audio.mp3 \
  --language kn
```

### Verbose Model Information
```bash
./target/release/dattavani-asr models list --verbose
```

## 🚨 Troubleshooting

### Common Issues

1. **"whisper command not found"**
   ```bash
   pip install openai-whisper
   ```

2. **"ffmpeg command not found"**
   ```bash
   # macOS
   brew install ffmpeg
   
   # Ubuntu/Debian
   sudo apt-get install ffmpeg
   ```

3. **"Google Drive authentication failed"**
   ```bash
   # Set up credentials
   export GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json
   
   # Or use gcloud
   gcloud auth application-default login
   ```

4. **Low confidence scores**
   - Check audio quality
   - Try different models
   - Adjust confidence thresholds in configuration

5. **Build errors**
   ```bash
   # Update Rust
   rustup update
   
   # Clean and rebuild
   cargo clean
   cargo build --release
   ```

## 📈 Performance Tips

1. **For faster processing**: Use smaller models like `whisper-base`
2. **For better accuracy**: Use larger models like `whisper-large-v3`
3. **For Kannada**: Use the specialized `whisper-kannada-small` model
4. **For batch processing**: Increase `max_workers` in configuration
5. **For large files**: Adjust `segment_duration` for memory management

## 🔗 Next Steps

- Read the full [README.md](README.md) for detailed information
- Check [PLUGGABLE_MODELS.md](PLUGGABLE_MODELS.md) for advanced model configuration
- Explore the [examples/](examples/) folder for more usage examples
- Set up Google Cloud integration for Drive and Storage support

## 💡 Tips

- The system automatically selects the best model based on language and confidence
- Transcripts are saved with timestamps and confidence scores
- You can process multiple languages by configuring different models
- The pluggable architecture allows easy addition of new models and providers

Happy transcribing! 🎉
