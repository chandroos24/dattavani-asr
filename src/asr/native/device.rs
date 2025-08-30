/*!
Device Management for Native Implementation - Demo Version

Handles device selection and memory management for ML inference.
This is a simplified demonstration version.
*/

use std::fmt;
use tracing::{info, warn, debug};
use crate::error::{DattavaniError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum DeviceType {
    Cpu,
    Cuda(u32),  // GPU index
    Metal(u32), // GPU index
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceType::Cpu => write!(f, "CPU"),
            DeviceType::Cuda(idx) => write!(f, "CUDA:{}", idx),
            DeviceType::Metal(idx) => write!(f, "Metal:{}", idx),
        }
    }
}

pub struct DeviceManager {
    pub selected_device: DeviceType,
    pub memory_limit_gb: Option<f32>,
    pub allow_fallback: bool,
}

impl DeviceManager {
    pub fn new() -> Result<Self> {
        let selected_device = Self::auto_select_device()?;
        
        Ok(Self {
            selected_device,
            memory_limit_gb: None,
            allow_fallback: true,
        })
    }
    
    pub fn with_device(device_type: DeviceType) -> Result<Self> {
        info!("Creating device manager with specified device: {}", device_type);
        
        // Validate device availability
        if !Self::is_device_available(&device_type) {
            warn!("Specified device {} not available, falling back to CPU", device_type);
            return Ok(Self {
                selected_device: DeviceType::Cpu,
                memory_limit_gb: None,
                allow_fallback: true,
            });
        }
        
        Ok(Self {
            selected_device: device_type,
            memory_limit_gb: None,
            allow_fallback: true,
        })
    }
    
    pub fn with_memory_limit(mut self, limit_gb: f32) -> Self {
        self.memory_limit_gb = Some(limit_gb);
        self
    }
    
    pub fn with_fallback(mut self, allow_fallback: bool) -> Self {
        self.allow_fallback = allow_fallback;
        self
    }
    
    fn auto_select_device() -> Result<DeviceType> {
        info!("Auto-selecting optimal device for ML inference");
        
        // Check for CUDA availability (demo)
        if Self::is_cuda_available() {
            info!("CUDA GPU detected, using CUDA:0");
            return Ok(DeviceType::Cuda(0));
        }
        
        // Check for Metal availability (demo)
        if Self::is_metal_available() {
            info!("Metal GPU detected, using Metal:0");
            return Ok(DeviceType::Metal(0));
        }
        
        // Fallback to CPU
        info!("No GPU acceleration available, using CPU");
        Ok(DeviceType::Cpu)
    }
    
    fn is_device_available(device: &DeviceType) -> bool {
        match device {
            DeviceType::Cpu => true,
            DeviceType::Cuda(_) => Self::is_cuda_available(),
            DeviceType::Metal(_) => Self::is_metal_available(),
        }
    }
    
    fn is_cuda_available() -> bool {
        // In a real implementation, this would check for CUDA runtime
        // For demo, we'll simulate based on environment
        std::env::var("CUDA_VISIBLE_DEVICES").is_ok() || 
        std::path::Path::new("/usr/local/cuda").exists()
    }
    
    fn is_metal_available() -> bool {
        // In a real implementation, this would check for Metal support
        // For demo, we'll check if we're on macOS
        cfg!(target_os = "macos")
    }
    
    pub fn get_device_info(&self) -> DeviceInfo {
        match &self.selected_device {
            DeviceType::Cpu => DeviceInfo {
                device_type: self.selected_device.clone(),
                name: "CPU".to_string(),
                memory_total_gb: Self::get_system_memory_gb(),
                memory_available_gb: Self::get_available_memory_gb(),
                compute_capability: None,
            },
            DeviceType::Cuda(idx) => DeviceInfo {
                device_type: self.selected_device.clone(),
                name: format!("CUDA GPU {}", idx),
                memory_total_gb: 8.0, // Demo value
                memory_available_gb: 6.0, // Demo value
                compute_capability: Some("8.6".to_string()),
            },
            DeviceType::Metal(idx) => DeviceInfo {
                device_type: self.selected_device.clone(),
                name: format!("Metal GPU {}", idx),
                memory_total_gb: 16.0, // Demo value for unified memory
                memory_available_gb: 12.0, // Demo value
                compute_capability: None,
            },
        }
    }
    
    fn get_system_memory_gb() -> f32 {
        // Simple memory detection (demo)
        #[cfg(target_os = "macos")]
        {
            // Try to get memory info from system
            if let Ok(output) = std::process::Command::new("sysctl")
                .args(&["-n", "hw.memsize"])
                .output()
            {
                if let Ok(mem_str) = String::from_utf8(output.stdout) {
                    if let Ok(mem_bytes) = mem_str.trim().parse::<u64>() {
                        return mem_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    }
                }
            }
        }
        
        // Default fallback
        8.0
    }
    
    fn get_available_memory_gb() -> f32 {
        // Simplified available memory calculation
        Self::get_system_memory_gb() * 0.7 // Assume 70% available
    }
    
    pub fn check_memory_requirements(&self, required_gb: f32) -> Result<()> {
        let device_info = self.get_device_info();
        
        if device_info.memory_available_gb < required_gb {
            let msg = format!(
                "Insufficient memory on {}: required {:.1}GB, available {:.1}GB",
                device_info.name, required_gb, device_info.memory_available_gb
            );
            
            if self.allow_fallback && !matches!(self.selected_device, DeviceType::Cpu) {
                warn!("{}, falling back to CPU", msg);
                return Ok(());
            } else {
                return Err(DattavaniError::configuration(msg));
            }
        }
        
        info!("Memory check passed: {:.1}GB required, {:.1}GB available on {}", 
              required_gb, device_info.memory_available_gb, device_info.name);
        
        Ok(())
    }
    
    pub fn get_optimal_batch_size(&self, model_size_gb: f32) -> usize {
        let device_info = self.get_device_info();
        let available_memory = device_info.memory_available_gb - model_size_gb;
        
        if available_memory <= 0.0 {
            return 1;
        }
        
        // Estimate batch size based on available memory
        // This is a simplified calculation
        let estimated_batch_size = (available_memory / 0.5).floor() as usize; // 0.5GB per batch item
        
        match self.selected_device {
            DeviceType::Cpu => estimated_batch_size.min(4), // CPU is slower
            DeviceType::Cuda(_) => estimated_batch_size.min(32),
            DeviceType::Metal(_) => estimated_batch_size.min(16),
        }
    }
    
    pub fn warm_up(&self) -> Result<()> {
        info!("Warming up device: {}", self.selected_device);
        
        match &self.selected_device {
            DeviceType::Cpu => {
                debug!("CPU warm-up: no special initialization needed");
            },
            DeviceType::Cuda(_) => {
                debug!("CUDA warm-up: initializing GPU context (demo)");
                // In real implementation: initialize CUDA context
            },
            DeviceType::Metal(_) => {
                debug!("Metal warm-up: initializing Metal device (demo)");
                // In real implementation: initialize Metal device
            },
        }
        
        Ok(())
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            selected_device: DeviceType::Cpu,
            memory_limit_gb: None,
            allow_fallback: true,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_type: DeviceType,
    pub name: String,
    pub memory_total_gb: f32,
    pub memory_available_gb: f32,
    pub compute_capability: Option<String>,
}

impl fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:.1}GB/{:.1}GB)", 
               self.name, self.memory_available_gb, self.memory_total_gb)?;
        
        if let Some(ref capability) = self.compute_capability {
            write!(f, " [Compute: {}]", capability)?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_device_manager_creation() {
        let manager = DeviceManager::new().unwrap();
        assert!(matches!(manager.selected_device, DeviceType::Cpu | DeviceType::Cuda(_) | DeviceType::Metal(_)));
        assert!(manager.allow_fallback);
    }
    
    #[test]
    fn test_device_type_display() {
        assert_eq!(DeviceType::Cpu.to_string(), "CPU");
        assert_eq!(DeviceType::Cuda(0).to_string(), "CUDA:0");
        assert_eq!(DeviceType::Metal(1).to_string(), "Metal:1");
    }
    
    #[test]
    fn test_memory_limit() {
        let manager = DeviceManager::new().unwrap().with_memory_limit(4.0);
        assert_eq!(manager.memory_limit_gb, Some(4.0));
    }
    
    #[test]
    fn test_device_info() {
        let manager = DeviceManager::new().unwrap();
        let info = manager.get_device_info();
        
        assert!(!info.name.is_empty());
        assert!(info.memory_total_gb > 0.0);
        assert!(info.memory_available_gb > 0.0);
    }
    
    #[test]
    fn test_batch_size_calculation() {
        let manager = DeviceManager::with_device(DeviceType::Cpu).unwrap();
        let batch_size = manager.get_optimal_batch_size(1.0);
        
        assert!(batch_size > 0);
        assert!(batch_size <= 4); // CPU limit
    }
}
