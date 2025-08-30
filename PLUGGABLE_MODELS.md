# Pluggable Model System

The Dattavani ASR system now features a comprehensive pluggable model architecture that supports multiple model providers, confidence-based fallbacks, and language-specific model selection.

## Features

### 🔌 Multiple Model Providers
- **WhisperCLI**: Traditional Whisper models via CLI
- **HuggingFace**: Models from Hugging Face Hub (like `vasista22/whisper-kannada-small`)
- **OpenAI**: OpenAI API integration (planned)
- **Local**: Local Candle-based models (planned)
- **Custom**: Custom provider implementations

### 🎯 Confidence-Based Fallbacks
- Automatic model selection based on confidence thresholds
- Configurable fallback chains
- Best-effort transcription with multiple attempts

### 🌍 Language-Specific Models
- Language-to-model mappings
- Automatic model selection based on language hints
- Support for multilingual fallbacks

## Configuration

### Basic Configuration (`dattavani-asr.toml`)

```toml
[models]
default_model = "whisper-kannada-small"
confidence_fallback_enabled = true
min_confidence_threshold = 0.6
max_model_attempts = 3
custom_models_file = "models.toml"
```

### Custom Models File (`models.toml`)

```toml
[registry]
default_model = "whisper-kannada-small"
confidence_fallback_enabled = true
min_confidence_threshold = 0.6

# Kannada Models
[[models]]
id = "whisper-kannada-small"
provider = "HuggingFace"
model_path = "vasista22/whisper-kannada-small"
language = "kn"
priority = 1
confidence_threshold = 0.75
fallback_models = ["whisper-kannada-base", "whisper-large-v3"]

[[models]]
id = "whisper-kannada-base"
provider = "HuggingFace"
model_path = "vasista22/whisper-kannada-base"
language = "kn"
priority = 2
confidence_threshold = 0.7
fallback_models = ["whisper-large-v3"]

# Language Mappings
[language_mappings]
kn = ["whisper-kannada-small", "whisper-kannada-base", "whisper-large-v3"]
en = ["whisper-large-v3", "whisper-base", "whisper-small"]
```

## Usage

### CLI Commands

#### List Available Models
```bash
# List all models
cargo run -- models list

# List models for specific language
cargo run -- models list --language kn

# Verbose output with parameters
cargo run -- models list --verbose
```

#### Add New Model
```bash
cargo run -- models add \
  --id "my-kannada-model" \
  --provider "HuggingFace" \
  --model-path "user/my-kannada-whisper" \
  --language "kn" \
  --priority 1 \
  --confidence-threshold 0.8 \
  --fallback-models "whisper-kannada-small,whisper-large-v3"
```

#### Test Model
```bash
cargo run -- models test \
  --model-id "whisper-kannada-small" \
  --audio-file "examples/kannada-audio.mp3" \
  --language "kn"
```

#### Benchmark Models
```bash
cargo run -- models benchmark \
  --audio-file "examples/kannada-audio.mp3" \
  --language "kn" \
  --max-models 3
```

### Programmatic Usage

#### Basic Usage with Default Models
```rust
use dattavani_asr::{config::Config, asr::DattavaniAsr};

let config = Config::load().await?;
let asr = DattavaniAsr::new(config).await?;

let result = asr.stream_process_single_file(
    "audio.mp3",
    Some("transcript.txt"),
    Some("kn"), // Kannada
    None,
).await?;
```

#### Custom Model Registry
```rust
use dattavani_asr::asr::models::{ModelRegistry, ModelConfig, ModelProvider};
use std::collections::HashMap;

let mut registry = ModelRegistry::default();

// Add custom Kannada model
let kannada_model = ModelConfig {
    id: "my-kannada-model".to_string(),
    provider: ModelProvider::HuggingFace,
    model_path: "vasista22/whisper-kannada-small".to_string(),
    language: Some("kn".to_string()),
    priority: 1,
    confidence_threshold: 0.75,
    fallback_models: vec!["whisper-large-v3".to_string()],
    parameters: HashMap::new(),
};

registry.add_model(kannada_model);

let asr = DattavaniAsr::new_with_custom_models(config, registry).await?;
```

#### Direct Model Manager Usage
```rust
use dattavani_asr::asr::models::ModelManager;

let model_manager = ModelManager::new_with_config(config).await?;

let result = model_manager.transcribe_with_best_model(
    &audio_path,
    Some("kn"), // Language hint
    Some(3),    // Max attempts
).await?;

println!("Selected model: {}", result.selected_model);
println!("Confidence: {:.3}", result.final_result.confidence.unwrap_or(0.0));
```

## Model Providers

### HuggingFace Provider

The HuggingFace provider supports models from the Hugging Face Hub. It uses Python with the `transformers` library for inference.

**Requirements:**
- Python 3.7+
- `transformers` library
- `torch` library
- `librosa` library

**Installation:**
```bash
pip install transformers torch librosa
```

**Supported Models:**
- `vasista22/whisper-kannada-small`
- `vasista22/whisper-kannada-base`
- Any Whisper-compatible model on Hugging Face

### WhisperCLI Provider

Uses the traditional Whisper CLI tool for transcription.

**Requirements:**
- `whisper` CLI tool installed
- Model files downloaded

**Installation:**
```bash
pip install openai-whisper
```

## Confidence-Based Fallbacks

The system automatically tries multiple models based on confidence scores:

1. **Primary Model**: Tries the highest priority model for the language
2. **Confidence Check**: If confidence < threshold, tries fallback models
3. **Best Result**: Returns the result with highest confidence above minimum threshold
4. **Fallback Chain**: Follows the configured fallback model chain

### Example Flow for Kannada Audio:

1. Try `whisper-kannada-small` (threshold: 0.75)
   - If confidence ≥ 0.75: ✅ Return result
   - If confidence < 0.75: ⬇️ Try fallback

2. Try `whisper-kannada-base` (threshold: 0.7)
   - If confidence ≥ 0.7: ✅ Return result
   - If confidence < 0.7: ⬇️ Try fallback

3. Try `whisper-large-v3` (threshold: 0.6)
   - Return best result found

## Advanced Features

### Custom Providers

You can implement custom model providers:

```rust
impl ModelManager {
    async fn transcribe_with_custom_provider(
        &self,
        audio_path: &Path,
        model_config: &ModelConfig,
        language: Option<&str>,
        provider_name: &str,
    ) -> Result<TranscriptionResult> {
        match provider_name {
            "my-custom-provider" => {
                // Your custom implementation
                self.my_custom_transcription(audio_path, model_config, language).await
            }
            _ => Err(DattavaniError::asr_processing(
                format!("Unknown custom provider: {}", provider_name)
            ))
        }
    }
}
```

### Model Parameters

Each model can have custom parameters:

```toml
[[models]]
id = "whisper-kannada-optimized"
provider = "HuggingFace"
model_path = "vasista22/whisper-kannada-small"
language = "kn"
priority = 1
confidence_threshold = 0.8

[models.parameters]
temperature = "0.0"
best_of = "5"
beam_size = "5"
patience = "1.0"
```

### Multi-Model Results

The system provides detailed information about all model attempts:

```rust
let result = model_manager.transcribe_with_best_model(&audio_path, Some("kn"), Some(3)).await?;

println!("Final result: {}", result.final_result.text.unwrap_or_default());
println!("Selected model: {}", result.selected_model);
println!("Total processing time: {:.2}s", result.total_processing_time);

for attempt in result.attempts {
    println!("Model: {}, Success: {}, Confidence: {:.3}", 
             attempt.model_id, attempt.success, attempt.confidence.unwrap_or(0.0));
}
```

## Best Practices

### 1. Model Selection
- Use language-specific models as primary choices
- Configure multilingual models as fallbacks
- Set appropriate confidence thresholds based on your quality requirements

### 2. Performance Optimization
- Order models by speed vs. accuracy trade-off
- Use smaller models for real-time applications
- Use larger models for batch processing

### 3. Confidence Thresholds
- **High Quality (0.8+)**: For production transcripts
- **Medium Quality (0.6-0.8)**: For draft transcripts
- **Low Quality (0.4-0.6)**: For rough transcripts or difficult audio

### 4. Fallback Chains
- Start with specialized models (language-specific)
- Fall back to general models (multilingual)
- Keep fallback chains short (2-3 models max)

## Troubleshooting

### Common Issues

1. **HuggingFace models not working**
   - Ensure Python dependencies are installed
   - Check model path is correct
   - Verify internet connection for model download

2. **Low confidence scores**
   - Check audio quality
   - Verify language setting
   - Try different models
   - Adjust confidence thresholds

3. **Model not found errors**
   - Verify model ID in configuration
   - Check models.toml file exists
   - Ensure model is properly registered

### Debug Mode

Enable verbose logging to see model selection process:

```bash
RUST_LOG=debug cargo run -- stream-process input.mp3 --language kn
```

This will show:
- Model selection logic
- Confidence scores for each attempt
- Fallback decisions
- Processing times

## Contributing

To add support for new model providers:

1. Extend the `ModelProvider` enum
2. Implement the transcription method in `ModelManager`
3. Add configuration support
4. Update documentation
5. Add tests

Example provider implementation in `src/asr/models.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelProvider {
    // ... existing providers
    MyCustomProvider,
}

impl ModelManager {
    async fn transcribe_with_model(&self, ...) -> Result<TranscriptionResult> {
        match &model_config.provider {
            // ... existing providers
            ModelProvider::MyCustomProvider => {
                self.transcribe_with_my_custom_provider(audio_path, model_config, language).await
            }
        }
    }
}
```
