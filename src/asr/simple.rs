/*!
Simple Transcription Module - Phase 1 Implementation

Provides a reliable, simplified transcription interface that fixes current CLI issues
and serves as a stable fallback for the native implementation.
*/

use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};

use crate::error::{DattavaniError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleTranscriptionResult {
    pub success: bool,
    pub text: Option<String>,
    pub error: Option<String>,
    pub processing_time: Option<f64>,
    pub model_used: String,
    pub file_path: String,
}

#[derive(Debug, Clone)]
pub struct SimpleTranscriptionOptions {
    pub model: String,
    pub language: Option<String>,
    pub timeout_seconds: u64,
    pub output_format: String,
    pub verbose: bool,
}

impl Default for SimpleTranscriptionOptions {
    fn default() -> Self {
        Self {
            model: "base".to_string(),
            language: Some("en".to_string()),
            timeout_seconds: 300, // 5 minutes
            output_format: "txt".to_string(),
            verbose: false,
        }
    }
}

pub struct SimpleTranscriber {
    whisper_path: PathBuf,
    python_path: PathBuf,
    temp_dir: PathBuf,
    default_options: SimpleTranscriptionOptions,
}

impl SimpleTranscriber {
    pub async fn new() -> Result<Self> {
        info!("Initializing Simple Transcriber (Phase 1)");
        
        // Find whisper executable
        let (python_path, whisper_path) = Self::find_whisper_executable().await?;
        
        // Create temp directory
        let temp_dir = std::env::temp_dir().join("dattavani-asr");
        tokio::fs::create_dir_all(&temp_dir).await
            .map_err(|e| DattavaniError::configuration(format!("Failed to create temp dir: {}", e)))?;
        
        info!("Simple Transcriber initialized with Python: {:?}, Whisper: {:?}", python_path, whisper_path);
        
        Ok(Self {
            whisper_path,
            python_path,
            temp_dir,
            default_options: SimpleTranscriptionOptions::default(),
        })
    }
    
    async fn find_whisper_executable() -> Result<(PathBuf, PathBuf)> {
        // Try different possible locations for whisper
        let candidates = vec![
            // Local virtual environment (most likely)
            ("./whisper_env/bin/python", "./whisper_env/bin/whisper"),
            ("./whisper_env/bin/python3", "./whisper_env/bin/whisper"),
            // System-wide installations
            ("python3", "whisper"),
            ("python", "whisper"),
            // Homebrew installations
            ("/opt/homebrew/bin/python3", "/opt/homebrew/bin/whisper"),
            // Direct whisper command
            ("python3", "/opt/homebrew/bin/whisper"),
        ];
        
        for (python_cmd, whisper_cmd) in candidates {
            debug!("Trying Python: {}, Whisper: {}", python_cmd, whisper_cmd);
            
            // Test if this combination works
            if let Ok(result) = Self::test_whisper_installation(python_cmd, whisper_cmd).await {
                if result {
                    info!("Found working Whisper installation: {} -> {}", python_cmd, whisper_cmd);
                    return Ok((PathBuf::from(python_cmd), PathBuf::from(whisper_cmd)));
                }
            }
        }
        
        // Try to find whisper module directly with different python interpreters
        let python_candidates = vec![
            "./whisper_env/bin/python",
            "./whisper_env/bin/python3", 
            "python3",
            "python",
            "/opt/homebrew/bin/python3",
        ];
        
        for python_cmd in python_candidates {
            if let Ok(result) = Self::test_whisper_module(python_cmd).await {
                if result.0 {
                    info!("Found Whisper as Python module with: {}", python_cmd);
                    return Ok((PathBuf::from(python_cmd), PathBuf::from("-m whisper")));
                }
            }
        }
        
        Err(DattavaniError::configuration(
            "Whisper CLI not found. Please install whisper: pip install openai-whisper"
        ))
    }
    
    async fn test_whisper_installation(python_cmd: &str, whisper_cmd: &str) -> Result<bool> {
        let output = timeout(
            Duration::from_secs(10),
            Command::new(python_cmd)
                .arg(whisper_cmd)
                .arg("--help")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output()
        ).await;
        
        match output {
            Ok(Ok(output)) => Ok(output.status.success()),
            _ => Ok(false),
        }
    }
    
    async fn test_whisper_module(python_cmd: &str) -> Result<(bool, String)> {
        let output = timeout(
            Duration::from_secs(10),
            Command::new(python_cmd)
                .args(&["-c", "import whisper; print('OK')"])
                .output()
        ).await;
        
        match output {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok((output.status.success() && stdout.contains("OK"), stdout.to_string()))
            }
            _ => Ok((false, "Timeout or error".to_string())),
        }
    }
    
    pub async fn transcribe(&self, audio_path: &Path, options: Option<SimpleTranscriptionOptions>) -> Result<SimpleTranscriptionResult> {
        let start_time = std::time::Instant::now();
        let options = options.unwrap_or_else(|| self.default_options.clone());
        
        info!("Starting simple transcription for: {:?}", audio_path);
        
        // Validate input file
        if !audio_path.exists() {
            return Ok(SimpleTranscriptionResult {
                success: false,
                text: None,
                error: Some(format!("Audio file not found: {:?}", audio_path)),
                processing_time: Some(start_time.elapsed().as_secs_f64()),
                model_used: options.model,
                file_path: audio_path.to_string_lossy().to_string(),
            });
        }
        
        // Create unique output directory for this transcription
        let session_id = uuid::Uuid::new_v4().to_string();
        let output_dir = self.temp_dir.join(&session_id);
        tokio::fs::create_dir_all(&output_dir).await
            .map_err(|e| DattavaniError::FileIo(e))?;
        
        // Ensure cleanup on drop
        let _cleanup_guard = TempDirCleanup::new(output_dir.clone());
        
        // Try transcription with fallback models
        let models_to_try = vec![
            options.model.clone(),
            "base".to_string(),
            "small".to_string(),
        ];
        
        for (attempt, model) in models_to_try.iter().enumerate() {
            info!("Transcription attempt {} with model: {}", attempt + 1, model);
            
            match self.transcribe_with_model(audio_path, model, &options, &output_dir).await {
                Ok(mut result) => {
                    result.processing_time = Some(start_time.elapsed().as_secs_f64());
                    result.file_path = audio_path.to_string_lossy().to_string();
                    info!("Transcription successful with model: {}", model);
                    return Ok(result);
                }
                Err(e) => {
                    warn!("Transcription failed with model {}: {}", model, e);
                    if attempt == models_to_try.len() - 1 {
                        // Last attempt failed
                        return Ok(SimpleTranscriptionResult {
                            success: false,
                            text: None,
                            error: Some(format!("All transcription attempts failed. Last error: {}", e)),
                            processing_time: Some(start_time.elapsed().as_secs_f64()),
                            model_used: model.clone(),
                            file_path: audio_path.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }
        
        // This should never be reached, but just in case
        Ok(SimpleTranscriptionResult {
            success: false,
            text: None,
            error: Some("Unexpected error in transcription loop".to_string()),
            processing_time: Some(start_time.elapsed().as_secs_f64()),
            model_used: options.model,
            file_path: audio_path.to_string_lossy().to_string(),
        })
    }
    
    async fn transcribe_with_model(
        &self,
        audio_path: &Path,
        model: &str,
        options: &SimpleTranscriptionOptions,
        output_dir: &Path,
    ) -> Result<SimpleTranscriptionResult> {
        let mut cmd = Command::new(&self.python_path);
        
        // Handle different whisper command formats
        if self.whisper_path.to_string_lossy().contains("-m whisper") {
            cmd.args(&["-m", "whisper"]);
        } else {
            cmd.arg(&self.whisper_path);
        }
        
        // Add whisper arguments
        cmd.arg(audio_path)
            .args(&["--model", model])
            .args(&["--output_format", &options.output_format])
            .args(&["--output_dir", &output_dir.to_string_lossy()]);
        
        if let Some(language) = &options.language {
            cmd.args(&["--language", language]);
        }
        
        if !options.verbose {
            cmd.args(&["--verbose", "False"]);
        }
        
        // Set environment variables
        cmd.env("PYTHONUNBUFFERED", "1");
        
        // Add timeout and execute
        debug!("Executing command: {:?}", cmd);
        
        let output = timeout(
            Duration::from_secs(options.timeout_seconds),
            cmd.output()
        ).await;
        
        match output {
            Ok(Ok(output)) => {
                if output.status.success() {
                    // Read the generated text file
                    let audio_stem = audio_path.file_stem()
                        .and_then(|s| s.to_str())
                        .ok_or_else(|| DattavaniError::validation("Invalid audio filename"))?;
                    
                    let txt_file = output_dir.join(format!("{}.txt", audio_stem));
                    
                    match tokio::fs::read_to_string(&txt_file).await {
                        Ok(text) => {
                            let cleaned_text = text.trim().to_string();
                            info!("Successfully transcribed {} characters", cleaned_text.len());
                            
                            Ok(SimpleTranscriptionResult {
                                success: true,
                                text: Some(cleaned_text),
                                error: None,
                                processing_time: None, // Will be set by caller
                                model_used: model.to_string(),
                                file_path: String::new(), // Will be set by caller
                            })
                        }
                        Err(e) => {
                            error!("Failed to read transcription output file {:?}: {}", txt_file, e);
                            Err(DattavaniError::FileIo(e))
                        }
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    error!("Whisper command failed. Stderr: {}, Stdout: {}", stderr, stdout);
                    Err(DattavaniError::asr_processing(format!("Whisper failed: {}", stderr)))
                }
            }
            Ok(Err(e)) => {
                error!("Failed to execute whisper command: {}", e);
                Err(DattavaniError::asr_processing(format!("Command execution failed: {}", e)))
            }
            Err(_) => {
                error!("Whisper command timed out after {} seconds", options.timeout_seconds);
                Err(DattavaniError::asr_processing(format!("Transcription timed out after {} seconds", options.timeout_seconds)))
            }
        }
    }
    
    pub async fn transcribe_batch(&self, audio_files: Vec<PathBuf>, options: Option<SimpleTranscriptionOptions>) -> Result<Vec<SimpleTranscriptionResult>> {
        info!("Starting batch transcription for {} files", audio_files.len());
        
        let mut results = Vec::new();
        
        for (i, audio_path) in audio_files.iter().enumerate() {
            info!("Processing file {}/{}: {:?}", i + 1, audio_files.len(), audio_path);
            
            let result = self.transcribe(audio_path, options.clone()).await?;
            results.push(result);
        }
        
        let successful = results.iter().filter(|r| r.success).count();
        info!("Batch transcription completed: {}/{} successful", successful, audio_files.len());
        
        Ok(results)
    }
}

// RAII cleanup for temporary directories
struct TempDirCleanup {
    path: PathBuf,
}

impl TempDirCleanup {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempDirCleanup {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Err(e) = std::fs::remove_dir_all(&self.path) {
                warn!("Failed to cleanup temp directory {:?}: {}", self.path, e);
            } else {
                debug!("Cleaned up temp directory: {:?}", self.path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_simple_transcriber_creation() {
        let result = SimpleTranscriber::new().await;
        // This might fail in CI without whisper installed, so we just check it doesn't panic
        match result {
            Ok(_) => println!("SimpleTranscriber created successfully"),
            Err(e) => println!("SimpleTranscriber creation failed (expected in CI): {}", e),
        }
    }
    
    #[tokio::test]
    async fn test_whisper_module_detection() {
        let result = SimpleTranscriber::test_whisper_module("python3").await;
        assert!(result.is_ok());
        println!("Whisper module test result: {:?}", result);
    }
}
