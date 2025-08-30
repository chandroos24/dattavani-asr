/*!
Pluggable Model System for Dattavani ASR

Supports multiple model providers, confidence-based fallbacks, and language-specific models.
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command;
use tracing::{info, warn, debug};

use crate::config::Config;
use crate::error::{DattavaniError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub provider: ModelProvider,
    pub model_path: String,
    pub language: Option<String>,
    pub priority: u8, // Lower number = higher priority
    pub confidence_threshold: f64,
    pub fallback_models: Vec<String>,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelProvider {
    WhisperCLI,
    HuggingFace,
    OpenAI,
    Local,
    Native,  // New: Native Rust implementation using Candle
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistry {
    pub models: HashMap<String, ModelConfig>,
    pub language_mappings: HashMap<String, Vec<String>>, // language -> model_ids
    pub default_model: String,
    pub confidence_fallback_enabled: bool,
    pub min_confidence_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionAttempt {
    pub model_id: String,
    pub success: bool,
    pub confidence: Option<f64>,
    pub text: Option<String>,
    pub error: Option<String>,
    pub processing_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiModelResult {
    pub final_result: super::TranscriptionResult,
    pub attempts: Vec<TranscriptionAttempt>,
    pub selected_model: String,
    pub total_processing_time: f64,
}

pub struct ModelManager {
    registry: ModelRegistry,
    config: Config,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        let mut models = HashMap::new();
        let mut language_mappings = HashMap::new();
        
        // Default Kannada models
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
        
        // Default English models
        models.insert("whisper-large-v3".to_string(), ModelConfig {
            id: "whisper-large-v3".to_string(),
            provider: ModelProvider::WhisperCLI,
            model_path: "large-v3".to_string(),
            language: Some("en".to_string()),
            priority: 1,
            confidence_threshold: 0.8,
            fallback_models: vec!["whisper-base".to_string()],
            parameters: HashMap::new(),
        });
        
        models.insert("whisper-base".to_string(), ModelConfig {
            id: "whisper-base".to_string(),
            provider: ModelProvider::WhisperCLI,
            model_path: "base".to_string(),
            language: Some("en".to_string()),
            priority: 2,
            confidence_threshold: 0.7,
            fallback_models: vec!["whisper-small".to_string()],
            parameters: HashMap::new(),
        });
        
        // Language mappings
        language_mappings.insert("kn".to_string(), vec![
            "whisper-kannada-small".to_string(),
            "whisper-kannada-base".to_string(),
            "whisper-large-v3".to_string(),
        ]);
        
        language_mappings.insert("en".to_string(), vec![
            "whisper-large-v3".to_string(),
            "whisper-base".to_string(),
        ]);
        
        Self {
            models,
            language_mappings,
            default_model: "whisper-kannada-small".to_string(),
            confidence_fallback_enabled: true,
            min_confidence_threshold: 0.6,
        }
    }
}

impl ModelManager {
    pub fn new(config: Config) -> Self {
        let registry = ModelRegistry::default();
        Self { registry, config }
    }
    
    pub async fn new_with_config(config: Config) -> Result<Self> {
        let registry = if let Some(models_file) = &config.models.custom_models_file {
            ModelRegistry::load_from_file(models_file).await?
        } else {
            ModelRegistry::default()
        };
        
        Ok(Self { registry, config })
    }
    
    pub fn with_registry(config: Config, registry: ModelRegistry) -> Self {
        Self { registry, config }
    }
    
    pub async fn transcribe_with_best_model(
        &self,
        audio_path: &Path,
        language: Option<&str>,
        max_attempts: Option<usize>,
    ) -> Result<MultiModelResult> {
        let start_time = std::time::Instant::now();
        let max_attempts = max_attempts.unwrap_or(3);
        
        // Get candidate models for the language
        let candidate_models = self.get_candidate_models(language);
        
        if candidate_models.is_empty() {
            return Err(DattavaniError::asr_processing(
                "No suitable models found for the specified language"
            ));
        }
        
        let mut attempts = Vec::new();
        let mut best_result: Option<super::TranscriptionResult> = None;
        let mut selected_model = String::new();
        
        for (attempt_num, model_id) in candidate_models.iter().enumerate() {
            if attempt_num >= max_attempts {
                break;
            }
            
            let model_config = self.registry.models.get(model_id)
                .ok_or_else(|| DattavaniError::asr_processing(
                    format!("Model configuration not found: {}", model_id)
                ))?;
            
            info!("Attempting transcription with model: {} (attempt {}/{})", 
                  model_id, attempt_num + 1, max_attempts);
            
            let attempt_start = std::time::Instant::now();
            let result = self.transcribe_with_model(audio_path, model_config, language).await;
            let processing_time = attempt_start.elapsed().as_secs_f64();
            
            match result {
                Ok(transcription_result) => {
                    let confidence = transcription_result.confidence.unwrap_or(0.0);
                    
                    attempts.push(TranscriptionAttempt {
                        model_id: model_id.clone(),
                        success: true,
                        confidence: Some(confidence),
                        text: transcription_result.text.clone(),
                        error: None,
                        processing_time,
                    });
                    
                    // Check if this result meets the confidence threshold
                    if confidence >= model_config.confidence_threshold {
                        info!("Model {} achieved sufficient confidence: {:.3}", 
                              model_id, confidence);
                        best_result = Some(transcription_result);
                        selected_model = model_id.clone();
                        break;
                    } else if best_result.is_none() || 
                              confidence > best_result.as_ref().unwrap().confidence.unwrap_or(0.0) {
                        // Keep track of the best result so far
                        best_result = Some(transcription_result);
                        selected_model = model_id.clone();
                    }
                    
                    warn!("Model {} confidence {:.3} below threshold {:.3}, trying fallback", 
                          model_id, confidence, model_config.confidence_threshold);
                }
                Err(e) => {
                    warn!("Model {} failed: {}", model_id, e);
                    attempts.push(TranscriptionAttempt {
                        model_id: model_id.clone(),
                        success: false,
                        confidence: None,
                        text: None,
                        error: Some(e.to_string()),
                        processing_time,
                    });
                }
            }
            
            // If confidence fallback is disabled, stop after first successful attempt
            if !self.registry.confidence_fallback_enabled && best_result.is_some() {
                break;
            }
        }
        
        let final_result = best_result.ok_or_else(|| 
            DattavaniError::asr_processing("All model attempts failed")
        )?;
        
        let total_processing_time = start_time.elapsed().as_secs_f64();
        
        info!("Multi-model transcription completed. Selected model: {}, Final confidence: {:.3}", 
              selected_model, final_result.confidence.unwrap_or(0.0));
        
        Ok(MultiModelResult {
            final_result,
            attempts,
            selected_model,
            total_processing_time,
        })
    }
    
    pub async fn transcribe_with_model(
        &self,
        audio_path: &Path,
        model_config: &ModelConfig,
        language: Option<&str>,
    ) -> Result<super::TranscriptionResult> {
        match &model_config.provider {
            ModelProvider::WhisperCLI => {
                self.transcribe_with_whisper_cli(audio_path, model_config, language).await
            }
            ModelProvider::HuggingFace => {
                self.transcribe_with_huggingface(audio_path, model_config, language).await
            }
            ModelProvider::Native => {
                self.transcribe_with_native(audio_path, model_config, language).await
            }
            _ => {
                Err(DattavaniError::asr_processing(
                    format!("Provider {:?} not implemented yet", model_config.provider)
                ))
            }
        }
    }
    
    async fn transcribe_with_native(
        &self,
        audio_path: &Path,
        model_config: &ModelConfig,
        language: Option<&str>,
    ) -> Result<super::TranscriptionResult> {
        info!("Native Rust Whisper implementation not available, falling back to CLI");
        
        // Fallback to CLI implementation
        self.transcribe_with_whisper_cli(audio_path, model_config, language).await
    }
    
    async fn transcribe_with_whisper_cli(
        &self,
        audio_path: &Path,
        model_config: &ModelConfig,
        language: Option<&str>,
    ) -> Result<super::TranscriptionResult> {
        let mut cmd = Command::new("whisper");
        cmd.arg(audio_path.to_str().unwrap());
        cmd.args(&["--model", &model_config.model_path]);
        cmd.args(&["--output_format", "json"]);
        cmd.args(&["--output_dir", "/tmp"]);
        
        // Add device parameter for GPU acceleration
        if &self.config.whisper.device != "cpu" {
            cmd.args(&["--device", &self.config.whisper.device]);
        }
        
        // Note: compute_type is not supported by OpenAI Whisper CLI, only by faster-whisper
        // cmd.args(&["--compute_type", &self.config.whisper.compute_type]);
        
        // Use language from model config or parameter
        let target_language = language.or(model_config.language.as_deref());
        if let Some(lang) = target_language {
            cmd.args(&["--language", lang]);
        }
        
        debug!("Executing whisper command: {:?}", cmd);
        
        let output = cmd.output().await
            .map_err(|e| DattavaniError::whisper_model(format!("Whisper CLI failed: {}", e)))?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Ok(super::TranscriptionResult {
                success: false,
                text: None,
                error: Some(format!("Whisper error: {}", error_msg)),
                processing_time: None,
                confidence: None,
                language: None,
                segments: None,
                file_path: Some(audio_path.to_string_lossy().to_string()),
            });
        }
        
        self.parse_whisper_output(audio_path, target_language).await
    }
    
    async fn transcribe_with_huggingface(
        &self,
        audio_path: &Path,
        model_config: &ModelConfig,
        language: Option<&str>,
    ) -> Result<super::TranscriptionResult> {
        // For now, fall back to whisper CLI with the model path
        // In a full implementation, this would use Python + transformers
        warn!("HuggingFace provider not fully implemented, falling back to Whisper CLI");
        
        let mut cmd = Command::new("whisper");
        cmd.arg(audio_path.to_str().unwrap());
        cmd.args(&["--model", "base"]); // Use base model as fallback
        cmd.args(&["--output_format", "json"]);
        cmd.args(&["--output_dir", "/tmp"]);
        
        // Add device parameter for GPU acceleration
        if &self.config.whisper.device != "cpu" {
            cmd.args(&["--device", &self.config.whisper.device]);
        }
        
        // Note: compute_type is not supported by OpenAI Whisper CLI, only by faster-whisper
        // cmd.args(&["--compute_type", &self.config.whisper.compute_type]);
        
        let target_language = language.or(model_config.language.as_deref());
        if let Some(lang) = target_language {
            cmd.args(&["--language", lang]);
        }
        
        let output = cmd.output().await
            .map_err(|e| DattavaniError::whisper_model(format!("Whisper CLI failed: {}", e)))?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Ok(super::TranscriptionResult {
                success: false,
                text: None,
                error: Some(format!("HuggingFace fallback error: {}", error_msg)),
                processing_time: None,
                confidence: None,
                language: None,
                segments: None,
                file_path: Some(audio_path.to_string_lossy().to_string()),
            });
        }
        
        self.parse_whisper_output(audio_path, target_language).await
    }
    
    async fn parse_whisper_output(
        &self,
        audio_path: &Path,
        language: Option<&str>,
    ) -> Result<super::TranscriptionResult> {
        let json_file = audio_path.with_extension("json");
        if tokio::fs::metadata(&json_file).await.is_ok() {
            let json_content = tokio::fs::read_to_string(&json_file).await
                .map_err(DattavaniError::FileIo)?;
            
            let whisper_result: serde_json::Value = serde_json::from_str(&json_content)
                .map_err(DattavaniError::Serialization)?;
            
            let text = whisper_result["text"].as_str()
                .unwrap_or("")
                .to_string();
            
            let detected_language = whisper_result["language"].as_str()
                .map(|s| s.to_string());
            
            let segments = if let Some(segments_array) = whisper_result["segments"].as_array() {
                let mut segments = Vec::new();
                for segment in segments_array {
                    segments.push(super::TranscriptionSegment {
                        start: segment["start"].as_f64().unwrap_or(0.0),
                        end: segment["end"].as_f64().unwrap_or(0.0),
                        text: segment["text"].as_str().unwrap_or("").to_string(),
                        confidence: segment["confidence"].as_f64(),
                    });
                }
                Some(segments)
            } else {
                None
            };
            
            // Calculate average confidence
            let confidence = segments.as_ref()
                .and_then(|segs| {
                    let confidences: Vec<f64> = segs.iter()
                        .filter_map(|s| s.confidence)
                        .collect();
                    if !confidences.is_empty() {
                        Some(confidences.iter().sum::<f64>() / confidences.len() as f64)
                    } else {
                        None
                    }
                });
            
            // Clean up JSON file
            let _ = tokio::fs::remove_file(&json_file).await;
            
            Ok(super::TranscriptionResult {
                success: true,
                text: Some(text),
                error: None,
                processing_time: None,
                confidence,
                language: detected_language.or_else(|| language.map(|s| s.to_string())),
                segments,
                file_path: Some(audio_path.to_string_lossy().to_string()),
            })
        } else {
            Err(DattavaniError::asr_processing(
                "Whisper output JSON file not found"
            ))
        }
    }
    
    fn get_candidate_models(&self, language: Option<&str>) -> Vec<String> {
        let mut candidates = Vec::new();
        
        if let Some(lang) = language {
            // Get language-specific models first
            if let Some(lang_models) = self.registry.language_mappings.get(lang) {
                candidates.extend(lang_models.clone());
            }
        }
        
        // Add default model if not already included
        if !candidates.contains(&self.registry.default_model) {
            candidates.push(self.registry.default_model.clone());
        }
        
        // Sort by priority (lower number = higher priority)
        candidates.sort_by(|a, b| {
            let priority_a = self.registry.models.get(a).map(|m| m.priority).unwrap_or(255);
            let priority_b = self.registry.models.get(b).map(|m| m.priority).unwrap_or(255);
            priority_a.cmp(&priority_b)
        });
        
        // Remove duplicates while preserving order
        let mut unique_candidates = Vec::new();
        for candidate in candidates {
            if !unique_candidates.contains(&candidate) {
                unique_candidates.push(candidate);
            }
        }
        
        unique_candidates
    }
    
    pub fn get_model_config(&self, model_id: &str) -> Option<&ModelConfig> {
        self.registry.models.get(model_id)
    }
    
    pub fn list_models(&self) -> Vec<&ModelConfig> {
        self.registry.models.values().collect()
    }
    
    pub fn list_models_for_language(&self, language: &str) -> Vec<&ModelConfig> {
        if let Some(model_ids) = self.registry.language_mappings.get(language) {
            model_ids.iter()
                .filter_map(|id| self.registry.models.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl ModelRegistry {
    pub async fn load_from_file(path: &Path) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| DattavaniError::configuration(format!("Failed to read models file {}: {}", path.display(), e)))?;
        
        let config: ModelsFileConfig = toml::from_str(&content)
            .map_err(|e| DattavaniError::configuration(format!("Failed to parse models file {}: {}", path.display(), e)))?;
        
        let mut registry = Self {
            models: HashMap::new(),
            language_mappings: config.language_mappings.unwrap_or_default(),
            default_model: config.registry.default_model,
            confidence_fallback_enabled: config.registry.confidence_fallback_enabled,
            min_confidence_threshold: config.registry.min_confidence_threshold,
        };
        
        // Add models from config
        for model_config in config.models {
            registry.models.insert(model_config.id.clone(), model_config);
        }
        
        Ok(registry)
    }
    
    pub async fn save_to_file(&self, path: &Path) -> Result<()> {
        let models: Vec<ModelConfig> = self.models.values().cloned().collect();
        
        let config = ModelsFileConfig {
            registry: RegistryConfig {
                default_model: self.default_model.clone(),
                confidence_fallback_enabled: self.confidence_fallback_enabled,
                min_confidence_threshold: self.min_confidence_threshold,
            },
            models,
            language_mappings: Some(self.language_mappings.clone()),
        };
        
        let content = toml::to_string_pretty(&config)
            .map_err(|e| DattavaniError::configuration(format!("Failed to serialize models config: {}", e)))?;
        
        tokio::fs::write(path, content).await
            .map_err(|e| DattavaniError::configuration(format!("Failed to write models file {}: {}", path.display(), e)))?;
        
        Ok(())
    }
    
    pub fn list_models(&self) -> Vec<&ModelConfig> {
        self.models.values().collect()
    }
    
    pub fn list_models_for_language(&self, language: &str) -> Vec<&ModelConfig> {
        if let Some(model_ids) = self.language_mappings.get(language) {
            model_ids.iter()
                .filter_map(|id| self.models.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelsFileConfig {
    registry: RegistryConfig,
    models: Vec<ModelConfig>,
    language_mappings: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryConfig {
    default_model: String,
    confidence_fallback_enabled: bool,
    min_confidence_threshold: f64,
}
