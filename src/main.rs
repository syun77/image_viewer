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
        renderer: eframe::Renderer::Glow, // Ensure OpenGL renderer for better performance
        ..Default::default()
    };

    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // Setup Japanese font support
            setup_custom_fonts(&cc.egui_ctx);

            // Enable continuous mode for high-frequency updates
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            Ok(Box::new(ImageViewerApp::new(cc)) as Box<dyn eframe::App>)
        }),
    )
}

fn setup_custom_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions, FontFamily};
    
    let mut fonts = FontDefinitions::default();
    
    // macOSで日本語フォントを追加
    // システムにインストールされている可能性の高いフォントファイルを試す
    let japanese_font_paths = [
        "/System/Library/Fonts/Apple SD Gothic Neo.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/PingFang.ttc", 
        "/Library/Fonts/Arial Unicode.ttf",
    ];
    
    let mut font_added = false;
    
    for font_path in &japanese_font_paths {
        if let Ok(font_data) = std::fs::read(font_path) {
            fonts.font_data.insert(
                "japanese_system".to_owned(),
                FontData::from_owned(font_data),
            );
            
            // フォントファミリーの最初に追加
            fonts.families
                .get_mut(&FontFamily::Proportional)
                .unwrap()
                .insert(0, "japanese_system".to_owned());
                
            fonts.families
                .get_mut(&FontFamily::Monospace)
                .unwrap() 
                .insert(0, "japanese_system".to_owned());
            
            println!("Japanese font loaded from: {}", font_path);
            font_added = true;
            break;
        }
    }
    
    if !font_added {
        println!("Warning: No Japanese system fonts found, using default");
        // デフォルトフォントでも基本的な日本語は表示されるはず
    }
    
    ctx.set_fonts(fonts);
}
