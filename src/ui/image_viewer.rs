use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Notify};

use crate::ui::app::{AppState, ImageViewerAction};
use crate::core::image_loader::ImageLoader;

pub struct ImageViewer {
    current_image: Option<egui::TextureHandle>,
    current_image_path: Option<PathBuf>,
    zoom_level: f32,
    pan_offset: egui::Vec2,
    error_message: Option<String>,
    is_loading: bool,
    // バックグラウンドでColorImageを生成し、UIスレッドでTexture化
    result_sender: Option<mpsc::Sender<Result<egui::ColorImage, String>>>,
    result_receiver: Option<mpsc::Receiver<Result<egui::ColorImage, String>>>,
    notify: Arc<Notify>,
}

impl ImageViewer {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(1);
        Self {
            current_image: None,
            current_image_path: None,
            zoom_level: 1.0,
            pan_offset: egui::Vec2::ZERO,
            error_message: None,
            is_loading: false,
            result_sender: Some(sender),
            result_receiver: Some(receiver),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn show(
        &mut self, 
        ctx: &egui::Context, 
        state: &mut AppState,
        images: &[crate::core::file_scanner::ImageFile],
    ) -> Option<ImageViewerAction> {
        let mut action = None;
        
        // Get current image
        let current_image_file = if let Some(selected_index) = state.selected_image {
            images.get(selected_index)
        } else {
            None
        };

        // Load image if needed
        if let Some(image_file) = current_image_file {
            if self.current_image_path != Some(image_file.path.clone()) {
                self.is_loading = true;
                self.error_message = None;
                self.load_image(ctx, &image_file.path);
                self.current_image_path = Some(image_file.path.clone());
            }
        }
        // ポーリングしてロード結果を反映（ColorImage→TextureHandle化はUIスレッドで）
        if let Some(receiver) = &mut self.result_receiver {
            if let Some(res) = receiver.try_recv().ok().and_then(|x| Some(x)) {
                match res {
                    Ok(color_image) => {
                        let texture = ctx.load_texture(
                            format!("full_image_{}", self.current_image_path.as_ref().unwrap_or(&PathBuf::default()).display()),
                            color_image,
                            egui::TextureOptions::LINEAR,
                        );
                        self.current_image = Some(texture);
                        self.is_loading = false;
                    }
                    Err(e) => {
                        self.error_message = Some(e);
                        self.is_loading = false;
                    }
                }
            }
        }

        // 非同期通知を待機
        if self.is_loading {
            let notify = self.notify.clone();
            let repaint_signal = ctx.clone().clone();
            tokio::spawn(async move {
                notify.notified().await;
                repaint_signal.request_repaint();
            });
        }

        // Full-screen modal
        egui::Window::new("")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .fixed_size(ctx.screen_rect().size())
            .frame(egui::Frame::none().fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                // Handle only close action (Escape key), navigation is handled in app.rs
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    action = Some(ImageViewerAction::Close);
                }

                // Image display area (full screen)
                let image_rect = ui.available_rect_before_wrap();
                
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(image_rect), |ui| {
                    if self.is_loading {
                        // Show loading state
                        let center = ui.available_rect_before_wrap().center();
                        ui.painter().text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            "画像を読み込み中...",  // "Loading image..."
                            egui::FontId::proportional(24.0),
                            egui::Color32::WHITE,
                        );
                    } else if let Some(error) = &self.error_message {
                        // Show error state
                        let center = ui.available_rect_before_wrap().center();
                        ui.painter().text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            &format!("読み込みエラー: {}", error),  // "Loading error: {}"
                            egui::FontId::proportional(18.0),
                            egui::Color32::from_rgb(255, 150, 150),
                        );
                        
                        // Show filename even when there's an error
                        if let Some(image_file) = current_image_file {
                            ui.painter().text(
                                egui::Pos2::new(center.x, center.y + 40.0),
                                egui::Align2::CENTER_CENTER,
                                &format!("ファイル: {}", image_file.name),  // "File: {}"
                                egui::FontId::proportional(14.0),
                                egui::Color32::WHITE,
                            );
                        }
                    } else if let Some(texture) = &self.current_image {
                        let texture_size = texture.size_vec2();
                        let available_size = ui.available_size();
                        
                        // Calculate size to fit the image within available space while maintaining aspect ratio
                        let scale_x = available_size.x / texture_size.x;
                        let scale_y = available_size.y / texture_size.y;
                        let scale = scale_x.min(scale_y) * self.zoom_level;
                        
                        let display_size = texture_size * scale;
                        let center_offset = (available_size - display_size) * 0.5;
                        
                        let image_rect = egui::Rect::from_min_size(
                            ui.min_rect().min + center_offset + self.pan_offset,
                            display_size,
                        );
                        
                        // Draw the image directly with the painter to avoid layout issues
                        ui.painter().image(
                            texture.id(),
                            image_rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        
                        // Draw image info overlay at the top
                        if let Some(index) = state.selected_image {
                            let info_text = format!("{} / {}", index + 1, images.len());
                            let info_rect = egui::Rect::from_min_size(
                                egui::Pos2::new(ui.available_width() - 150.0, 10.0),
                                egui::Vec2::new(140.0, 30.0),
                            );
                            
                            // Semi-transparent background
                            ui.painter().rect_filled(
                                info_rect,
                                egui::Rounding::same(5.0),
                                egui::Color32::from_black_alpha(150),
                            );
                            
                            // Text
                            ui.painter().text(
                                info_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                &info_text,
                                egui::FontId::proportional(16.0),
                                egui::Color32::WHITE,
                            );
                            
                            // Draw filename overlay at the top left
                            if let Some(image_file) = current_image_file {
                                let filename_rect = egui::Rect::from_min_size(
                                    egui::Pos2::new(10.0, 10.0),
                                    egui::Vec2::new(ui.available_width() - 170.0, 30.0),
                                );
                                
                                // Semi-transparent background
                                ui.painter().rect_filled(
                                    filename_rect,
                                    egui::Rounding::same(5.0),
                                    egui::Color32::from_black_alpha(150),
                                );
                                
                                // Filename text (truncate if too long)
                                let filename = if image_file.name.len() > 50 {
                                    format!("{}...", &image_file.name[..47])
                                } else {
                                    image_file.name.clone()
                                };
                                
                                ui.painter().text(
                                    egui::Pos2::new(filename_rect.min.x + 10.0, filename_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    &filename,
                                    egui::FontId::proportional(14.0),
                                    egui::Color32::WHITE,
                                );
                            }
                        }
                        
                        // Handle mouse interactions for full screen
                        let response = ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::click_and_drag());
                        
                        // Pan with mouse drag
                        if response.dragged() {
                            self.pan_offset += response.drag_delta();
                        }
                        
                        // Zoom with scroll wheel
                        ui.input(|i| {
                            if i.raw_scroll_delta.y != 0.0 {
                                let zoom_factor = if i.raw_scroll_delta.y > 0.0 { 1.1 } else { 0.9 };
                                self.zoom_level = (self.zoom_level * zoom_factor).clamp(0.1, 10.0);
                            }
                        });
                    } else {
                        // Show placeholder when no image is loaded but not in error/loading state
                        let center = ui.available_rect_before_wrap().center();
                        ui.painter().text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            "画像なし",  // "No image"
                            egui::FontId::proportional(18.0),
                            egui::Color32::WHITE,
                        );
                    }
                });
            });
        
        action
    }
    
    fn load_image(&mut self, ctx: &egui::Context, path: &PathBuf) {
        use tokio::task;
        self.current_image = None;
        self.error_message = None;
        self.zoom_level = 1.0;
        self.pan_offset = egui::Vec2::ZERO;
        self.is_loading = true;

        let sender = self.result_sender.clone().unwrap();
        let path = path.clone();
        let notify = self.notify.clone();

        task::spawn(async move {
            let res = match ImageLoader::load_image(&path) {
                Ok(image) => {
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        [image.width() as usize, image.height() as usize],
                        &image.to_rgba8(),
                    );
                    Ok(color_image)
                }
                Err(e) => Err(e.to_string()),
            };
            sender.send(res).await.unwrap();
            notify.notify_one();
        });
    }
}