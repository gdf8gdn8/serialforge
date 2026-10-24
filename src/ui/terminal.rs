use crate::app::SerialForgeApp;
use crate::types::ViewMode;
use eframe::egui;

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
