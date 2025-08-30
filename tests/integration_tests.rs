/*!
Integration tests for Dattavani ASR
*/

use dattavani_asr::{Config, DattavaniAsr, VideoProcessor};
use std::path::Path;

#[tokio::test]
async fn test_config_loading() {
    let config = Config::default();
    assert_eq!(config.whisper.model_size, "large-v3");
    assert_eq!(config.processing.max_workers, 4);
}

#[tokio::test]
async fn test_video_processor_format_support() {
    let supported_video = VideoProcessor::supported_video_formats();
    assert!(supported_video.contains(&"mp4"));
    assert!(supported_video.contains(&"avi"));
    assert!(supported_video.contains(&"mov"));
    
    let supported_audio = VideoProcessor::supported_audio_formats();
    assert!(supported_audio.contains(&"mp3"));
    assert!(supported_audio.contains(&"wav"));
    assert!(supported_audio.contains(&"m4a"));
}

#[tokio::test]
async fn test_format_detection() {
    assert!(VideoProcessor::is_supported_format("test.mp4"));
    assert!(VideoProcessor::is_supported_format("test.mp3"));
    assert!(VideoProcessor::is_supported_format("test.wav"));
    assert!(!VideoProcessor::is_supported_format("test.txt"));
}

#[tokio::test]
#[ignore] // Requires actual credentials
async fn test_asr_initialization() {
    let config = Config::default();
    
    // This test requires actual Google credentials
    if std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok() {
        let result = DattavaniAsr::new(config).await;
        match result {
            Ok(_asr) => {
                // ASR initialized successfully
            }
            Err(e) => {
                eprintln!("ASR initialization failed (expected without credentials): {}", e);
            }
        }
    }
}

#[test]
fn test_error_types() {
    use dattavani_asr::DattavaniError;
    
    let auth_error = DattavaniError::authentication("test auth error");
    assert!(matches!(auth_error, DattavaniError::Authentication(_)));
    
    let video_error = DattavaniError::video_processing("test video error");
    assert!(matches!(video_error, DattavaniError::VideoProcessing(_)));
}

#[tokio::test]
async fn test_config_validation() {
    let mut config = Config::default();
    
    // Valid configuration should pass
    assert!(config.validate().is_ok());
    
    // Invalid model size should fail
    config.whisper.model_size = "invalid-model".to_string();
    assert!(config.validate().is_err());
    
    // Zero workers should fail
    config.whisper.model_size = "large-v3".to_string();
    config.processing.max_workers = 0;
    assert!(config.validate().is_err());
}
