/*!
Command Line Interface for Dattavani ASR

Provides CLI commands for single file and batch processing of audio and video files.
*/

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, error, warn};
use indicatif::{ProgressBar, ProgressStyle};

use crate::config::Config;
use crate::error::{DattavaniError, Result};
use crate::asr::DattavaniAsr;
use crate::streaming::StreamingProcessor;
use crate::video::VideoProcessor;

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
}

impl Cli {
    pub async fn run(config: Config) -> Result<()> {
        let cli = <Self as clap::Parser>::parse();
        
        // Set verbose logging if requested
        if cli.verbose {
            std::env::set_var("RUST_LOG", "debug");
        }
        
        match cli.command {
            Commands::StreamProcess { 
                input, 
                output, 
                language, 
                segment_duration 
            } => {
                Self::stream_process_single(config, input, output, language, segment_duration).await
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
        }
    }
    
    async fn stream_process_single(
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
}
