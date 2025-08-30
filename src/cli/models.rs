/*!
CLI commands for managing ASR models
*/

use clap::{Args, Subcommand};
use std::path::PathBuf;
use tracing::{info, error};

use crate::asr::models::{ModelRegistry, ModelConfig, ModelProvider};
use crate::config::Config;
use crate::error::Result;

#[derive(Debug, Args)]
pub struct ModelsArgs {
    #[command(subcommand)]
    pub command: ModelsCommand,
}

#[derive(Debug, Subcommand)]
pub enum ModelsCommand {
    /// List all available models
    List {
        /// Filter by language
        #[arg(short, long)]
        language: Option<String>,
        
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Add a new model configuration
    Add {
        /// Model ID
        #[arg(short, long)]
        id: String,
        
        /// Model provider (WhisperCLI, HuggingFace, OpenAI, Local, Custom)
        #[arg(short, long)]
        provider: String,
        
        /// Model path or identifier
        #[arg(short = 'P', long)]
        model_path: String,
        
        /// Language code (optional)
        #[arg(short, long)]
        language: Option<String>,
        
        /// Priority (lower = higher priority)
        #[arg(long, default_value = "5")]
        priority: u8,
        
        /// Confidence threshold
        #[arg(long, default_value = "0.7")]
        confidence_threshold: f64,
        
        /// Fallback model IDs (comma-separated)
        #[arg(long)]
        fallback_models: Option<String>,
    },
    
    /// Remove a model configuration
    Remove {
        /// Model ID to remove
        id: String,
    },
    
    /// Test a model with an audio file
    Test {
        /// Model ID to test
        #[arg(short, long)]
        model_id: String,
        
        /// Audio file path
        #[arg(short, long)]
        audio_file: PathBuf,
        
        /// Language hint
        #[arg(short, long)]
        language: Option<String>,
    },
    
    /// Benchmark models with confidence comparison
    Benchmark {
        /// Audio file path
        #[arg(short, long)]
        audio_file: PathBuf,
        
        /// Language hint
        #[arg(short, long)]
        language: Option<String>,
        
        /// Maximum number of models to test
        #[arg(short, long, default_value = "3")]
        max_models: usize,
    },
    
    /// Export model configuration to file
    Export {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },
    
    /// Import model configuration from file
    Import {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,
    },
}

pub async fn handle_models_command(args: ModelsArgs, config: Config) -> Result<()> {
    match args.command {
        ModelsCommand::List { language, verbose } => {
            list_models(config, language, verbose).await
        }
        ModelsCommand::Add { 
            id, provider, model_path, language, priority, 
            confidence_threshold, fallback_models 
        } => {
            add_model(config, id, provider, model_path, language, 
                     priority, confidence_threshold, fallback_models).await
        }
        ModelsCommand::Remove { id } => {
            remove_model(config, id).await
        }
        ModelsCommand::Test { model_id, audio_file, language } => {
            test_model(config, model_id, audio_file, language).await
        }
        ModelsCommand::Benchmark { audio_file, language, max_models } => {
            benchmark_models(config, audio_file, language, max_models).await
        }
        ModelsCommand::Export { output } => {
            export_models(config, output).await
        }
        ModelsCommand::Import { input } => {
            import_models(config, input).await
        }
    }
}

async fn list_models(config: Config, language_filter: Option<String>, verbose: bool) -> Result<()> {
    let registry = if let Some(models_file) = &config.models.custom_models_file {
        ModelRegistry::load_from_file(models_file).await?
    } else {
        ModelRegistry::default()
    };
    
    let models = if let Some(lang) = language_filter {
        registry.list_models_for_language(&lang)
    } else {
        registry.list_models()
    };
    
    if models.is_empty() {
        println!("No models found.");
        return Ok(());
    }
    
    println!("Available Models:");
    println!("================");
    
    for model in models {
        println!("ID: {}", model.id);
        println!("  Provider: {:?}", model.provider);
        println!("  Path: {}", model.model_path);
        
        if let Some(lang) = &model.language {
            println!("  Language: {}", lang);
        }
        
        println!("  Priority: {}", model.priority);
        println!("  Confidence Threshold: {:.2}", model.confidence_threshold);
        
        if !model.fallback_models.is_empty() {
            println!("  Fallback Models: {}", model.fallback_models.join(", "));
        }
        
        if verbose && !model.parameters.is_empty() {
            println!("  Parameters:");
            for (key, value) in &model.parameters {
                println!("    {}: {}", key, value);
            }
        }
        
        println!();
    }
    
    Ok(())
}

async fn add_model(
    config: Config,
    id: String,
    provider: String,
    model_path: String,
    language: Option<String>,
    priority: u8,
    confidence_threshold: f64,
    fallback_models: Option<String>,
) -> Result<()> {
    let provider = match provider.to_lowercase().as_str() {
        "whispercli" => ModelProvider::WhisperCLI,
        "huggingface" => ModelProvider::HuggingFace,
        "openai" => ModelProvider::OpenAI,
        "local" => ModelProvider::Local,
        custom => ModelProvider::Custom(custom.to_string()),
    };
    
    let fallback_models = fallback_models
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();
    
    let model_config = ModelConfig {
        id: id.clone(),
        provider,
        model_path,
        language,
        priority,
        confidence_threshold,
        fallback_models,
        parameters: std::collections::HashMap::new(),
    };
    
    let mut registry = if let Some(models_file) = &config.models.custom_models_file {
        ModelRegistry::load_from_file(models_file).await.unwrap_or_default()
    } else {
        ModelRegistry::default()
    };
    
    registry.models.insert(id.clone(), model_config);
    
    if let Some(models_file) = &config.models.custom_models_file {
        registry.save_to_file(models_file).await?;
        info!("Added model '{}' and saved to {}", id, models_file.display());
    } else {
        info!("Added model '{}' to in-memory registry", id);
    }
    
    Ok(())
}

async fn remove_model(config: Config, id: String) -> Result<()> {
    let mut registry = if let Some(models_file) = &config.models.custom_models_file {
        ModelRegistry::load_from_file(models_file).await?
    } else {
        return Err(crate::error::DattavaniError::configuration(
            "No custom models file configured"
        ));
    };
    
    if registry.models.remove(&id).is_some() {
        if let Some(models_file) = &config.models.custom_models_file {
            registry.save_to_file(models_file).await?;
            info!("Removed model '{}' and saved to {}", id, models_file.display());
        }
    } else {
        error!("Model '{}' not found", id);
    }
    
    Ok(())
}

async fn test_model(
    config: Config,
    model_id: String,
    audio_file: PathBuf,
    language: Option<String>,
) -> Result<()> {
    use crate::asr::models::ModelManager;
    
    let model_manager = ModelManager::new_with_config(config).await?;
    
    let model_config = model_manager.get_model_config(&model_id)
        .ok_or_else(|| crate::error::DattavaniError::asr_processing(
            format!("Model '{}' not found", model_id)
        ))?;
    
    info!("Testing model '{}' with audio file: {}", model_id, audio_file.display());
    
    let start_time = std::time::Instant::now();
    let result = model_manager.transcribe_with_model(
        &audio_file, 
        model_config, 
        language.as_deref()
    ).await?;
    let processing_time = start_time.elapsed();
    
    println!("Test Results for Model '{}':", model_id);
    println!("============================");
    println!("Success: {}", result.success);
    println!("Processing Time: {:.2}s", processing_time.as_secs_f64());
    
    if let Some(confidence) = result.confidence {
        println!("Confidence: {:.3}", confidence);
    }
    
    if let Some(language) = &result.language {
        println!("Detected Language: {}", language);
    }
    
    if let Some(text) = &result.text {
        println!("Transcription:");
        println!("{}", text);
    }
    
    if let Some(error) = &result.error {
        println!("Error: {}", error);
    }
    
    Ok(())
}

async fn benchmark_models(
    config: Config,
    audio_file: PathBuf,
    language: Option<String>,
    max_models: usize,
) -> Result<()> {
    use crate::asr::models::ModelManager;
    
    let model_manager = ModelManager::new_with_config(config).await?;
    
    info!("Benchmarking models with audio file: {}", audio_file.display());
    
    let result = model_manager.transcribe_with_best_model(
        &audio_file,
        language.as_deref(),
        Some(max_models),
    ).await?;
    
    println!("Benchmark Results:");
    println!("==================");
    println!("Selected Model: {}", result.selected_model);
    println!("Total Processing Time: {:.2}s", result.total_processing_time);
    println!();
    
    println!("Model Attempts:");
    for (i, attempt) in result.attempts.iter().enumerate() {
        println!("{}. Model: {}", i + 1, attempt.model_id);
        println!("   Success: {}", attempt.success);
        println!("   Processing Time: {:.2}s", attempt.processing_time);
        
        if let Some(confidence) = attempt.confidence {
            println!("   Confidence: {:.3}", confidence);
        }
        
        if let Some(error) = &attempt.error {
            println!("   Error: {}", error);
        }
        
        println!();
    }
    
    println!("Final Transcription:");
    if let Some(text) = &result.final_result.text {
        println!("{}", text);
    }
    
    Ok(())
}

async fn export_models(config: Config, output: PathBuf) -> Result<()> {
    let registry = if let Some(models_file) = &config.models.custom_models_file {
        ModelRegistry::load_from_file(models_file).await?
    } else {
        ModelRegistry::default()
    };
    
    registry.save_to_file(&output).await?;
    info!("Exported models configuration to {}", output.display());
    
    Ok(())
}

async fn import_models(config: Config, input: PathBuf) -> Result<()> {
    let registry = ModelRegistry::load_from_file(&input).await?;
    
    if let Some(models_file) = &config.models.custom_models_file {
        registry.save_to_file(models_file).await?;
        info!("Imported models configuration from {} to {}", 
              input.display(), models_file.display());
    } else {
        return Err(crate::error::DattavaniError::configuration(
            "No custom models file configured for import"
        ));
    }
    
    Ok(())
}
