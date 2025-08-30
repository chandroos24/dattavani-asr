/*!
Example: Kannada Transcription with Pluggable Model System

This example demonstrates how to use the new pluggable model system
to transcribe Kannada audio with confidence-based fallbacks.
*/

use std::path::PathBuf;
use dattavani_asr::{
    config::Config,
    asr::{DattavaniAsr, models::{ModelRegistry, ModelConfig, ModelProvider}},
    error::Result,
};
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🎯 Kannada Transcription Example with Pluggable Models");

    // Load configuration
    let config = Config::load().await?;
    
    // Create custom model registry for Kannada
    let mut registry = create_kannada_model_registry();
    
    // Initialize ASR with custom models
    let asr = DattavaniAsr::new_with_custom_models(config, registry).await?;
    
    // Example audio file (replace with your Kannada audio file)
    let audio_file = PathBuf::from("examples/shanishanti-live-23-aug-25.mp3");
    
    if !audio_file.exists() {
        info!("⚠️  Audio file not found: {}", audio_file.display());
        info!("Please place a Kannada audio file at the specified path");
        return Ok(());
    }
    
    info!("🎵 Processing Kannada audio: {}", audio_file.display());
    
    // Process with automatic model selection and confidence-based fallbacks
    let result = asr.stream_process_single_file(
        audio_file.to_str().unwrap(),
        Some("examples/kannada_transcript.txt"),
        Some("kn"), // Kannada language code
        None,
    ).await?;
    
    // Display results
    if result.success {
        info!("✅ Transcription successful!");
        
        if let Some(text) = &result.text {
            info!("📝 Transcribed text:");
            println!("{}", text);
        }
        
        if let Some(confidence) = result.confidence {
            info!("🎯 Confidence: {:.1}%", confidence * 100.0);
        }
        
        if let Some(processing_time) = result.processing_time {
            info!("⏱️  Processing time: {:.2}s", processing_time);
        }
        
        if let Some(language) = &result.language {
            info!("🌍 Detected language: {}", language);
        }
    } else {
        info!("❌ Transcription failed: {}", 
              result.error.as_deref().unwrap_or("Unknown error"));
    }
    
    // Demonstrate model management
    demonstrate_model_management().await?;
    
    Ok(())
}

fn create_kannada_model_registry() -> ModelRegistry {
    use std::collections::HashMap;
    
    let mut models = HashMap::new();
    let mut language_mappings = HashMap::new();
    
    // Primary Kannada model (HuggingFace)
    models.insert("whisper-kannada-small".to_string(), ModelConfig {
        id: "whisper-kannada-small".to_string(),
        provider: ModelProvider::HuggingFace,
        model_path: "vasista22/whisper-kannada-small".to_string(),
        language: Some("kn".to_string()),
        priority: 1,
        confidence_threshold: 0.75,
        fallback_models: vec!["whisper-kannada-base".to_string()],
        parameters: HashMap::new(),
    });
    
    // Secondary Kannada model
    models.insert("whisper-kannada-base".to_string(), ModelConfig {
        id: "whisper-kannada-base".to_string(),
        provider: ModelProvider::HuggingFace,
        model_path: "vasista22/whisper-kannada-base".to_string(),
        language: Some("kn".to_string()),
        priority: 2,
        confidence_threshold: 0.7,
        fallback_models: vec!["whisper-large-v3".to_string()],
        parameters: HashMap::new(),
    });
    
    // Fallback multilingual model
    models.insert("whisper-large-v3".to_string(), ModelConfig {
        id: "whisper-large-v3".to_string(),
        provider: ModelProvider::WhisperCLI,
        model_path: "large-v3".to_string(),
        language: None, // Multilingual
        priority: 3,
        confidence_threshold: 0.6,
        fallback_models: vec![],
        parameters: HashMap::new(),
    });
    
    // Language mappings
    language_mappings.insert("kn".to_string(), vec![
        "whisper-kannada-small".to_string(),
        "whisper-kannada-base".to_string(),
        "whisper-large-v3".to_string(),
    ]);
    
    ModelRegistry {
        models,
        language_mappings,
        default_model: "whisper-kannada-small".to_string(),
        confidence_fallback_enabled: true,
        min_confidence_threshold: 0.6,
    }
}

async fn demonstrate_model_management() -> Result<()> {
    info!("🔧 Demonstrating Model Management");
    
    // Load models from file
    let models_file = PathBuf::from("models.toml");
    if models_file.exists() {
        let registry = ModelRegistry::load_from_file(&models_file).await?;
        info!("📁 Loaded {} models from {}", 
              registry.models.len(), models_file.display());
        
        // List Kannada models
        let kannada_models = registry.list_models_for_language("kn");
        info!("🇮🇳 Available Kannada models:");
        for model in kannada_models {
            info!("  • {} (priority: {}, threshold: {:.2})", 
                  model.id, model.priority, model.confidence_threshold);
        }
    } else {
        info!("⚠️  Models file not found: {}", models_file.display());
        info!("Using default model registry");
    }
    
    Ok(())
}

// Additional helper functions for advanced usage
pub async fn benchmark_kannada_models(audio_file: &PathBuf) -> Result<()> {
    use dattavani_asr::asr::models::ModelManager;
    
    let config = Config::load().await?;
    let model_manager = ModelManager::new_with_config(config).await?;
    
    info!("🏁 Benchmarking Kannada models with: {}", audio_file.display());
    
    let result = model_manager.transcribe_with_best_model(
        audio_file,
        Some("kn"),
        Some(3), // Try up to 3 models
    ).await?;
    
    info!("🏆 Benchmark Results:");
    info!("Selected Model: {}", result.selected_model);
    info!("Total Time: {:.2}s", result.total_processing_time);
    
    for (i, attempt) in result.attempts.iter().enumerate() {
        info!("  {}. {} - Success: {}, Confidence: {:.3}, Time: {:.2}s",
              i + 1,
              attempt.model_id,
              attempt.success,
              attempt.confidence.unwrap_or(0.0),
              attempt.processing_time);
    }
    
    Ok(())
}

pub async fn create_custom_kannada_model() -> Result<ModelConfig> {
    // Example of creating a custom model configuration
    let mut parameters = std::collections::HashMap::new();
    parameters.insert("temperature".to_string(), "0.0".to_string());
    parameters.insert("best_of".to_string(), "5".to_string());
    
    Ok(ModelConfig {
        id: "custom-kannada-model".to_string(),
        provider: ModelProvider::Custom("custom-provider".to_string()),
        model_path: "/path/to/custom/kannada/model".to_string(),
        language: Some("kn".to_string()),
        priority: 1,
        confidence_threshold: 0.8,
        fallback_models: vec!["whisper-kannada-small".to_string()],
        parameters,
    })
}
