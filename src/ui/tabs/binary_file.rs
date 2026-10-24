use crate::app::SerialForgeApp;
use crate::types::TxCmd;
use eframe::egui;
use rfd::FileDialog;

pub fn render_binary_file_tab(app: &mut SerialForgeApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Binary Payload:");
        if ui.button("📁 Select File").clicked()
            && let Some(path) = FileDialog::new().pick_file()
        {
            app.file_path = Some(path);
        }

        if let Some(ref path) = app.file_path {
            ui.label(path.file_name().unwrap_or_default().to_string_lossy());
        }

        ui.add(egui::DragValue::new(&mut app.chunk_size).prefix("Chunk size: "));
        ui.add(egui::DragValue::new(&mut app.chunk_delay_ms).prefix("Delay ms: "));

        if ui.button("📤 Transmit File").clicked()
            && app.is_connected
            && let Some(ref path) = app.file_path
        {
            let _ = app.cmd_tx.send(TxCmd::SendFile {
                path: path.clone(),
                chunk_size: app.chunk_size,
                delay_ms: app.chunk_delay_ms,
            });
        }
    });

    if let Some((sent, total)) = app.file_progress {
        let progress = (sent as f32) / (total as f32);
        ui.add(egui::ProgressBar::new(progress).text(format!("{sent}/{total} Bytes")));
    }
}
