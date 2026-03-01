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
    grid_cols: usize,
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
            grid_cols: 1,
        }
    }

    pub fn load_folder(&mut self, path: PathBuf) {
        if let Ok(scanner) = self.file_scanner.lock() {
            if let Ok(images) = scanner.scan_images_in_directory(&path) {
                // Clear old data when loading new folder
                self.current_images = images;
                self.selected_index = None;
                self.thumbnails.clear();
                self.loading_thumbnails.clear();
                println!("Loaded {} images from {}", self.current_images.len(), path.display());
            } else {
                println!("Failed to scan directory: {}", path.display());
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, thumbnail_size: f32, is_focused: bool, viewer_open: bool) -> (Option<usize>, bool, bool) {
        self.thumbnail_size = thumbnail_size;
        let mut selected_image = None;
        let mut was_clicked = false;
        let mut should_open_viewer = false;
        let available_width = ui.available_width();
        // Calculate columns considering padding and margins
        let padding = ui.style().spacing.item_spacing.x * 2.0; // Left and right padding
        let effective_width = (available_width - padding).max(thumbnail_size);
        let cols = (effective_width / thumbnail_size).max(1.0) as usize;
        self.grid_cols = cols;

        // Show focus indicator with darker color
        if is_focused {
            ui.painter().rect_stroke(
                ui.available_rect_before_wrap(),
                egui::Rounding::same(2.0),
                egui::Stroke::new(2.0, egui::Color32::from_rgb(40, 80, 40)), // Darker green
            );
        }

        // Clone the images to avoid borrowing issues
        let images_to_process = self.current_images.clone();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // Handle keyboard navigation only when focused and viewer is not open
            let (should_scroll, open_viewer) = if is_focused && !viewer_open {
                self.handle_keyboard_input(ui, cols)
            } else {
                (false, false)
            };
            
            if open_viewer {
                should_open_viewer = true;
            }
            
            // Calculate updated selection rect after keyboard input
            let selected_rect = if let Some(selected_idx) = self.selected_index {
                let row = selected_idx / cols;
                let col = selected_idx % cols;
                // Note: This rect calculation is approximate for scrolling purposes
                // The actual rendering uses UI layout positioning
                Some(egui::Rect::from_min_size(
                    egui::Pos2::new(
                        col as f32 * thumbnail_size,
                        row as f32 * (thumbnail_size + 20.0), // Include text height
                    ),
                    egui::Vec2::new(thumbnail_size, thumbnail_size + 20.0),
                ))
            } else {
                None
            };
            
            // Only scroll when keyboard navigation occurred
            if should_scroll && is_focused {
                if let Some(rect) = selected_rect {
                    ui.scroll_to_rect(rect, Some(egui::Align::Min));
                }
            }
            // Manual grid layout with proper UI positioning
            let item_height = thumbnail_size + 20.0;
            let total_rows = (images_to_process.len() + cols - 1) / cols; // Ceiling division
            
            // Use vertical layout for rows
            ui.vertical(|ui| {
                for row in 0..total_rows {
                    ui.horizontal(|ui| {
                        for col in 0..cols {
                            let index = row * cols + col;
                            if index >= images_to_process.len() {
                                break;
                            }
                            
                            let image_file = &images_to_process[index];
                            let is_selected = self.selected_index == Some(index);
                            
                            let response = ui.allocate_response(
                                egui::Vec2::new(thumbnail_size, item_height),
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
                                
                                ui.painter().image(
                                    texture.id(),
                                    image_rect,
                                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                                    egui::Color32::WHITE,
                                );
                                
                                // Draw filename
                                let text_pos = egui::Pos2::new(response.rect.min.x, response.rect.min.y + thumbnail_size);
                                ui.painter().text(
                                    text_pos,
                                    egui::Align2::LEFT_TOP,
                                    &image_file.name,
                                    egui::FontId::proportional(12.0),
                                    if is_selected { egui::Color32::WHITE } else { ui.style().visuals.text_color() },
                                );
                            } else {
                                // Loading placeholder or error state
                                let is_loading = self.loading_thumbnails.contains(&image_file.path);
                                let bg_color = if is_loading {
                                    egui::Color32::from_gray(60)  // Dark gray for loading
                                } else {
                                    egui::Color32::from_rgb(80, 60, 60)  // Reddish for error
                                };
                                
                                ui.painter().rect_filled(
                                    response.rect,
                                    egui::Rounding::same(5.0),
                                    bg_color,
                                );
                                
                                let center = response.rect.center();
                                let text = if is_loading {
                                    "読み込み中..."  // Japanese "Loading..."
                                } else {
                                    "エラー"  // Japanese "Error"
                                };
                                
                                ui.painter().text(
                                    center,
                                    egui::Align2::CENTER_CENTER,
                                    text,
                                    egui::FontId::default(),
                                    egui::Color32::WHITE,
                                );
                            }
                            
                            if response.clicked() {
                                self.selected_index = Some(index);
                                selected_image = Some(index);
                                was_clicked = true;
                            }
                            
                            // Handle double-click to open viewer
                            if response.double_clicked() {
                                self.selected_index = Some(index);
                                selected_image = Some(index);
                                should_open_viewer = true;
                            }
                        }
                    });
                }
            });
        });

        // Load thumbnails for images that need it (after the immutable borrow ends)
        let mut images_to_load = Vec::new();
        for image_file in &images_to_process {
            if !self.thumbnails.contains_key(&image_file.path) && 
               !self.loading_thumbnails.contains(&image_file.path) {
                images_to_load.push(image_file.clone());
            }
        }
        
        // Load thumbnails for new images
        for image_file in images_to_load {
            self.load_thumbnail(ui.ctx(), image_file);
        }



        (selected_image, was_clicked, should_open_viewer)
    }

    fn load_thumbnail(&mut self, ctx: &egui::Context, image_file: ImageFile) {
        // Mark as loading to prevent duplicate loading attempts
        self.loading_thumbnails.insert(image_file.path.clone());
        
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
                
                let texture_id = format!("thumbnail_{}_{}", 
                    image_file.path.file_name().unwrap_or_default().to_string_lossy(),
                    image_file.modified.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
                );
                
                let texture = ctx.load_texture(
                    texture_id,
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                
                self.thumbnails.insert(image_file.path.clone(), texture);
                self.loading_thumbnails.remove(&image_file.path);
                return;
            }
        }
        
        // Load thumbnail synchronously to avoid thread spawn issues
        match ImageLoader::load_thumbnail(&image_file.path, 160) {
            Ok(thumbnail) => {
                // Store in cache
                if let Ok(mut cache) = self.thumbnail_cache.lock() {
                    cache.put(cache_key, thumbnail.clone());
                }
                
                // Convert to texture with unique ID
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [thumbnail.width() as usize, thumbnail.height() as usize],
                    &thumbnail.to_rgba8(),
                );
                
                let texture_id = format!("thumbnail_{}_{}", 
                    image_file.path.file_name().unwrap_or_default().to_string_lossy(),
                    image_file.modified.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
                );
                
                let texture = ctx.load_texture(
                    texture_id,
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                
                self.thumbnails.insert(image_file.path.clone(), texture);
                self.loading_thumbnails.remove(&image_file.path);
                
                // Request repaint to update UI
                ctx.request_repaint();
            }
            Err(e) => {
                println!("Failed to load thumbnail for {}: {}", image_file.path.display(), e);
                // Remove from loading set if failed and create an error placeholder
                self.loading_thumbnails.remove(&image_file.path);
                
                // Create error placeholder texture
                let error_image = create_error_placeholder_image();
                let texture_id = format!("error_{}_{}", 
                    image_file.path.file_name().unwrap_or_default().to_string_lossy(),
                    image_file.modified.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
                );
                
                let texture = ctx.load_texture(
                    texture_id,
                    error_image,
                    egui::TextureOptions::LINEAR,
                );
                
                self.thumbnails.insert(image_file.path.clone(), texture);
            }
        }
    }

    fn handle_keyboard_input(&mut self, ui: &mut egui::Ui, cols: usize) -> (bool, bool) {
        let mut selection_changed = false;
        let mut should_open_viewer = false;
        
        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.move_selection(-1, cols as i32);
            selection_changed = true;
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.move_selection(1, cols as i32);
            selection_changed = true;
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            self.move_selection_horizontal(-1, cols);
            selection_changed = true;
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.move_selection_horizontal(1, cols);
            selection_changed = true;
        }
        
        // Check for Space or Enter key to open viewer
        if ui.input(|i| i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::Enter)) {
            if self.selected_index.is_some() {
                should_open_viewer = true;
            }
        }
        
        (selection_changed, should_open_viewer)
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

    fn move_selection_horizontal(&mut self, direction: i32, cols: usize) {
        if self.current_images.is_empty() {
            return;
        }
        
        let current = self.selected_index.unwrap_or(0);
        let current_row = current / cols;
        let current_col = current % cols;
        
        let new_col = if direction > 0 {
            // Moving right
            if current_col + 1 < cols && current + 1 < self.current_images.len() {
                current_col + 1
            } else {
                current_col // Stay in same position if at edge
            }
        } else {
            // Moving left
            if current_col > 0 {
                current_col - 1
            } else {
                current_col // Stay in same position if at edge
            }
        };
        
        let new_index = current_row * cols + new_col;
        if new_index < self.current_images.len() {
            self.selected_index = Some(new_index);
        }
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
    
    pub fn set_selected_index(&mut self, index: usize) {
        if index < self.current_images.len() {
            self.selected_index = Some(index);
        }
    }
    
    pub fn get_current_images(&self) -> Vec<ImageFile> {
        self.current_images.clone()
    }
}

// Helper function to create error placeholder image
fn create_error_placeholder_image() -> egui::ColorImage {
    let size = 160;
    let mut pixels = vec![egui::Color32::from_rgb(80, 80, 80); size * size];
    
    // Draw a simple "X" pattern for error indication
    for i in 0..size {
        for j in 0..size {
            if (i == j) || (i + j == size - 1) {
                if i > 2 && i < size - 3 && j > 2 && j < size - 3 {
                    pixels[i * size + j] = egui::Color32::from_rgb(200, 100, 100);
                }
            }
        }
    }
    
    egui::ColorImage::from_rgba_unmultiplied(
        [size, size],
        &pixels.iter().flat_map(|c| [c.r(), c.g(), c.b(), c.a()]).collect::<Vec<u8>>()
    )
}