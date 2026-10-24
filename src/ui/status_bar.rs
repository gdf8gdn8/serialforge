use crate::app::SerialForgeApp;
use eframe::egui;

pub fn render_status_bar(app: &SerialForgeApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(format!("Status: {}", app.status_msg));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(format!(
                "Tx: {} bytes | Rx: {} bytes",
                app.tx_bytes, app.rx_bytes
            ));
        });
    });
}
