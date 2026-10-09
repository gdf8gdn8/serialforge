use eframe::egui;

pub fn scrollable_script_field(
    ui: &mut egui::Ui,
    id: &str,
    editor: egui::TextEdit<'_>,
) -> egui::containers::scroll_area::ScrollAreaOutput<egui::Response> {
    egui::ScrollArea::both()
        .id_salt(id)
        .max_height(ui.available_height())
        .auto_shrink([false; 2])
        .show(ui, |ui| ui.add(editor))
}

pub fn highlight_rhai_code(ui: &egui::Ui, code: &str) -> egui::text::LayoutJob {
    use egui::text::LayoutJob;
    use egui::{
        Color32,
        TextFormat,
    };

    let is_dark = ui.visuals().dark_mode;
    let font_id = egui::FontId::monospace(13.0);

    let default_color = if is_dark {
        Color32::from_rgb(220, 220, 220)
    } else {
        Color32::from_rgb(30, 30, 30)
    };
    let keyword_color = if is_dark {
        Color32::from_rgb(86, 156, 214)
    } else {
        Color32::from_rgb(0, 0, 255)
    };
    let string_color = if is_dark {
        Color32::from_rgb(214, 157, 133)
    } else {
        Color32::from_rgb(163, 21, 21)
    };
    let comment_color = if is_dark {
        Color32::from_rgb(87, 166, 74)
    } else {
        Color32::from_rgb(0, 128, 0)
    };
    let fn_color = if is_dark {
        Color32::from_rgb(220, 220, 170)
    } else {
        Color32::from_rgb(121, 94, 38)
    };

    let mut job = LayoutJob::default();

    let keywords = [
        "let", "const", "if", "else", "while", "for", "in", "return", "fn", "break", "continue",
        "true", "false",
    ];
    let builtin_fns = ["print", "serial_send", "serial_read_line"];

    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Comments //
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' {
            let start = i;
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            job.append(
                &text,
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color: comment_color,
                    ..Default::default()
                },
            );
            continue;
        }

        // String Literals "..."
        if chars[i] == '"' {
            let start = i;
            i += 1;
            while i < len && chars[i] != '"' && chars[i] != '\n' {
                if chars[i] == '\\' && i + 1 < len {
                    i += 1;
                }
                i += 1;
            }
            if i < len && chars[i] == '"' {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            job.append(
                &text,
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color: string_color,
                    ..Default::default()
                },
            );
            continue;
        }

        // Keywords / Identifiers
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let color = if keywords.contains(&word.as_str()) {
                keyword_color
            } else if builtin_fns.contains(&word.as_str()) {
                fn_color
            } else {
                default_color
            };
            job.append(
                &word,
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color,
                    ..Default::default()
                },
            );
            continue;
        }

        // Plain Text (group consecutive plain characters)
        let start = i;
        while i < len {
            let is_comment = i + 1 < len && chars[i] == '/' && chars[i + 1] == '/';
            let is_string = chars[i] == '"';
            let is_ident = chars[i].is_alphabetic() || chars[i] == '_';
            if is_comment || is_string || is_ident {
                break;
            }
            i += 1;
        }
        let text: String = chars[start..i].iter().collect();
        job.append(
            &text,
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color: default_color,
                ..Default::default()
            },
        );
    }

    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_tactical_rpg_script_has_bounded_scroll_viewport() {
        let source = include_str!("../scripts/tactical_rpg.rhai");
        for size in [egui::vec2(320.0, 180.0), egui::vec2(640.0, 400.0)] {
            let ctx = egui::Context::default();
            let mut text = source.to_owned();
            for _frame in 0..3 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    ..Default::default()
                };
                ctx.run_ui(input, |ui| {
                    let available = ui.available_size();
                    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, _: f32| {
                        ui.painter()
                            .layout_job(highlight_rhai_code(ui, text.as_str()))
                    };
                    let output = scrollable_script_field(
                        ui,
                        "test_script_scroll",
                        egui::TextEdit::multiline(&mut text)
                            .layouter(&mut layouter)
                            .desired_width(available.x),
                    );
                    assert!(output.inner_rect.height() <= available.y);
                    assert!(output.inner_rect.width() <= available.x);
                    assert!(output.content_size.y > output.inner_rect.height());
                    assert!(ui.min_rect().height() <= available.y + 1.0);
                })
                .drop_without_applying_deltas();
            }
            assert_eq!(text, source);
        }
    }
}
