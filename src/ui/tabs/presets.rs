use crate::app::SerialForgeApp;
use crate::types::{
    LineEnding,
    PreconfigCommand,
};
use eframe::egui;

pub fn render_presets_tab(app: &mut SerialForgeApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Send String:");
        let text_entry = ui.add(
            egui::TextEdit::singleline(&mut app.send_text)
                .desired_width(280.0)
                .hint_text("Type command or use ↑/↓ for history"),
        );

        if text_entry.has_focus() {
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                if !app.cmd_history.is_empty() {
                    let next_idx = match app.cmd_history_idx {
                        None => app.cmd_history.len().saturating_sub(1),
                        Some(idx) => idx.saturating_sub(1),
                    };
                    app.cmd_history_idx = Some(next_idx);
                    app.send_text.clone_from(&app.cmd_history[next_idx]);
                }
            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowDown))
                && let Some(idx) = app.cmd_history_idx
            {
                if idx + 1 < app.cmd_history.len() {
                    let next_idx = idx + 1;
                    app.cmd_history_idx = Some(next_idx);
                    app.send_text.clone_from(&app.cmd_history[next_idx]);
                } else {
                    app.cmd_history_idx = None;
                    app.send_text.clear();
                }
            }
        }

        egui::ComboBox::from_id_salt("cmd_history_combo")
            .selected_text("📜 History")
            .show_ui(ui, |ui| {
                if app.cmd_history.is_empty() {
                    ui.label("No history yet");
                } else {
                    for h in app.cmd_history.iter().rev() {
                        if ui.selectable_label(false, h).clicked() {
                            app.send_text.clone_from(h);
                        }
                    }
                }
            });

        ui.label("EOL:");
        egui::ComboBox::from_id_salt("line_ending_combo")
            .selected_text(match app.line_ending {
                LineEnding::None => "None",
                LineEnding::Null => "NULL (0x00)",
                LineEnding::CR => "CR (\\r)",
                LineEnding::LF => "LF (\\n)",
                LineEnding::Crlf => "CRLF (\\r\\n)",
                LineEnding::Etx => "ETX (0x03)",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.line_ending, LineEnding::None, "None");
                ui.selectable_value(&mut app.line_ending, LineEnding::Null, "NULL (0x00)");
                ui.selectable_value(&mut app.line_ending, LineEnding::CR, "CR (\\r)");
                ui.selectable_value(&mut app.line_ending, LineEnding::LF, "LF (\\n)");
                ui.selectable_value(&mut app.line_ending, LineEnding::Crlf, "CRLF (\\r\\n)");
                ui.selectable_value(&mut app.line_ending, LineEnding::Etx, "ETX (0x03)");
            });

        ui.checkbox(&mut app.send_as_hex, "HEX")
            .on_hover_text("Send as raw hexadecimal bytes (e.g., 01 03 00 00 00 0A C5 CD)");

        let enter_pressed =
            text_entry.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let trigger_send = ui.button("Send").clicked() || enter_pressed;

        if trigger_send {
            let text = app.send_text.clone();
            app.send_string_command(text);
            app.send_text.clear();
            if enter_pressed {
                text_entry.request_focus();
            }
        }
    });

    // Periodic / Repeat Transmit Bar
    ui.horizontal(|ui| {
        ui.checkbox(&mut app.repeat_send_enabled, "🔁 Repeat Every:");
        ui.add(
            egui::DragValue::new(&mut app.repeat_send_interval_ms)
                .range(20..=60_000)
                .suffix(" ms"),
        );
        if app.repeat_send_enabled && !app.is_connected {
            ui.label(
                egui::RichText::new("(Connect port to begin repeating)")
                    .color(egui::Color32::from_rgb(200, 140, 40)),
            );
        }
    });

    ui.separator();

    ui.label(egui::RichText::new("⭐ Preset Command Macros").strong());
    let mut to_send = None;
    let mut to_remove = None;

    egui::Grid::new("preconfig_grid")
        .striped(true)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Preset Name").strong());
            ui.label(egui::RichText::new("Command String").strong());
            ui.label(egui::RichText::new("Actions").strong());
            ui.end_row();

            for (idx, item) in app.preconfig_commands.iter().enumerate() {
                ui.label(&item.name);
                ui.monospace(&item.cmd);

                ui.horizontal(|ui| {
                    if ui.button("▶ Send").clicked() {
                        to_send = Some(item.cmd.clone());
                    }
                    if ui.button("📋 Load").clicked() {
                        app.send_text.clone_from(&item.cmd);
                    }
                    if ui.button("🗑").clicked() {
                        to_remove = Some(idx);
                    }
                });
                ui.end_row();
            }
        });

    if let Some(cmd) = to_send {
        app.send_string_command(cmd);
    }
    if let Some(idx) = to_remove {
        app.preconfig_commands.remove(idx);
    }

    ui.horizontal(|ui| {
        ui.label("New Preset:");
        ui.add(
            egui::TextEdit::singleline(&mut app.new_preconfig_name)
                .hint_text("Name")
                .desired_width(120.0),
        );
        ui.add(
            egui::TextEdit::singleline(&mut app.new_preconfig_cmd)
                .hint_text("Command String")
                .desired_width(220.0),
        );

        if ui.button("➕ Add Preset").clicked() && !app.new_preconfig_cmd.is_empty() {
            let name = if app.new_preconfig_name.is_empty() {
                app.new_preconfig_cmd.clone()
            } else {
                app.new_preconfig_name.clone()
            };
            app.preconfig_commands.push(PreconfigCommand {
                name,
                cmd: app.new_preconfig_cmd.clone(),
            });
            app.new_preconfig_name.clear();
            app.new_preconfig_cmd.clear();
        }
    });
}
