/*!
Video Processing Module for Dattavani ASR

Extracts audio from various video formats and processes them through the ASR pipeline.
Supports a wide range of video formats including MP4, AVI, MOV, MKV, WebM, FLV, and more.
*/

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command as AsyncCommand;
use tracing::{info, debug, warn};
use tempfile::NamedTempFile;

use crate::config::Config;
use crate::error::{DattavaniError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub video_codec: String,
    pub audio_codec: Option<String>,
    pub audio_sample_rate: Option<u32>,
    pub audio_channels: Option<u32>,
    pub file_size: u64,
    pub format_name: String,
    pub bitrate: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioExtractionResult {
    pub success: bool,
    pub audio_path: Option<PathBuf>,
    pub error: Option<String>,
    pub extraction_time: Option<f64>,
    pub original_duration: Option<f64>,
    pub extracted_duration: Option<f64>,
    pub video_info: Option<VideoInfo>,
}

pub struct VideoProcessor {
    config: Config,
}

impl VideoProcessor {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
    
    pub fn supported_video_formats() -> Vec<&'static str> {
        vec![
            "mp4", "avi", "mov", "mkv", "webm", "flv", "wmv", "m4v",
            "3gp", "3g2", "asf", "divx", "f4v", "m2ts", "mts", "ts",
            "vob", "ogv", "rm", "rmvb", "mpg", "mpeg", "m1v", "m2v",
            "mxf", "roq", "nsv", "amv", "drc", "gif", "gifv", "mng",
            "qt", "yuv", "rgb", "bmp", "tiff", "tga", "dpx", "exr",
        ]
    }
    
    pub fn supported_audio_formats() -> Vec<&'static str> {
        vec![
            "mp3", "wav", "m4a", "flac", "ogg", "wma", "aac", "ac3",
            "aiff", "au", "ra", "3ga", "amr", "awb", "dss", "dvf",
            "m4b", "m4p", "mmf", "mpc", "msv", "nmf", "oga", "opus",
            "qcp", "tta", "voc", "w64", "wv", "webm", "8svx",
        ]
    }
    
    pub fn is_supported_format(file_path: &str) -> bool {
        let path = Path::new(file_path);
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                let ext_lower = ext_str.to_lowercase();
                return Self::supported_video_formats().contains(&ext_lower.as_str()) ||
                       Self::supported_audio_formats().contains(&ext_lower.as_str());
            }
        }
        false
    }
    
    pub async fn check_ffmpeg() -> Result<()> {
        let output = AsyncCommand::new("ffmpeg")
            .arg("-version")
            .output()
            .await
            .map_err(|e| DattavaniError::ffmpeg(format!("FFmpeg not found: {}", e)))?;
        
        if !output.status.success() {
            return Err(DattavaniError::ffmpeg("FFmpeg version check failed"));
        }
        
        let version_output = String::from_utf8_lossy(&output.stdout);
        debug!("FFmpeg version: {}", version_output.lines().next().unwrap_or("Unknown"));
        
        Ok(())
    }
    
    pub async fn get_video_info(&self, input_path: &str) -> Result<VideoInfo> {
        debug!("Getting video info for: {}", input_path);
        
        let output = AsyncCommand::new("ffprobe")
            .args(&[
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
                input_path
            ])
            .output()
            .await
            .map_err(|e| DattavaniError::ffmpeg(format!("FFprobe failed: {}", e)))?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(DattavaniError::ffmpeg(format!("FFprobe error: {}", error_msg)));
        }
        
        let probe_output = String::from_utf8_lossy(&output.stdout);
        let probe_data: serde_json::Value = serde_json::from_str(&probe_output)
            .map_err(|e| DattavaniError::ffmpeg(format!("Failed to parse FFprobe output: {}", e)))?;
        
        self.parse_video_info(probe_data)
    }
    
    fn parse_video_info(&self, probe_data: serde_json::Value) -> Result<VideoInfo> {
        let format = probe_data["format"].as_object()
            .ok_or_else(|| DattavaniError::ffmpeg("No format information in FFprobe output"))?;
        
        let streams = probe_data["streams"].as_array()
            .ok_or_else(|| DattavaniError::ffmpeg("No streams information in FFprobe output"))?;
        
        let duration = format["duration"].as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        
        let file_size = format["size"].as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        let format_name = format["format_name"].as_str()
            .unwrap_or("unknown")
            .to_string();
        
        let bitrate = format["bit_rate"].as_str()
            .and_then(|s| s.parse::<u64>().ok());
        
        // Find video stream
        let video_stream = streams.iter()
            .find(|stream| stream["codec_type"].as_str() == Some("video"));
        
        let (width, height, fps, video_codec) = if let Some(video) = video_stream {
            let width = video["width"].as_u64().unwrap_or(0) as u32;
            let height = video["height"].as_u64().unwrap_or(0) as u32;
            let fps = Self::parse_fps(video);
            let codec = video["codec_name"].as_str().unwrap_or("unknown").to_string();
            (width, height, fps, codec)
        } else {
            (0, 0, 0.0, "none".to_string())
        };
        
        // Find audio stream
        let audio_stream = streams.iter()
            .find(|stream| stream["codec_type"].as_str() == Some("audio"));
        
        let (audio_codec, audio_sample_rate, audio_channels) = if let Some(audio) = audio_stream {
            let codec = audio["codec_name"].as_str().map(|s| s.to_string());
            let sample_rate = audio["sample_rate"].as_str()
                .and_then(|s| s.parse::<u32>().ok());
            let channels = audio["channels"].as_u64().map(|c| c as u32);
            (codec, sample_rate, channels)
        } else {
            (None, None, None)
        };
        
        Ok(VideoInfo {
            duration,
            width,
            height,
            fps,
            video_codec,
            audio_codec,
            audio_sample_rate,
            audio_channels,
            file_size,
            format_name,
            bitrate,
        })
    }
    
    fn parse_fps(video_stream: &serde_json::Value) -> f64 {
        // Try different FPS fields
        if let Some(fps_str) = video_stream["r_frame_rate"].as_str() {
            if let Some((num, den)) = fps_str.split_once('/') {
                if let (Ok(n), Ok(d)) = (num.parse::<f64>(), den.parse::<f64>()) {
                    if d != 0.0 {
                        return n / d;
                    }
                }
            }
        }
        
        if let Some(fps_str) = video_stream["avg_frame_rate"].as_str() {
            if let Some((num, den)) = fps_str.split_once('/') {
                if let (Ok(n), Ok(d)) = (num.parse::<f64>(), den.parse::<f64>()) {
                    if d != 0.0 {
                        return n / d;
                    }
                }
            }
        }
        
        0.0
    }
    
    pub async fn extract_audio(&self, input_path: &str, output_path: Option<&str>) -> Result<AudioExtractionResult> {
        let start_time = std::time::Instant::now();
        
        info!("Extracting audio from: {}", input_path);
        
        // Get video info first
        let video_info = match self.get_video_info(input_path).await {
            Ok(info) => Some(info),
            Err(e) => {
                warn!("Could not get video info: {}", e);
                None
            }
        };
        
        // Determine output path
        let output_path = if let Some(path) = output_path {
            PathBuf::from(path)
        } else {
            let temp_file = NamedTempFile::new()
                .map_err(DattavaniError::FileIo)?;
            let mut temp_path = temp_file.path().to_path_buf();
            temp_path.set_extension("wav");
            temp_path
        };
        
        // Extract audio using FFmpeg
        let mut cmd = AsyncCommand::new("ffmpeg");
        cmd.args(&[
            "-i", input_path,
            "-vn", // No video
            "-acodec", "pcm_s16le", // PCM 16-bit little-endian
            "-ar", &self.config.processing.target_sample_rate.to_string(), // Sample rate
            "-ac", "1", // Mono
            "-y", // Overwrite output file
        ]);
        
        cmd.arg(output_path.to_str().unwrap());
        
        debug!("Running FFmpeg command: {:?}", cmd);
        
        let output = cmd.output().await
            .map_err(|e| DattavaniError::ffmpeg(format!("FFmpeg execution failed: {}", e)))?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Ok(AudioExtractionResult {
                success: false,
                audio_path: None,
                error: Some(format!("FFmpeg error: {}", error_msg)),
                extraction_time: Some(start_time.elapsed().as_secs_f64()),
                original_duration: video_info.as_ref().map(|v| v.duration),
                extracted_duration: None,
                video_info,
            });
        }
        
        // Verify extracted audio
        let extracted_duration = match self.get_audio_duration(&output_path).await {
            Ok(duration) => Some(duration),
            Err(e) => {
                warn!("Could not get extracted audio duration: {}", e);
                None
            }
        };
        
        let extraction_time = start_time.elapsed().as_secs_f64();
        
        info!("Audio extraction completed in {:.2}s", extraction_time);
        
        Ok(AudioExtractionResult {
            success: true,
            audio_path: Some(output_path),
            error: None,
            extraction_time: Some(extraction_time),
            original_duration: video_info.as_ref().map(|v| v.duration),
            extracted_duration,
            video_info,
        })
    }
    
    async fn get_audio_duration(&self, audio_path: &Path) -> Result<f64> {
        let output = AsyncCommand::new("ffprobe")
            .args(&[
                "-v", "quiet",
                "-show_entries", "format=duration",
                "-of", "csv=p=0",
                audio_path.to_str().unwrap()
            ])
            .output()
            .await
            .map_err(|e| DattavaniError::ffmpeg(format!("FFprobe duration check failed: {}", e)))?;
        
        if !output.status.success() {
            return Err(DattavaniError::ffmpeg("Failed to get audio duration"));
        }
        
        let duration_str = String::from_utf8_lossy(&output.stdout);
        duration_str.trim().parse::<f64>()
            .map_err(|e| DattavaniError::ffmpeg(format!("Invalid duration format: {}", e)))
    }
    
    pub async fn extract_audio_segment(&self, input_path: &str, start_time: f64, duration: f64, output_path: &Path) -> Result<AudioExtractionResult> {
        let start_instant = std::time::Instant::now();
        
        info!("Extracting audio segment from {:.2}s to {:.2}s", start_time, start_time + duration);
        
        let mut cmd = AsyncCommand::new("ffmpeg");
        cmd.args(&[
            "-ss", &start_time.to_string(), // Start time
            "-i", input_path,
            "-t", &duration.to_string(), // Duration
            "-vn", // No video
            "-acodec", "pcm_s16le", // PCM 16-bit little-endian
            "-ar", &self.config.processing.target_sample_rate.to_string(), // Sample rate
            "-ac", "1", // Mono
            "-y", // Overwrite output file
        ]);
        
        cmd.arg(output_path.to_str().unwrap());
        
        let output = cmd.output().await
            .map_err(|e| DattavaniError::ffmpeg(format!("FFmpeg segment extraction failed: {}", e)))?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Ok(AudioExtractionResult {
                success: false,
                audio_path: None,
                error: Some(format!("FFmpeg segment error: {}", error_msg)),
                extraction_time: Some(start_instant.elapsed().as_secs_f64()),
                original_duration: Some(duration),
                extracted_duration: None,
                video_info: None,
            });
        }
        
        let extracted_duration = self.get_audio_duration(output_path).await.ok();
        let extraction_time = start_instant.elapsed().as_secs_f64();
        
        Ok(AudioExtractionResult {
            success: true,
            audio_path: Some(output_path.to_path_buf()),
            error: None,
            extraction_time: Some(extraction_time),
            original_duration: Some(duration),
            extracted_duration,
            video_info: None,
        })
    }
    
    pub async fn convert_audio_format(&self, input_path: &Path, output_path: &Path, target_format: &str) -> Result<()> {
        info!("Converting audio format: {} -> {}", input_path.display(), target_format);
        
        let mut cmd = AsyncCommand::new("ffmpeg");
        cmd.args(&[
            "-i", input_path.to_str().unwrap(),
            "-acodec", target_format,
            "-ar", &self.config.processing.target_sample_rate.to_string(),
            "-ac", "1", // Mono
            "-y", // Overwrite
        ]);
        
        cmd.arg(output_path.to_str().unwrap());
        
        let output = cmd.output().await
            .map_err(|e| DattavaniError::ffmpeg(format!("Audio conversion failed: {}", e)))?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(DattavaniError::ffmpeg(format!("Audio conversion error: {}", error_msg)));
        }
        
        Ok(())
    }
}
