mod app;
mod canvas;
mod commands;
mod clipboard;
mod icons;
mod model;
mod palette;
mod theme;

use app::TopologyApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(1400.0, 900.0))
            .with_title("Network Topology Studio"),
        ..Default::default()
    };
    eframe::run_native(
        "Network Topology Studio",
        options,
        Box::new(|_cc| Ok(Box::new(TopologyApp::new(_cc)))),
    )
}
