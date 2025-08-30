# Simplified Dattavani ASR Architecture

## Current Problem
```
Rust Program → Python Whisper CLI → PyTorch/Whisper Models
```
This creates unnecessary complexity and compatibility issues.

## Recommended Solutions (in order of preference)

### Option 1: Direct HTTP API to Whisper Service
```
Rust Program → HTTP API → Whisper Service (Docker/Cloud)
```

**Benefits:**
- No Python dependencies in Rust
- Scalable and containerized
- Easy to deploy and maintain
- Language agnostic

**Implementation:**
```rust
// Direct HTTP calls to Whisper API
async fn transcribe_via_api(audio_path: &Path) -> Result<String> {
    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new()
        .file("audio", audio_path).await?;
    
    let response = client
        .post("http://whisper-service:8000/transcribe")
        .multipart(form)
        .send()
        .await?;
    
    let result: TranscriptionResult = response.json().await?;
    Ok(result.text)
}
```

### Option 2: WebAssembly Whisper
```
Rust Program → WASM Whisper Module
```

**Benefits:**
- Pure Rust/WASM
- No external dependencies
- Portable across platforms

### Option 3: Simplified CLI Wrapper
```
Rust Program → Optimized CLI Wrapper → whisper-cpp
```

**Benefits:**
- Uses whisper.cpp (C++) instead of Python
- Much faster and lighter
- Single binary deployment

### Option 4: Remove Whisper Dependency Entirely
```
Rust Program → Cloud Speech APIs (Google/AWS/Azure)
```

**Benefits:**
- No local model dependencies
- Highly accurate
- Supports many languages
- Auto-scaling

## Immediate Fix for Current Issue

Let's fix the current Python CLI issue by:

1. **Removing complex model configurations**
2. **Using a simple, working Whisper setup**
3. **Adding proper error handling and fallbacks**

### Quick Fix Implementation

```rust
// Simplified transcription without complex model management
pub async fn simple_transcribe(audio_path: &Path, language: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("whisper");
    cmd.arg(audio_path);
    cmd.args(&["--model", "base"]);  // Use simple base model
    cmd.args(&["--output_format", "txt"]);
    cmd.args(&["--output_dir", "/tmp"]);
    
    if let Some(lang) = language {
        cmd.args(&["--language", lang]);
    }
    
    let output = cmd.output().await?;
    
    if output.status.success() {
        // Read the generated text file
        let txt_file = format!("/tmp/{}.txt", 
            audio_path.file_stem().unwrap().to_str().unwrap());
        let text = tokio::fs::read_to_string(txt_file).await?;
        Ok(text.trim().to_string())
    } else {
        Err(anyhow::anyhow!("Whisper failed: {}", 
            String::from_utf8_lossy(&output.stderr)))
    }
}
```

## Recommended Next Steps

1. **Immediate**: Fix the current Python CLI issues
2. **Short-term**: Implement Option 1 (HTTP API)
3. **Long-term**: Consider Option 2 (WASM) or Option 4 (Cloud APIs)

This approach eliminates the complex multi-language dependency chain while maintaining functionality.
