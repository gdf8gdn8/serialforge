use crate::app::SerialForgeApp;
use crate::highlighter::{
    highlight_rhai_code,
    scrollable_script_field,
};
use eframe::egui;
use rfd::FileDialog;
use std::sync::atomic::Ordering;

pub fn render_scripting_tab(app: &mut SerialForgeApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();

    ui.horizontal(|ui| {
        let is_running = app.script_running.load(Ordering::Relaxed);
        if !is_running {
            if ui.button("▶ Run Script").clicked() && app.is_connected {
                app.run_script(&ctx);
            }
        } else if ui.button("⏹ Stop Script").clicked() {
            app.script_running.store(false, Ordering::Relaxed);
            app.log_system("Stopping Rhai script...");
        }
        if ui.button("📂 Open Script").clicked()
            && let Some(path) = FileDialog::new()
                .add_filter("Rhai scripts", &["rhai"])
                .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(script) => {
                    app.script_text = script;
                    app.status_msg = format!(
                        "Loaded script: {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    );
                }
                Err(error) => {
                    app.status_msg = format!("Script load failed: {error}");
                    app.log_system(&app.status_msg.clone());
                }
            }
        }
        if is_running {
            ui.spinner();
            ui.label("Script executing...");
        }
    });

    ui.columns(2, |cols| {
        let ctx = cols[0].ctx().clone();
        let editor_id = egui::Id::new("rhai_script_editor");
        let paste_requested = cols[0]
            .horizontal(|ui| {
                let paste_requested = ui.button("📋 Paste").clicked();
                ui.label("Rhai Script Editor:");
                paste_requested
            })
            .inner;
        let editor_has_focus = ctx.memory(|memory| memory.has_focus(editor_id));
        let key_paste_requested = editor_has_focus
            && ctx.input(|input| input.key_pressed(egui::Key::V) && input.modifiers.command);

        let layouter = &mut |ui: &egui::Ui, string: &dyn egui::TextBuffer, _wrap_width: f32| {
            ui.painter()
                .layout_job(highlight_rhai_code(ui, string.as_str()))
        };

        let mut paste_text: Option<String> = if editor_has_focus {
            ctx.input(|input| {
                input.events.iter().find_map(|event| match event {
                    egui::Event::Paste(text) => Some(text.clone()),
                    _ => None,
                })
            })
        } else {
            None
        };

        if paste_text.is_none() && (paste_requested || key_paste_requested) {
            match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
                Ok(text) => paste_text = Some(text),
                Err(error) => {
                    app.log_system(&format!("Clipboard paste failed: {error}"));
                }
            }
        }

        if let Some(text) = paste_text {
            ctx.input_mut(|input| {
                input
                    .events
                    .retain(|event| !matches!(event, egui::Event::Paste(_)));
            });
            app.insert_script_text(&ctx, editor_id, &text);
        }

        let editor_width = cols[0].available_width();
        let editor_response = scrollable_script_field(
            &mut cols[0],
            "rhai_editor_scroll",
            egui::TextEdit::multiline(&mut app.script_text)
                .id(editor_id)
                .font(egui::TextStyle::Monospace)
                .layouter(layouter)
                .desired_width(editor_width),
        )
        .inner;

        if paste_requested {
            editor_response.request_focus();
        }

        cols[1].label("Script Log:");
        let mut logs = app.script_log.lock().unwrap().join("\n");
        let log_width = cols[1].available_width();
        scrollable_script_field(
            &mut cols[1],
            "rhai_log_scroll",
            egui::TextEdit::multiline(&mut logs)
                .font(egui::TextStyle::Monospace)
                .desired_width(log_width),
        );
    });
}
