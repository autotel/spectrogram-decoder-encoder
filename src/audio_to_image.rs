use hound;
use image::{ImageBuffer, Rgb};
use rustfft::{FftPlanner, num_complex::Complex};
use std::path::{Path, PathBuf};

const FFT_SIZE: usize = 2048;
const HOP_SIZE: usize = 512;

pub fn audio_to_spectrogram(
    audio_path: &Path,
    output_path: &Path,
    use_log_scale: bool,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Read WAV file
    let mut reader = hound::WavReader::open(audio_path)?;
    let spec = reader.spec();

    println!("Audio format: {:?}, bits_per_sample: {}, sample_rate: {}, channels: {}",
             spec.sample_format, spec.bits_per_sample, spec.sample_rate, spec.channels);

    let mut samples: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Float, 32) => {
            reader.samples::<f32>()
                .map(|s| s.expect("Failed to read f32 sample"))
                .collect()
        }
        (hound::SampleFormat::Int, 8) => {
            reader.samples::<i8>()
                .map(|s| s.expect("Failed to read i8 sample") as f32 / i8::MAX as f32)
                .collect()
        }
        (hound::SampleFormat::Int, 16) => {
            reader.samples::<i16>()
                .map(|s| s.expect("Failed to read i16 sample") as f32 / i16::MAX as f32)
                .collect()
        }
        (hound::SampleFormat::Int, 24) => {
            reader.samples::<i32>()
                .map(|s| s.expect("Failed to read i32 (24-bit) sample") as f32 / 8388608.0) // 2^23
                .collect()
        }
        (hound::SampleFormat::Int, 32) => {
            reader.samples::<i32>()
                .map(|s| s.expect("Failed to read i32 sample") as f32 / i32::MAX as f32)
                .collect()
        }
        _ => {
            return Err(format!(
                "Unsupported audio format: {:?} with {} bits per sample",
                spec.sample_format, spec.bits_per_sample
            ).into());
        }
    };

    // Convert stereo to mono if needed
    if spec.channels == 2 {
        println!("Converting stereo to mono by averaging channels");
        let mono_samples: Vec<f32> = samples
            .chunks(2)
            .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
            .collect();
        samples = mono_samples;
    } else if spec.channels > 2 {
        return Err(format!("Only mono and stereo audio supported, got {} channels", spec.channels).into());
    }

    // Compute STFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);

    let num_frames = (samples.len() - FFT_SIZE) / HOP_SIZE + 1;
    let num_bins_linear = FFT_SIZE / 2 + 1; // Only positive frequencies (no mirror)

    let mut spectrogram_mag_linear = vec![vec![0.0f32; num_frames]; num_bins_linear];
    let mut spectrogram_phase_linear = vec![vec![0.0f32; num_frames]; num_bins_linear];

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
        for (bin, &value) in buffer.iter().take(num_bins_linear).enumerate() {
            spectrogram_mag_linear[bin][frame_idx] = value.norm();
            spectrogram_phase_linear[bin][frame_idx] = value.arg(); // Phase angle in radians
        }
    }

    // Apply frequency scale transformation if needed
    let (spectrogram_mag, spectrogram_phase, num_bins) = if use_log_scale {
        // Convert to logarithmic frequency scale
        let sample_rate = spec.sample_rate as f32;
        let nyquist = sample_rate / 2.0;
        let min_freq = 20.0; // Lower limit of human hearing
        let max_freq = nyquist;

        println!("Encoding log scale: nyquist={}, min_freq={}, max_freq={}", nyquist, min_freq, max_freq);
        println!("num_bins_linear={}, num_bins_log will be={}", num_bins_linear, num_bins_linear);

        // Use the same number of bins for consistency in image size
        let num_bins_log = num_bins_linear;

        let mut spectrogram_mag_log = vec![vec![0.0f32; num_frames]; num_bins_log];
        let mut spectrogram_phase_log = vec![vec![0.0f32; num_frames]; num_bins_log];

        // Create logarithmic frequency mapping
        for log_bin in 0..num_bins_log {
            // Calculate the frequency for this logarithmic bin
            let t = log_bin as f32 / (num_bins_log - 1) as f32;
            let freq_log = min_freq * (max_freq / min_freq).powf(t);

            // Convert frequency to linear bin (fractional)
            let bin_linear_float = freq_log / nyquist * (num_bins_linear - 1) as f32;

            // Debug: print mapping for 440 Hz region
            if freq_log >= 420.0 && freq_log <= 460.0 {
                println!("Forward: log_bin={} -> freq={:.1} Hz -> linear_bin={:.2}", log_bin, freq_log, bin_linear_float);
            }

            // Interpolate magnitude and phase from linear bins
            for frame_idx in 0..num_frames {
                let (mag, phase) = interpolate_spectrum(
                    &spectrogram_mag_linear,
                    &spectrogram_phase_linear,
                    bin_linear_float,
                    frame_idx,
                );
                spectrogram_mag_log[log_bin][frame_idx] = mag;
                spectrogram_phase_log[log_bin][frame_idx] = phase;
            }
        }

        (spectrogram_mag_log, spectrogram_phase_log, num_bins_log)
    } else {
        // Use linear frequency scale as-is
        (spectrogram_mag_linear, spectrogram_phase_linear, num_bins_linear)
    };
    
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
    
    // Save image with sample rate and scale mode in filename
    // Format: filename_SR{sample_rate}_LOG.png or filename_SR{sample_rate}_LIN.png
    let sample_rate = spec.sample_rate;
    let scale_suffix = if use_log_scale { "_LOG" } else { "_LIN" };
    let output_with_sr = if let Some(stem) = output_path.file_stem() {
        let parent = output_path.parent().unwrap_or(Path::new(""));
        parent.join(format!("{}_SR{}{}.png", stem.to_string_lossy(), sample_rate, scale_suffix))
    } else {
        output_path.to_path_buf()
    };

    img.save(&output_with_sr)?;
    Ok(output_with_sr)
}

// Interpolate magnitude and phase from linear spectrum at a fractional bin index
fn interpolate_spectrum(
    mag: &Vec<Vec<f32>>,
    phase: &Vec<Vec<f32>>,
    bin_float: f32,
    frame: usize,
) -> (f32, f32) {
    let bin_floor = bin_float.floor() as usize;
    let bin_ceil = (bin_float.ceil() as usize).min(mag.len() - 1);
    let frac = bin_float - bin_floor as f32;

    if bin_floor >= mag.len() {
        return (0.0, 0.0);
    }

    if bin_floor == bin_ceil {
        return (mag[bin_floor][frame], phase[bin_floor][frame]);
    }

    // Linear interpolation for magnitude
    let mag_interp = mag[bin_floor][frame] * (1.0 - frac) + mag[bin_ceil][frame] * frac;

    // Phase interpolation (handle wraparound)
    let phase1 = phase[bin_floor][frame];
    let phase2 = phase[bin_ceil][frame];
    let mut phase_diff = phase2 - phase1;

    // Wrap phase difference to [-π, π]
    while phase_diff > std::f32::consts::PI {
        phase_diff -= 2.0 * std::f32::consts::PI;
    }
    while phase_diff < -std::f32::consts::PI {
        phase_diff += 2.0 * std::f32::consts::PI;
    }

    let phase_interp = phase1 + phase_diff * frac;

    (mag_interp, phase_interp)
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
