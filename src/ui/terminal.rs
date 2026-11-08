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
        let prev_ts = app.show_timestamps;
        ui.checkbox(&mut app.show_timestamps, "Timestamps");
        if app.show_timestamps != prev_ts {
            app.cached_formatted_text = None;
        }

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
            app.rx_entries.clear();
            app.cached_formatted_text = None;
        }

        ui.separator();
        let search_btn = if app.show_search_bar {
            "🔍 Close Search"
        } else {
            "🔍 Search (Ctrl+F)"
        };
        if ui.button(search_btn).clicked() {
            app.show_search_bar = !app.show_search_bar;
        }
    });

    // Check for Ctrl+F shortcut
    let ctrl_f = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::F);
    if ui.input_mut(|i| i.consume_shortcut(&ctrl_f)) {
        app.show_search_bar = !app.show_search_bar;
    }

    // 2. LIVE FILTER BAR
    ui.horizontal(|ui| {
        ui.label("Filter:");
        let prev_filter = app.filter_text.clone();
        let _filter_edit = ui.add(
            egui::TextEdit::singleline(&mut app.filter_text)
                .desired_width(160.0)
                .hint_text("Live regex / substring..."),
        );
        if app.filter_text != prev_filter {
            app.cached_formatted_text = None;
        }

        let prev_regex = app.filter_is_regex;
        ui.checkbox(&mut app.filter_is_regex, "Regex");
        if app.filter_is_regex != prev_regex {
            app.cached_formatted_text = None;
        }

        let prev_invert = app.filter_invert;
        ui.checkbox(&mut app.filter_invert, "Invert (Exclude)");
        if app.filter_invert != prev_invert {
            app.cached_formatted_text = None;
        }

        if !app.filter_text.is_empty() && ui.button("✖").on_hover_text("Clear filter").clicked() {
            app.filter_text.clear();
            app.cached_formatted_text = None;
        }
    });

    // 3. OPTIONAL CONSOLE SEARCH BAR (Ctrl+F)
    if app.show_search_bar {
        ui.horizontal(|ui| {
            ui.label("Find:");
            let search_edit = ui.add(
                egui::TextEdit::singleline(&mut app.search_query)
                    .desired_width(180.0)
                    .hint_text("Search term / regex..."),
            );
            if app.show_search_bar && search_edit.gained_focus() {
                // Focus captured
            }

            ui.checkbox(&mut app.search_is_regex, "Regex");
            ui.checkbox(&mut app.search_case_sensitive, "Match Case");

            let count = app.search_match_count;
            if count > 0 {
                ui.label(format!("{}/{} matches", app.current_search_idx + 1, count));
            } else if !app.search_query.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), "No matches");
            }

            if ui.button("◀ Prev").clicked() && count > 0 {
                if app.current_search_idx == 0 {
                    app.current_search_idx = count - 1;
                } else {
                    app.current_search_idx -= 1;
                }
                app.scroll_to_search_idx = Some(app.current_search_idx);
            }

            if ui.button("Next ▶").clicked() && count > 0 {
                app.current_search_idx = (app.current_search_idx + 1) % count;
                app.scroll_to_search_idx = Some(app.current_search_idx);
            }

            if ui.button("✖").on_hover_text("Close Search").clicked() {
                app.show_search_bar = false;
            }
        });
    }

    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .stick_to_bottom(app.auto_scroll)
        .show(ui, |ui| {
            if app.cached_formatted_text.is_none() {
                app.cached_formatted_text = Some(app.compute_formatted_text());
            }
            let formatted_text = app.cached_formatted_text.as_mut().unwrap();

            // Compute search matches on the formatted text
            let mut search_byte_ranges = Vec::new();
            if app.show_search_bar && !app.search_query.is_empty() {
                if app.search_is_regex {
                    let re_res = if app.search_case_sensitive {
                        regex::Regex::new(&app.search_query)
                    } else {
                        regex::RegexBuilder::new(&app.search_query)
                            .case_insensitive(true)
                            .build()
                    };
                    if let Ok(re) = re_res {
                        for m in re.find_iter(formatted_text) {
                            search_byte_ranges.push((m.start(), m.end()));
                        }
                    }
                } else if app.search_case_sensitive {
                    let mut start = 0;
                    while let Some(pos) = formatted_text[start..].find(&app.search_query) {
                        let actual_pos = start + pos;
                        let end = actual_pos + app.search_query.len();
                        search_byte_ranges.push((actual_pos, end));
                        start = end;
                    }
                } else {
                    let lower_text = formatted_text.to_lowercase();
                    let lower_query = app.search_query.to_lowercase();
                    let mut start = 0;
                    while let Some(pos) = lower_text[start..].find(&lower_query) {
                        let actual_pos = start + pos;
                        let end = actual_pos + lower_query.len();
                        search_byte_ranges.push((actual_pos, end));
                        start = end;
                    }
                }
            }

            app.search_match_count = search_byte_ranges.len();
            if app.search_match_count == 0 || app.current_search_idx >= app.search_match_count {
                app.current_search_idx = 0;
            }

            let search_ranges_ref = if !search_byte_ranges.is_empty() {
                Some(&search_byte_ranges[..])
            } else {
                None
            };
            let active_idx = app.current_search_idx;

            let is_ascii = app.view_mode == ViewMode::Ascii;
            let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                let text = buf.as_str();
                let mut layout_job = if is_ascii {
                    crate::highlighter::highlight_terminal_ansi_with_search(
                        ui,
                        text,
                        search_ranges_ref,
                        active_idx,
                    )
                } else {
                    let mut job = egui::text::LayoutJob::default();
                    job.append(
                        text,
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::monospace(12.0),
                            color: if ui.visuals().dark_mode {
                                egui::Color32::from_rgb(215, 215, 215)
                            } else {
                                egui::Color32::from_rgb(30, 30, 30)
                            },
                            ..Default::default()
                        },
                    );
                    job
                };
                layout_job.wrap.max_width = wrap_width;
                ui.painter().layout_job(layout_job)
            };

            let text_edit = egui::TextEdit::multiline(formatted_text)
                .id_salt("terminal_console_textedit")
                .layouter(&mut layouter)
                .desired_width(f32::INFINITY)
                .lock_focus(true);

            let mut output = text_edit.show(ui);

            // Handle scroll / cursor navigation to current match
            if let Some(target_idx) = app.scroll_to_search_idx.take()
                && let Some(&(byte_start, byte_end)) = search_byte_ranges.get(target_idx)
            {
                let char_start = formatted_text[..byte_start].chars().count();
                let char_end = formatted_text[..byte_end].chars().count();
                output
                    .state
                    .cursor
                    .set_char_range(Some(egui::text_selection::CCursorRange::two(
                        egui::text::CCursor::new(char_start),
                        egui::text::CCursor::new(char_end),
                    )));
                output.state.store(ui.ctx(), output.response.id);
                output.response.scroll_to_me(Some(egui::Align::Center));
            }
        });
}
