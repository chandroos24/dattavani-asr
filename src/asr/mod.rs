/*!
ASR (Automatic Speech Recognition) Module for Dattavani ASR

Integrates with Whisper model for high-accuracy speech recognition.
Phase 1: Uses simplified, reliable CLI interface
Phase 2+: Will use native Rust implementation with Candle framework
*/

pub mod models;
pub mod simple;  // Phase 1: Simple transcription without complex model management
pub mod native;  // Phase 2: Native Rust implementation with Candle framework

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;
use futures::future::join_all;
use tracing::{info, warn, error};

use crate::config::Config;
use crate::error::{DattavaniError, Result};
use crate::streaming::StreamingProcessor;
use crate::video::VideoProcessor;
use crate::gdrive::GDriveClient;

pub use models::{ModelManager, ModelConfig, ModelProvider, ModelRegistry, MultiModelResult};
pub use simple::{SimpleTranscriber, SimpleTranscriptionResult, SimpleTranscriptionOptions};
pub use native::{NativeTranscriber, NativeWhisperModel, NativeTranscriptionResult, NativeTranscriptionOptions};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub success: bool,
    pub text: Option<String>,
    pub error: Option<String>,
    pub processing_time: Option<f64>,
    pub confidence: Option<f64>,
    pub language: Option<String>,
    pub segments: Option<Vec<TranscriptionSegment>>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub total_files: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<TranscriptionResult>,
    pub total_processing_time: f64,
}

pub struct DattavaniAsr {
    config: Config,
    model_manager: ModelManager,
    streaming_processor: StreamingProcessor,
    video_processor: VideoProcessor,
    gdrive_client: Option<GDriveClient>,
}

// Remove the old WhisperModel struct and implementation
// It's now replaced by the pluggable ModelManager system

impl DattavaniAsr {
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing Dattavani ASR with pluggable model system");
        
        // Initialize model manager with config-based model loading
        let model_manager = ModelManager::new_with_config(config.clone()).await?;
        
        // Initialize processors
        let streaming_processor = StreamingProcessor::new(config.clone())?
            .with_gdrive().await?;
        let video_processor = VideoProcessor::new(config.clone());
        
        // Initialize Google Drive client if credentials are available
        let gdrive_client = if config.google.application_credentials.is_some() {
            Some(GDriveClient::new(config.clone()).await?)
        } else {
            None
        };
        
        Ok(Self {
            config,
            model_manager,
            streaming_processor,
            video_processor,
            gdrive_client,
        })
    }
    
    pub async fn new_with_custom_models(config: Config, model_registry: ModelRegistry) -> Result<Self> {
        info!("Initializing Dattavani ASR with custom model registry");
        
        // Initialize model manager with custom registry
        let model_manager = ModelManager::with_registry(config.clone(), model_registry);
        
        // Initialize processors
        let streaming_processor = StreamingProcessor::new(config.clone())?
            .with_gdrive().await?;
        let video_processor = VideoProcessor::new(config.clone());
        
        // Initialize Google Drive client if credentials are available
        let gdrive_client = if config.google.application_credentials.is_some() {
            Some(GDriveClient::new(config.clone()).await?)
        } else {
            None
        };
        
        Ok(Self {
            config,
            model_manager,
            streaming_processor,
            video_processor,
            gdrive_client,
        })
    }
    
    pub async fn stream_process_single_file(
        &self,
        input_uri: &str,
        output_uri: Option<&str>,
        language: Option<&str>,
        _segment_duration: Option<u64>,
    ) -> Result<TranscriptionResult> {
        let start_time = std::time::Instant::now();
        
        info!("Processing single file: {}", input_uri);
        
        // Extract audio using streaming processor
        let streaming_result = self.streaming_processor
            .stream_extract_audio(input_uri, None).await?;
        
        if !streaming_result.success {
            return Ok(TranscriptionResult {
                success: false,
                text: None,
                error: streaming_result.error,
                processing_time: Some(start_time.elapsed().as_secs_f64()),
                confidence: None,
                language: None,
                segments: None,
                file_path: Some(input_uri.to_string()),
            });
        }
        
        let audio_path = streaming_result.audio_path
            .ok_or_else(|| DattavaniError::asr_processing("No audio path in streaming result"))?;
        
        // Transcribe the audio using the best available model
        let multi_model_result = self.model_manager
            .transcribe_with_best_model(&audio_path, language, Some(3)).await?;
        
        let mut transcription_result = multi_model_result.final_result;
        
        // Save transcript if output URI is provided
        if let Some(output) = output_uri {
            if let Some(text) = &transcription_result.text {
                self.save_transcript(text, output).await?;
            }
        } else {
            // Auto-generate output path
            let output_path = self.generate_output_path(input_uri)?;
            if let Some(text) = &transcription_result.text {
                self.save_transcript(text, &output_path).await?;
            }
        }
        
        // Clean up temporary audio file
        let _ = tokio::fs::remove_file(&audio_path).await;
        
        transcription_result.processing_time = Some(start_time.elapsed().as_secs_f64());
        transcription_result.file_path = Some(input_uri.to_string());
        
        Ok(transcription_result)
    }
    
    pub async fn stream_process_batch(
        &self,
        folder_uri: &str,
        output_uri: Option<&str>,
        language: Option<&str>,
        max_workers: usize,
        pattern: Option<&str>,
    ) -> Result<BatchResult> {
        let start_time = std::time::Instant::now();
        
        info!("Starting batch processing for folder: {}", folder_uri);
        
        // Get list of files to process
        let files = self.list_files_in_folder(folder_uri, pattern).await?;
        
        if files.is_empty() {
            warn!("No files found in folder: {}", folder_uri);
            return Ok(BatchResult {
                total_files: 0,
                successful: 0,
                failed: 0,
                results: Vec::new(),
                total_processing_time: start_time.elapsed().as_secs_f64(),
            });
        }
        
        info!("Found {} files to process", files.len());
        
        // Create semaphore to limit concurrent processing
        let semaphore = Arc::new(Semaphore::new(max_workers));
        
        // Process files concurrently
        let mut handles = Vec::new();
        for file_uri in files {
            let semaphore = semaphore.clone();
            let _language_owned = language.map(|s| s.to_string());
            let _output_uri_owned = output_uri.map(|s| s.to_string());
            
            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                // For now, return a placeholder result
                // In a real implementation, we'd need to restructure to avoid self reference
                Ok(TranscriptionResult {
                    success: false,
                    text: None,
                    error: Some("Batch processing not fully implemented".to_string()),
                    processing_time: None,
                    confidence: None,
                    language: None,
                    segments: None,
                    file_path: Some(file_uri),
                })
            });
            handles.push(handle);
        }
        
        let results: Vec<Result<TranscriptionResult>> = join_all(handles).await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        
        // Collect results
        let mut transcription_results = Vec::new();
        let mut successful = 0;
        let mut failed = 0;
        
        for result in results {
            match result {
                Ok(transcription_result) => {
                    if transcription_result.success {
                        successful += 1;
                    } else {
                        failed += 1;
                    }
                    transcription_results.push(transcription_result);
                }
                Err(e) => {
                    failed += 1;
                    transcription_results.push(TranscriptionResult {
                        success: false,
                        text: None,
                        error: Some(e.to_string()),
                        processing_time: None,
                        confidence: None,
                        language: None,
                        segments: None,
                        file_path: None,
                    });
                }
            }
        }
        
        let total_processing_time = start_time.elapsed().as_secs_f64();
        
        info!("Batch processing completed: {}/{} successful", successful, transcription_results.len());
        
        Ok(BatchResult {
            total_files: transcription_results.len(),
            successful,
            failed,
            results: transcription_results,
            total_processing_time,
        })
    }
    
    async fn list_files_in_folder(&self, folder_uri: &str, pattern: Option<&str>) -> Result<Vec<String>> {
        if GDriveClient::is_google_drive_url(folder_uri) {
            // Google Drive folder
            let gdrive = self.gdrive_client.as_ref()
                .ok_or_else(|| DattavaniError::google_drive("Google Drive client not initialized"))?;
            
            let folder_id = GDriveClient::extract_file_id_from_url(folder_uri)?;
            let files = gdrive.list_files_in_folder(&folder_id, pattern).await?;
            
            Ok(files.into_iter()
                .filter(|f| VideoProcessor::is_supported_format(&f.name))
                .map(|f| format!("https://drive.google.com/file/d/{}/view", f.id))
                .collect())
        } else if folder_uri.starts_with("gs://") {
            // Google Cloud Storage
            self.list_gcs_files(folder_uri, pattern).await
        } else {
            // Local folder
            self.list_local_files(folder_uri, pattern).await
        }
    }
    
    async fn list_gcs_files(&self, _bucket_uri: &str, _pattern: Option<&str>) -> Result<Vec<String>> {
        // For now, return an error indicating GCS support is not implemented
        // In a full implementation, this would use the Google Cloud Storage API
        Err(DattavaniError::configuration(
            "Google Cloud Storage support not implemented in this version. Use local files or Google Drive."
        ))
    }
    
    async fn list_local_files(&self, folder_path: &str, pattern: Option<&str>) -> Result<Vec<String>> {
        use walkdir::WalkDir;
        
        let mut files = Vec::new();
        
        for entry in WalkDir::new(folder_path) {
            let entry = entry.map_err(|e| DattavaniError::unknown(e.to_string()))?;
            
            if entry.file_type().is_file() {
                let path = entry.path();
                if VideoProcessor::is_supported_format(path.to_str().unwrap_or("")) {
                    if let Some(pattern) = pattern {
                        if path.file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| n.contains(&pattern.replace("*", "")))
                            .unwrap_or(false) {
                            files.push(path.to_string_lossy().to_string());
                        }
                    } else {
                        files.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
        
        Ok(files)
    }
    
    async fn save_transcript(&self, text: &str, output_uri: &str) -> Result<()> {
        if output_uri.starts_with("gs://") {
            self.save_to_gcs(text, output_uri).await
        } else if GDriveClient::is_google_drive_url(output_uri) {
            self.save_to_gdrive(text, output_uri).await
        } else {
            self.save_to_local_file(text, output_uri).await
        }
    }
    
    async fn save_to_gcs(&self, _text: &str, _gcs_uri: &str) -> Result<()> {
        // For now, return an error indicating GCS support is not implemented
        Err(DattavaniError::configuration(
            "Google Cloud Storage upload not implemented in this version. Use local files."
        ))
    }
    
    async fn save_to_gdrive(&self, text: &str, _gdrive_uri: &str) -> Result<()> {
        let gdrive = self.gdrive_client.as_ref()
            .ok_or_else(|| DattavaniError::google_drive("Google Drive client not initialized"))?;
        
        // Create transcript in gen-transcript folder
        let transcript_folder_id = self.ensure_transcript_folder().await?;
        
        let filename = format!("transcript_{}.txt", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        gdrive.upload_file(&filename, text.as_bytes(), Some(&transcript_folder_id), Some("text/plain")).await?;
        
        Ok(())
    }
    
    async fn save_to_local_file(&self, text: &str, file_path: &str) -> Result<()> {
        let path = Path::new(file_path);
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(DattavaniError::FileIo)?;
        }
        
        tokio::fs::write(path, text).await
            .map_err(DattavaniError::FileIo)?;
        
        Ok(())
    }
    
    async fn ensure_transcript_folder(&self) -> Result<String> {
        let gdrive = self.gdrive_client.as_ref()
            .ok_or_else(|| DattavaniError::google_drive("Google Drive client not initialized"))?;
        
        // For now, create in root. In a real implementation, you might want to
        // search for existing gen-transcript folder first
        gdrive.create_folder("gen-transcript", None).await
    }
    
    fn generate_output_path(&self, input_uri: &str) -> Result<String> {
        let input_path = Path::new(input_uri);
        let filename = input_path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| DattavaniError::validation("Invalid input filename"))?;
        
        let output_filename = format!("{}_transcript.txt", filename);
        
        if input_uri.starts_with("gs://") {
            // Generate GCS path
            let bucket_part = input_uri.trim_start_matches("gs://");
            let bucket_name = bucket_part.split('/').next().unwrap_or("");
            Ok(format!("gs://{}/{}/{}", bucket_name, self.config.storage.output_prefix, output_filename))
        } else {
            // Generate local path
            let output_dir = input_path.parent().unwrap_or(Path::new("."));
            let transcript_dir = output_dir.join(&self.config.storage.output_prefix);
            Ok(transcript_dir.join(output_filename).to_string_lossy().to_string())
        }
    }
}
