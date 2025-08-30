/*!
Command Line Interface for Dattavani ASR

Provides CLI commands for single file and batch processing of audio and video files.
*/

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use tracing::{info, error, warn};
use indicatif::{ProgressBar, ProgressStyle};

use crate::config::Config;
use crate::error::{DattavaniError, Result};
use crate::asr::DattavaniAsr;
use crate::streaming::StreamingProcessor;
use crate::video::VideoProcessor;

pub mod models;
pub use models::{ModelsArgs, handle_models_command};

#[derive(Parser)]
#[command(name = "dattavani-asr")]
#[command(about = "High-performance Automatic Speech Recognition with Google Drive integration")]
#[command(version = "1.0.0")]
#[command(author = "Veteran AI/ML Engineer")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    #[arg(short, long, global = true)]
    pub verbose: bool,
    
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Simple transcription without complex model management
    SimpleTranscribe {
        /// Input audio file path
        input: String,
        
        /// Language code (e.g., en, es, fr, hi)
        #[arg(short, long)]
        language: Option<String>,
        
        /// Whisper model to use (base, small, medium, large)
        #[arg(short, long, default_value = "base")]
        model: String,
    },
    
    /// Native Rust transcription using Candle framework (Phase 2)
    NativeTranscribe {
        /// Input audio file path
        input: String,
        
        /// Model ID from HuggingFace Hub
        #[arg(short, long, default_value = "openai/whisper-base")]
        model_id: String,
        
        /// Language code (e.g., en, es, fr, hi)
        #[arg(short, long)]
        language: Option<String>,
        
        /// Use FP16 precision for faster inference
        #[arg(long, default_value = "true")]
        fp16: bool,
        
        /// Temperature for sampling (0.0 = greedy)
        #[arg(long, default_value = "0.0")]
        temperature: f32,
    },
    
    /// Process a single audio or video file via streaming
    StreamProcess {
        /// Input file URI (gs://, https://drive.google.com/, or local path)
        input: String,
        
        /// Output file URI (optional, defaults to gen-transcript folder)
        #[arg(short, long)]
        output: Option<String>,
        
        /// Language code (e.g., en, es, fr, hi)
        #[arg(short, long)]
        language: Option<String>,
        
        /// Segment duration for large files (seconds)
        #[arg(long, default_value = "300")]
        segment_duration: u64,
    },
    
    /// Capture video segment with start/end time and process ASR
    CaptureAndProcess {
        /// Input video file URI (gs://, https://drive.google.com/, or local path)
        input: String,
        
        /// Start time in HH:MM:SS format
        #[arg(long)]
        start_time: String,
        
        /// End time in HH:MM:SS format  
        #[arg(long)]
        end_time: String,
        
        /// Video title for output filename
        #[arg(long)]
        title: String,
        
        /// Language code (e.g., en, es, fr, hi)
        #[arg(short, long)]
        language: Option<String>,
        
        /// Output folder (optional, defaults to /Volumes/ssd1/video-capture on macOS)
        #[arg(long)]
        output_folder: Option<String>,
    },
    
    /// Process multiple files in a folder via streaming
    StreamBatch {
        /// Input folder URI (gs:// or local path)
        folder: String,
        
        /// Output folder URI (optional)
        #[arg(short, long)]
        output: Option<String>,
        
        /// Language code (e.g., en, es, fr, hi)
        #[arg(short, long)]
        language: Option<String>,
        
        /// Maximum number of concurrent workers
        #[arg(long, default_value = "4")]
        max_workers: usize,
        
        /// File pattern to match (e.g., "*.mp4")
        #[arg(long)]
        pattern: Option<String>,
    },
    
    /// Analyze a video/audio file without processing
    AnalyzeStream {
        /// Input file URI
        input: String,
    },
    
    /// List supported audio and video formats
    SupportedFormats,
    
    /// Health check for the system
    HealthCheck,
    
    /// Test Google Drive authentication
    TestAuth,
    
    /// Generate configuration file template
    GenerateConfig {
        /// Output path for config file
        #[arg(short, long, default_value = "dattavani-asr.toml")]
        output: PathBuf,
    },
    
    /// Manage ASR models
    Models(ModelsArgs),
}

impl Cli {
    pub async fn run(config: Config) -> Result<()> {
        let cli = <Self as clap::Parser>::parse();
        
        // Set verbose logging if requested
        if cli.verbose {
            std::env::set_var("RUST_LOG", "debug");
        }
        
        match cli.command {
            Commands::SimpleTranscribe { 
                input, 
                language, 
                model 
            } => {
                Self::simple_transcribe(input, language, model).await
            }
            
            Commands::NativeTranscribe {
                input,
                model_id,
                language,
                fp16,
                temperature,
            } => {
                Self::native_transcribe(input, model_id, language, fp16, temperature).await
            }
            
            Commands::StreamProcess { 
                input, 
                output, 
                language, 
                segment_duration 
            } => {
                Self::process_stream(config, input, output, language, segment_duration).await
            }
            
            Commands::CaptureAndProcess {
                input,
                start_time,
                end_time,
                title,
                language,
                output_folder,
            } => {
                Self::capture_and_process(config, input, start_time, end_time, title, language, output_folder).await
            }
            
            Commands::StreamBatch { 
                folder, 
                output, 
                language, 
                max_workers, 
                pattern 
            } => {
                Self::stream_process_batch(config, folder, output, language, max_workers, pattern).await
            }
            
            Commands::AnalyzeStream { input } => {
                Self::analyze_stream(config, input).await
            }
            
            Commands::SupportedFormats => {
                Self::show_supported_formats().await
            }
            
            Commands::HealthCheck => {
                Self::health_check(config).await
            }
            
            Commands::TestAuth => {
                Self::test_auth(config).await
            }
            
            Commands::GenerateConfig { output } => {
                Self::generate_config(config, output).await
            }
            
            Commands::Models(models_args) => {
                handle_models_command(models_args, config).await
            }
        }
    }
    
    async fn simple_transcribe(
        input: String,
        language: Option<String>,
        model: String,
    ) -> Result<()> {
        use crate::asr::simple::{SimpleTranscriber, SimpleTranscriptionOptions};
        use std::path::Path;
        
        info!("🎤 Starting simple transcription");
        info!("📁 Input: {}", input);
        info!("🌍 Language: {:?}", language);
        info!("🤖 Model: {}", model);
        
        let input_path = Path::new(&input);
        
        if !input_path.exists() {
            error!("❌ Input file does not exist: {}", input);
            return Err(DattavaniError::validation("Input file not found"));
        }
        
        let transcriber = SimpleTranscriber::new().await?;
        
        let options = SimpleTranscriptionOptions {
            model,
            language,
            timeout_seconds: 300, // 5 minutes
            output_format: "txt".to_string(),
            verbose: false,
        };
        
        let progress = ProgressBar::new_spinner();
        progress.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
        );
        progress.set_message("Transcribing audio...");
        
        match transcriber.transcribe(input_path, Some(options)).await {
            Ok(result) => {
                progress.finish_with_message("Transcription completed!");
                
                if result.success {
                    println!("✅ Transcription completed!");
                    if let Some(text) = &result.text {
                        println!("📝 Text: {}", text);
                        
                        // Save to file
                        let output_file = format!("{}.txt", 
                            input_path.file_stem().unwrap().to_str().unwrap());
                        tokio::fs::write(&output_file, text).await?;
                        info!("💾 Saved to: {}", output_file);
                    }
                    
                    if let Some(processing_time) = result.processing_time {
                        info!("⏱️  Processing time: {:.2}s", processing_time);
                    }
                    info!("🤖 Model used: {}", result.model_used);
                } else {
                    error!("❌ Transcription failed: {}", result.error.as_deref().unwrap_or("Unknown error"));
                    return Err(DattavaniError::asr_processing(result.error.unwrap_or_default()));
                }
                
                Ok(())
            }
            Err(e) => {
                progress.finish_with_message("Transcription failed!");
                error!("❌ Transcription failed: {}", e);
                Err(e)
            }
        }
    }
    
    async fn native_transcribe(
        input: String,
        model_id: String,
        language: Option<String>,
        fp16: bool,
        temperature: f32,
    ) -> Result<()> {
        #[cfg(feature = "native")]
        {
            use crate::asr::native::{NativeTranscriber, NativeTranscriptionOptions, TranscriptionTask};
            use std::path::Path;
            
            info!("🚀 Starting native transcription (Phase 2)");
            info!("📁 Input: {}", input);
            info!("🤖 Model: {}", model_id);
            info!("🌍 Language: {:?}", language);
            info!("⚡ FP16: {}", fp16);
            info!("🌡️ Temperature: {}", temperature);
            
            let input_path = Path::new(&input);
            
            if !input_path.exists() {
                error!("❌ Input file does not exist: {}", input);
                return Err(DattavaniError::validation("Input file not found"));
            }
            
            let progress = ProgressBar::new_spinner();
            progress.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} [{elapsed_precise}] {msg}")
                    .unwrap()
            );
            progress.set_message("Initializing native transcriber...");
            
            let mut transcriber = NativeTranscriber::new().await?;
            
            let options = NativeTranscriptionOptions {
                model_id: model_id.clone(),
                language: language.clone(),
                task: TranscriptionTask::Transcribe,
                temperature,
                fp16,
                ..Default::default()
            };
            
            progress.set_message("Loading model and transcribing...");
            
            match transcriber.transcribe(input_path, options).await {
                Ok(result) => {
                    progress.finish_with_message("Native transcription completed!");
                    
                    println!("✅ Native transcription completed!");
                    println!("📝 Text: {}", result.text);
                    
                    // Save to file
                    let output_file = format!("{}_native.txt", 
                        input_path.file_stem().unwrap().to_str().unwrap());
                    tokio::fs::write(&output_file, &result.text).await?;
                    info!("💾 Saved to: {}", output_file);
                    
                    info!("⏱️  Processing time: {:.2}s", result.processing_time);
                    info!("🤖 Model used: {}", result.model_used);
                    
                    if let Some(confidence) = result.confidence {
                        info!("🎯 Confidence: {:.2}%", confidence * 100.0);
                    }
                    
                    if let Some(lang) = result.language {
                        info!("🌍 Language: {}", lang);
                    }
                    
                    Ok(())
                }
                Err(e) => {
                    progress.finish_with_message("Native transcription failed!");
                    error!("❌ Native transcription failed: {}", e);
                    Err(e)
                }
            }
        }
        
        #[cfg(not(feature = "native"))]
        {
            let _ = (input, model_id, language, fp16, temperature);
            error!("❌ Native transcription not available. Enable 'native' feature or use simple-transcribe.");
            Err(DattavaniError::configuration(
                "Native implementation not available. Use 'simple-transcribe' command instead."
            ))
        }
    }
    
    async fn process_stream(
        config: Config,
        input: String,
        output: Option<String>,
        language: Option<String>,
        segment_duration: u64,
    ) -> Result<()> {
        info!("Starting stream processing for: {}", input);
        
        let progress = ProgressBar::new_spinner();
        progress.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
        );
        progress.set_message("Initializing ASR processor...");
        
        let asr = DattavaniAsr::new(config.clone()).await?;
        
        progress.set_message("Processing file...");
        let result = asr.stream_process_single_file(
            &input,
            output.as_deref(),
            language.as_deref(),
            Some(segment_duration),
        ).await?;
        
        progress.finish_with_message("Processing complete!");
        
        if result.success {
            info!("✅ Transcription successful!");
            info!("📝 Text: {}", result.text.as_deref().unwrap_or("No text"));
            info!("⏱️  Processing time: {:.2}s", result.processing_time.unwrap_or(0.0));
            if let Some(confidence) = result.confidence {
                info!("🎯 Confidence: {:.2}%", confidence * 100.0);
            }
            if let Some(lang) = result.language {
                info!("🌍 Detected language: {}", lang);
            }
        } else {
            error!("❌ Transcription failed: {}", result.error.as_deref().unwrap_or("Unknown error"));
            return Err(DattavaniError::asr_processing(result.error.unwrap_or_default()));
        }
        
        Ok(())
    }
    
    async fn stream_process_batch(
        config: Config,
        folder: String,
        output: Option<String>,
        language: Option<String>,
        max_workers: usize,
        pattern: Option<String>,
    ) -> Result<()> {
        info!("Starting batch stream processing for folder: {}", folder);
        
        let asr = DattavaniAsr::new(config.clone()).await?;
        
        let results = asr.stream_process_batch(
            &folder,
            output.as_deref(),
            language.as_deref(),
            max_workers,
            pattern.as_deref(),
        ).await?;
        
        let total = results.total_files;
        let successful = results.successful;
        let failed = results.failed;
        
        info!("📊 Batch processing complete!");
        info!("✅ Successful: {}/{}", successful, total);
        if failed > 0 {
            warn!("❌ Failed: {}", failed);
        }
        
        // Show detailed results
        for (i, result) in results.results.iter().enumerate() {
            if result.success {
                info!("  {}. ✅ Processing time: {:.2}s", 
                     i + 1, result.processing_time.unwrap_or(0.0));
            } else {
                error!("  {}. ❌ Error: {}", 
                      i + 1, result.error.as_deref().unwrap_or("Unknown"));
            }
        }
        
        Ok(())
    }
    
    async fn analyze_stream(config: Config, input: String) -> Result<()> {
        info!("Analyzing stream: {}", input);
        
        let processor = StreamingProcessor::new(config)?;
        let info = processor.analyze_stream(&input).await?;
        
        info!("📹 Stream Analysis Results:");
        info!("  Duration: {:.2} seconds", info.duration);
        info!("  Resolution: {}x{}", info.width, info.height);
        info!("  FPS: {:.2}", info.fps);
        info!("  Video Codec: {}", info.video_codec);
        if let Some(audio_codec) = info.audio_codec {
            info!("  Audio Codec: {}", audio_codec);
        }
        if let Some(sample_rate) = info.audio_sample_rate {
            info!("  Sample Rate: {} Hz", sample_rate);
        }
        if let Some(channels) = info.audio_channels {
            info!("  Audio Channels: {}", channels);
        }
        info!("  File Size: {} bytes", info.file_size);
        info!("  Format: {}", info.format_name);
        
        Ok(())
    }
    
    async fn show_supported_formats() -> Result<()> {
        info!("🎵 Supported Audio Formats:");
        let audio_formats = VideoProcessor::supported_audio_formats();
        for format in audio_formats {
            info!("  • {}", format);
        }
        
        info!("\n🎬 Supported Video Formats:");
        let video_formats = VideoProcessor::supported_video_formats();
        for format in video_formats {
            info!("  • {}", format);
        }
        
        info!("\n🌍 Supported Languages:");
        let languages = [
            "en (English)", "es (Spanish)", "fr (French)", "de (German)",
            "hi (Hindi)", "zh (Chinese)", "ja (Japanese)", "ar (Arabic)",
            "ru (Russian)", "pt (Portuguese)", "it (Italian)", "ko (Korean)",
            "nl (Dutch)", "sv (Swedish)", "da (Danish)", "no (Norwegian)",
            "fi (Finnish)", "pl (Polish)", "cs (Czech)", "sk (Slovak)",
            "hu (Hungarian)", "ro (Romanian)", "bg (Bulgarian)", "hr (Croatian)",
            "sl (Slovenian)", "et (Estonian)", "lv (Latvian)", "lt (Lithuanian)",
            "mt (Maltese)", "ga (Irish)", "cy (Welsh)", "eu (Basque)",
            "ca (Catalan)", "gl (Galician)", "is (Icelandic)", "mk (Macedonian)",
            "sq (Albanian)", "sr (Serbian)", "bs (Bosnian)", "me (Montenegrin)",
        ];
        
        for lang in languages {
            info!("  • {}", lang);
        }
        
        Ok(())
    }
    
    async fn health_check(config: Config) -> Result<()> {
        info!("🏥 Running health check...");
        
        // Check configuration
        info!("  ✅ Configuration loaded successfully");
        
        // Check Google credentials
        match crate::gdrive::GDriveClient::new(config.clone()).await {
            Ok(_) => info!("  ✅ Google Drive authentication successful"),
            Err(e) => {
                error!("  ❌ Google Drive authentication failed: {}", e);
                return Err(e);
            }
        }
        
        // Check Whisper model
        match DattavaniAsr::new(config.clone()).await {
            Ok(_) => info!("  ✅ Whisper model loaded successfully"),
            Err(e) => {
                error!("  ❌ Whisper model loading failed: {}", e);
                return Err(e);
            }
        }
        
        // Check FFmpeg
        match VideoProcessor::check_ffmpeg().await {
            Ok(_) => info!("  ✅ FFmpeg available"),
            Err(e) => {
                warn!("  ⚠️  FFmpeg check failed: {}", e);
            }
        }
        
        // Check temp directory
        let temp_dir = &config.processing.temp_dir;
        if let Err(e) = tokio::fs::create_dir_all(temp_dir).await {
            error!("  ❌ Cannot create temp directory {}: {}", temp_dir.display(), e);
            return Err(DattavaniError::configuration(format!("Temp directory error: {}", e)));
        } else {
            info!("  ✅ Temp directory accessible: {}", temp_dir.display());
        }
        
        info!("🎉 All health checks passed!");
        Ok(())
    }
    
    async fn test_auth(config: Config) -> Result<()> {
        info!("🔐 Testing Google authentication...");
        
        let client = crate::gdrive::GDriveClient::new(config).await?;
        let user_info = client.get_user_info().await?;
        
        info!("✅ Authentication successful!");
        info!("  User: {}", user_info.display_name);
        info!("  Email: {}", user_info.email_address);
        
        Ok(())
    }
    
    async fn generate_config(config: Config, output: PathBuf) -> Result<()> {
        info!("📝 Generating configuration file: {}", output.display());
        
        config.save_to_file(output.to_str().unwrap()).await?;
        
        info!("✅ Configuration file generated successfully!");
        info!("💡 Edit the file to customize your settings, then set CONFIG_FILE environment variable");
        
        Ok(())
    }
    
    async fn capture_and_process(
        config: Config,
        input: String,
        start_time: String,
        end_time: String,
        title: String,
        language: Option<String>,
        output_folder: Option<String>,
    ) -> Result<()> {
        use chrono::{DateTime, Utc};
        use std::path::Path;
        
        info!("🎬 Starting video capture and ASR processing");
        info!("📁 Input: {}", input);
        info!("⏰ Start time: {}", start_time);
        info!("⏰ End time: {}", end_time);
        info!("📝 Title: {}", title);
        
        // Validate time format
        if !Self::validate_time_format(&start_time) || !Self::validate_time_format(&end_time) {
            return Err(DattavaniError::validation("Invalid time format. Use HH:MM:SS"));
        }
        
        // Set default output folder for macOS
        let base_folder = output_folder.unwrap_or_else(|| "/Volumes/ssd1/video-capture".to_string());
        
        // Create timestamp for folder name
        let now: DateTime<Utc> = Utc::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        let folder_name = format!("{}-{}", title, timestamp);
        let output_dir = Path::new(&base_folder).join(&folder_name);
        
        // Create output directory
        tokio::fs::create_dir_all(&output_dir).await
            .map_err(|e| DattavaniError::file_io(e))?;
        
        info!("📁 Created output directory: {}", output_dir.display());
        
        let progress = ProgressBar::new_spinner();
        progress.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
        );
        
        // Step 1: Capture video segment
        progress.set_message("Capturing video segment...");
        let video_filename = format!("{}-{}.mp4", title, timestamp);
        let video_path = output_dir.join(&video_filename);
        
        Self::capture_video_segment(&input, &start_time, &end_time, &video_path).await?;
        info!("✅ Video captured: {}", video_path.display());
        
        // Step 2: Extract audio as MP3
        progress.set_message("Extracting audio...");
        let audio_filename = format!("{}-{}.mp3", title, timestamp);
        let audio_path = output_dir.join(&audio_filename);
        
        Self::extract_audio_as_mp3(&video_path, &audio_path).await?;
        info!("✅ Audio extracted: {}", audio_path.display());
        
        // Step 3: Perform ASR
        progress.set_message("Performing speech recognition...");
        let asr = DattavaniAsr::new(config.clone()).await?;
        
        let transcript_path = output_dir.join(format!("{}-{}.txt", title, timestamp));
        let result = asr.stream_process_single_file(
            audio_path.to_str().unwrap(),
            Some(transcript_path.to_str().unwrap()),
            language.as_deref(),
            None,
        ).await?;
        
        progress.finish_with_message("Processing complete!");
        
        if result.success {
            info!("✅ ASR processing successful!");
            info!("📝 Transcript saved: {}", transcript_path.display());
            if let Some(text) = &result.text {
                info!("📄 Preview: {}", &text[..text.len().min(200)]);
            }
            info!("⏱️  Processing time: {:.2}s", result.processing_time.unwrap_or(0.0));
            if let Some(confidence) = result.confidence {
                info!("🎯 Confidence: {:.2}%", confidence * 100.0);
            }
        } else {
            error!("❌ ASR processing failed: {}", result.error.as_deref().unwrap_or("Unknown error"));
            return Err(DattavaniError::asr_processing(result.error.unwrap_or_default()));
        }
        
        info!("🎉 All files saved in: {}", output_dir.display());
        Ok(())
    }
    
    fn validate_time_format(time_str: &str) -> bool {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 3 {
            return false;
        }
        
        for part in &parts {
            if part.parse::<u32>().is_err() {
                return false;
            }
        }
        
        let hours: u32 = parts[0].parse().unwrap_or(25);
        let minutes: u32 = parts[1].parse().unwrap_or(61);
        let seconds: u32 = parts[2].parse().unwrap_or(61);
        
        hours < 24 && minutes < 60 && seconds < 60
    }
    
    async fn capture_video_segment(
        input: &str,
        start_time: &str,
        end_time: &str,
        output_path: &Path,
    ) -> Result<()> {
        use tokio::process::Command;
        
        // Check if input is a YouTube URL
        if input.contains("youtube.com") || input.contains("youtu.be") {
            // For YouTube, use yt-dlp to stream directly to ffmpeg
            let mut cmd = Command::new("yt-dlp");
            cmd.args([
                "--quiet",
                "--no-warnings",
                "-f", "best[ext=mp4]",
                "--get-url",
                input,
            ]);
            
            let output = cmd.output().await
                .map_err(|e| DattavaniError::ffmpeg(format!("yt-dlp execution failed: {}", e)))?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(DattavaniError::ffmpeg(format!("yt-dlp failed: {}", stderr)));
            }
            
            let stream_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            
            // Now use ffmpeg with the stream URL
            let mut ffmpeg_cmd = Command::new("ffmpeg");
            ffmpeg_cmd.args([
                "-i", &stream_url,
                "-ss", start_time,
                "-to", end_time,
                "-c", "copy",
                "-avoid_negative_ts", "make_zero",
                "-y",
                output_path.to_str().unwrap(),
            ]);
            
            let ffmpeg_output = ffmpeg_cmd.output().await
                .map_err(|e| DattavaniError::ffmpeg(format!("FFmpeg execution failed: {}", e)))?;
            
            if !ffmpeg_output.status.success() {
                let stderr = String::from_utf8_lossy(&ffmpeg_output.stderr);
                return Err(DattavaniError::ffmpeg(format!("FFmpeg failed: {}", stderr)));
            }
        } else {
            // Original logic for local files and other URLs
            let mut cmd = Command::new("ffmpeg");
            cmd.args([
                "-i", input,
                "-ss", start_time,
                "-to", end_time,
                "-c", "copy",
                "-avoid_negative_ts", "make_zero",
                "-y",
                output_path.to_str().unwrap(),
            ]);
            
            let output = cmd.output().await
                .map_err(|e| DattavaniError::ffmpeg(format!("FFmpeg execution failed: {}", e)))?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(DattavaniError::ffmpeg(format!("FFmpeg failed: {}", stderr)));
            }
        }
        
        Ok(())
    }
    
    async fn extract_audio_as_mp3(
        video_path: &Path,
        audio_path: &Path,
    ) -> Result<()> {
        use tokio::process::Command;
        
        let mut cmd = Command::new("ffmpeg");
        cmd.args([
            "-i", video_path.to_str().unwrap(),
            "-vn", // No video
            "-acodec", "mp3",
            "-ab", "192k", // Audio bitrate
            "-ar", "44100", // Sample rate
            "-y", // Overwrite output file
            audio_path.to_str().unwrap(),
        ]);
        
        let output = cmd.output().await
            .map_err(|e| DattavaniError::ffmpeg(format!("FFmpeg execution failed: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DattavaniError::ffmpeg(format!("FFmpeg failed: {}", stderr)));
        }
        
        Ok(())
    }
}
