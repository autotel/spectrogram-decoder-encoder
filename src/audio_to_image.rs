use hound;
use image::{ImageBuffer, Rgb};
use rustfft::{FftPlanner, num_complex::Complex};
use std::path::{Path, PathBuf};

const FFT_SIZE: usize = 2048;
const HOP_SIZE: usize = 512;

pub fn audio_to_spectrogram(
    audio_path: &Path,
    output_path: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Read WAV file
    let mut reader = hound::WavReader::open(audio_path)?;
    let spec = reader.spec();
    
    let samples: Vec<f32> = if spec.sample_format == hound::SampleFormat::Float {
        reader.samples::<f32>().map(|s| s.unwrap()).collect()
    } else {
        reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / i16::MAX as f32)
            .collect()
    };
    
    // Compute STFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    
    let num_frames = (samples.len() - FFT_SIZE) / HOP_SIZE + 1;
    let num_bins = FFT_SIZE / 2 + 1; // Only positive frequencies (no mirror)
    
    let mut spectrogram_mag = vec![vec![0.0f32; num_frames]; num_bins];
    let mut spectrogram_phase = vec![vec![0.0f32; num_frames]; num_bins];
    
    for frame_idx in 0..num_frames {
        let start = frame_idx * HOP_SIZE;
        let end = start + FFT_SIZE;
        
        if end > samples.len() {
            break;
        }
        
        // Apply Hann window and prepare FFT input
        let mut buffer: Vec<Complex<f32>> = samples[start..end]
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let window = 0.5 * (1.0 - ((2.0 * std::f32::consts::PI * i as f32) / (FFT_SIZE as f32 - 1.0)).cos());
                Complex::new(s * window, 0.0)
            })
            .collect();
        
        fft.process(&mut buffer);
        
        // Store magnitude and phase spectrum (only positive frequencies)
        for (bin, &value) in buffer.iter().take(num_bins).enumerate() {
            spectrogram_mag[bin][frame_idx] = value.norm();
            spectrogram_phase[bin][frame_idx] = value.arg(); // Phase angle in radians
        }
    }
    
    // Convert to HSV image (Hue = phase, Saturation = 1, Value = magnitude)
    let width = num_frames as u32;
    let height = num_bins as u32;
    
    println!("Creating spectrogram image: {}x{} (width x height)", width, height);
    println!("FFT_SIZE: {}, num_bins: {}", FFT_SIZE, num_bins);
    
    let mut img = ImageBuffer::new(width, height);
    
    // Find max magnitude for normalization
    let max_val = spectrogram_mag.iter()
        .flat_map(|row| row.iter())
        .cloned()
        .fold(0.0f32, f32::max);
    
    for (bin, mag_row) in spectrogram_mag.iter().enumerate() {
        for (frame, &magnitude) in mag_row.iter().enumerate() {
            let phase = spectrogram_phase[bin][frame];
            
            // Aggressive dynamic range compression for visibility
            let value = if max_val > 0.0 {
                let normalized = magnitude / max_val;
                
                // Option 1: Power law compression (gamma = 0.3 makes quiet parts much brighter)
                let compressed = normalized.powf(0.3);
                
                // Option 2: Double log for extreme compression (uncomment to use)
                // let compressed = ((1.0 + normalized * 100.0).ln() / (101.0f32.ln())).powf(0.5);
                
                // Option 3: Adaptive log scale (good balance)
                // let compressed = (1.0 + normalized * 1000.0).ln() / (1001.0f32.ln());
                
                compressed.min(1.0)
            } else {
                0.0
            };
            
            // Convert phase from [-π, π] to [0, 360] degrees
            let hue = ((phase + std::f32::consts::PI) / (2.0 * std::f32::consts::PI) * 360.0) % 360.0;
            
            // Convert HSV to RGB
            let rgb = hsv_to_rgb(hue, 1.0, value);
            
            // Flip vertically (high frequencies at top)
            let y = height - 1 - bin as u32;
            img.put_pixel(frame as u32, y, Rgb(rgb));
        }
    }
    
    // Save image with sample rate in metadata
    // We'll encode it in the filename for simplicity: filename_SR{sample_rate}.png
    let sample_rate = spec.sample_rate;
    let output_with_sr = if let Some(stem) = output_path.file_stem() {
        let parent = output_path.parent().unwrap_or(Path::new(""));
        parent.join(format!("{}_SR{}.png", stem.to_string_lossy(), sample_rate))
    } else {
        output_path.to_path_buf()
    };
    
    img.save(&output_with_sr)?;
    Ok(output_with_sr)
}

// Convert HSV to RGB
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;
    
    let (r, g, b) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    
    [
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    ]
}
