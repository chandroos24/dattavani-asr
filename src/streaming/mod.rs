/*!
Streaming Video Processor for Dattavani ASR

Processes videos directly from URLs without downloading the entire file.
Supports streaming from Google Drive, YouTube, and other video sources.
*/

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::process::Command as AsyncCommand;
use reqwest::Client;
use tracing::info;

use crate::config::Config;
use crate::error::{DattavaniError, Result};
use crate::gdrive::GDriveClient;
use crate::video::{VideoInfo, AudioExtractionResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingResult {
    pub success: bool,
    pub audio_path: Option<PathBuf>,
    pub video_info: Option<VideoInfo>,
    pub error: Option<String>,
    pub processing_time: Option<f64>,
    pub stream_url: Option<String>,
    pub bytes_processed: Option<u64>,
}

pub struct StreamingProcessor {
    config: Config,
    client: Client,
    gdrive_client: Option<GDriveClient>,
}

impl StreamingProcessor {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.processing.timeout_seconds))
            .build()
            .map_err(DattavaniError::Network)?;
        
        Ok(Self {
            config,
            client,
            gdrive_client: None,
        })
    }
    
    pub async fn with_gdrive(mut self) -> Result<Self> {
        self.gdrive_client = Some(GDriveClient::new(self.config.clone()).await?);
        Ok(self)
    }
    
    pub async fn analyze_stream(&self, input_url: &str) -> Result<VideoInfo> {
        info!("Analyzing stream: {}", input_url);
        
        if self.is_google_drive_url(input_url) {
            self.analyze_gdrive_stream(input_url).await
        } else if self.is_youtube_url(input_url) {
            self.analyze_youtube_stream(input_url).await
        } else if self.is_http_url(input_url) {
            self.analyze_http_stream(input_url).await
        } else {
            // Assume local file
            self.analyze_local_file(input_url).await
        }
    }
    
    async fn analyze_gdrive_stream(&self, url: &str) -> Result<VideoInfo> {
        let gdrive = self.gdrive_client.as_ref()
            .ok_or_else(|| DattavaniError::google_drive("Google Drive client not initialized"))?;
        
        let file_id = GDriveClient::extract_file_id_from_url(url)?;
        let file_info = gdrive.get_file_info(&file_id).await?;
        
        // For Google Drive, we need to stream a small portion to analyze
        let temp_file = self.create_temp_file("analysis", "tmp").await?;
        let partial_data = gdrive.get_partial_content(&file_id, 0, Some(1024 * 1024)).await?; // First 1MB
        
        tokio::fs::write(&temp_file, &partial_data).await
            .map_err(DattavaniError::FileIo)?;
        
        // Use FFprobe on the partial file (this might not work for all formats)
        match self.get_video_info_from_file(temp_file.to_str().unwrap()).await {
            Ok(mut info) => {
                // Update file size from Drive API
                if let Some(size) = file_info.size {
                    info.file_size = size;
                }
                Ok(info)
            }
            Err(_) => {
                // Fallback: create a basic VideoInfo from file metadata
                Ok(VideoInfo {
                    duration: 0.0, // Unknown without full analysis
                    width: 0,
                    height: 0,
                    fps: 0.0,
                    video_codec: "unknown".to_string(),
                    audio_codec: Some("unknown".to_string()),
                    audio_sample_rate: None,
                    audio_channels: None,
                    file_size: file_info.size.unwrap_or(0),
                    format_name: file_info.mime_type,
                    bitrate: None,
                })
            }
        }
    }
    
    async fn analyze_youtube_stream(&self, url: &str) -> Result<VideoInfo> {
        // Use yt-dlp to get video information
        let output = AsyncCommand::new("yt-dlp")
            .args(&[
                "--dump-json",
                "--no-download",
                url
            ])
            .output()
            .await
            .map_err(|e| DattavaniError::streaming(format!("yt-dlp failed: {}", e)))?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(DattavaniError::streaming(format!("yt-dlp error: {}", error_msg)));
        }
        
        let json_output = String::from_utf8_lossy(&output.stdout);
        let video_data: serde_json::Value = serde_json::from_str(&json_output)
            .map_err(|e| DattavaniError::streaming(format!("Failed to parse yt-dlp output: {}", e)))?;
        
        Ok(VideoInfo {
            duration: video_data["duration"].as_f64().unwrap_or(0.0),
            width: video_data["width"].as_u64().unwrap_or(0) as u32,
            height: video_data["height"].as_u64().unwrap_or(0) as u32,
            fps: video_data["fps"].as_f64().unwrap_or(0.0),
            video_codec: video_data["vcodec"].as_str().unwrap_or("unknown").to_string(),
            audio_codec: video_data["acodec"].as_str().map(|s| s.to_string()),
            audio_sample_rate: video_data["asr"].as_u64().map(|r| r as u32),
            audio_channels: None, // Not always available in yt-dlp output
            file_size: video_data["filesize"].as_u64().unwrap_or(0),
            format_name: video_data["ext"].as_str().unwrap_or("unknown").to_string(),
            bitrate: video_data["tbr"].as_u64(),
        })
    }
    
    async fn analyze_http_stream(&self, url: &str) -> Result<VideoInfo> {
        // Get content length and type from HEAD request
        let response = self.client.head(url).send().await?;
        
        let file_size = response.headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        let content_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        
        // Try to get a small portion for analysis
        let partial_response = self.client
            .get(url)
            .header("Range", "bytes=0-1048576") // First 1MB
            .send()
            .await?;
        
        if partial_response.status().is_success() || partial_response.status().as_u16() == 206 {
            let bytes = partial_response.bytes().await?;
            let temp_file = self.create_temp_file("http_analysis", "tmp").await?;
            tokio::fs::write(&temp_file, &bytes).await
                .map_err(DattavaniError::FileIo)?;
            
            match self.get_video_info_from_file(temp_file.to_str().unwrap()).await {
                Ok(mut info) => {
                    info.file_size = file_size;
                    Ok(info)
                }
                Err(_) => {
                    // Fallback
                    Ok(VideoInfo {
                        duration: 0.0,
                        width: 0,
                        height: 0,
                        fps: 0.0,
                        video_codec: "unknown".to_string(),
                        audio_codec: Some("unknown".to_string()),
                        audio_sample_rate: None,
                        audio_channels: None,
                        file_size,
                        format_name: content_type,
                        bitrate: None,
                    })
                }
            }
        } else {
            Err(DattavaniError::streaming("Failed to get partial content for analysis"))
        }
    }
    
    async fn analyze_local_file(&self, path: &str) -> Result<VideoInfo> {
        self.get_video_info_from_file(path).await
    }
    
    async fn get_video_info_from_file(&self, path: &str) -> Result<VideoInfo> {
        let output = AsyncCommand::new("ffprobe")
            .args(&[
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
                path
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
            .ok_or_else(|| DattavaniError::ffmpeg("No format information"))?;
        
        let streams = probe_data["streams"].as_array()
            .ok_or_else(|| DattavaniError::ffmpeg("No streams information"))?;
        
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
        
        // Find video and audio streams
        let video_stream = streams.iter()
            .find(|stream| stream["codec_type"].as_str() == Some("video"));
        
        let audio_stream = streams.iter()
            .find(|stream| stream["codec_type"].as_str() == Some("audio"));
        
        let (width, height, fps, video_codec) = if let Some(video) = video_stream {
            let width = video["width"].as_u64().unwrap_or(0) as u32;
            let height = video["height"].as_u64().unwrap_or(0) as u32;
            let fps = self.parse_fps(video);
            let codec = video["codec_name"].as_str().unwrap_or("unknown").to_string();
            (width, height, fps, codec)
        } else {
            (0, 0, 0.0, "none".to_string())
        };
        
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
    
    fn parse_fps(&self, video_stream: &serde_json::Value) -> f64 {
        if let Some(fps_str) = video_stream["r_frame_rate"].as_str() {
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
    
    pub async fn stream_extract_audio(&self, input_url: &str, output_path: Option<&str>) -> Result<StreamingResult> {
        let start_time = std::time::Instant::now();
        
        info!("Starting streaming audio extraction from: {}", input_url);
        
        let result = if self.is_google_drive_url(input_url) {
            self.stream_extract_from_gdrive(input_url, output_path).await
        } else if self.is_youtube_url(input_url) {
            self.stream_extract_from_youtube(input_url, output_path).await
        } else if self.is_http_url(input_url) {
            self.stream_extract_from_http(input_url, output_path).await
        } else {
            // Local file - use direct extraction
            self.extract_from_local_file(input_url, output_path).await
        };
        
        match result {
            Ok(mut streaming_result) => {
                streaming_result.processing_time = Some(start_time.elapsed().as_secs_f64());
                Ok(streaming_result)
            }
            Err(e) => {
                Ok(StreamingResult {
                    success: false,
                    audio_path: None,
                    video_info: None,
                    error: Some(e.to_string()),
                    processing_time: Some(start_time.elapsed().as_secs_f64()),
                    stream_url: Some(input_url.to_string()),
                    bytes_processed: None,
                })
            }
        }
    }
    
    async fn stream_extract_from_gdrive(&self, url: &str, output_path: Option<&str>) -> Result<StreamingResult> {
        let gdrive = self.gdrive_client.as_ref()
            .ok_or_else(|| DattavaniError::google_drive("Google Drive client not initialized"))?;
        
        let file_id = GDriveClient::extract_file_id_from_url(url)?;
        let file_info = gdrive.get_file_info(&file_id).await?;
        
        info!("Streaming from Google Drive file: {} ({})", file_info.name, file_info.id);
        
        // Create temporary file for the stream
        let temp_input = self.create_temp_file("gdrive_stream", "tmp").await?;
        
        // Stream the file content
        let mut response = gdrive.get_download_stream(&file_id).await?;
        let mut file = tokio::fs::File::create(&temp_input).await
            .map_err(DattavaniError::FileIo)?;
        
        let mut bytes_processed = 0u64;
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await
                .map_err(DattavaniError::FileIo)?;
            bytes_processed += chunk.len() as u64;
        }
        
        file.flush().await.map_err(DattavaniError::FileIo)?;
        
        // Extract audio using FFmpeg
        let output_file = if let Some(path) = output_path {
            PathBuf::from(path)
        } else {
            self.create_temp_file("extracted_audio", "wav").await?
        };
        
        let extraction_result = self.extract_audio_with_ffmpeg(temp_input.to_str().unwrap(), &output_file).await?;
        
        // Get video info
        let video_info = self.get_video_info_from_file(temp_input.to_str().unwrap()).await.ok();
        
        // Clean up temp input file
        let _ = tokio::fs::remove_file(&temp_input).await;
        
        Ok(StreamingResult {
            success: extraction_result.success,
            audio_path: extraction_result.audio_path,
            video_info,
            error: extraction_result.error,
            processing_time: None, // Will be set by caller
            stream_url: Some(url.to_string()),
            bytes_processed: Some(bytes_processed),
        })
    }
    
    async fn stream_extract_from_youtube(&self, url: &str, output_path: Option<&str>) -> Result<StreamingResult> {
        info!("Streaming from YouTube: {}", url);
        
        let output_file = if let Some(path) = output_path {
            PathBuf::from(path)
        } else {
            self.create_temp_file("youtube_audio", "wav").await?
        };
        
        // Use yt-dlp to extract audio directly
        let mut cmd = AsyncCommand::new("yt-dlp");
        cmd.args(&[
            "--extract-audio",
            "--audio-format", "wav",
            "--audio-quality", "0", // Best quality
            "--output", output_file.to_str().unwrap(),
            url
        ]);
        
        let output = cmd.output().await
            .map_err(|e| DattavaniError::streaming(format!("yt-dlp failed: {}", e)))?;
        
        if output.status.success() {
            // Get video info
            let video_info = self.analyze_youtube_stream(url).await.ok();
            
            Ok(StreamingResult {
                success: true,
                audio_path: Some(output_file),
                video_info,
                error: None,
                processing_time: None,
                stream_url: Some(url.to_string()),
                bytes_processed: None,
            })
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            Ok(StreamingResult {
                success: false,
                audio_path: None,
                video_info: None,
                error: Some(format!("yt-dlp error: {}", error_msg)),
                processing_time: None,
                stream_url: Some(url.to_string()),
                bytes_processed: None,
            })
        }
    }
    
    async fn stream_extract_from_http(&self, url: &str, output_path: Option<&str>) -> Result<StreamingResult> {
        info!("Streaming from HTTP: {}", url);
        
        // For HTTP streams, we can use FFmpeg directly with the URL
        let output_file = if let Some(path) = output_path {
            PathBuf::from(path)
        } else {
            self.create_temp_file("http_audio", "wav").await?
        };
        
        let extraction_result = self.extract_audio_with_ffmpeg(url, &output_file).await?;
        
        // Try to get video info
        let video_info = self.analyze_http_stream(url).await.ok();
        
        Ok(StreamingResult {
            success: extraction_result.success,
            audio_path: extraction_result.audio_path,
            video_info,
            error: extraction_result.error,
            processing_time: None,
            stream_url: Some(url.to_string()),
            bytes_processed: None,
        })
    }
    
    async fn extract_from_local_file(&self, path: &str, output_path: Option<&str>) -> Result<StreamingResult> {
        let output_file = if let Some(path) = output_path {
            PathBuf::from(path)
        } else {
            self.create_temp_file("local_audio", "wav").await?
        };
        
        let extraction_result = self.extract_audio_with_ffmpeg(path, &output_file).await?;
        let video_info = self.get_video_info_from_file(path).await.ok();
        
        Ok(StreamingResult {
            success: extraction_result.success,
            audio_path: extraction_result.audio_path,
            video_info,
            error: extraction_result.error,
            processing_time: None,
            stream_url: Some(path.to_string()),
            bytes_processed: None,
        })
    }
    
    async fn extract_audio_with_ffmpeg(&self, input: &str, output: &Path) -> Result<AudioExtractionResult> {
        let mut cmd = AsyncCommand::new("ffmpeg");
        cmd.args(&[
            "-i", input,
            "-vn", // No video
            "-acodec", "pcm_s16le", // PCM 16-bit
            "-ar", &self.config.processing.target_sample_rate.to_string(),
            "-ac", "1", // Mono
            "-y", // Overwrite
        ]);
        
        cmd.arg(output.to_str().unwrap());
        
        let ffmpeg_output = cmd.output().await
            .map_err(|e| DattavaniError::ffmpeg(format!("FFmpeg failed: {}", e)))?;
        
        if ffmpeg_output.status.success() {
            Ok(AudioExtractionResult {
                success: true,
                audio_path: Some(output.to_path_buf()),
                error: None,
                extraction_time: None,
                original_duration: None,
                extracted_duration: None,
                video_info: None,
            })
        } else {
            let error_msg = String::from_utf8_lossy(&ffmpeg_output.stderr);
            Ok(AudioExtractionResult {
                success: false,
                audio_path: None,
                error: Some(format!("FFmpeg error: {}", error_msg)),
                extraction_time: None,
                original_duration: None,
                extracted_duration: None,
                video_info: None,
            })
        }
    }
    
    async fn create_temp_file(&self, prefix: &str, extension: &str) -> Result<PathBuf> {
        let temp_dir = &self.config.processing.temp_dir;
        tokio::fs::create_dir_all(temp_dir).await
            .map_err(DattavaniError::FileIo)?;
        
        let filename = format!("{}_{}.{}", prefix, uuid::Uuid::new_v4(), extension);
        Ok(temp_dir.join(filename))
    }
    
    fn is_google_drive_url(&self, url: &str) -> bool {
        GDriveClient::is_google_drive_url(url)
    }
    
    fn is_youtube_url(&self, url: &str) -> bool {
        url.contains("youtube.com") || url.contains("youtu.be")
    }
    
    fn is_http_url(&self, url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }
}
