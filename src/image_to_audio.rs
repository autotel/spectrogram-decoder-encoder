use image;
use hound;
use rustfft::{FftPlanner, num_complex::Complex};
use std::path::Path;
use crate::config::SpectrogramConfig;

pub fn spectrogram_to_audio(
    image_path: &Path,
    output_path: &Path,
    config: &SpectrogramConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Extract sample rate, scale mode, and phase encoding from filename
    let (sample_rate, use_log_scale, use_phase_encoding) = if let Some(stem) = image_path.file_stem() {
        let stem_str = stem.to_string_lossy();
        println!("Filename stem: {}", stem_str);
        
        let sample_rate = if let Some(sr_pos) = stem_str.rfind("_SR") {
            let after_sr = &stem_str[sr_pos + 3..];
            let sr_str: String = after_sr.chars().take_while(|c| c.is_numeric()).collect();
            let parsed_sr = sr_str.parse::<u32>().unwrap_or(44100);
            println!("Extracted sample rate from filename: {}", parsed_sr);
            parsed_sr
        } else {
            println!("No _SR found in filename, using default 44100");
            44100
        };

        let use_log_scale = stem_str.contains("_LOG");
        println!("use_log_scale: {}", use_log_scale);
        
        let use_phase_encoding = if stem_str.contains("_PHASE") {
            println!("Phase encoding: ENABLED");
            true
        } else if stem_str.contains("_MAG") {
            println!("Phase encoding: DISABLED");
            false
        } else {
            println!("Phase encoding: ENABLED (legacy)");
            true
        };

        (sample_rate, use_log_scale, use_phase_encoding)
    } else {
        (44100, false, true)
    };
    
    let img = image::open(image_path)?.to_rgb8();
    let (width, height) = img.dimensions();

    let num_frames = width as usize;
    let num_bins_image = height as usize;

    let fft_size = (num_bins_image - 1) * 2;
    let num_bins_linear = fft_size / 2 + 1;

    println!("Image size: {}x{}", width, height);
    println!("FFT size: {}, HOP_SIZE: {}", fft_size, config.hop_size);

    let mut spectrogram_mag_image = vec![vec![0.0f32; num_frames]; num_bins_image];
    let mut spectrogram_phase_image = vec![vec![0.0f32; num_frames]; num_bins_image];
    
    // Decode magnitude and phase
    for frame in 0..num_frames {
        for bin in 0..num_bins_image {
            let y = height - 1 - bin as u32;
            let pixel = img.get_pixel(frame as u32, y);
            let (h, s, v) = rgb_to_hsv(pixel[0], pixel[1], pixel[2]);

            // Calculate frequency for this bin
            let bin_freq = if use_log_scale {
                let nyquist = sample_rate as f32 / 2.0;
                let t = bin as f32 / (num_bins_image - 1) as f32;
                config.min_freq * (nyquist / config.min_freq).powf(t)
            } else {
                let nyquist = sample_rate as f32 / 2.0;
                (bin as f32 / (num_bins_image - 1) as f32) * nyquist
            };

            let phase = if use_phase_encoding {
                let decoded_phase = (h / 360.0) * 2.0 * std::f32::consts::PI - std::f32::consts::PI;
                if s < 0.1 && frame > 0 {
                    spectrogram_phase_image[bin][frame - 1]
                } else {
                    decoded_phase
                }
            } else {
                // For magnitude-only: use instantaneous frequency
                if frame == 0 {
                    // Start with zero phase
                    0.0
                } else {
                    // Phase advance based on frequency
                    let prev_phase = spectrogram_phase_image[bin][frame - 1];
                    let phase_advance = 2.0 * std::f32::consts::PI * bin_freq * (config.hop_size as f32 / sample_rate as f32);
                    
                    // Wrap to [-π, π]
                    let mut new_phase = prev_phase + phase_advance;
                    while new_phase > std::f32::consts::PI {
                        new_phase -= 2.0 * std::f32::consts::PI;
                    }
                    while new_phase < -std::f32::consts::PI {
                        new_phase += 2.0 * std::f32::consts::PI;
                    }
                    new_phase
                }
            };

            let boost_db = if bin_freq > config.boost_start_freq {
                config.boost_db_per_octave * (bin_freq / config.boost_start_freq).log2()
            } else {
                0.0
            };

            let db = v * (config.db_max - config.db_min) + config.db_min;
            let db_without_boost = db - boost_db;
            let magnitude = 10.0f32.powf(db_without_boost / 20.0);

            spectrogram_mag_image[bin][frame] = magnitude.max(0.0);
            spectrogram_phase_image[bin][frame] = phase;
        }
    }

    // Apply inverse frequency scale transformation
    let (spectrogram_mag, spectrogram_phase) = if use_log_scale {
        let nyquist = sample_rate as f32 / 2.0;
        let min_freq = config.min_freq;
        let max_freq = nyquist;

        let mut spectrogram_mag_linear = vec![vec![0.0f32; num_frames]; num_bins_linear];
        let mut spectrogram_phase_linear = vec![vec![0.0f32; num_frames]; num_bins_linear];

        for linear_bin in 0..num_bins_linear {
            let freq_linear = (linear_bin as f32 / (num_bins_linear - 1) as f32) * nyquist;
            let log_bin_float = if freq_linear <= min_freq {
                0.0
            } else {
                let t = (freq_linear / min_freq).ln() / (max_freq / min_freq).ln();
                t * (num_bins_image - 1) as f32
            };

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
        (spectrogram_mag_image, spectrogram_phase_image)
    };
    
    // Inverse STFT
    let mut planner = FftPlanner::new();
    let ifft = planner.plan_fft_inverse(fft_size);
    
    let output_len = (num_frames - 1) * config.hop_size + fft_size;
    let mut output = vec![0.0f32; output_len];
    let mut window_sum = vec![0.0f32; output_len];
    
    for frame_idx in 0..num_frames {
        let mut spectrum = vec![Complex::new(0.0, 0.0); fft_size];

        let num_bins_to_use = spectrogram_mag.len().min(fft_size / 2 + 1);
        for bin in 0..num_bins_to_use {
            let magnitude = spectrogram_mag[bin][frame_idx];
            let phase = spectrogram_phase[bin][frame_idx];
            spectrum[bin] = Complex::new(magnitude * phase.cos(), magnitude * phase.sin());
        }

        for bin in 1..num_bins_to_use.min(fft_size / 2) {
            spectrum[fft_size - bin] = spectrum[bin].conj();
        }
        
        ifft.process(&mut spectrum);
        
        let start = frame_idx * config.hop_size;
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
        if window_sum[i] > 1e-8 {
            output[i] /= window_sum[i];
        }
    }
    
    // Normalize output
    let max_sample = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    if max_sample > 1e-8 {
        for sample in output.iter_mut() {
            *sample = (*sample / max_sample) * 0.95;
        }
    }
    
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
    
    println!("Saved audio to: {}", output_path.display());
    Ok(())
}

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

    let mag_interp = mag[bin_floor][frame] * (1.0 - frac) + mag[bin_ceil][frame] * frac;

    let phase1 = phase[bin_floor][frame];
    let phase2 = phase[bin_ceil][frame];
    let mut phase_diff = phase2 - phase1;

    while phase_diff > std::f32::consts::PI {
        phase_diff -= 2.0 * std::f32::consts::PI;
    }
    while phase_diff < -std::f32::consts::PI {
        phase_diff += 2.0 * std::f32::consts::PI;
    }

    let phase_interp = phase1 + phase_diff * frac;

    (mag_interp, phase_interp)
}
