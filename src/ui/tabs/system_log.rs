use crate::app::SerialForgeApp;
use eframe::egui;

pub fn render_system_log_tab(app: &mut SerialForgeApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("System Activity Console").strong());
        if ui.button("🗑 Clear Log").clicked() {
            app.system_log.clear();
        }
    });

    let mut logs = app.system_log.join("\n");
    ui.add(
        egui::TextEdit::multiline(&mut logs)
            .font(egui::TextStyle::Monospace)
            .desired_rows(6)
            .desired_width(f32::INFINITY),
    );
}
