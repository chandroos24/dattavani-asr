/*!
Configuration module for Dattavani ASR

Handles loading and managing configuration from environment variables, files, and defaults.
*/

use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use crate::error::{DattavaniError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub google: GoogleConfig,
    pub whisper: WhisperConfig,
    pub models: ModelsConfig,
    pub processing: ProcessingConfig,
    pub logging: LoggingConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleConfig {
    pub application_credentials: Option<PathBuf>,
    pub project_id: Option<String>,
    pub drive_api_version: String,
    pub storage_api_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperConfig {
    pub model_size: String,
    pub model_path: Option<PathBuf>,
    pub device: String,
    pub compute_type: String,
    pub language: Option<String>,
    pub task: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    pub default_model: String,
    pub confidence_fallback_enabled: bool,
    pub min_confidence_threshold: f64,
    pub max_model_attempts: usize,
    pub custom_models_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    pub max_workers: usize,
    pub temp_dir: PathBuf,
    pub segment_duration: u64,
    pub target_sample_rate: u32,
    pub chunk_size: usize,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: PathBuf,
    pub max_file_size: u64,
    pub max_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub output_bucket: Option<String>,
    pub output_prefix: String,
    pub cache_dir: PathBuf,
    pub max_cache_size: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            google: GoogleConfig::default(),
            whisper: WhisperConfig::default(),
            models: ModelsConfig::default(),
            processing: ProcessingConfig::default(),
            logging: LoggingConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

impl Default for GoogleConfig {
    fn default() -> Self {
        Self {
            application_credentials: env::var("GOOGLE_APPLICATION_CREDENTIALS")
                .ok()
                .map(PathBuf::from),
            project_id: env::var("GOOGLE_CLOUD_PROJECT").ok(),
            drive_api_version: "v3".to_string(),
            storage_api_version: "v1".to_string(),
        }
    }
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            model_size: env::var("WHISPER_MODEL_SIZE")
                .unwrap_or_else(|_| "large-v3".to_string()),
            model_path: env::var("WHISPER_MODEL_PATH")
                .ok()
                .map(PathBuf::from),
            device: env::var("WHISPER_DEVICE")
                .unwrap_or_else(|_| "auto".to_string()),
            compute_type: env::var("WHISPER_COMPUTE_TYPE")
                .unwrap_or_else(|_| "float16".to_string()),
            language: env::var("WHISPER_LANGUAGE").ok(),
            task: env::var("WHISPER_TASK")
                .unwrap_or_else(|_| "transcribe".to_string()),
        }
    }
}

impl Default for ModelsConfig {
    fn default() -> Self {
        Self {
            default_model: env::var("DEFAULT_MODEL")
                .unwrap_or_else(|_| "whisper-kannada-small".to_string()),
            confidence_fallback_enabled: env::var("CONFIDENCE_FALLBACK_ENABLED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            min_confidence_threshold: env::var("MIN_CONFIDENCE_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.6),
            max_model_attempts: env::var("MAX_MODEL_ATTEMPTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            custom_models_file: env::var("CUSTOM_MODELS_FILE")
                .ok()
                .map(PathBuf::from),
        }
    }
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            max_workers: env::var("MAX_WORKERS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4),
            temp_dir: env::var("TEMP_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| std::env::temp_dir().join("dattavani_asr")),
            segment_duration: env::var("SEGMENT_DURATION")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300), // 5 minutes
            target_sample_rate: env::var("TARGET_SAMPLE_RATE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(16000),
            chunk_size: env::var("CHUNK_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8192),
            timeout_seconds: env::var("TIMEOUT_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600), // 1 hour
            retry_attempts: env::var("RETRY_ATTEMPTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
            file: env::var("LOG_FILE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("dattavani_asr.log")),
            max_file_size: env::var("LOG_MAX_FILE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10 * 1024 * 1024), // 10 MB
            max_files: env::var("LOG_MAX_FILES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(7),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            output_bucket: env::var("OUTPUT_BUCKET").ok(),
            output_prefix: env::var("OUTPUT_PREFIX")
                .unwrap_or_else(|_| "gen-transcript".to_string()),
            cache_dir: env::var("CACHE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| std::env::temp_dir().join("dattavani_cache")),
            max_cache_size: env::var("MAX_CACHE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1024 * 1024 * 1024), // 1 GB
        }
    }
}

impl Config {
    pub async fn load() -> Result<Self> {
        // Load from .env file if it exists
        if let Err(_) = dotenv::dotenv() {
            // .env file doesn't exist, that's okay
        }
        
        // Start with defaults
        let mut config = Config::default();
        
        // Try to load from config file
        if let Ok(config_path) = env::var("CONFIG_FILE") {
            config = Self::load_from_file(&config_path).await?;
        } else {
            // Try common config file locations
            let config_paths = [
                "dattavani-asr.toml",
                "config/dattavani-asr.toml",
                "~/.config/dattavani-asr/config.toml",
            ];
            
            for path in &config_paths {
                if let Ok(loaded_config) = Self::load_from_file(path).await {
                    config = loaded_config;
                    break;
                }
            }
        }
        
        // Validate configuration
        config.validate()?;
        
        Ok(config)
    }
    
    async fn load_from_file(path: &str) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| DattavaniError::configuration(format!("Failed to read config file {}: {}", path, e)))?;
        
        let config: Config = toml::from_str(&content)
            .map_err(|e| DattavaniError::configuration(format!("Failed to parse config file {}: {}", path, e)))?;
        
        Ok(config)
    }
    
    pub fn validate(&self) -> Result<()> {
        // Validate Google credentials
        if self.google.application_credentials.is_none() && self.google.project_id.is_none() {
            return Err(DattavaniError::configuration(
                "Either GOOGLE_APPLICATION_CREDENTIALS or GOOGLE_CLOUD_PROJECT must be set"
            ));
        }
        
        // Validate Whisper model size
        let valid_models = ["tiny", "base", "small", "medium", "large", "large-v2", "large-v3"];
        // Allow custom Hugging Face models (contain '/' character)
        if !valid_models.contains(&self.whisper.model_size.as_str()) && !self.whisper.model_size.contains('/') {
            return Err(DattavaniError::configuration(
                format!("Invalid Whisper model size: {}. Valid options: {:?} or Hugging Face model format (e.g., 'user/model-name')", 
                       self.whisper.model_size, valid_models)
            ));
        }
        
        // Validate processing configuration
        if self.processing.max_workers == 0 {
            return Err(DattavaniError::configuration(
                "max_workers must be greater than 0"
            ));
        }
        
        if self.processing.segment_duration == 0 {
            return Err(DattavaniError::configuration(
                "segment_duration must be greater than 0"
            ));
        }
        
        // Validate sample rate
        let valid_sample_rates = [8000, 16000, 22050, 44100, 48000];
        if !valid_sample_rates.contains(&self.processing.target_sample_rate) {
            return Err(DattavaniError::configuration(
                format!("Invalid target sample rate: {}. Valid options: {:?}", 
                       self.processing.target_sample_rate, valid_sample_rates)
            ));
        }
        
        Ok(())
    }
    
    pub async fn save_to_file(&self, path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| DattavaniError::configuration(format!("Failed to serialize config: {}", e)))?;
        
        tokio::fs::write(path, content).await
            .map_err(|e| DattavaniError::configuration(format!("Failed to write config file {}: {}", path, e)))?;
        
        Ok(())
    }
}
