use eframe::egui;

mod ui;
mod core;
mod utils;

use ui::app::ImageViewerApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| Ok(Box::new(ImageViewerApp::new(cc)))),
    )
}
