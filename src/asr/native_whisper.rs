use anyhow::Result;
use std::path::Path;
use tracing::{debug, info, warn};
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

use crate::error::DattavaniError;

pub struct NativeWhisperModel {
    ctx: WhisperContext,
}

impl NativeWhisperModel {
    pub async fn new(model_path: &str) -> Result<Self> {
        info!("Loading native Whisper model: {}", model_path);
        
        // For now, we'll use a default model path
        // In production, you'd download the model or use a local path
        let model_path = format!("models/ggml-{}.bin", model_path);
        
        let ctx = WhisperContext::new_with_params(
            &model_path,
            WhisperContextParameters::default(),
        ).map_err(|e| DattavaniError::asr_processing(format!("Failed to load Whisper model: {}", e)))?;
        
        info!("Native Whisper model loaded successfully");
        
        Ok(Self { ctx })
    }
    
    pub async fn transcribe(&self, audio_path: &Path, language: Option<&str>) -> Result<String> {
        info!("Transcribing audio with native Whisper: {:?}", audio_path);
        
        // Load audio data
        let audio_data = self.load_audio(audio_path)?;
        
        // Set up transcription parameters
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        
        // Set language if specified
        if let Some(lang) = language {
            params.set_language(Some(lang));
        }
        
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        
        // Run transcription
        let mut state = self.ctx.create_state().map_err(|e| {
            DattavaniError::asr_processing(format!("Failed to create Whisper state: {}", e))
        })?;
        
        state.full(params, &audio_data).map_err(|e| {
            DattavaniError::asr_processing(format!("Transcription failed: {}", e))
        })?;
        
        // Extract transcribed text
        let num_segments = state.full_n_segments().map_err(|e| {
            DattavaniError::asr_processing(format!("Failed to get segment count: {}", e))
        })?;
        
        let mut full_text = String::new();
        for i in 0..num_segments {
            let segment_text = state.full_get_segment_text(i).map_err(|e| {
                DattavaniError::asr_processing(format!("Failed to get segment text: {}", e))
            })?;
            full_text.push_str(&segment_text);
        }
        
        debug!("Transcription completed: {} characters", full_text.len());
        Ok(full_text.trim().to_string())
    }
    
    fn load_audio(&self, audio_path: &Path) -> Result<Vec<f32>> {
        // Load WAV file using hound
        let mut reader = hound::WavReader::open(audio_path)
            .map_err(|e| DattavaniError::audio_processing(format!("Failed to open audio file: {}", e)))?;
        
        let spec = reader.spec();
        
        // Convert samples to f32 and normalize
        let samples: Result<Vec<f32>, _> = match spec.sample_format {
            hound::SampleFormat::Float => {
                reader.samples::<f32>().collect()
            }
            hound::SampleFormat::Int => {
                let max_val = (1i32 << (spec.bits_per_sample - 1)) as f32;
                reader.samples::<i32>()
                    .map(|s| s.map(|sample| sample as f32 / max_val))
                    .collect()
            }
        };
        
        let mut samples = samples
            .map_err(|e| DattavaniError::audio_processing(format!("Failed to read audio samples: {}", e)))?;
        
        // Convert stereo to mono if needed
        if spec.channels == 2 {
            samples = samples.chunks(2)
                .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
                .collect();
        }
        
        // Resample to 16kHz if needed (Whisper expects 16kHz)
        if spec.sample_rate != 16000 {
            warn!("Audio sample rate is {}Hz, but Whisper expects 16kHz. Consider resampling.", spec.sample_rate);
            // TODO: Add proper resampling
        }
        
        Ok(samples)
    }
}

// Simplified version for immediate use
pub struct SimpleNativeWhisper {
    model_name: String,
}

impl SimpleNativeWhisper {
    pub fn new(model_name: String) -> Self {
        Self { model_name }
    }
    
    pub async fn transcribe(&self, audio_path: &Path, language: Option<&str>) -> Result<String> {
        info!("Attempting native Whisper transcription for: {:?}", audio_path);
        
        // Try to create and use the native model
        match NativeWhisperModel::new(&self.model_name).await {
            Ok(model) => {
                model.transcribe(audio_path, language).await
            }
            Err(e) => {
                warn!("Failed to initialize native Whisper model: {}", e);
                Err(DattavaniError::asr_processing(
                    format!("Native Whisper model '{}' not available: {}", self.model_name, e)
                ).into())
            }
        }
    }
}
