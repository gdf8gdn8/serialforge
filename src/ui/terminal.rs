use crate::app::SerialForgeApp;
use crate::types::ViewMode;
use chrono::Local;
use eframe::egui;
use rfd::FileDialog;
use std::fs::File;
use std::io::Write;

pub fn render_terminal(app: &mut SerialForgeApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let prev_mode = app.view_mode;
        ui.selectable_value(&mut app.view_mode, ViewMode::Ascii, "ASCII");
        ui.selectable_value(&mut app.view_mode, ViewMode::Hex, "HEX");
        ui.selectable_value(&mut app.view_mode, ViewMode::Mixed, "Mixed");
        if app.view_mode != prev_mode {
            app.cached_formatted_text = None;
        }

        ui.separator();
        ui.checkbox(&mut app.auto_scroll, "Auto-Scroll");

        ui.separator();

        // 1. Log-to-File (Live Capture Stream)
        if app.log_file.is_none() {
            if ui.button("🔴 Start Logging").clicked() {
                let default_name = format!("session_{}.log", Local::now().format("%Y%m%d_%H%M%S"));
                if let Some(path) = FileDialog::new().set_file_name(&default_name).save_file() {
                    match File::create(&path) {
                        Ok(file) => {
                            app.log_file = Some(file);
                            app.log_file_path = Some(path.clone());
                            app.log_file_bytes = 0;
                            app.log_system(&format!("Started recording log to {}", path.display()));
                        }
                        Err(err) => {
                            app.log_system(&format!("Failed to create log file: {err}"));
                        }
                    }
                }
            }
        } else {
            let path_name = app
                .log_file_path
                .as_ref()
                .and_then(|p| p.file_name())
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if ui
                .button(format!("⏹ Stop Logging ({} bytes)", app.log_file_bytes))
                .clicked()
            {
                app.log_file = None;
                app.log_file_path = None;
                app.log_system(&format!("Stopped recording log ({path_name})."));
            }
        }

        // 2. Export Current Console View
        if ui.button("💾 Export View").clicked() {
            let default_name = format!("export_{}.txt", Local::now().format("%Y%m%d_%H%M%S"));
            if let Some(path) = FileDialog::new().set_file_name(&default_name).save_file() {
                if app.cached_formatted_text.is_none() {
                    app.cached_formatted_text = Some(app.compute_formatted_text());
                }
                if let Some(ref text) = app.cached_formatted_text {
                    match File::create(&path).and_then(|mut f| f.write_all(text.as_bytes())) {
                        Ok(_) => {
                            app.log_system(&format!(
                                "Exported terminal view to {}",
                                path.display()
                            ));
                        }
                        Err(err) => {
                            app.log_system(&format!("Failed to export terminal view: {err}"));
                        }
                    }
                }
            }
        }

        if ui.button("🗑 Clear Console").clicked() {
            app.rx_buffer.clear();
            app.cached_formatted_text = None;
        }
    });

    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .stick_to_bottom(app.auto_scroll)
        .show(ui, |ui| {
            if app.cached_formatted_text.is_none() {
                app.cached_formatted_text = Some(app.compute_formatted_text());
            }
            let formatted_text = app.cached_formatted_text.as_mut().unwrap();

            ui.add(
                egui::TextEdit::multiline(formatted_text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .lock_focus(true),
            );
        });
}
