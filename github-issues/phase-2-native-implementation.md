# Phase 2: Native Implementation - Replace Python CLI with Rust

## 🎯 **Objective**
Replace the Python CLI dependency with a native Rust implementation using the Candle framework for 10-50x performance improvements and eliminate external dependencies.

## 📋 **Tasks**

### 1. Integrate Candle Framework
- [ ] **Add Candle dependencies to Cargo.toml**
  ```toml
  [dependencies]
  candle-core = "0.3"
  candle-nn = "0.3"
  candle-transformers = "0.3"
  candle-datasets = "0.3"
  hf-hub = "0.3"
  tokenizers = "0.14"
  safetensors = "0.4"
  
  # Audio processing
  symphonia = { version = "0.5", features = ["all"] }
  rubato = "0.14"  # For resampling
  
  # Optional GPU support
  candle-cuda = { version = "0.3", optional = true }
  candle-metal = { version = "0.3", optional = true }
  
  [features]
  default = []
  cuda = ["candle-cuda"]
  metal = ["candle-metal"]
  ```

### 2. Implement Native Whisper Model
- [ ] **Create core model structure**
  ```rust
  pub struct NativeWhisperModel {
      model: whisper::model::Whisper,
      tokenizer: Tokenizer,
      device: Device,
      config: Config,
      mel_filters: Tensor,
  }
  
  impl NativeWhisperModel {
      pub async fn load_from_hub(model_id: &str) -> Result<Self> {
          let device = Self::select_device()?;
          let api = hf_hub::api::tokio::Api::new()?;
          let repo = api.model(model_id.to_string());
          
          // Load model components
          let config = Self::load_config(&repo).await?;
          let tokenizer = Self::load_tokenizer(&repo).await?;
          let weights = Self::load_weights(&repo, &device).await?;
          let mel_filters = Self::create_mel_filters(&config, &device)?;
          
          let model = whisper::model::Whisper::load(&weights, config.clone())?;
          
          Ok(Self { model, tokenizer, device, config, mel_filters })
      }
  }
  ```

- [ ] **Implement model loading and caching**
  ```rust
  pub struct ModelCache {
      models: HashMap<String, Arc<NativeWhisperModel>>,
      cache_dir: PathBuf,
      max_models: usize,
  }
  
  impl ModelCache {
      pub async fn get_or_load(&mut self, model_id: &str) -> Result<Arc<NativeWhisperModel>> {
          if let Some(model) = self.models.get(model_id) {
              return Ok(model.clone());
          }
          
          // Load model and cache it
          let model = Arc::new(NativeWhisperModel::load_from_hub(model_id).await?);
          
          // Evict oldest model if cache is full
          if self.models.len() >= self.max_models {
              self.evict_oldest();
          }
          
          self.models.insert(model_id.to_string(), model.clone());
          Ok(model)
      }
  }
  ```

### 3. Add GPU Acceleration Support
- [ ] **Implement device selection logic**
  ```rust
  impl NativeWhisperModel {
      fn select_device() -> Result<Device> {
          // Try CUDA first
          #[cfg(feature = "cuda")]
          if let Ok(device) = Device::new_cuda(0) {
              info!("Using CUDA GPU acceleration");
              return Ok(device);
          }
          
          // Try Metal on macOS
          #[cfg(feature = "metal")]
          if let Ok(device) = Device::new_metal(0) {
              info!("Using Metal GPU acceleration");
              return Ok(device);
          }
          
          // Fallback to CPU
          info!("Using CPU (no GPU acceleration available)");
          Ok(Device::Cpu)
      }
  }
  ```

- [ ] **Add GPU memory management**
  ```rust
  pub struct GpuMemoryManager {
      device: Device,
      allocated_bytes: AtomicU64,
      max_memory: u64,
  }
  
  impl GpuMemoryManager {
      pub fn check_memory_available(&self, required_bytes: u64) -> Result<()> {
          let current = self.allocated_bytes.load(Ordering::Relaxed);
          if current + required_bytes > self.max_memory {
              return Err(DattavaniError::gpu_memory("Insufficient GPU memory"));
          }
          Ok(())
      }
  }
  ```

### 4. Create Model Persistence Layer
- [ ] **Implement model serialization**
  ```rust
  #[derive(Serialize, Deserialize)]
  pub struct SerializedModel {
      model_id: String,
      config: Config,
      weights_path: PathBuf,
      tokenizer_path: PathBuf,
      created_at: SystemTime,
      last_used: SystemTime,
  }
  
  pub struct ModelPersistence {
      cache_dir: PathBuf,
      metadata: HashMap<String, SerializedModel>,
  }
  
  impl ModelPersistence {
      pub async fn save_model(&mut self, model_id: &str, model: &NativeWhisperModel) -> Result<()> {
          let model_dir = self.cache_dir.join(model_id);
          tokio::fs::create_dir_all(&model_dir).await?;
          
          // Save weights and tokenizer
          let weights_path = model_dir.join("model.safetensors");
          let tokenizer_path = model_dir.join("tokenizer.json");
          
          model.save_weights(&weights_path).await?;
          model.save_tokenizer(&tokenizer_path).await?;
          
          // Update metadata
          self.metadata.insert(model_id.to_string(), SerializedModel {
              model_id: model_id.to_string(),
              config: model.config.clone(),
              weights_path,
              tokenizer_path,
              created_at: SystemTime::now(),
              last_used: SystemTime::now(),
          });
          
          self.save_metadata().await?;
          Ok(())
      }
  }
  ```

### 5. Implement Audio Preprocessing Pipeline
- [ ] **Create efficient audio loading**
  ```rust
  pub struct AudioProcessor {
      target_sample_rate: u32,
      resampler: Option<rubato::FftFixedInOut<f32>>,
  }
  
  impl AudioProcessor {
      pub async fn load_and_preprocess(&self, path: &Path) -> Result<Vec<f32>> {
          // Use symphonia for efficient audio decoding
          let file = std::fs::File::open(path)?;
          let mss = symphonia::core::io::MediaSourceStream::new(
              Box::new(file), 
              Default::default()
          );
          
          let mut format = symphonia::default::get_probe()
              .format(&Default::default(), mss, &Default::default(), &Default::default())?
              .format;
          
          let track = format.tracks()
              .iter()
              .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
              .ok_or_else(|| DattavaniError::audio_processing("No audio track found"))?;
          
          let mut decoder = symphonia::default::get_codecs()
              .make(&track.codec_params, &Default::default())?;
          
          let mut audio_data = Vec::new();
          
          // Decode audio packets
          loop {
              let packet = match format.next_packet() {
                  Ok(packet) => packet,
                  Err(symphonia::core::errors::Error::IoError(e)) 
                      if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                  Err(e) => return Err(DattavaniError::audio_processing(e.to_string())),
              };
              
              let decoded = decoder.decode(&packet)?;
              
              // Convert to f32 and append
              let samples = self.convert_to_f32(decoded)?;
              audio_data.extend(samples);
          }
          
          // Resample if needed
          if let Some(resampler) = &self.resampler {
              audio_data = resampler.process(&[audio_data], None)?[0].clone();
          }
          
          Ok(audio_data)
      }
  }
  ```

### 6. Create High-Level Transcription API
- [ ] **Implement main transcription interface**
  ```rust
  pub struct NativeTranscriber {
      model_cache: ModelCache,
      audio_processor: AudioProcessor,
      config: TranscriptionConfig,
  }
  
  impl NativeTranscriber {
      pub async fn transcribe(&mut self, audio_path: &Path, options: TranscriptionOptions) -> Result<TranscriptionResult> {
          let start_time = Instant::now();
          
          // Load model
          let model = self.model_cache.get_or_load(&options.model_id).await?;
          
          // Process audio
          let audio_data = self.audio_processor.load_and_preprocess(audio_path).await?;
          
          // Create mel spectrogram
          let mel_spectrogram = model.create_mel_spectrogram(&audio_data)?;
          
          // Run inference
          let tokens = model.model.forward(&mel_spectrogram)?;
          
          // Decode to text
          let text = model.decode_tokens(&tokens)?;
          
          Ok(TranscriptionResult {
              text,
              confidence: Some(self.calculate_confidence(&tokens)?),
              processing_time: start_time.elapsed().as_secs_f64(),
              model_used: options.model_id,
              language: options.language,
              segments: None, // TODO: Implement segment detection
          })
      }
  }
  ```

## 🎯 **Acceptance Criteria**

### ✅ **Functional Requirements**
- [ ] **Native Rust implementation** replaces Python CLI completely
- [ ] **Model loading** from HuggingFace Hub works reliably
- [ ] **GPU acceleration** works on CUDA and Metal devices
- [ ] **Model caching** prevents repeated downloads
- [ ] **Audio preprocessing** handles common formats (WAV, MP3, M4A, FLAC)
- [ ] **Transcription accuracy** matches or exceeds Python implementation
- [ ] **Memory usage** is efficient and predictable

### 📊 **Performance Targets**
- [ ] **Model loading**: < 0.5s (cached), < 30s (first load)
- [ ] **Inference time**: 2-5x faster than Python CLI
- [ ] **Memory usage**: < 1GB for base model
- [ ] **Startup time**: < 2s (vs 10-20s current)
- [ ] **Throughput**: > 10x real-time for base model

### 🔧 **Technical Requirements**
- [ ] **Cross-platform** support (Linux, macOS, Windows)
- [ ] **Optional GPU** features (graceful CPU fallback)
- [ ] **Thread safety** for concurrent requests
- [ ] **Error handling** with detailed error messages
- [ ] **Logging** with configurable levels

## 🔧 **Implementation Details**

### **New File Structure**
```
src/asr/
├── mod.rs                  # Updated main interface
├── simple.rs              # Phase 1 fallback
├── native/
│   ├── mod.rs             # Native implementation entry
│   ├── model.rs           # NativeWhisperModel
│   ├── cache.rs           # ModelCache and persistence
│   ├── audio.rs           # AudioProcessor
│   ├── device.rs          # GPU device management
│   └── transcriber.rs     # High-level API
└── models.rs              # Legacy model management (deprecated)
```

### **Configuration Updates**
```toml
# Add to dattavani-asr.toml
[native_models]
cache_dir = "./model_cache"
max_cached_models = 3
default_model = "openai/whisper-base"
gpu_memory_limit = "4GB"

[native_models.models]
base = { model_id = "openai/whisper-base", precision = "fp16" }
small = { model_id = "openai/whisper-small", precision = "fp32" }
large = { model_id = "openai/whisper-large-v3", precision = "fp16" }
```

## 🧪 **Testing Requirements**

### **Unit Tests**
- [ ] Model loading and caching
- [ ] Audio preprocessing pipeline
- [ ] GPU device selection
- [ ] Token decoding accuracy

### **Integration Tests**
- [ ] End-to-end transcription pipeline
- [ ] Model switching and caching
- [ ] GPU vs CPU performance comparison
- [ ] Memory usage under load

### **Performance Tests**
- [ ] Benchmark against Python CLI
- [ ] Memory leak detection
- [ ] Concurrent request handling
- [ ] Large file processing

### **Compatibility Tests**
- [ ] Various audio formats and sample rates
- [ ] Different model sizes and precisions
- [ ] Cross-platform functionality

## 📝 **Documentation Updates**
- [ ] Native implementation architecture guide
- [ ] GPU setup and troubleshooting
- [ ] Performance tuning guide
- [ ] Migration guide from Python CLI

## 🔗 **Dependencies**
### **New Major Dependencies**
- `candle-core`, `candle-nn`, `candle-transformers` - ML framework
- `hf-hub` - HuggingFace model downloads
- `symphonia` - Audio decoding
- `rubato` - Audio resampling
- `safetensors` - Model serialization

### **Optional Dependencies**
- `candle-cuda` - NVIDIA GPU support
- `candle-metal` - Apple GPU support

## ⏱️ **Estimated Timeline**
- **Total**: 1 week (5-7 days)
- **Candle integration**: 1-2 days
- **Model implementation**: 2-3 days
- **Audio processing**: 1-2 days
- **Testing & optimization**: 1-2 days

## 🚀 **Success Metrics**
- **10-50x faster** model loading (persistent vs reload)
- **2-5x faster** inference time
- **4-8x lower** memory usage
- **Zero Python dependencies**
- **Native GPU acceleration**

## 🔄 **Rollback Plan**
- Phase 1 simple transcriber remains as fallback
- Feature flag to switch between native and CLI implementations
- Gradual migration with A/B testing capability

---

**Labels**: `performance`, `enhancement`, `priority-high`, `phase-2`, `native-rust`
**Assignees**: Senior Rust developers
**Milestone**: Performance Optimization Phase 2
**Dependencies**: Requires Phase 1 completion
