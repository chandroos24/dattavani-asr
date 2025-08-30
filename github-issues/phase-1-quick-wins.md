# Phase 1: Quick Wins - Fix Current Performance Issues

## 🎯 **Objective**
Fix immediate performance bottlenecks and stability issues in the current Python CLI-based implementation to provide a stable foundation for future optimizations.

## 📋 **Tasks**

### 1. Fix Python CLI Integration Issues
- [ ] **Resolve Whisper CLI execution failures**
  - Fix "No such file or directory" errors
  - Ensure proper Python environment activation
  - Add robust error handling for CLI failures
  
- [ ] **Implement proper CLI command construction**
  ```rust
  // Current problematic approach
  Command::new("whisper").args(&[...])
  
  // Fixed approach with proper environment
  Command::new("python")
    .env("PATH", whisper_env_path)
    .args(&["-m", "whisper", ...])
  ```

### 2. Add Simple Fallback Mechanism
- [ ] **Create simplified transcription function**
  - Remove complex multi-model attempts for now
  - Use single, reliable model (base or small)
  - Add timeout handling (30-60 seconds)
  
- [ ] **Implement graceful degradation**
  ```rust
  pub async fn simple_transcribe(audio_path: &Path) -> Result<String> {
      // Try whisper-base first
      match try_whisper_base(audio_path).await {
          Ok(result) => Ok(result),
          Err(_) => {
              // Fallback to whisper-small
              try_whisper_small(audio_path).await
          }
      }
  }
  ```

### 3. Improve Error Handling
- [ ] **Add comprehensive error types**
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum TranscriptionError {
      #[error("Whisper CLI not found: {0}")]
      WhisperNotFound(String),
      
      #[error("Audio file processing failed: {0}")]
      AudioProcessingFailed(String),
      
      #[error("Transcription timeout after {seconds}s")]
      Timeout { seconds: u64 },
      
      #[error("Model loading failed: {model}")]
      ModelLoadFailed { model: String },
  }
  ```

- [ ] **Add retry mechanism with exponential backoff**
- [ ] **Implement proper logging for debugging**

### 4. Optimize Temporary File Management
- [ ] **Fix temp file cleanup issues**
  - Ensure all temporary files are cleaned up
  - Use RAII pattern for automatic cleanup
  - Add cleanup on process termination
  
- [ ] **Optimize temp file locations**
  ```rust
  pub struct TempFileManager {
      temp_dir: PathBuf,
      cleanup_on_drop: bool,
  }
  
  impl Drop for TempFileManager {
      fn drop(&mut self) {
          if self.cleanup_on_drop {
              let _ = std::fs::remove_dir_all(&self.temp_dir);
          }
      }
  }
  ```

### 5. Add Process Monitoring and Timeouts
- [ ] **Implement process timeout handling**
  ```rust
  use tokio::time::{timeout, Duration};
  
  pub async fn transcribe_with_timeout(
      audio_path: &Path, 
      timeout_secs: u64
  ) -> Result<String> {
      let future = transcribe_internal(audio_path);
      
      match timeout(Duration::from_secs(timeout_secs), future).await {
          Ok(result) => result,
          Err(_) => Err(TranscriptionError::Timeout { seconds: timeout_secs }),
      }
  }
  ```

- [ ] **Add progress monitoring for long-running transcriptions**
- [ ] **Implement graceful process termination**

## 🎯 **Acceptance Criteria**

### ✅ **Success Metrics**
- [ ] **Zero CLI execution failures** on supported audio formats
- [ ] **Consistent transcription results** across multiple runs
- [ ] **Proper error messages** instead of generic failures
- [ ] **No memory leaks** or orphaned processes
- [ ] **Cleanup of all temporary files** after processing
- [ ] **Timeout handling** prevents hanging processes
- [ ] **Comprehensive logging** for debugging issues

### 📊 **Performance Targets**
- [ ] **Startup time**: < 5 seconds (down from 10-20s)
- [ ] **Error recovery**: < 2 seconds for fallback attempts
- [ ] **Memory usage**: Stable (no growth over time)
- [ ] **Success rate**: > 95% for supported formats

## 🔧 **Implementation Details**

### **File Structure Changes**
```
src/asr/
├── mod.rs              # Main ASR interface
├── simple.rs           # New: Simplified transcription (THIS PHASE)
├── models.rs           # Existing: Complex model management
└── native_whisper.rs   # Future: Native implementation
```

### **Key Code Changes**

1. **Create `src/asr/simple.rs`**:
```rust
use std::path::Path;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

pub struct SimpleTranscriber {
    whisper_path: PathBuf,
    timeout_seconds: u64,
}

impl SimpleTranscriber {
    pub fn new() -> Result<Self> {
        let whisper_path = find_whisper_executable()?;
        Ok(Self {
            whisper_path,
            timeout_seconds: 300, // 5 minutes default
        })
    }
    
    pub async fn transcribe(&self, audio_path: &Path) -> Result<String> {
        let future = self.transcribe_internal(audio_path);
        
        timeout(Duration::from_secs(self.timeout_seconds), future)
            .await
            .map_err(|_| TranscriptionError::Timeout { 
                seconds: self.timeout_seconds 
            })?
    }
}
```

2. **Update `src/asr/mod.rs`** to use simple transcriber as fallback

3. **Add proper environment detection**:
```rust
fn find_whisper_executable() -> Result<PathBuf> {
    // Try different possible locations
    let candidates = [
        "whisper",
        "./whisper_env/bin/whisper",
        "/opt/homebrew/bin/whisper",
        which::which("whisper").ok(),
    ];
    
    for candidate in candidates.iter().flatten() {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }
    
    Err(TranscriptionError::WhisperNotFound(
        "Whisper CLI not found in PATH or common locations".to_string()
    ))
}
```

## 🧪 **Testing Requirements**

### **Unit Tests**
- [ ] Test CLI command construction
- [ ] Test error handling for various failure modes
- [ ] Test timeout functionality
- [ ] Test temp file cleanup

### **Integration Tests**
- [ ] Test with various audio formats (MP3, WAV, M4A)
- [ ] Test with different file sizes (small, medium, large)
- [ ] Test error recovery scenarios
- [ ] Test concurrent transcription requests

### **Performance Tests**
- [ ] Measure startup time improvements
- [ ] Verify memory usage stability
- [ ] Test timeout accuracy

## 📝 **Documentation Updates**
- [ ] Update README with troubleshooting guide
- [ ] Add environment setup instructions
- [ ] Document error codes and solutions
- [ ] Create debugging guide for CLI issues

## 🔗 **Dependencies**
- No new external dependencies required
- Uses existing tokio, anyhow, thiserror crates
- May add `which` crate for executable detection

## ⏱️ **Estimated Timeline**
- **Total**: 1-2 days
- **Setup & CLI fixes**: 4-6 hours
- **Error handling**: 2-3 hours  
- **Testing & validation**: 2-4 hours
- **Documentation**: 1-2 hours

## 🚀 **Next Phase**
After completion, this phase enables **Phase 2: Native Implementation** by providing a stable baseline for comparison and fallback.

---

**Labels**: `performance`, `bug-fix`, `priority-high`, `phase-1`
**Assignees**: Core development team
**Milestone**: Performance Optimization Phase 1
