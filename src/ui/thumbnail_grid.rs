use eframe::egui;
use image::GenericImageView;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::core::{
    file_scanner::{FileScanner, ImageFile},
    thumbnail_cache::ThumbnailCache,
    image_loader::ImageLoader,
};

pub struct ThumbnailGrid {
    file_scanner: Arc<Mutex<FileScanner>>,
    thumbnail_cache: Arc<Mutex<ThumbnailCache>>,
    current_images: Vec<ImageFile>,
    selected_index: Option<usize>,
    thumbnails: HashMap<PathBuf, egui::TextureHandle>,
    loading_thumbnails: std::collections::HashSet<PathBuf>,
    thumbnail_size: f32,
}

impl ThumbnailGrid {
    pub fn new(
        file_scanner: Arc<Mutex<FileScanner>>,
        thumbnail_cache: Arc<Mutex<ThumbnailCache>>,
    ) -> Self {
        Self {
            file_scanner,
            thumbnail_cache,
            current_images: Vec::new(),
            selected_index: None,
            thumbnails: HashMap::new(),
            loading_thumbnails: std::collections::HashSet::new(),
            thumbnail_size: 160.0,
        }
    }

    pub fn load_folder(&mut self, path: PathBuf) {
        if let Ok(scanner) = self.file_scanner.lock() {
            if let Ok(images) = scanner.scan_images_in_directory(&path) {
                self.current_images = images;
                self.selected_index = None;
                self.thumbnails.clear();
                self.loading_thumbnails.clear();
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, thumbnail_size: f32) -> Option<usize> {
        self.thumbnail_size = thumbnail_size;
        let mut selected_image = None;
        let available_width = ui.available_width();
        let cols = (available_width / (thumbnail_size + 10.0)).max(1.0) as usize;

        // Clone the images to avoid borrowing issues
        let images_to_process = self.current_images.clone();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let _grid_ui = egui::Grid::new("thumbnail_grid")
                .num_columns(cols)
                .spacing([5.0, 5.0])
                .show(ui, |ui| {
                    for (index, image_file) in images_to_process.iter().enumerate() {
                        let is_selected = self.selected_index == Some(index);
                        
                        let response = ui.allocate_response(
                            egui::Vec2::new(thumbnail_size, thumbnail_size + 20.0),
                            egui::Sense::click(),
                        );
                        
                        // Draw thumbnail
                        if let Some(texture) = self.thumbnails.get(&image_file.path) {
                            let image_rect = egui::Rect::from_min_size(
                                response.rect.min,
                                egui::Vec2::new(thumbnail_size, thumbnail_size),
                            );
                            
                            if is_selected {
                                ui.painter().rect_filled(
                                    response.rect,
                                    egui::Rounding::same(5.0),
                                    egui::Color32::from_rgb(100, 150, 200),
                                );
                            }
                            
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(image_rect), |ui| {
                                ui.image(egui::load::SizedTexture::new(texture.id(), egui::Vec2::new(thumbnail_size, thumbnail_size)));
                            });
                            
                            // Draw filename
                            let text_rect = egui::Rect::from_min_size(
                                egui::Pos2::new(response.rect.min.x, response.rect.min.y + thumbnail_size),
                                egui::Vec2::new(thumbnail_size, 20.0),
                            );
                            
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(text_rect), |ui| {
                                ui.label(
                                    egui::RichText::new(&image_file.name)
                                        .small()
                                        .color(if is_selected { egui::Color32::WHITE } else { ui.style().visuals.text_color() }),
                                );
                            });
                        } else {
                            // Loading placeholder
                            ui.painter().rect_filled(
                                response.rect,
                                egui::Rounding::same(5.0),
                                egui::Color32::from_gray(100),
                            );
                            
                            let center = response.rect.center();
                            ui.painter().text(
                                center,
                                egui::Align2::CENTER_CENTER,
                                "Loading...",
                                egui::FontId::default(),
                                egui::Color32::WHITE,
                            );
                            
                            // Start loading thumbnail if not already loading
                            if !self.loading_thumbnails.contains(&image_file.path) {
                                // We'll handle this after the immutable borrow ends
                                self.loading_thumbnails.insert(image_file.path.clone());
                            }
                        }
                        
                        if response.clicked() {
                            self.selected_index = Some(index);
                            selected_image = Some(index);
                        }
                        
                        // Handle double-click to open viewer
                        if response.double_clicked() {
                            self.selected_index = Some(index);
                            selected_image = Some(index);
                        }
                        
                        if (index + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
        });

        // Load thumbnails for images that need it (after the immutable borrow ends)
        for image_file in &images_to_process {
            if !self.thumbnails.contains_key(&image_file.path) && 
               self.loading_thumbnails.contains(&image_file.path) {
                self.load_thumbnail(ui.ctx(), image_file.clone());
            }
        }

        // Handle keyboard navigation
        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.move_selection(-1, cols as i32);
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.move_selection(1, cols as i32);
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            self.move_selection_horizontal(-1);
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.move_selection_horizontal(1);
        }

        selected_image
    }

    fn load_thumbnail(&mut self, ctx: &egui::Context, image_file: ImageFile) {
        let cache_key = crate::core::thumbnail_cache::ThumbnailCache::generate_key(
            &image_file.path,
            image_file.modified,
            image_file.size,
        );
        
        // Check cache first
        if let Ok(mut cache) = self.thumbnail_cache.lock() {
            if let Some(thumbnail) = cache.get(&cache_key) {
                // Convert to texture
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [thumbnail.width() as usize, thumbnail.height() as usize],
                    &thumbnail.to_rgba8(),
                );
                
                let texture = ctx.load_texture(
                    format!("thumbnail_{}", image_file.path.display()),
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                
                self.thumbnails.insert(image_file.path.clone(), texture);
                self.loading_thumbnails.remove(&image_file.path);
                return;
            }
        }
        
        // Load thumbnail in background
        let path = image_file.path.clone();
        let thumbnail_cache = self.thumbnail_cache.clone();
        let ctx_clone = ctx.clone();
        
        std::thread::spawn(move || {
            if let Ok(thumbnail) = ImageLoader::load_thumbnail(&path, 160) {
                // Store in cache
                if let Ok(mut cache) = thumbnail_cache.lock() {
                    cache.put(cache_key, thumbnail.clone());
                }
                
                // Convert to texture on main thread
                ctx_clone.request_repaint();
            }
        });
    }

    fn move_selection(&mut self, direction: i32, cols: i32) {
        if self.current_images.is_empty() {
            return;
        }
        
        let current = self.selected_index.unwrap_or(0) as i32;
        let new_index = (current + direction * cols)
            .max(0)
            .min(self.current_images.len() as i32 - 1) as usize;
        
        self.selected_index = Some(new_index);
    }

    fn move_selection_horizontal(&mut self, direction: i32) {
        if self.current_images.is_empty() {
            return;
        }
        
        let current = self.selected_index.unwrap_or(0) as i32;
        let new_index = (current + direction)
            .max(0)
            .min(self.current_images.len() as i32 - 1) as usize;
        
        self.selected_index = Some(new_index);
    }

    pub fn get_image_count(&self) -> usize {
        self.current_images.len()
    }

    pub fn get_current_image(&self) -> Option<&ImageFile> {
        if let Some(index) = self.selected_index {
            self.current_images.get(index)
        } else {
            None
        }
    }

    pub fn get_selected_index(&self) -> Option<usize> {
        self.selected_index
    }
}