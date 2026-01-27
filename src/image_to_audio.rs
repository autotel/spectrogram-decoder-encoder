use image;
use hound;
use rustfft::{FftPlanner, num_complex::Complex};
use std::path::Path;

const HOP_SIZE: usize = 512;

pub fn spectrogram_to_audio(
    image_path: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Extract sample rate from filename (format: filename_SR44100.png)
    let sample_rate = if let Some(stem) = image_path.file_stem() {
        let stem_str = stem.to_string_lossy();
        if let Some(sr_pos) = stem_str.rfind("_SR") {
            let sr_str = &stem_str[sr_pos + 3..];
            sr_str.parse::<u32>().unwrap_or(44100)
        } else {
            44100 // Default fallback
        }
    } else {
        44100
    };
    
    // Read image
    let img = image::open(image_path)?.to_rgb8();
    let (width, height) = img.dimensions();
    
    let num_frames = width as usize;
    let num_bins = height as usize;
    
    // Calculate FFT size from number of bins (bins = FFT_SIZE/2 + 1, so FFT_SIZE = (bins-1)*2)
    let fft_size = (num_bins - 1) * 2;
    
    // Convert image to magnitude and phase spectrogram (decode HSV)
    let mut spectrogram_mag = vec![vec![0.0f32; num_frames]; num_bins];
    let mut spectrogram_phase = vec![vec![0.0f32; num_frames]; num_bins];
    
    for frame in 0..num_frames {
        for bin in 0..num_bins {
            // Flip vertically (high frequencies at top in image)
            let y = height - 1 - bin as u32;
            let pixel = img.get_pixel(frame as u32, y);
            
            // Convert RGB back to HSV
            let (h, _s, v) = rgb_to_hsv(pixel[0], pixel[1], pixel[2]);
            
            // Decode phase from hue [0, 360] to [-π, π]
            let phase = (h / 360.0) * 2.0 * std::f32::consts::PI - std::f32::consts::PI;
            
            // Decode magnitude from value (reverse the compression)
            // This must match the compression used in audio_to_image.rs
            
            // Reverse Option 1: Power law decompression (gamma = 1/0.3 = 3.333)
            let magnitude = v.powf(1.0 / 0.3);
            
            // Reverse Option 2: Double log decompression (uncomment if using Option 2)
            // let magnitude = ((1001.0f32.ln() * v.powf(2.0)).exp() - 1.0) / 100.0;
            
            // Reverse Option 3: Adaptive log decompression (uncomment if using Option 3)
            // let magnitude = ((1001.0f32.ln() * v).exp() - 1.0) / 1000.0;
            
            spectrogram_mag[bin][frame] = magnitude.max(0.0);
            spectrogram_phase[bin][frame] = phase;
        }
    }
    
    // Normalize magnitudes
    let max_val = spectrogram_mag.iter()
        .flat_map(|row| row.iter())
        .cloned()
        .fold(0.0f32, f32::max);
    
    if max_val > 0.0 {
        for row in spectrogram_mag.iter_mut() {
            for val in row.iter_mut() {
                *val /= max_val;
            }
        }
    }
    
    // Perform inverse STFT with decoded phase
    let mut planner = FftPlanner::new();
    let ifft = planner.plan_fft_inverse(fft_size);
    
    let output_len = (num_frames - 1) * HOP_SIZE + fft_size;
    let mut output = vec![0.0f32; output_len];
    let mut window_sum = vec![0.0f32; output_len];
    
    for frame_idx in 0..num_frames {
        let mut spectrum = vec![Complex::new(0.0, 0.0); fft_size];
        
        // Set magnitude and phase spectrum from decoded values
        for bin in 0..num_bins.min(fft_size / 2 + 1) {
            let magnitude = spectrogram_mag[bin][frame_idx];
            let phase = spectrogram_phase[bin][frame_idx];
            
            spectrum[bin] = Complex::new(
                magnitude * phase.cos(),
                magnitude * phase.sin(),
            );
        }
        
        // Mirror for negative frequencies (ensure real output)
        for bin in 1..num_bins.min(fft_size / 2) {
            spectrum[fft_size - bin] = spectrum[bin].conj();
        }
        
        ifft.process(&mut spectrum);
        
        // Overlap-add with Hann window
        let start = frame_idx * HOP_SIZE;
        for (i, &value) in spectrum.iter().take(fft_size).enumerate() {
            if start + i < output_len {
                let window = 0.5 * (1.0 - ((2.0 * std::f32::consts::PI * i as f32) / (fft_size as f32 - 1.0)).cos());
                output[start + i] += value.re * window / fft_size as f32;
                window_sum[start + i] += window;
            }
        }
    }
    
    // Normalize by window sum
    for i in 0..output_len {
        if window_sum[i] > 0.0 {
            output[i] /= window_sum[i];
        }
    }
    
    // Normalize output
    let max_sample = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    if max_sample > 0.0 {
        for sample in output.iter_mut() {
            *sample = (*sample / max_sample) * 0.95; // Leave some headroom
        }
    }
    
    // Write WAV file
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let mut writer = hound::WavWriter::create(output_path, spec)?;
    
    for &sample in output.iter() {
        let sample_i16 = (sample * i16::MAX as f32) as i16;
        writer.write_sample(sample_i16)?;
    }
    
    writer.finalize()?;
    Ok(())
}

// Convert RGB to HSV
fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    
    let h = if h < 0.0 { h + 360.0 } else { h };
    
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let v = max;
    
    (h, s, v)
}
