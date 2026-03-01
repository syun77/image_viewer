use eframe::egui;

use crate::ui::app::{AppState, ImageViewerAction};

pub struct ImageViewer {
    current_image: Option<egui::TextureHandle>,
    zoom_level: f32,
    pan_offset: egui::Vec2,
}

impl ImageViewer {
    pub fn new() -> Self {
        Self {
            current_image: None,
            zoom_level: 1.0,
            pan_offset: egui::Vec2::ZERO,
        }
    }

    pub fn show(
        &mut self, 
        ctx: &egui::Context, 
        state: &mut AppState
    ) -> Option<ImageViewerAction> {
        let mut action = None;
        
        // Dark background
        egui::Area::new(egui::Id::new("image_viewer_background"))
            .fixed_pos(egui::Pos2::ZERO)
            .show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(screen_rect), |ui| {
                    ui.painter().rect_filled(
                        screen_rect,
                        egui::Rounding::ZERO,
                        egui::Color32::from_black_alpha(200),
                    );
                });
            });

        // Main viewer window
        egui::Window::new("Image Viewer")
            .collapsible(false)
            .resizable(false)
            .title_bar(true)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                // Handle keyboard input
                if ui.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Space)) {
                    action = Some(ImageViewerAction::Close);
                }
                
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    action = Some(ImageViewerAction::Previous);
                }
                
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    action = Some(ImageViewerAction::Next);
                }
                
                if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                    action = Some(ImageViewerAction::Previous);
                }
                
                if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                    action = Some(ImageViewerAction::Next);
                }

                // Toolbar
                ui.horizontal(|ui| {
                    if ui.button("Previous (←)").clicked() {
                        action = Some(ImageViewerAction::Previous);
                    }
                    
                    if ui.button("Next (→)").clicked() {
                        action = Some(ImageViewerAction::Next);
                    }
                    
                    ui.separator();
                    
                    if ui.button("Close (Esc)").clicked() {
                        action = Some(ImageViewerAction::Close);
                    }
                    
                    ui.separator();
                    
                    ui.label(format!("Zoom: {:.0}%", self.zoom_level * 100.0));
                    
                    if ui.button("+").clicked() {
                        self.zoom_level = (self.zoom_level * 1.2).min(5.0);
                    }
                    
                    if ui.button("-").clicked() {
                        self.zoom_level = (self.zoom_level / 1.2).max(0.1);
                    }
                    
                    if ui.button("Fit").clicked() {
                        self.zoom_level = 1.0;
                        self.pan_offset = egui::Vec2::ZERO;
                    }
                });
                
                ui.separator();
                
                // Image display area
                let available_size = ui.available_size();
                
                egui::ScrollArea::both()
                    .max_height(available_size.y - 60.0)
                    .show(ui, |ui| {
                        // Load and display current image
                        if let Some(selected_index) = state.selected_image {
                            // This is a placeholder - in a real implementation,
                            // you would load the actual image from the thumbnail grid
                            ui.allocate_response(
                                egui::Vec2::new(400.0, 300.0),
                                egui::Sense::drag(),
                            );
                            
                            ui.painter().rect_filled(
                                ui.max_rect(),
                                egui::Rounding::same(5.0),
                                egui::Color32::from_gray(80),
                            );
                            
                            ui.centered_and_justified(|ui| {
                                ui.label(format!("Image {} (placeholder)", selected_index + 1));
                            });
                        } else {
                            ui.label("No image selected");
                        }
                    });
                
                // Status bar
                ui.separator();
                ui.horizontal(|ui| {
                    if let Some(index) = state.selected_image {
                        ui.label(format!("Image {} of {}", index + 1, "N"));
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("Use ←→↑↓ to navigate, Esc/Space to close");
                    });
                });
            });
        
        action
    }
}