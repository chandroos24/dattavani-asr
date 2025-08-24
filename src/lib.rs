/*!
# Dattavani ASR Library

High-performance Automatic Speech Recognition library for Rust.

This library provides streaming audio/video processing with Google Drive integration
and Whisper-based transcription capabilities.

## Example Usage

```rust
use dattavani_asr::{DattavaniAsr, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().await?;
    let asr = DattavaniAsr::new(config).await?;
    
    let result = asr.stream_process_single_file(
        "gs://bucket/audio.mp3",
        None,
        Some("en"),
        None,
    ).await?;
    
    if result.success {
        println!("Transcription: {}", result.text.unwrap_or_default());
    }
    
    Ok(())
}
```
*/

pub mod asr;
pub mod cli;
pub mod config;
pub mod error;
pub mod gdrive;
pub mod streaming;
pub mod video;

// Re-export main types for convenience
pub use asr::{DattavaniAsr, TranscriptionResult, BatchResult, TranscriptionSegment};
pub use config::Config;
pub use error::{DattavaniError, Result};
pub use gdrive::{GDriveClient, DriveFile, UserInfo};
pub use streaming::{StreamingProcessor, StreamingResult};
pub use video::{VideoProcessor, VideoInfo, AudioExtractionResult};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Initialize the library with default configuration
pub async fn init() -> Result<DattavaniAsr> {
    let config = Config::load().await?;
    DattavaniAsr::new(config).await
}

/// Initialize the library with custom configuration
pub async fn init_with_config(config: Config) -> Result<DattavaniAsr> {
    DattavaniAsr::new(config).await
}
