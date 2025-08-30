/*!
Audio Processing for Native Whisper Implementation - Demo Version

Handles audio loading, preprocessing, and conversion for the native Whisper model.
This is a simplified demonstration version.
*/

use std::path::Path;
use tracing::{info, debug, warn};
use crate::error::{DattavaniError, Result};

#[cfg(feature = "native")]
use hound::{WavReader, SampleFormat};

pub struct AudioProcessor {
    pub target_sample_rate: u32,
    pub n_fft: usize,
    pub hop_length: usize,
    pub n_mels: usize,
}

impl AudioProcessor {
    pub fn new(target_sample_rate: u32) -> Result<Self> {
        Ok(Self {
            target_sample_rate,
            n_fft: 400,
            hop_length: 160,
            n_mels: 80,
        })
    }
    
    pub async fn load_audio(&self, path: &Path) -> Result<Vec<f32>> {
        info!("Loading audio file: {:?} (DEMO MODE)", path);
        
        // Check if it's a WAV file first
        if path.extension().and_then(|s| s.to_str()) == Some("wav") {
            #[cfg(feature = "native")]
            {
                // Try to load as WAV file directly
                if let Ok(mut reader) = WavReader::open(path) {
                    let spec = reader.spec();
                    info!("Audio format: {} channels, {} Hz, {} bits", 
                          spec.channels, spec.sample_rate, spec.bits_per_sample);
                    
                    let samples: std::result::Result<Vec<f32>, hound::Error> = match spec.sample_format {
                        SampleFormat::Float => {
                            reader.samples::<f32>().collect()
                        },
                        SampleFormat::Int => {
                            let int_samples: std::result::Result<Vec<i32>, hound::Error> = reader.samples().collect();
                            match int_samples {
                                Ok(samples) => {
                                    let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                                    Ok(samples.into_iter().map(|s| s as f32 / max_val).collect())
                                },
                                Err(e) => Err(e),
                            }
                        }
                    };
                    
                    let mut audio_data = samples
                        .map_err(|e| DattavaniError::asr_processing(format!("Failed to read audio samples: {}", e)))?;
                    
                    // Convert to mono if stereo
                    if spec.channels == 2 {
                        audio_data = self.stereo_to_mono(&audio_data);
                    }
                    
                    // Resample if needed
                    if spec.sample_rate != self.target_sample_rate {
                        audio_data = self.resample(&audio_data, spec.sample_rate, self.target_sample_rate)?;
                    }
                    
                    info!("Loaded {} samples at {} Hz", audio_data.len(), self.target_sample_rate);
                    return Ok(audio_data);
                }
            }
        } else {
            // For non-WAV files (MP3, etc.), use VideoProcessor to convert to WAV first
            info!("Non-WAV file detected, converting to WAV using VideoProcessor");
            
            use crate::video::VideoProcessor;
            use crate::config::Config;
            
            // Create a temporary config for VideoProcessor
            let config = Config::default();
            let video_processor = VideoProcessor::new(config);
            
            // Extract audio to temporary WAV file
            let extraction_result = video_processor.extract_audio(
                path.to_str().unwrap(), 
                None // Use temporary file
            ).await?;
            
            if !extraction_result.success {
                return Err(DattavaniError::asr_processing(
                    format!("Failed to convert audio: {}", 
                           extraction_result.error.unwrap_or_else(|| "Unknown error".to_string()))
                ));
            }
            
            if let Some(wav_path) = extraction_result.audio_path {
                info!("Successfully converted to WAV: {:?}", wav_path);
                
                // Now load the converted WAV file
                #[cfg(feature = "native")]
                {
                    if let Ok(mut reader) = WavReader::open(&wav_path) {
                        let spec = reader.spec();
                        info!("Converted audio format: {} channels, {} Hz, {} bits", 
                              spec.channels, spec.sample_rate, spec.bits_per_sample);
                        
                        let samples: std::result::Result<Vec<f32>, hound::Error> = match spec.sample_format {
                            SampleFormat::Float => {
                                reader.samples::<f32>().collect()
                            },
                            SampleFormat::Int => {
                                let int_samples: std::result::Result<Vec<i32>, hound::Error> = reader.samples().collect();
                                match int_samples {
                                    Ok(samples) => {
                                        let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                                        Ok(samples.into_iter().map(|s| s as f32 / max_val).collect())
                                    },
                                    Err(e) => Err(e),
                                }
                            }
                        };
                        
                        let mut audio_data = samples
                            .map_err(|e| DattavaniError::asr_processing(format!("Failed to read converted audio samples: {}", e)))?;
                        
                        // Convert to mono if stereo
                        if spec.channels == 2 {
                            audio_data = self.stereo_to_mono(&audio_data);
                        }
                        
                        // Resample if needed
                        if spec.sample_rate != self.target_sample_rate {
                            audio_data = self.resample(&audio_data, spec.sample_rate, self.target_sample_rate)?;
                        }
                        
                        info!("Loaded {} samples from converted audio at {} Hz", audio_data.len(), self.target_sample_rate);
                        
                        // Clean up temporary file
                        if let Err(e) = std::fs::remove_file(&wav_path) {
                            warn!("Failed to clean up temporary WAV file: {}", e);
                        }
                        
                        return Ok(audio_data);
                    }
                }
            }
        }
        
        // Fallback: generate demo audio data
        warn!("Could not load audio file, generating demo data");
        self.generate_demo_audio().await
    }
    
    pub async fn load_audio_from_bytes(&self, data: &[u8]) -> Result<Vec<f32>> {
        info!("Loading audio from {} bytes (DEMO MODE)", data.len());
        
        #[cfg(feature = "native")]
        {
            use std::io::Cursor;
            
            // Try to parse as WAV
            if let Ok(mut reader) = WavReader::new(Cursor::new(data)) {
                let spec = reader.spec();
                
                let samples: std::result::Result<Vec<f32>, hound::Error> = match spec.sample_format {
                    SampleFormat::Float => {
                        reader.samples::<f32>().collect()
                    },
                    SampleFormat::Int => {
                        let int_samples: std::result::Result<Vec<i32>, hound::Error> = reader.samples().collect();
                        match int_samples {
                            Ok(samples) => {
                                let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                                Ok(samples.into_iter().map(|s| s as f32 / max_val).collect())
                            },
                            Err(e) => Err(e),
                        }
                    }
                };
                
                let mut audio_data = samples
                    .map_err(|e| DattavaniError::asr_processing(format!("Failed to read audio samples: {}", e)))?;
                
                // Convert to mono if stereo
                if spec.channels == 2 {
                    audio_data = self.stereo_to_mono(&audio_data);
                }
                
                // Resample if needed
                if spec.sample_rate != self.target_sample_rate {
                    audio_data = self.resample(&audio_data, spec.sample_rate, self.target_sample_rate)?;
                }
                
                return Ok(audio_data);
            }
        }
        
        // Fallback: generate demo audio based on data size
        warn!("Could not parse audio bytes, generating demo data");
        let duration = (data.len() as f64 / 1000.0).max(1.0).min(30.0); // Estimate duration
        self.generate_demo_audio_with_duration(duration).await
    }
    
    async fn generate_demo_audio(&self) -> Result<Vec<f32>> {
        self.generate_demo_audio_with_duration(5.0).await
    }
    
    async fn generate_demo_audio_with_duration(&self, duration_seconds: f64) -> Result<Vec<f32>> {
        info!("Generating demo audio: {:.1}s at {} Hz", duration_seconds, self.target_sample_rate);
        
        let num_samples = (duration_seconds * self.target_sample_rate as f64) as usize;
        let mut audio_data = Vec::with_capacity(num_samples);
        
        // Generate a mix of sine waves to simulate speech-like audio
        for i in 0..num_samples {
            let t = i as f64 / self.target_sample_rate as f64;
            
            // Mix of different frequencies to simulate speech
            let fundamental = (2.0 * std::f64::consts::PI * 200.0 * t).sin() * 0.3;
            let harmonic1 = (2.0 * std::f64::consts::PI * 400.0 * t).sin() * 0.2;
            let harmonic2 = (2.0 * std::f64::consts::PI * 600.0 * t).sin() * 0.1;
            let noise = (rand::random::<f64>() - 0.5) * 0.05;
            
            // Add some amplitude modulation to simulate speech patterns
            let envelope = (2.0 * std::f64::consts::PI * 3.0 * t).sin().abs() * 0.8 + 0.2;
            
            let sample = (fundamental + harmonic1 + harmonic2 + noise) * envelope;
            audio_data.push(sample as f32);
        }
        
        info!("Generated {} demo audio samples", audio_data.len());
        Ok(audio_data)
    }
    
    fn stereo_to_mono(&self, stereo_data: &[f32]) -> Vec<f32> {
        debug!("Converting stereo to mono: {} samples", stereo_data.len());
        
        stereo_data
            .chunks_exact(2)
            .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
            .collect()
    }
    
    fn resample(&self, audio_data: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
        if from_rate == to_rate {
            return Ok(audio_data.to_vec());
        }
        
        info!("Resampling audio: {} Hz -> {} Hz", from_rate, to_rate);
        
        // Simple linear interpolation resampling (for demo purposes)
        let ratio = to_rate as f64 / from_rate as f64;
        let output_len = (audio_data.len() as f64 * ratio) as usize;
        let mut resampled = Vec::with_capacity(output_len);
        
        for i in 0..output_len {
            let src_index = i as f64 / ratio;
            let src_index_floor = src_index.floor() as usize;
            let src_index_ceil = (src_index_floor + 1).min(audio_data.len() - 1);
            let fraction = src_index - src_index_floor as f64;
            
            if src_index_floor < audio_data.len() {
                let sample = audio_data[src_index_floor] * (1.0 - fraction) as f32 +
                           audio_data[src_index_ceil] * fraction as f32;
                resampled.push(sample);
            }
        }
        
        debug!("Resampled {} -> {} samples", audio_data.len(), resampled.len());
        Ok(resampled)
    }
    
    pub fn normalize_audio(&self, audio_data: &mut [f32]) {
        if audio_data.is_empty() {
            return;
        }
        
        // Find the maximum absolute value
        let max_val = audio_data.iter()
            .map(|&x| x.abs())
            .fold(0.0f32, f32::max);
        
        if max_val > 0.0 {
            let scale = 0.95 / max_val; // Leave some headroom
            for sample in audio_data.iter_mut() {
                *sample *= scale;
            }
            debug!("Normalized audio with scale factor: {:.3}", scale);
        }
    }
    
    pub fn apply_pre_emphasis(&self, audio_data: &mut [f32], coefficient: f32) {
        if audio_data.len() < 2 {
            return;
        }
        
        // Apply pre-emphasis filter: y[n] = x[n] - coefficient * x[n-1]
        for i in (1..audio_data.len()).rev() {
            audio_data[i] -= coefficient * audio_data[i - 1];
        }
        
        debug!("Applied pre-emphasis filter with coefficient: {}", coefficient);
    }
    
    pub fn pad_or_trim(&self, audio_data: &[f32], target_length: usize) -> Vec<f32> {
        let mut result = audio_data.to_vec();
        
        if result.len() > target_length {
            result.truncate(target_length);
            debug!("Trimmed audio from {} to {} samples", audio_data.len(), target_length);
        } else if result.len() < target_length {
            result.resize(target_length, 0.0);
            debug!("Padded audio from {} to {} samples", audio_data.len(), target_length);
        }
        
        result
    }
    
    pub fn get_audio_info(&self, audio_data: &[f32]) -> AudioInfo {
        let duration = audio_data.len() as f64 / self.target_sample_rate as f64;
        let rms = if !audio_data.is_empty() {
            let sum_squares: f64 = audio_data.iter().map(|&x| (x as f64).powi(2)).sum();
            (sum_squares / audio_data.len() as f64).sqrt()
        } else {
            0.0
        };
        
        let peak = audio_data.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
        
        AudioInfo {
            sample_rate: self.target_sample_rate,
            duration_seconds: duration,
            num_samples: audio_data.len(),
            rms_level: rms,
            peak_level: peak as f64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioInfo {
    pub sample_rate: u32,
    pub duration_seconds: f64,
    pub num_samples: usize,
    pub rms_level: f64,
    pub peak_level: f64,
}

impl std::fmt::Display for AudioInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Audio: {:.1}s, {} Hz, {} samples, RMS: {:.3}, Peak: {:.3}",
               self.duration_seconds, self.sample_rate, self.num_samples, 
               self.rms_level, self.peak_level)
    }
}

// Simple random number generation for demo
mod rand {
    use std::cell::Cell;
    
    thread_local! {
        static RNG_STATE: Cell<u64> = Cell::new(1);
    }
    
    pub fn random<T>() -> T 
    where 
        T: From<f64>
    {
        RNG_STATE.with(|state| {
            let mut x = state.get();
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            state.set(x);
            T::from((x as f64) / (u64::MAX as f64))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audio_processor_creation() {
        let processor = AudioProcessor::new(16000).unwrap();
        assert_eq!(processor.target_sample_rate, 16000);
        assert_eq!(processor.n_mels, 80);
    }
    
    #[test]
    fn test_stereo_to_mono() {
        let processor = AudioProcessor::new(16000).unwrap();
        let stereo = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let mono = processor.stereo_to_mono(&stereo);
        
        assert_eq!(mono.len(), 3);
        assert_eq!(mono[0], 1.5); // (1.0 + 2.0) / 2
        assert_eq!(mono[1], 3.5); // (3.0 + 4.0) / 2
        assert_eq!(mono[2], 5.5); // (5.0 + 6.0) / 2
    }
    
    #[test]
    fn test_pad_or_trim() {
        let processor = AudioProcessor::new(16000).unwrap();
        let audio = vec![1.0, 2.0, 3.0];
        
        // Test padding
        let padded = processor.pad_or_trim(&audio, 5);
        assert_eq!(padded.len(), 5);
        assert_eq!(padded[0..3], [1.0, 2.0, 3.0]);
        assert_eq!(padded[3..], [0.0, 0.0]);
        
        // Test trimming
        let trimmed = processor.pad_or_trim(&audio, 2);
        assert_eq!(trimmed.len(), 2);
        assert_eq!(trimmed, [1.0, 2.0]);
    }
    
    #[tokio::test]
    async fn test_demo_audio_generation() {
        let processor = AudioProcessor::new(16000).unwrap();
        let audio = processor.generate_demo_audio().await.unwrap();
        
        assert!(!audio.is_empty());
        assert_eq!(audio.len(), 16000 * 5); // 5 seconds at 16kHz
        
        // Check that audio has some variation (not all zeros)
        let max_val = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
        assert!(max_val > 0.0);
    }
    
    #[test]
    fn test_audio_info() {
        let processor = AudioProcessor::new(16000).unwrap();
        let audio = vec![0.5, -0.3, 0.8, -0.2];
        let info = processor.get_audio_info(&audio);
        
        assert_eq!(info.sample_rate, 16000);
        assert_eq!(info.num_samples, 4);
        assert!(info.duration_seconds > 0.0);
        assert!(info.rms_level > 0.0);
        assert!(info.peak_level > 0.0);
    }
}
