mod app;
mod highlighter;
mod types;
mod worker;
fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 760.0])
            .with_title("SerialForge - Cross-Platform Serial Terminal"),
        ..Default::default()
    };

    eframe::run_native(
        "SerialForge Terminal",
        native_options,
        Box::new(|cc| Ok(Box::new(app::SerialForgeApp::new(cc)))),
    )
}
