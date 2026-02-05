use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrogramConfig {
    /// FFT window size - affects frequency resolution
    /// Larger = better frequency resolution, worse time resolution
    #[serde(default = "default_fft_size")]
    pub fft_size: usize,
    
    /// Hop size - distance between consecutive FFT windows
    /// Smaller = more time resolution (wider images), more overlap
    #[serde(default = "default_hop_size")]
    pub hop_size: usize,
    
    /// Minimum frequency for logarithmic scale (Hz)
    #[serde(default = "default_min_freq")]
    pub min_freq: f32,
    
    /// Minimum dB level for visualization
    #[serde(default = "default_db_min")]
    pub db_min: f32,
    
    /// Maximum dB level for visualization
    #[serde(default = "default_db_max")]
    pub db_max: f32,
    
    /// High-frequency boost starting frequency (Hz)
    #[serde(default = "default_boost_start_freq")]
    pub boost_start_freq: f32,
    
    /// High-frequency boost amount (dB per octave)
    #[serde(default = "default_boost_db_per_octave")]
    pub boost_db_per_octave: f32,
    
    /// Whether to use color (hue) to encode/decode phase information
    /// true = color encodes phase (perfect reconstruction)
    /// false = grayscale magnitude only (phase lost, but easier to edit visually)
    #[serde(default = "default_use_phase_encoding")]
    pub use_phase_encoding: bool,
    
    /// Whether to use logarithmic frequency scale (musical/note-based)
    /// true = logarithmic scale (better for music, notes equally spaced)
    /// false = linear scale (better for technical analysis)
    #[serde(default = "default_use_log_scale")]
    pub use_log_scale: bool,
    
    /// Number of Griffin-Lim iterations for magnitude-only reconstruction
    /// Only used when use_phase_encoding = false
    /// More iterations = better quality but slower (typical: 10-50)
    #[serde(default = "default_griffin_lim_iterations")]
    pub griffin_lim_iterations: usize,
}

// Default values - now with higher time resolution
fn default_fft_size() -> usize { 4096 }
fn default_hop_size() -> usize { 128 }  // Changed from 512 to 128 for 4x time resolution
fn default_min_freq() -> f32 { 20.0 }
fn default_db_min() -> f32 { -80.0 }
fn default_db_max() -> f32 { 0.0 }
fn default_boost_start_freq() -> f32 { 1000.0 }
fn default_boost_db_per_octave() -> f32 { 6.0 }
fn default_use_phase_encoding() -> bool { true }
fn default_use_log_scale() -> bool { true }  // Default to log scale for music
fn default_griffin_lim_iterations() -> usize { 30 }  // 30 iterations is a good balance

impl Default for SpectrogramConfig {
    fn default() -> Self {
        Self {
            fft_size: default_fft_size(),
            hop_size: default_hop_size(),
            min_freq: default_min_freq(),
            db_min: default_db_min(),
            db_max: default_db_max(),
            boost_start_freq: default_boost_start_freq(),
            boost_db_per_octave: default_boost_db_per_octave(),
            use_phase_encoding: default_use_phase_encoding(),
            use_log_scale: default_use_log_scale(),
            griffin_lim_iterations: default_griffin_lim_iterations(),
        }
    }
}

impl SpectrogramConfig {
    const CONFIG_FILE: &'static str = "spectrogram_config.toml";
    
    /// Load configuration from file, creating/updating it if needed
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Path::new(Self::CONFIG_FILE);
        
        if config_path.exists() {
            // Try to load existing config
            let contents = fs::read_to_string(config_path)?;
            let mut config: Self = toml::from_str(&contents)
                .unwrap_or_else(|e| {
                    eprintln!("Warning: Error parsing config file: {}. Using defaults.", e);
                    Self::default()
                });
            
            // Ensure all fields have valid values by re-serializing with defaults
            config.validate_and_fix();
            
            // Save back to update any missing fields
            config.save()?;
            
            Ok(config)
        } else {
            // Create new config with defaults
            let config = Self::default();
            config.save()?;
            println!("Created new configuration file: {}", Self::CONFIG_FILE);
            Ok(config)
        }
    }
    
    /// Save configuration to file
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let toml_string = toml::to_string_pretty(self)?;
        fs::write(Self::CONFIG_FILE, toml_string)?;
        Ok(())
    }
    
    /// Validate and fix any invalid values
    fn validate_and_fix(&mut self) {
        // Ensure FFT size is a power of 2 and reasonable
        if self.fft_size < 256 || self.fft_size > 16384 || !self.fft_size.is_power_of_two() {
            eprintln!("Warning: Invalid fft_size {}, using default", self.fft_size);
            self.fft_size = default_fft_size();
        }
        
        // Ensure hop size is reasonable (not larger than FFT size)
        if self.hop_size == 0 || self.hop_size > self.fft_size {
            eprintln!("Warning: Invalid hop_size {}, using default", self.hop_size);
            self.hop_size = default_hop_size();
        }
        
        // Ensure frequency range is valid
        if self.min_freq <= 0.0 || self.min_freq >= 20000.0 {
            eprintln!("Warning: Invalid min_freq {}, using default", self.min_freq);
            self.min_freq = default_min_freq();
        }
        
        // Ensure dB range is valid
        if self.db_min >= self.db_max {
            eprintln!("Warning: Invalid dB range, using defaults");
            self.db_min = default_db_min();
            self.db_max = default_db_max();
        }
    }
    
    /// Print current configuration
    pub fn print_info(&self) {
        println!("\n=== Spectrogram Configuration ===");
        println!("FFT Size: {} samples", self.fft_size);
        println!("Hop Size: {} samples", self.hop_size);
        println!("Overlap: {:.1}%", (1.0 - self.hop_size as f32 / self.fft_size as f32) * 100.0);
        println!("Frequency range (log scale): {:.0} Hz - Nyquist", self.min_freq);
        println!("Dynamic range: {} to {} dB", self.db_min, self.db_max);
        println!("HF Boost: {} dB/octave above {} Hz", self.boost_db_per_octave, self.boost_start_freq);
        println!("Phase Encoding: {}", if self.use_phase_encoding { "Enabled (color)" } else { "Disabled (grayscale)" });
        println!("Frequency Scale: {}", if self.use_log_scale { "Logarithmic (musical)" } else { "Linear (technical)" });
        println!("=================================\n");
    }
}
