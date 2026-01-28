use eframe::egui;
use rfd::FileDialog;
use std::path::{Path, PathBuf};
use hound;

mod audio_to_image;
mod image_to_audio;

use audio_to_image::audio_to_spectrogram;
use image_to_audio::spectrogram_to_audio;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 200.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Spectrogram Converter",
        options,
        Box::new(|_cc| Ok(Box::new(SpectrogramApp::default()))),
    )
}

#[derive(Default)]
struct SpectrogramApp {
    selected_file: Option<PathBuf>,
    status_message: String,
    use_log_scale: bool,
}

impl eframe::App for SpectrogramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Spectrogram Converter");
            ui.add_space(10.0);
            
            ui.label("Drop a file here or click to select:");
            ui.add_space(5.0);

            // Frequency scale checkbox
            ui.checkbox(&mut self.use_log_scale, "Logarithmic frequency scale (note-based)");
            ui.add_space(5.0);

            // File selection button
            if ui.button("📁 Select File").clicked() {
                if let Some(path) = FileDialog::new()
                    .add_filter("Audio/Image", &["wav", "png", "jpg", "jpeg"])
                    .pick_file()
                {
                    self.selected_file = Some(path);
                    self.status_message = String::new();
                }
            }
            
            ui.add_space(10.0);
            
            // Display selected file
            if let Some(ref path) = self.selected_file {
                ui.label(format!("Selected: {}", path.display()));
                ui.add_space(5.0);
                
                // Show what the output will be named
                if let Ok(output_path) = get_output_path(path, self.use_log_scale) {
                    ui.label(format!("Will export to: {}", output_path.display()));
                }
                
                ui.add_space(10.0);
                
                // Export button
                if ui.button("🚀 Export").clicked() {
                    self.status_message = match process_file(path, self.use_log_scale) {
                        Ok(output_path) => format!("✓ Successfully exported to: {}", output_path.display()),
                        Err(e) => format!("✗ Error: {}", e),
                    };
                }
            } else {
                ui.label("No file selected");
            }
            
            ui.add_space(10.0);
            
            // Status message
            if !self.status_message.is_empty() {
                ui.label(&self.status_message);
            }
            
            // File drop zone
            preview_files_being_dropped(ctx);
            
            // Handle dropped files
            ctx.input(|i| {
                if !i.raw.dropped_files.is_empty() {
                    if let Some(dropped_file) = i.raw.dropped_files.first() {
                        if let Some(path) = &dropped_file.path {
                            self.selected_file = Some(path.clone());
                            self.status_message = String::new();
                        }
                    }
                }
            });
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

fn get_output_path(path: &PathBuf, use_log_scale: bool) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let extension = path.extension()
        .and_then(|s| s.to_str())
        .ok_or("Unable to determine file extension")?
        .to_lowercase();

    match extension.as_str() {
        "wav" => {
            // For WAV files, we need to read the sample rate to predict the filename
            let reader = hound::WavReader::open(path)?;
            let sample_rate = reader.spec().sample_rate;
            let scale_suffix = if use_log_scale { "_LOG" } else { "_LIN" };

            if let Some(stem) = path.file_stem() {
                let parent = path.parent().unwrap_or(Path::new(""));
                Ok(parent.join(format!("{}_SR{}{}.png", stem.to_string_lossy(), sample_rate, scale_suffix)))
            } else {
                Ok(path.with_extension("png"))
            }
        }
        "png" | "jpg" | "jpeg" => {
            Ok(path.with_extension("wav"))
        }
        _ => Err("Unsupported file format".into())
    }
}

fn process_file(path: &PathBuf, use_log_scale: bool) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let extension = path.extension()
        .and_then(|s| s.to_str())
        .ok_or("Unable to determine file extension")?
        .to_lowercase();

    match extension.as_str() {
        "wav" => {
            // Audio to image - audio_to_spectrogram now returns the actual path
            let output_path = path.with_extension("png");
            audio_to_spectrogram(path, &output_path, use_log_scale)
        }
        "png" | "jpg" | "jpeg" => {
            // Image to audio
            let output_path = path.with_extension("wav");
            spectrogram_to_audio(path, &output_path)?;
            Ok(output_path)
        }
        _ => Err("Unsupported file format. Use WAV for audio or PNG/JPG for images.".into())
    }
}
