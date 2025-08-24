/*!
Error handling module for Dattavani ASR

Provides comprehensive error types and handling for all components of the system.
*/

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DattavaniError {
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    #[error("Google Drive API error: {0}")]
    GoogleDrive(String),
    
    #[error("Google Cloud Storage error: {0}")]
    GoogleCloudStorage(String),
    
    #[error("Audio processing error: {0}")]
    AudioProcessing(String),
    
    #[error("Video processing error: {0}")]
    VideoProcessing(String),
    
    #[error("Streaming error: {0}")]
    Streaming(String),
    
    #[error("ASR processing error: {0}")]
    AsrProcessing(String),
    
    #[error("Whisper model error: {0}")]
    WhisperModel(String),
    
    #[error("File I/O error: {0}")]
    FileIo(#[from] std::io::Error),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Concurrent processing error: {0}")]
    Concurrency(String),
    
    #[error("Memory error: {0}")]
    Memory(String),
    
    #[error("FFmpeg error: {0}")]
    Ffmpeg(String),
    
    #[error("URL parsing error: {0}")]
    UrlParsing(#[from] url::ParseError),
    
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
    
    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, DattavaniError>;

impl DattavaniError {
    pub fn authentication<S: Into<String>>(msg: S) -> Self {
        Self::Authentication(msg.into())
    }
    
    pub fn google_drive<S: Into<String>>(msg: S) -> Self {
        Self::GoogleDrive(msg.into())
    }
    
    pub fn google_cloud_storage<S: Into<String>>(msg: S) -> Self {
        Self::GoogleCloudStorage(msg.into())
    }
    
    pub fn audio_processing<S: Into<String>>(msg: S) -> Self {
        Self::AudioProcessing(msg.into())
    }
    
    pub fn video_processing<S: Into<String>>(msg: S) -> Self {
        Self::VideoProcessing(msg.into())
    }
    
    pub fn streaming<S: Into<String>>(msg: S) -> Self {
        Self::Streaming(msg.into())
    }
    
    pub fn asr_processing<S: Into<String>>(msg: S) -> Self {
        Self::AsrProcessing(msg.into())
    }
    
    pub fn whisper_model<S: Into<String>>(msg: S) -> Self {
        Self::WhisperModel(msg.into())
    }
    
    pub fn configuration<S: Into<String>>(msg: S) -> Self {
        Self::Configuration(msg.into())
    }
    
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Self::Validation(msg.into())
    }
    
    pub fn unsupported_format<S: Into<String>>(format: S) -> Self {
        Self::UnsupportedFormat(format.into())
    }
    
    pub fn timeout<S: Into<String>>(msg: S) -> Self {
        Self::Timeout(msg.into())
    }
    
    pub fn concurrency<S: Into<String>>(msg: S) -> Self {
        Self::Concurrency(msg.into())
    }
    
    pub fn memory<S: Into<String>>(msg: S) -> Self {
        Self::Memory(msg.into())
    }
    
    pub fn ffmpeg<S: Into<String>>(msg: S) -> Self {
        Self::Ffmpeg(msg.into())
    }
    
    pub fn unknown<S: Into<String>>(msg: S) -> Self {
        Self::Unknown(msg.into())
    }
    
    pub fn file_io(err: std::io::Error) -> Self {
        Self::FileIo(err)
    }
    
    pub fn network(err: reqwest::Error) -> Self {
        Self::Network(err)
    }
    
    pub fn serialization(err: serde_json::Error) -> Self {
        Self::Serialization(err)
    }
}

/// Convenience macro for creating errors
#[macro_export]
macro_rules! dattavani_error {
    ($kind:ident, $($arg:tt)*) => {
        $crate::error::DattavaniError::$kind(format!($($arg)*))
    };
}

/// Convenience macro for creating results
#[macro_export]
macro_rules! dattavani_bail {
    ($kind:ident, $($arg:tt)*) => {
        return Err($crate::error::DattavaniError::$kind(format!($($arg)*)))
    };
}
