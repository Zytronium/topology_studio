mod app;
mod canvas;
mod commands;
mod clipboard;
mod export;
mod icons;
mod model;
mod palette;
mod theme;

use app::TopologyApp;

fn load_icon() -> egui::IconData {
    let svg_data = include_bytes!("../assets/icon.svg");
    let tree = resvg::usvg::Tree::from_data(svg_data, &resvg::usvg::Options::default()).expect("Failed to parse icon SVG");
    let size = tree.size();
    let width = size.width() as u32;
    let height = size.height() as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).expect("Failed to create pixmap");
    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap_mut);
    let rgba = pixmap_mut.data_mut().to_vec();
    egui::IconData { rgba, width, height }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(1400.0, 900.0))
            .with_title("Network Topology Studio")
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Network Topology Studio",
        options,
        Box::new(|_cc| Ok(Box::new(TopologyApp::new(_cc)))),
    )
}
