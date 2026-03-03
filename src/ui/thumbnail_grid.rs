mod loading;

use eframe::egui;
use std::sync::mpsc::{self, Receiver, Sender};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crate::core::{
    file_scanner::{FileScanner, ImageFile},
    thumbnail_cache::ThumbnailCache,
};
use crate::ui::app::LoadingState;

pub struct ThumbnailGrid {
    file_scanner: Arc<Mutex<FileScanner>>,
    thumbnail_cache: Arc<Mutex<ThumbnailCache>>,
    current_images: Vec<ImageFile>,
    selected_index: Option<usize>,
    thumbnails: HashMap<PathBuf, egui::TextureHandle>,
    loading_thumbnails: std::collections::HashSet<PathBuf>,
    loading_started_at: HashMap<PathBuf, Instant>,
    thumbnail_size: f32,
    grid_cols: usize,
    priority_load_queue: VecDeque<PathBuf>,
    thumbnail_result_sender: Sender<(PathBuf, image::DynamicImage)>,
    thumbnail_result_receiver: Receiver<(PathBuf, image::DynamicImage)>,
}

impl ThumbnailGrid {
    pub fn new(
        file_scanner: Arc<Mutex<FileScanner>>,
        thumbnail_cache: Arc<Mutex<ThumbnailCache>>,
    ) -> Self {
        let (thumbnail_result_sender, thumbnail_result_receiver) = mpsc::channel();

        Self {
            file_scanner,
            thumbnail_cache,
            current_images: Vec::new(),
            selected_index: None,
            thumbnails: HashMap::new(),
            loading_thumbnails: std::collections::HashSet::new(),
            loading_started_at: HashMap::new(),
            thumbnail_size: 160.0,
            grid_cols: 1,
            priority_load_queue: VecDeque::new(),
            thumbnail_result_sender,
            thumbnail_result_receiver,
        }
    }

    // New method for async loading
    pub fn clear_images(&mut self) {
        self.current_images.clear();
        self.selected_index = None;
        self.thumbnails.clear();
        self.loading_thumbnails.clear();
        self.loading_started_at.clear();
        self.priority_load_queue.clear();
    }
    
    pub fn set_images(&mut self, images: Vec<ImageFile>) {
        self.current_images = images;
        self.selected_index = None;
        println!("ThumbnailGrid: Set {} images", self.current_images.len());
    }
    
    pub fn add_image(&mut self, image: ImageFile, ctx: &egui::Context) {
        println!("📁 Found new image file: {} - immediately visible", image.name);
        self.current_images.push(image.clone());
        println!("[add_image] current_images.len() = {}", self.current_images.len());
        // 画像追加時に必ずサムネイル生成を開始
        self.load_thumbnail_async(&image.path, ctx);
        // 強制的に複数回repaint
        ctx.request_repaint();
        ctx.request_repaint_after(std::time::Duration::from_millis(1));
        ctx.request_repaint_after(std::time::Duration::from_millis(5));
    }
    
    pub fn prioritize_thumbnail_load(&mut self, path: PathBuf) {
        if !self.priority_load_queue.contains(&path) {
            self.priority_load_queue.push_front(path);
        }
    }
    
    pub fn get_image_path_at_index(&self, index: usize) -> Option<PathBuf> {
        self.current_images.get(index).map(|img| img.path.clone())
    }

    // Legacy method - kept for compatibility but now just calls clear_images
    pub fn load_folder(&mut self, path: PathBuf) {
        self.clear_images();
        
        let images = {
            if let Ok(scanner) = self.file_scanner.lock() {
                scanner.scan_images_in_directory(&path)
            } else {
                Err(anyhow::anyhow!("Failed to acquire scanner lock"))
            }
        };
        
        match images {
            Ok(images) => {
                self.set_images(images);
            }
            Err(_) => {
                println!("Failed to scan directory: {}", path.display());
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, thumbnail_size: f32, is_focused: bool, viewer_open: bool, loading_state: &LoadingState) -> (Option<usize>, bool, bool) {
        self.cleanup_stale_loading();
        self.process_thumbnail_results(ui.ctx());
        println!("[show] current_images.len() = {}", self.current_images.len());
        self.thumbnail_size = thumbnail_size;
        let mut selected_image = None;
        let mut was_clicked = false;
        let mut should_open_viewer = false;
        let available_width = ui.available_width();
        // Calculate columns considering padding and margins
        let padding = ui.style().spacing.item_spacing.x * 2.0 + 10.0; // Add extra padding for better spacing
        let effective_width = (available_width - padding).max(thumbnail_size);
        let cols = (effective_width / (thumbnail_size + 10.0)).max(1.0) as usize; // Adjust for spacing between thumbnails
        self.grid_cols = cols;

        // Show focus indicator with darker color
        if is_focused {
            ui.painter().rect_stroke(
                ui.available_rect_before_wrap(),
                egui::Rounding::same(2.0),
                egui::Stroke::new(2.0, egui::Color32::from_rgb(40, 80, 40)), // Darker green
            );
        }
        
        // Show loading state when appropriate, but don't prevent normal display after loading
        match loading_state {
            LoadingState::Loading => {
                if self.current_images.is_empty() {
                    // 画像が1枚もない場合のみテキスト＋spinnerを表示してreturn
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.spinner();
                        ui.label("フォルダをスキャン中...");
                        ui.label("見つかった画像がすぐに表示されます");
                        ui.add_space(20.0);
                    });
                    return (selected_image, was_clicked, should_open_viewer);
                } else {
                    // 画像が1枚でもあれば、グリッド上部にスキャン中テキストを表示し、必ずグリッド描画
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(format!("スキャン中... ({} 個の画像が表示中)", self.current_images.len()));
                    });
                    ui.separator();
                }
            }
            LoadingState::Failed(error) => {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.colored_label(egui::Color32::RED, "Failed to load folder");
                    ui.label(error);
                    ui.add_space(20.0);
                    if ui.button("Retry").clicked() {
                        // Request to reload - would need to be handled by parent
                    }
                });
                return (selected_image, was_clicked, should_open_viewer);
            }
            LoadingState::Loaded | LoadingState::Idle => {
                // Normal operation - proceed to show images
            }
        }

        // Show empty state if no images（ロード完了時のみ）
        if self.current_images.is_empty() {
            println!("=== ThumbnailGrid: No images to display - current_images.len() = {} ===", self.current_images.len());
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No images found");
            });
            return (selected_image, was_clicked, should_open_viewer);
        }

        println!("=== ThumbnailGrid: About to show {} images in grid ===", self.current_images.len());

        self.show_image_grid(ui, is_focused, viewer_open, &mut selected_image, &mut was_clicked, &mut should_open_viewer);

        (selected_image, was_clicked, should_open_viewer)
    }
    
    fn show_image_grid(&mut self, ui: &mut egui::Ui, is_focused: bool, viewer_open: bool, selected_image: &mut Option<usize>, was_clicked: &mut bool, should_open_viewer: &mut bool) {
        println!("=== show_image_grid called with {} images ===", self.current_images.len());
        println!("Thumbnails cache size: {}", self.thumbnails.len());
        println!("Loading thumbnails: {}", self.loading_thumbnails.len());
        
        // Clone the images to avoid borrowing issues
        let images_to_process = self.current_images.clone();
        let cols = self.grid_cols;
        
        println!("Grid cols: {}, Images to process: {}", cols, images_to_process.len());

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // Handle keyboard navigation only when focused and viewer is not open
            let (should_scroll, open_viewer) = if is_focused && !viewer_open {
                self.handle_keyboard_input(ui, cols)
            } else {
                (false, false)
            };
            
            if open_viewer {
                *should_open_viewer = true;
            }
            
            // Calculate updated selection rect after keyboard input
            let selected_rect = if let Some(selected_idx) = self.selected_index {
                let row = selected_idx / cols;
                let col = selected_idx % cols;
                // Note: This rect calculation is approximate for scrolling purposes
                // The actual rendering uses UI layout positioning
                Some(egui::Rect::from_min_size(
                    egui::Pos2::new(
                        col as f32 * self.thumbnail_size,
                        row as f32 * (self.thumbnail_size + 20.0), // Include text height
                    ),
                    egui::Vec2::new(self.thumbnail_size, self.thumbnail_size + 20.0),
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
            let item_height = self.thumbnail_size + 20.0;
            let total_rows = (images_to_process.len() + cols - 1) / cols; // Ceiling division
            
            // Render image grid efficiently
            println!("🎨 Rendering {} images with {} thumbnails loaded", images_to_process.len(), self.thumbnails.len());
            
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
                            
                            // Only log when thumbnail state changes
                            if index < 5 || self.thumbnails.contains_key(&image_file.path) {
                                println!("🖼️ {} - Thumbnail: {}", 
                                    image_file.name, 
                                    if self.thumbnails.contains_key(&image_file.path) { "Ready" } else { "Loading" }
                                );
                            }
                            
                            let response = ui.allocate_response(
                                egui::Vec2::new(self.thumbnail_size, item_height),
                                egui::Sense::click(),
                            );
                            
                            // Draw thumbnail or placeholder
                            let image_rect = egui::Rect::from_min_size(
                                response.rect.min,
                                egui::Vec2::new(self.thumbnail_size, self.thumbnail_size),
                            );
                            
                            if is_selected {
                                ui.painter().rect_filled(
                                    response.rect,
                                    egui::Rounding::same(5.0),
                                    egui::Color32::from_rgb(80, 120, 180), // Softer blue for selection
                                );
                            }
                            
                            if let Some(texture) = self.thumbnails.get(&image_file.path) {
                                println!("Found texture for {}, rendering with ID: {:?}", image_file.name, texture.id());
                                ui.painter().image(
                                    texture.id(),
                                    image_rect,
                                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                                    egui::Color32::WHITE,
                                );
                            } else {
                                println!("No texture for {}, rendering placeholder", image_file.name);
                                // Show highly visible placeholder immediately when file is found
                                let is_loading = self.loading_thumbnails.contains(&image_file.path);
                                
                                // Adjust placeholder colors to be less distracting
                                let bg_color = if is_loading {
                                    egui::Color32::from_rgb(100, 100, 100) // Neutral gray for loading
                                } else {
                                    egui::Color32::from_rgb(150, 150, 150) // Slightly lighter gray for found but not loading
                                };
                                
                                // Draw prominent background
                                ui.painter().rect_filled(
                                    image_rect,
                                    egui::Rounding::same(8.0),  // Larger rounding for visibility
                                    bg_color,
                                );
                                
                                // Add thick white border for high contrast
                                ui.painter().rect_stroke(
                                    image_rect,
                                    egui::Rounding::same(8.0),
                                    egui::Stroke::new(3.0, egui::Color32::WHITE),
                                );
                                
                                // Add inner border for extra visibility
                                let inner_rect = image_rect.shrink(8.0);
                                ui.painter().rect_stroke(
                                    inner_rect,
                                    egui::Rounding::same(4.0),
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 200)),
                                );
                                
                                let center = image_rect.center();
                                
                                // Draw large image icon/symbol
                                ui.painter().text(
                                    center + egui::Vec2::new(0.0, -15.0),
                                    egui::Align2::CENTER_CENTER,
                                    "🖼️",  // Image emoji for immediate recognition
                                    egui::FontId::proportional(24.0),
                                    egui::Color32::WHITE,
                                );
                                
                                let status_text = if is_loading {
                                    "読み込み中"
                                } else {
                                    "画像ファイル"
                                };
                                
                                // Draw status text with larger font
                                ui.painter().text(
                                    center + egui::Vec2::new(0.0, 10.0),
                                    egui::Align2::CENTER_CENTER,
                                    status_text,
                                    egui::FontId::proportional(14.0),
                                    egui::Color32::WHITE,
                                );
                                
                                // Auto-start thumbnail loading in background after showing placeholder
                                if !is_loading && !self.thumbnails.contains_key(&image_file.path) {
                                    println!("Starting background thumbnail load for: {}", image_file.name);
                                    self.load_thumbnail_async(&image_file.path, ui.ctx());
                                }
                            }
                            
                            // Draw prominent filename - make file presence immediately obvious
                            let display_name = if image_file.name.len() > 12 {
                                format!("{}...", &image_file.name[..9])
                            } else {
                                image_file.name.clone()
                            };
                            
                            let text_pos = egui::Pos2::new(response.rect.min.x, response.rect.min.y + self.thumbnail_size);
                            
                            // Draw larger text area for filename
                            let text_rect = egui::Rect::from_min_size(
                                text_pos,
                                egui::Vec2::new(self.thumbnail_size, 20.0),
                            );
                            
                            // Prominent background for filename visibility
                            let filename_bg_color = if is_selected {
                                egui::Color32::from_rgb(100, 150, 200)  // Blue when selected
                            } else {
                                egui::Color32::from_black_alpha(150)    // Semi-transparent black
                            };
                            
                            ui.painter().rect_filled(
                                text_rect,
                                egui::Rounding::same(4.0),
                                filename_bg_color,
                            );
                            
                            // White border around filename for extra visibility
                            ui.painter().rect_stroke(
                                text_rect,
                                egui::Rounding::same(4.0),
                                egui::Stroke::new(1.0, egui::Color32::WHITE),
                            );
                            
                            // Large, bold filename text
                            ui.painter().text(
                                text_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                &display_name,
                                egui::FontId::proportional(11.0),  // Slightly larger font
                                egui::Color32::WHITE,
                            );
                            
                            if response.clicked() {
                                self.selected_index = Some(index);
                                *selected_image = Some(index);
                                *was_clicked = true;
                            }
                            
                            // Handle double-click to open viewer
                            if response.double_clicked() {
                                self.selected_index = Some(index);
                                *selected_image = Some(index);
                                *should_open_viewer = true;
                            }
                        }
                    });
                }
            });
        });
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
    
    // Debug method to get detailed image information
    pub fn get_image_debug_info(&self) -> (usize, Vec<String>) {
        let count = self.current_images.len();
        let names = self.current_images.iter().take(10).map(|img| img.name.clone()).collect();
        (count, names)
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
    
    pub fn update_progress(&self, ui: &mut egui::Ui, total_images: usize) {
        let loaded_images = self.current_images.len();
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(format!("スキャン中... ({} / {} 個の画像が表示中)", loaded_images, total_images));
        });
        ui.separator();
    }
}

impl ThumbnailGrid {
    fn cleanup_stale_loading(&mut self) {
        let now = Instant::now();
        let timeout = Duration::from_secs(20);

        let stale_paths: Vec<PathBuf> = self
            .loading_started_at
            .iter()
            .filter_map(|(path, started)| {
                if now.duration_since(*started) > timeout {
                    Some(path.clone())
                } else {
                    None
                }
            })
            .collect();

        for path in stale_paths {
            self.loading_started_at.remove(&path);
            self.loading_thumbnails.remove(&path);
        }
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

// Moved the println! statement into a valid function or block context
fn debug_thumbnail_cache() {
    println!("Debug: Checking thumbnail cache and loader");
}