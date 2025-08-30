/*!
Native Whisper Implementation using Candle Framework

This module provides a pure Rust implementation of Whisper using the Candle ML framework,
eliminating Python dependencies and providing significant performance improvements.
*/

#[cfg(feature = "native")]
pub mod model;
#[cfg(feature = "native")]
pub mod audio;
#[cfg(feature = "native")]
pub mod device;
#[cfg(feature = "native")]
pub mod cache;

#[cfg(feature = "native")]
pub use model::NativeWhisperModel;
#[cfg(feature = "native")]
pub use audio::AudioProcessor;
#[cfg(feature = "native")]
pub use device::DeviceManager;
#[cfg(feature = "native")]
pub use cache::ModelCache;

#[cfg(not(feature = "native"))]
pub struct NativeWhisperModel;

#[cfg(not(feature = "native"))]
impl NativeWhisperModel {
    pub async fn new(_model_id: &str) -> crate::error::Result<Self> {
        Err(crate::error::DattavaniError::configuration(
            "Native implementation not available. Enable 'native' feature."
        ))
    }
}

use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeTranscriptionResult {
    pub text: String,
    pub confidence: Option<f64>,
    pub language: Option<String>,
    pub segments: Option<Vec<TranscriptionSegment>>,
    pub processing_time: f64,
    pub model_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct NativeTranscriptionOptions {
    pub model_id: String,
    pub language: Option<String>,
    pub task: TranscriptionTask,
    pub temperature: f32,
    pub best_of: usize,
    pub beam_size: usize,
    pub patience: f32,
    pub suppress_tokens: Vec<u32>,
    pub initial_prompt: Option<String>,
    pub condition_on_previous_text: bool,
    pub fp16: bool,
    pub compression_ratio_threshold: f32,
    pub logprob_threshold: f32,
    pub no_speech_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TranscriptionTask {
    Transcribe,
    Translate,
}

impl Default for NativeTranscriptionOptions {
    fn default() -> Self {
        Self {
            model_id: "openai/whisper-base".to_string(),
            language: None,
            task: TranscriptionTask::Transcribe,
            temperature: 0.0,
            best_of: 5,
            beam_size: 5,
            patience: 1.0,
            suppress_tokens: vec![], // Default suppress tokens (empty for demo)
            initial_prompt: None,
            condition_on_previous_text: true,
            fp16: true,
            compression_ratio_threshold: 2.4,
            logprob_threshold: -1.0,
            no_speech_threshold: 0.6,
        }
    }
}

/// High-level native transcriber interface
pub struct NativeTranscriber {
    #[cfg(feature = "native")]
    model_cache: ModelCache,
    #[cfg(feature = "native")]
    audio_processor: AudioProcessor,
    #[cfg(feature = "native")]
    device_manager: DeviceManager,
    
    #[cfg(not(feature = "native"))]
    _phantom: std::marker::PhantomData<()>,
}

impl NativeTranscriber {
    pub async fn new() -> Result<Self> {
        #[cfg(feature = "native")]
        {
            let device_manager = DeviceManager::new()?;
            let audio_processor = AudioProcessor::new(16000)?; // 16kHz sample rate for Whisper
            let model_cache = ModelCache::new("./model_cache".into())?;
            
            Ok(Self {
                model_cache,
                audio_processor,
                device_manager,
            })
        }
        
        #[cfg(not(feature = "native"))]
        {
            Err(crate::error::DattavaniError::configuration(
                "Native implementation not available. Enable 'native' feature or use simple transcriber."
            ))
        }
    }
    
    pub async fn transcribe(&mut self, audio_path: &Path, options: NativeTranscriptionOptions) -> Result<NativeTranscriptionResult> {
        #[cfg(feature = "native")]
        {
            use std::time::Instant;
            
            let start_time = Instant::now();
            
            // Load model from cache or download
            let model_path = self.model_cache.get_or_download(&options.model_id).await?;
            
            // Load the model from cache
            let model = NativeWhisperModel::load_from_cache(&options.model_id, &model_path).await?;
            
            // Process audio
            let audio_data = self.audio_processor.load_audio(audio_path).await?;
            
            // Transcribe
            let result = model.transcribe(&audio_data, &options).await?;
            
            let processing_time = start_time.elapsed().as_secs_f64();
            
            Ok(NativeTranscriptionResult {
                text: result.text,
                confidence: result.confidence,
                language: result.language,
                segments: result.segments,
                processing_time,
                model_used: options.model_id,
            })
        }
        
        #[cfg(not(feature = "native"))]
        {
            let _ = (audio_path, options);
            Err(crate::error::DattavaniError::configuration(
                "Native implementation not available. Enable 'native' feature."
            ))
        }
    }
    
    pub async fn transcribe_batch(&mut self, audio_files: Vec<&Path>, options: NativeTranscriptionOptions) -> Result<Vec<NativeTranscriptionResult>> {
        #[cfg(feature = "native")]
        {
            let mut results = Vec::new();
            
            for audio_path in audio_files {
                let result = self.transcribe(audio_path, options.clone()).await?;
                results.push(result);
            }
            
            Ok(results)
        }
        
        #[cfg(not(feature = "native"))]
        {
            let _ = (audio_files, options);
            Err(crate::error::DattavaniError::configuration(
                "Native implementation not available. Enable 'native' feature."
            ))
        }
    }
}
