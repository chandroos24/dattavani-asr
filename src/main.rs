/*!
# Dattavani ASR - Rust Implementation

A high-performance Automatic Speech Recognition (ASR) system that processes audio and video files
from Google Drive using streaming-only approach and generates accurate transcriptions using
Whisper model. Built for scalable deployment on Google Cloud Platform with near 99% accuracy.

## Features

- **High Accuracy**: Uses Whisper large-v3 model for near 99% transcription accuracy
- **Video Support**: Processes 25+ video formats (MP4, AVI, MOV, MKV, WebM, FLV, WMV, etc.)
- **Audio Support**: Multiple audio formats (MP3, WAV, M4A, FLAC, OGG, WMA, AAC)
- **Streaming-Only Processing**: No downloads required - processes files directly from streams
- **Space Efficient**: Conserves storage by avoiding temporary file downloads
- **Google Drive Integration**: Seamlessly processes files from Google Drive using official API
- **Batch Processing**: Efficiently processes multiple files concurrently via streaming
- **Cloud-Native**: Designed for GCP deployment with high scalability

Author: Veteran AI/ML Engineer
Version: 1.0.0
*/

use anyhow::Result;
use std::env;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod asr;
mod cli;
mod config;
mod error;
mod gdrive;
mod streaming;
mod video;

use cli::Cli;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logging()?;
    
    info!("Starting Dattavani ASR v1.0.0");
    
    // Load configuration
    let config = Config::load().await
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
    
    // Set default Google Application Credentials if not set
    if env::var("GOOGLE_APPLICATION_CREDENTIALS").is_err() {
        let credentials_path = std::path::Path::new("service-account-key.json");
        if credentials_path.exists() {
            env::set_var("GOOGLE_APPLICATION_CREDENTIALS", credentials_path);
            info!("Using default service account key: service-account-key.json");
        }
    }
    
    // Run CLI
    match Cli::run(config).await {
        Ok(_) => {
            info!("Dattavani ASR completed successfully");
            Ok(())
        }
        Err(e) => {
            eprintln!("Dattavani ASR failed: {}", e);
            Err(anyhow::anyhow!("Dattavani ASR failed: {}", e))
        }
    }
}

fn init_logging() -> Result<()> {
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    
    let file_appender = tracing_appender::rolling::daily("logs", "dattavani-asr.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level))
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(true)
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .json()
        )
        .init();
    
    Ok(())
}
