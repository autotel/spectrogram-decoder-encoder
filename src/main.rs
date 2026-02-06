use eframe::egui;
use rfd::FileDialog;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use hound;

mod audio_to_image;
mod image_to_audio;
mod config;

use audio_to_image::audio_to_spectrogram;
use image_to_audio::spectrogram_to_audio;
use config::SpectrogramConfig;

#[derive(Clone)]
enum ProcessingState {
    Idle,
    Processing { status: String },
    Complete { output_path: PathBuf },
    Error { message: String },
}

struct SpectrogramApp {
    selected_file: Option<PathBuf>,
    status_message: String,
    config: SpectrogramConfig,
    show_config: bool,
    processing_state: Arc<Mutex<ProcessingState>>,
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 500.0])
            .with_min_inner_size([500.0, 400.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Spectrogram Converter",
        options,
        Box::new(|_cc| Ok(Box::new(SpectrogramApp::new()))),
    )
}


impl SpectrogramApp {
    fn new() -> Self {
        let config = SpectrogramConfig::load().unwrap_or_else(|e| {
            eprintln!("Error loading config: {}. Using defaults.", e);
            SpectrogramConfig::default()
        });
        
        config.print_info();
        
        Self {
            selected_file: None,
            status_message: String::new(),
            config,
            show_config: false,
            processing_state: Arc::new(Mutex::new(ProcessingState::Idle)),
        }
    }
    
    fn reload_config(&mut self) {
        match SpectrogramConfig::load() {
            Ok(config) => {
                self.config = config;
                self.status_message = "✓ Config reloaded successfully".to_string();
                self.config.print_info();
            }
            Err(e) => {
                self.status_message = format!("✗ Error reloading config: {}", e);
            }
        }
    }
}

impl Default for SpectrogramApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for SpectrogramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check processing state
        let current_state = self.processing_state.lock().unwrap().clone();
        
        match current_state {
            ProcessingState::Complete { output_path } => {
                self.status_message = format!("✓ Successfully exported to: {}", output_path.display());
                *self.processing_state.lock().unwrap() = ProcessingState::Idle;
            }
            ProcessingState::Error { message } => {
                self.status_message = format!("✗ Error: {}", message);
                *self.processing_state.lock().unwrap() = ProcessingState::Idle;
            }
            ProcessingState::Processing { .. } => {
                // Request continuous repaint while processing
                ctx.request_repaint();
            }
            _ => {}
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Spectrogram Converter");
            ui.add_space(10.0);
            
            // Show spinner if processing
            if let ProcessingState::Processing { status } = &*self.processing_state.lock().unwrap() {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(status);
                });
                ui.add_space(10.0);
            }
            
            ui.label("Drop a file here or click to select:");
            ui.add_space(5.0);

            // Config toggle
            ui.horizontal(|ui| {
                if ui.button(if self.show_config { "▼ Hide Config" } else { "▶ Show Config" }).clicked() {
                    self.show_config = !self.show_config;
                }
                
                if ui.button("🔄 Reload Config").clicked() {
                    self.reload_config();
                }
                
                if ui.button("📝 Open Config File").clicked() {
                    if let Err(e) = open::that("spectrogram_config.toml") {
                        self.status_message = format!("✗ Could not open config file: {}", e);
                    } else {
                        self.status_message = "✓ Opened config file in default editor".to_string();
                    }
                }
            });
            
            if self.show_config {
                ui.add_space(5.0);
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("Current Configuration:");
                            ui.separator();
                            
                            ui.horizontal(|ui| {
                                ui.label("FFT Size:");
                                ui.label(format!("{} samples", self.config.fft_size));
                            });
                            
                            ui.horizontal(|ui| {
                                ui.label("Hop Size:");
                                ui.label(format!("{} samples", self.config.hop_size));
                            });
                            
                            ui.horizontal(|ui| {
                                ui.label("Overlap:");
                                let overlap = (1.0 - self.config.hop_size as f32 / self.config.fft_size as f32) * 100.0;
                                ui.label(format!("{:.1}%", overlap));
                            });
                            
                            ui.horizontal(|ui| {
                                ui.label("Time Resolution:");
                                let ms_per_frame = (self.config.hop_size as f32 / 44100.0) * 1000.0;
                                ui.label(format!("~{:.1} ms/pixel @ 44.1kHz", ms_per_frame));
                            });
                            
                            ui.horizontal(|ui| {
                                ui.label("Min Frequency:");
                                ui.label(format!("{} Hz", self.config.min_freq));
                            });
                            
                            ui.horizontal(|ui| {
                                ui.label("Dynamic Range:");
                                ui.label(format!("{} to {} dB", self.config.db_min, self.config.db_max));
                            });
                            
                            ui.horizontal(|ui| {
                                ui.label("Phase Encoding:");
                                ui.label(if self.config.use_phase_encoding { 
                                    "Enabled (color)" 
                                } else { 
                                    "Disabled (grayscale)" 
                                });
                            });
                            
                            ui.horizontal(|ui| {
                                ui.label("Frequency Scale:");
                                ui.label(if self.config.use_log_scale { 
                                    "Logarithmic (musical)" 
                                } else { 
                                    "Linear (technical)" 
                                });
                            });
                            
                            if !self.config.use_phase_encoding {
                                ui.horizontal(|ui| {
                                    ui.label("Griffin-Lim Iterations:");
                                    ui.label(format!("{}", self.config.griffin_lim_iterations));
                                });
                            }
                            
                            ui.label("Edit spectrogram_config.toml to change these values");
                        });
                    });
            }
            
            ui.add_space(10.0);

            // File selection button
            let is_processing = matches!(*self.processing_state.lock().unwrap(), ProcessingState::Processing { .. });
            
            ui.add_enabled_ui(!is_processing, |ui| {
                if ui.button("📁 Select File").clicked() {
                    // Use non-blocking file dialog
                    if let Some(path) = FileDialog::new()
                        .add_filter("Audio/Image", &["wav", "png", "jpg", "jpeg"])
                        .pick_file()
                    {
                        self.selected_file = Some(path);
                        self.status_message = String::new(); // Clear previous status
                    }
                }
            });
            
            ui.add_space(10.0);
            
            // Display selected file with scrollable path
            if let Some(ref path) = self.selected_file {
                egui::ScrollArea::horizontal()
                    .id_source("file_path_scroll")
                    .show(ui, |ui| {
                        ui.label(format!("Selected: {}", path.display()));
                    });
                ui.add_space(5.0);
                
                // Show what the output will be named and estimated size
                if let Ok((output_path, est_width)) = get_output_info(path, &self.config) {
                    egui::ScrollArea::horizontal()
                        .id_source("output_path_scroll")
                        .show(ui, |ui| {
                            ui.label(format!("Will export to: {}", output_path.display()));
                        });
                    if let Some(width) = est_width {
                        ui.label(format!("Estimated image width: {} pixels", width));
                    }
                }
                
                ui.add_space(10.0);
                
                // Export button
                ui.add_enabled_ui(!is_processing, |ui| {
                    if ui.button("🚀 Export").clicked() {
                        self.status_message = String::new(); // Clear previous status
                        
                        let path = path.clone();
                        let config = self.config.clone();
                        let state = self.processing_state.clone();
                        
                        // Start processing in background thread
                        thread::spawn(move || {
                            *state.lock().unwrap() = ProcessingState::Processing {
                                status: "Processing...".to_string(),
                            };
                            
                            match process_file(&path, &config, state.clone()) {
                                Ok(output_path) => {
                                    *state.lock().unwrap() = ProcessingState::Complete { output_path };
                                }
                                Err(e) => {
                                    *state.lock().unwrap() = ProcessingState::Error {
                                        message: e.to_string(),
                                    };
                                }
                            }
                        });
                    }
                });
            } else {
                ui.label("No file selected");
            }
            
            ui.add_space(10.0);
            
            // Status message
            if !self.status_message.is_empty() {
                ui.separator();
                egui::ScrollArea::horizontal()
                    .id_source("status_scroll")
                    .show(ui, |ui| {
                        ui.label(&self.status_message);
                    });
            }
            
            // File drop zone
            preview_files_being_dropped(ctx);
            
            // Handle dropped files
            if !is_processing {
                ctx.input(|i| {
                    if !i.raw.dropped_files.is_empty() {
                        if let Some(dropped_file) = i.raw.dropped_files.first() {
                            if let Some(path) = &dropped_file.path {
                                self.selected_file = Some(path.clone());
                                self.status_message = String::new(); // Clear previous status
                            }
                        }
                    }
                });
            }
        });
    }
}

fn preview_files_being_dropped(ctx: &egui::Context) {
    use egui::*;
    
    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let text = ctx.input(|i| {
            let mut text = "Dropping:\n".to_owned();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    text += &format!("  {}\n", path.display());
                }
            }
            text
        });

        let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));
        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(20.0),
            Color32::WHITE,
        );
    }
}

fn get_output_info(
    path: &PathBuf,
    config: &SpectrogramConfig,
) -> Result<(PathBuf, Option<usize>), Box<dyn std::error::Error>> {
    let extension = path.extension()
        .and_then(|s| s.to_str())
        .ok_or("Unable to determine file extension")?
        .to_lowercase();

    match extension.as_str() {
        "wav" => {
            // For WAV files, we need to read the sample rate and calculate estimated width
            let reader = hound::WavReader::open(path)?;
            let spec = reader.spec();
            let sample_rate = spec.sample_rate;
            let scale_suffix = if config.use_log_scale { "_LOG" } else { "_LIN" };
            let phase_suffix = if config.use_phase_encoding { "_PHASE" } else { "_MAG" };
            
            // Calculate estimated width
            let total_samples = reader.duration() as usize;
            let mono_samples = if spec.channels == 2 {
                total_samples / 2
            } else {
                total_samples
            };
            let est_width = (mono_samples - config.fft_size) / config.hop_size + 1;

            if let Some(stem) = path.file_stem() {
                let parent = path.parent().unwrap_or(Path::new(""));
                let output_path = parent.join(format!("{}_SR{}{}{}.png", stem.to_string_lossy(), sample_rate, scale_suffix, phase_suffix));
                Ok((output_path, Some(est_width)))
            } else {
                Ok((path.with_extension("png"), Some(est_width)))
            }
        }
        "png" | "jpg" | "jpeg" => {
            Ok((path.with_extension("wav"), None))
        }
        _ => Err("Unsupported file format".into())
    }
}

fn process_file(
    path: &PathBuf,
    config: &SpectrogramConfig,
    progress_state: Arc<Mutex<ProcessingState>>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let extension = path.extension()
        .and_then(|s| s.to_str())
        .ok_or("Unable to determine file extension")?
        .to_lowercase();

    match extension.as_str() {
        "wav" => {
            *progress_state.lock().unwrap() = ProcessingState::Processing {
                status: "Reading audio file...".to_string(),
            };
            
            let output_path = path.with_extension("png");
            
            *progress_state.lock().unwrap() = ProcessingState::Processing {
                status: "Computing spectrogram...".to_string(),
            };
            
            let result = audio_to_spectrogram(path, &output_path, config)?;
            
            Ok(result)
        }
        "png" | "jpg" | "jpeg" => {
            *progress_state.lock().unwrap() = ProcessingState::Processing {
                status: "Reading image file...".to_string(),
            };
            
            let output_path = path.with_extension("wav");
            
            // Check if Griffin-Lim will be used
            let use_griffin_lim = if let Some(stem) = path.file_stem() {
                let stem_str = stem.to_string_lossy();
                stem_str.contains("_MAG")
            } else {
                false
            };
            
            if use_griffin_lim {
                *progress_state.lock().unwrap() = ProcessingState::Processing {
                    status: "Reconstructing audio (Griffin-Lim algorithm running...)".to_string(),
                };
            } else {
                *progress_state.lock().unwrap() = ProcessingState::Processing {
                    status: "Reconstructing audio...".to_string(),
                };
            }
            
            spectrogram_to_audio(path, &output_path, config)?;
            
            Ok(output_path)
        }
        _ => Err("Unsupported file format. Use WAV for audio or PNG/JPG for images.".into())
    }
}
