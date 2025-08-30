/*!
Model Cache Management for Native Implementation - Demo Version

Handles model downloading, caching, and version management.
This is a simplified demonstration version.
*/

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tracing::{info, debug, warn};
use serde::{Deserialize, Serialize};

use crate::error::{DattavaniError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub model_id: String,
    pub cache_path: PathBuf,
    pub size_bytes: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

pub struct ModelCache {
    pub cache_dir: PathBuf,
    pub max_cache_size_gb: f32,
    pub entries: HashMap<String, CacheEntry>,
}

impl ModelCache {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir)
                .map_err(|e| DattavaniError::configuration(format!("Failed to create cache directory: {}", e)))?;
        }
        
        let mut cache = Self {
            cache_dir,
            max_cache_size_gb: 10.0, // Default 10GB cache limit
            entries: HashMap::new(),
        };
        
        // Load existing cache entries
        cache.load_cache_index()?;
        
        info!("Initialized model cache at: {:?}", cache.cache_dir);
        debug!("Cache contains {} entries", cache.entries.len());
        
        Ok(cache)
    }
    
    pub fn with_max_size(mut self, max_size_gb: f32) -> Self {
        self.max_cache_size_gb = max_size_gb;
        self
    }
    
    pub async fn get_or_download(&mut self, model_id: &str) -> Result<PathBuf> {
        info!("Getting model from cache: {}", model_id);
        
        // Check if model is already cached
        if let Some(entry) = self.entries.get(model_id) {
            if entry.cache_path.exists() {
                // Update last accessed time
                let cache_path = entry.cache_path.clone();
                if let Some(entry) = self.entries.get_mut(model_id) {
                    entry.last_accessed = chrono::Utc::now();
                }
                self.save_cache_index()?;
                
                info!("Model found in cache: {:?}", cache_path);
                return Ok(cache_path);
            } else {
                // Cache entry exists but files are missing
                warn!("Cache entry exists but files missing for: {}", model_id);
                self.entries.remove(model_id);
            }
        }
        
        // Download model (demo implementation)
        self.download_model(model_id).await
    }
    
    async fn download_model(&mut self, model_id: &str) -> Result<PathBuf> {
        info!("Downloading model: {} (DEMO MODE)", model_id);
        
        let model_cache_dir = self.cache_dir.join(model_id.replace("/", "_"));
        
        // Create model directory
        if !model_cache_dir.exists() {
            std::fs::create_dir_all(&model_cache_dir)
                .map_err(|e| DattavaniError::configuration(format!("Failed to create model cache dir: {}", e)))?;
        }
        
        // Simulate downloading model files
        self.create_demo_model_files(&model_cache_dir, model_id).await?;
        
        // Calculate total size
        let size_bytes = self.calculate_directory_size(&model_cache_dir).await?;
        
        // Create cache entry
        let entry = CacheEntry {
            model_id: model_id.to_string(),
            cache_path: model_cache_dir.clone(),
            size_bytes,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            version: "demo-1.0".to_string(),
        };
        
        self.entries.insert(model_id.to_string(), entry);
        self.save_cache_index()?;
        
        // Check cache size and cleanup if needed
        self.cleanup_if_needed().await?;
        
        info!("Model downloaded and cached: {:?}", model_cache_dir);
        Ok(model_cache_dir)
    }
    
    async fn create_demo_model_files(&self, cache_dir: &Path, model_id: &str) -> Result<()> {
        use tokio::fs;
        
        // Create demo config.json
        let config = serde_json::json!({
            "model_type": "whisper",
            "model_id": model_id,
            "vocab_size": 51865,
            "max_position_embeddings": 448,
            "d_model": 512,
            "encoder_layers": 6,
            "decoder_layers": 6,
            "encoder_attention_heads": 8,
            "decoder_attention_heads": 8,
            "demo_mode": true
        });
        
        let config_path = cache_dir.join("config.json");
        fs::write(&config_path, serde_json::to_string_pretty(&config)?).await
            .map_err(|e| DattavaniError::configuration(format!("Failed to write config: {}", e)))?;
        
        // Create demo tokenizer.json
        let tokenizer = serde_json::json!({
            "version": "1.0",
            "truncation": null,
            "padding": null,
            "added_tokens": [],
            "normalizer": null,
            "pre_tokenizer": null,
            "post_processor": null,
            "decoder": null,
            "model": {
                "type": "BPE",
                "vocab": {},
                "merges": []
            },
            "demo_mode": true
        });
        
        let tokenizer_path = cache_dir.join("tokenizer.json");
        fs::write(&tokenizer_path, serde_json::to_string_pretty(&tokenizer)?).await
            .map_err(|e| DattavaniError::configuration(format!("Failed to write tokenizer: {}", e)))?;
        
        // Create demo model weights file (empty for demo)
        let model_path = cache_dir.join("model.safetensors");
        fs::write(&model_path, b"DEMO_MODEL_WEIGHTS").await
            .map_err(|e| DattavaniError::configuration(format!("Failed to write model: {}", e)))?;
        
        debug!("Created demo model files in: {:?}", cache_dir);
        
        // Simulate download delay
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        Ok(())
    }
    
    async fn calculate_directory_size(&self, dir: &Path) -> Result<u64> {
        let mut total_size = 0u64;
        
        if dir.is_dir() {
            let mut entries = tokio::fs::read_dir(dir).await
                .map_err(|e| DattavaniError::configuration(format!("Failed to read directory: {}", e)))?;
            
            while let Some(entry) = entries.next_entry().await
                .map_err(|e| DattavaniError::configuration(format!("Failed to read directory entry: {}", e)))? {
                
                let metadata = entry.metadata().await
                    .map_err(|e| DattavaniError::configuration(format!("Failed to get metadata: {}", e)))?;
                
                if metadata.is_file() {
                    total_size += metadata.len();
                } else if metadata.is_dir() {
                    total_size += Box::pin(self.calculate_directory_size(&entry.path())).await?;
                }
            }
        }
        
        Ok(total_size)
    }
    
    async fn cleanup_if_needed(&mut self) -> Result<()> {
        let total_size_gb = self.get_total_cache_size_gb();
        
        if total_size_gb > self.max_cache_size_gb {
            info!("Cache size ({:.1}GB) exceeds limit ({:.1}GB), cleaning up", 
                  total_size_gb, self.max_cache_size_gb);
            
            // Sort entries by last accessed time (oldest first)
            let (entries_to_remove, total_removed_size): (Vec<String>, f32) = {
                let mut entries: Vec<_> = self.entries.iter().collect();
                entries.sort_by_key(|(_, e)| e.last_accessed);
                
                let mut to_remove = Vec::new();
                let mut removed_size = 0.0;
                let target_size = self.max_cache_size_gb * 0.8; // Clean up to 80% of limit
                
                for (model_id, entry) in entries {
                    if total_size_gb - removed_size <= target_size {
                        break;
                    }
                    
                    removed_size += entry.size_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    to_remove.push(model_id.clone());
                }
                
                (to_remove, removed_size)
            };
            
            // Remove the selected entries
            for model_id in entries_to_remove {
                if let Some(entry) = self.entries.remove(&model_id) {
                    info!("Removing cached model: {}", model_id);
                    
                    // Remove files
                    if entry.cache_path.exists() {
                        tokio::fs::remove_dir_all(&entry.cache_path).await
                            .map_err(|e| DattavaniError::configuration(format!("Failed to remove cache dir: {}", e)))?;
                    }
                }
            }
            
            self.save_cache_index()?;
            info!("Cache cleanup completed, removed {:.1}GB", total_removed_size);
        }
        
        Ok(())
    }
    
    fn get_total_cache_size_gb(&self) -> f32 {
        let total_bytes: u64 = self.entries.values().map(|e| e.size_bytes).sum();
        total_bytes as f32 / (1024.0 * 1024.0 * 1024.0)
    }
    
    fn load_cache_index(&mut self) -> Result<()> {
        let index_path = self.cache_dir.join("cache_index.json");
        
        if !index_path.exists() {
            debug!("No cache index found, starting with empty cache");
            return Ok(());
        }
        
        let content = std::fs::read_to_string(&index_path)
            .map_err(|e| DattavaniError::configuration(format!("Failed to read cache index: {}", e)))?;
        
        let entries: HashMap<String, CacheEntry> = serde_json::from_str(&content)
            .map_err(|e| DattavaniError::configuration(format!("Failed to parse cache index: {}", e)))?;
        
        // Verify that cached files still exist
        for (model_id, entry) in entries {
            if entry.cache_path.exists() {
                self.entries.insert(model_id, entry);
            } else {
                debug!("Cache entry {} points to missing files, skipping", model_id);
            }
        }
        
        debug!("Loaded {} cache entries", self.entries.len());
        Ok(())
    }
    
    fn save_cache_index(&self) -> Result<()> {
        let index_path = self.cache_dir.join("cache_index.json");
        
        let content = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| DattavaniError::configuration(format!("Failed to serialize cache index: {}", e)))?;
        
        std::fs::write(&index_path, content)
            .map_err(|e| DattavaniError::configuration(format!("Failed to write cache index: {}", e)))?;
        
        debug!("Saved cache index with {} entries", self.entries.len());
        Ok(())
    }
    
    pub fn clear_cache(&mut self) -> Result<()> {
        info!("Clearing entire model cache");
        
        for entry in self.entries.values() {
            if entry.cache_path.exists() {
                std::fs::remove_dir_all(&entry.cache_path)
                    .map_err(|e| DattavaniError::configuration(format!("Failed to remove cache dir: {}", e)))?;
            }
        }
        
        self.entries.clear();
        self.save_cache_index()?;
        
        info!("Cache cleared successfully");
        Ok(())
    }
    
    pub fn get_cache_info(&self) -> CacheInfo {
        let total_size_gb = self.get_total_cache_size_gb();
        let num_models = self.entries.len();
        
        let oldest_entry = self.entries.values()
            .min_by_key(|e| e.created_at)
            .map(|e| e.created_at);
        
        let newest_entry = self.entries.values()
            .max_by_key(|e| e.created_at)
            .map(|e| e.created_at);
        
        CacheInfo {
            cache_dir: self.cache_dir.clone(),
            total_size_gb,
            max_size_gb: self.max_cache_size_gb,
            num_models,
            oldest_entry,
            newest_entry,
        }
    }
    
    pub fn list_cached_models(&self) -> Vec<&CacheEntry> {
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by_key(|e| e.last_accessed);
        entries.reverse(); // Most recently accessed first
        entries
    }
}

#[derive(Debug)]
pub struct CacheInfo {
    pub cache_dir: PathBuf,
    pub total_size_gb: f32,
    pub max_size_gb: f32,
    pub num_models: usize,
    pub oldest_entry: Option<chrono::DateTime<chrono::Utc>>,
    pub newest_entry: Option<chrono::DateTime<chrono::Utc>>,
}

impl std::fmt::Display for CacheInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cache: {} models, {:.1}GB/{:.1}GB used", 
               self.num_models, self.total_size_gb, self.max_size_gb)?;
        
        if let Some(oldest) = self.oldest_entry {
            write!(f, ", oldest: {}", oldest.format("%Y-%m-%d"))?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_cache_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache = ModelCache::new(temp_dir.path().to_path_buf()).unwrap();
        
        assert_eq!(cache.entries.len(), 0);
        assert_eq!(cache.max_cache_size_gb, 10.0);
        assert!(cache.cache_dir.exists());
    }
    
    #[tokio::test]
    async fn test_demo_model_download() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = ModelCache::new(temp_dir.path().to_path_buf()).unwrap();
        
        let model_path = cache.get_or_download("openai/whisper-base").await.unwrap();
        
        assert!(model_path.exists());
        assert!(model_path.join("config.json").exists());
        assert!(model_path.join("tokenizer.json").exists());
        assert!(model_path.join("model.safetensors").exists());
        
        assert_eq!(cache.entries.len(), 1);
        assert!(cache.entries.contains_key("openai/whisper-base"));
    }
    
    #[tokio::test]
    async fn test_cache_info() {
        let temp_dir = TempDir::new().unwrap();
        let cache = ModelCache::new(temp_dir.path().to_path_buf()).unwrap();
        
        let info = cache.get_cache_info();
        assert_eq!(info.num_models, 0);
        assert_eq!(info.total_size_gb, 0.0);
        assert_eq!(info.max_size_gb, 10.0);
    }
}
