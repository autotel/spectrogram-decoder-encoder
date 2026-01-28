use image;
use hound;
use rustfft::{FftPlanner, num_complex::Complex};
use std::path::Path;

const HOP_SIZE: usize = 512;

pub fn spectrogram_to_audio(
    image_path: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Extract sample rate and scale mode from filename (format: filename_SR44100_LOG.png or filename_SR44100_LIN.png)
    let (sample_rate, use_log_scale) = if let Some(stem) = image_path.file_stem() {
        let stem_str = stem.to_string_lossy();
        println!("Filename stem: {}", stem_str);
        let sample_rate = if let Some(sr_pos) = stem_str.rfind("_SR") {
            let after_sr = &stem_str[sr_pos + 3..];
            // Extract just the number part (before _LOG or _LIN if present)
            let sr_str: String = after_sr.chars().take_while(|c| c.is_numeric()).collect();
            let parsed_sr = sr_str.parse::<u32>().unwrap_or(44100);
            println!("Extracted sample rate from filename: {} (from string: '{}')", parsed_sr, sr_str);
            parsed_sr
        } else {
            println!("No _SR found in filename, using default 44100");
            44100 // Default fallback
        };

        let use_log_scale = stem_str.contains("_LOG");
        println!("use_log_scale: {}", use_log_scale);

        (sample_rate, use_log_scale)
    } else {
        (44100, false)
    };
    
    // Read image
    let img = image::open(image_path)?.to_rgb8();
    let (width, height) = img.dimensions();

    let num_frames = width as usize;
    let num_bins_image = height as usize;

    // Calculate FFT size from number of bins (bins = FFT_SIZE/2 + 1, so FFT_SIZE = (bins-1)*2)
    let fft_size = (num_bins_image - 1) * 2;
    let num_bins_linear = fft_size / 2 + 1;

    // Convert image to magnitude and phase spectrogram (decode HSV)
    let mut spectrogram_mag_image = vec![vec![0.0f32; num_frames]; num_bins_image];
    let mut spectrogram_phase_image = vec![vec![0.0f32; num_frames]; num_bins_image];
    
    for frame in 0..num_frames {
        for bin in 0..num_bins_image {
            // Flip vertically (high frequencies at top in image)
            let y = height - 1 - bin as u32;
            let pixel = img.get_pixel(frame as u32, y);

            // Convert RGB back to HSV
            let (h, _s, v) = rgb_to_hsv(pixel[0], pixel[1], pixel[2]);

            // Decode phase from hue [0, 360] to [-π, π]
            let phase = (h / 360.0) * 2.0 * std::f32::consts::PI - std::f32::consts::PI;

            // Calculate frequency for this bin to reverse the boost
            let bin_freq = if use_log_scale {
                let nyquist = sample_rate as f32 / 2.0;
                let min_freq = 20.0;
                let t = bin as f32 / (num_bins_image - 1) as f32;
                min_freq * (nyquist / min_freq).powf(t)
            } else {
                let nyquist = sample_rate as f32 / 2.0;
                (bin as f32 / (num_bins_image - 1) as f32) * nyquist
            };

            // Reverse the high-frequency boost
            let boost_db = if bin_freq > 1000.0 {
                6.0 * (bin_freq / 1000.0).log2()
            } else {
                0.0
            };

            // Decode magnitude from value (reverse the dB scale and boost)
            let db_min = -80.0;
            let db_max = 0.0;
            let db = v * (db_max - db_min) + db_min;
            let db_without_boost = db - boost_db; // Remove the boost
            let magnitude = 10.0f32.powf(db_without_boost / 20.0);

            spectrogram_mag_image[bin][frame] = magnitude.max(0.0);
            spectrogram_phase_image[bin][frame] = phase;
        }
    }

    // Apply inverse frequency scale transformation if needed
    let (mut spectrogram_mag, spectrogram_phase) = if use_log_scale {
        // Convert from logarithmic frequency scale back to linear
        let nyquist = sample_rate as f32 / 2.0;
        let min_freq = 20.0;
        let max_freq = nyquist;

        println!("Decoding log scale: nyquist={}, min_freq={}, max_freq={}", nyquist, min_freq, max_freq);
        println!("num_bins_image={}, num_bins_linear={}", num_bins_image, num_bins_linear);

        let mut spectrogram_mag_linear = vec![vec![0.0f32; num_frames]; num_bins_linear];
        let mut spectrogram_phase_linear = vec![vec![0.0f32; num_frames]; num_bins_linear];

        // Map from log bins back to linear bins
        for linear_bin in 0..num_bins_linear {
            // Calculate the frequency for this linear bin
            let freq_linear = (linear_bin as f32 / (num_bins_linear - 1) as f32) * nyquist;

            // Find the corresponding logarithmic bin (fractional)
            let log_bin_float = if freq_linear <= min_freq {
                0.0
            } else {
                // Use log base 2 to compute t from frequency
                let t = (freq_linear / min_freq).ln() / (max_freq / min_freq).ln();
                t * (num_bins_image - 1) as f32
            };

            // Debug: print mapping for 440 Hz region
            if freq_linear >= 200.0 && freq_linear <= 500.0 && linear_bin % 5 == 0 {
                println!("Inverse: linear_bin={} -> freq={:.1} Hz -> log_bin={:.2}", linear_bin, freq_linear, log_bin_float);
            }

            // Interpolate magnitude and phase from log bins
            for frame_idx in 0..num_frames {
                let (mag, phase) = interpolate_spectrum(
                    &spectrogram_mag_image,
                    &spectrogram_phase_image,
                    log_bin_float,
                    frame_idx,
                );
                spectrogram_mag_linear[linear_bin][frame_idx] = mag;
                spectrogram_phase_linear[linear_bin][frame_idx] = phase;
            }
        }

        (spectrogram_mag_linear, spectrogram_phase_linear)
    } else {
        // Use the image data as-is (linear frequency scale)
        (spectrogram_mag_image, spectrogram_phase_image)
    };
    
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
        let num_bins_to_use = spectrogram_mag.len().min(fft_size / 2 + 1);
        for bin in 0..num_bins_to_use {
            let magnitude = spectrogram_mag[bin][frame_idx];
            let phase = spectrogram_phase[bin][frame_idx];

            spectrum[bin] = Complex::new(
                magnitude * phase.cos(),
                magnitude * phase.sin(),
            );
        }

        // Mirror for negative frequencies (ensure real output)
        for bin in 1..num_bins_to_use.min(fft_size / 2) {
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

// Interpolate magnitude and phase from spectrum at a fractional bin index
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
