use cansi::v3::{
    categorise_text,
    Color as AnsiColor,
    Intensity,
};
use eframe::egui;

pub fn ansi_to_egui_color(color: AnsiColor, intensity: Intensity, is_dark: bool) -> egui::Color32 {
    let bright = matches!(intensity, Intensity::Bold);

    match (color, bright) {
        (AnsiColor::Black, false) => {
            if is_dark {
                egui::Color32::from_rgb(60, 60, 60)
            } else {
                egui::Color32::from_rgb(30, 30, 30)
            }
        }
        (AnsiColor::Black, true) => egui::Color32::from_rgb(128, 128, 128),
        (AnsiColor::Red, false) => egui::Color32::from_rgb(205, 49, 49),
        (AnsiColor::Red, true) => egui::Color32::from_rgb(241, 76, 76),
        (AnsiColor::Green, false) => egui::Color32::from_rgb(13, 188, 121),
        (AnsiColor::Green, true) => egui::Color32::from_rgb(35, 209, 139),
        (AnsiColor::Yellow, false) => egui::Color32::from_rgb(229, 229, 16),
        (AnsiColor::Yellow, true) => egui::Color32::from_rgb(245, 245, 67),
        (AnsiColor::Blue, false) => egui::Color32::from_rgb(36, 114, 200),
        (AnsiColor::Blue, true) => egui::Color32::from_rgb(59, 142, 234),
        (AnsiColor::Magenta, false) => egui::Color32::from_rgb(188, 63, 188),
        (AnsiColor::Magenta, true) => egui::Color32::from_rgb(214, 112, 214),
        (AnsiColor::Cyan, false) => egui::Color32::from_rgb(17, 168, 205),
        (AnsiColor::Cyan, true) => egui::Color32::from_rgb(41, 184, 219),
        (AnsiColor::White, false) => egui::Color32::from_rgb(229, 229, 229),
        (AnsiColor::White, true) => egui::Color32::from_rgb(255, 255, 255),
        _ => {
            if is_dark {
                egui::Color32::from_rgb(210, 210, 210)
            } else {
                egui::Color32::from_rgb(30, 30, 30)
            }
        }
    }
}

pub fn highlight_terminal_ansi(ui: &egui::Ui, text: &str) -> egui::text::LayoutJob {
    use egui::text::LayoutJob;
    use egui::TextFormat;

    let is_dark = ui.visuals().dark_mode;
    let font_id = egui::FontId::monospace(12.0);
    let mut job = LayoutJob::default();

    let default_fg = if is_dark {
        egui::Color32::from_rgb(215, 215, 215)
    } else {
        egui::Color32::from_rgb(30, 30, 30)
    };

    let slices = categorise_text(text);
    for slice in slices {
        if slice.text.is_empty() {
            continue;
        }

        let intensity = slice.intensity.unwrap_or(Intensity::Normal);
        let fg_color = slice.fg.unwrap_or(AnsiColor::Black);

        let fg = if slice.fg.is_none() || (fg_color == AnsiColor::Black && intensity == Intensity::Normal && is_dark) {
            default_fg
        } else if fg_color == AnsiColor::White && !is_dark {
            default_fg
        } else {
            ansi_to_egui_color(fg_color, intensity, is_dark)
        };

        let mut format = TextFormat {
            font_id: font_id.clone(),
            color: fg,
            ..Default::default()
        };

        if slice.underline.unwrap_or(false) {
            format.underline = egui::Stroke::new(1.0, fg);
        }

        job.append(slice.text, 0.0, format);
    }

    job
}

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
    fn test_ansi_escape_parsing() {
        let ctx = egui::Context::default();
        let input = egui::RawInput::default();
        ctx.run_ui(input, |ui| {
            let colored = "\x1b[31mRed text\x1b[0m and \x1b[32mGreen text\x1b[0m";
            let job = highlight_terminal_ansi(ui, colored);
            assert_eq!(job.sections.len(), 3);
            assert_eq!(&job.text, "Red text and Green text");
        })
        .drop_without_applying_deltas();
    }

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
