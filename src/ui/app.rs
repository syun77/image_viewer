use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, SyncSender};
use tokio::runtime::Runtime;

use crate::core::{file_scanner::FileScanner, thumbnail_cache::ThumbnailCache};
use super::{tree_view::TreeView, thumbnail_grid::ThumbnailGrid, image_viewer::ImageViewer};

#[derive(Debug)]
pub enum ImageViewerAction {
    Close,
    Previous,
    Next,
}

#[derive(Debug)]
pub enum AsyncLoadMessage {
    FolderLoadStarted(PathBuf),
    FolderLoadCompleted(PathBuf, Vec<crate::core::file_scanner::ImageFile>),
    FolderLoadFailed(PathBuf, String),
    PriorityImageLoad(PathBuf),
    // New messages for progressive loading
    ImageFound(PathBuf, crate::core::file_scanner::ImageFile), // folder_path, image_file
    ScanProgress(PathBuf, usize, usize), // folder_path, current_count, total_estimated
    ScanCompleted(PathBuf, usize), // folder_path, actual_total_count
}

#[derive(Default, PartialEq)]
pub enum FocusState {
    #[default]
    TreeView,
    ThumbnailGrid,
}

#[derive(Default, PartialEq)]
pub enum LoadingState {
    #[default]
    Idle,
    Loading,
    Loaded,
    Failed(String),
}

pub struct AppState {
    pub current_folder: Option<PathBuf>,
    pub selected_image: Option<usize>,
    pub show_viewer: bool,
    pub left_panel_width: f32,
    pub viewer_size: f32,
    pub focus_state: FocusState,
    pub loading_state: LoadingState,
    pub priority_load_path: Option<PathBuf>,
    // Image count monitoring for missed images detection
    pub last_check_time: std::time::Instant,
    pub expected_image_count: Option<usize>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_folder: None,
            selected_image: None,
            show_viewer: false,
            left_panel_width: 250.0,
            viewer_size: 160.0,
            focus_state: FocusState::TreeView,
            loading_state: LoadingState::Idle,
            priority_load_path: None,
            last_check_time: std::time::Instant::now(),
            expected_image_count: None,
        }
    }
}

pub struct ImageViewerApp {
    state: AppState,
    file_scanner: Arc<Mutex<FileScanner>>,
    thumbnail_cache: Arc<Mutex<ThumbnailCache>>,
    tree_view: TreeView,
    thumbnail_grid: ThumbnailGrid,
    image_viewer: ImageViewer,
    root_path: String,
    // Async loading support
    async_runtime: Arc<Runtime>,
    load_receiver: Receiver<AsyncLoadMessage>,
    load_sender: SyncSender<AsyncLoadMessage>,
    // Frame counter for reduced repaint frequency
    frame_counter: u64,
}

impl ImageViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Enable continuous mode for high-frequency updates
        cc.egui_ctx.set_pixels_per_point(cc.egui_ctx.pixels_per_point()); // Ensure pixel perfect rendering
        
        let file_scanner = Arc::new(Mutex::new(FileScanner::new()));
        let thumbnail_cache = Arc::new(Mutex::new(ThumbnailCache::new()));
        let async_runtime = Arc::new(Runtime::new().expect("Failed to create tokio runtime"));
        
        // Use bounded channel with backpressure to control message flow
        // Buffer size balances memory usage and processing smoothness
        let (load_sender, load_receiver) = mpsc::sync_channel(200); // Max 200 pending messages
        
        Self {
            state: AppState {
                left_panel_width: 250.0,
                viewer_size: 160.0,
                last_check_time: std::time::Instant::now(),
                expected_image_count: None,
                ..Default::default()
            },
            file_scanner: file_scanner.clone(),
            thumbnail_cache: thumbnail_cache.clone(),
            tree_view: TreeView::new(file_scanner.clone()),
            thumbnail_grid: ThumbnailGrid::new(file_scanner.clone(), thumbnail_cache.clone()),
            image_viewer: ImageViewer::new(),
            root_path: String::new(),
            async_runtime,
            load_receiver,
            load_sender,
            frame_counter: 0,
        }
    }

    fn handle_focus_navigation(&mut self, ctx: &egui::Context) {
        // Always enable focus navigation regardless of loading state
        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
            self.state.focus_state = match self.state.focus_state {
                FocusState::TreeView => FocusState::ThumbnailGrid,
                FocusState::ThumbnailGrid => FocusState::TreeView,
            };
        }
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("ルートパス:");  // Japanese test
            
            // Ensure text edit is always enabled regardless of loading state
            ui.add_enabled(true, egui::TextEdit::singleline(&mut self.root_path));
            
            // Ensure buttons are always enabled regardless of loading state
            if ui.add_enabled(true, egui::Button::new("参照")).clicked() {  // Japanese "Browse" button
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.root_path = path.to_string_lossy().to_string();
                    self.set_root_path(path);
                }
            }
            
            if ui.add_enabled(true, egui::Button::new("設定")).clicked() {  // Japanese "Set" button
                let path = PathBuf::from(&self.root_path);
                if path.exists() {
                    self.set_root_path(path);
                }
            }
            
            // Debug: show Japanese characters explicitly
            ui.label("テスト: 日本語表示");
        });
    }

    fn set_root_path(&mut self, path: PathBuf) {
        println!("Setting root path: {}", path.display());
        self.state.current_folder = Some(path.clone());
        self.tree_view.set_root(path.clone());
        self.load_folder_async(path);
    }
    
    fn load_folder_async(&mut self, path: PathBuf) {
        println!("Starting sequential async load for: {}", path.display());
        self.state.loading_state = LoadingState::Loading;
        self.thumbnail_grid.clear_images();

        let sender = self.load_sender.clone();
        let file_scanner = self.file_scanner.clone();
        let runtime = self.async_runtime.clone();
        let path_clone = path.clone();

        runtime.spawn(async move {
            println!("Async sequential task started for: {}", path_clone.display());
            let _ = sender.send(AsyncLoadMessage::FolderLoadStarted(path_clone.clone()));

            // 画像パス一覧をblocking poolで取得
            let image_paths = tokio::task::spawn_blocking({
                let file_scanner = file_scanner.clone();
                let path_clone = path_clone.clone();
                move || {
                    if let Ok(scanner) = file_scanner.lock() {
                        scanner.get_image_paths_in_directory(&path_clone)
                    } else {
                        Err(anyhow::anyhow!("Failed to acquire scanner lock"))
                    }
                }
            }).await.unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking failed: {}", e)));

            match image_paths {
                Ok(paths) => {
                    let total_count = paths.len();
                    for (idx, img_path) in paths.iter().enumerate() {
                        // 1枚ロード（blocking poolで）
                        let image_file = tokio::task::spawn_blocking({
                            let file_scanner = file_scanner.clone();
                            let img_path = img_path.clone();
                            move || {
                                if let Ok(scanner) = file_scanner.lock() {
                                    scanner.load_image_file(&img_path)
                                } else {
                                    Err(anyhow::anyhow!("Failed to acquire scanner lock"))
                                }
                            }
                        }).await.unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking failed: {}", e)));

                        match image_file {
                            Ok(img) => {
                                // 1枚通知
                                let _ = sender.send(AsyncLoadMessage::ImageFound(path_clone.clone(), img));
                                // 進捗通知
                                let _ = sender.send(AsyncLoadMessage::ScanProgress(path_clone.clone(), idx+1, total_count));
                            }
                            Err(e) => {
                                println!("Failed to load image file: {}", e);
                            }
                        }
                        // 1枚ごとにawaitでUIへ制御を戻す
                        tokio::time::sleep(std::time::Duration::from_millis(0)).await;
                    }
                    // 完了通知
                    let _ = sender.send(AsyncLoadMessage::FolderLoadCompleted(path_clone.clone(), vec![]));
                    let _ = sender.send(AsyncLoadMessage::ScanCompleted(path_clone.clone(), total_count));
                }
                Err(e) => {
                    println!("Scan failed for {}: {}", path_clone.display(), e);
                    let _ = sender.send(AsyncLoadMessage::FolderLoadFailed(path_clone.clone(), e.to_string()));
                    let _ = sender.send(AsyncLoadMessage::ScanCompleted(path_clone.clone(), 0));
                }
            }
        });
    }
    
    fn request_priority_load(&mut self, path: PathBuf) {
        self.state.priority_load_path = Some(path.clone());
        let sender = self.load_sender.clone();
        let _ = sender.send(AsyncLoadMessage::PriorityImageLoad(path));
    }
    
    fn handle_async_messages(&mut self, ctx: &egui::Context) {
        let mut message_count = 0;
        let max_messages_per_frame = 200; // Process many more messages for instant responsiveness
        
        // Process all available messages immediately without batching
        while let Ok(msg) = self.load_receiver.try_recv() {
            message_count += 1;
            match msg {
                AsyncLoadMessage::FolderLoadStarted(_path) => {
                    println!("Setting loading state to Loading");
                    self.state.loading_state = LoadingState::Loading;
                    ctx.request_repaint();
                }
                AsyncLoadMessage::ImageFound(folder_path, image_file) => {
                    if Some(&folder_path) == self.state.current_folder.as_ref() {
                        // 画像発見時に必ず即追加・即表示
                        self.add_images_to_grid(&vec![image_file], ctx);
                        ctx.request_repaint();
                        ctx.request_repaint_after(std::time::Duration::from_millis(1));
                    }
                }
                AsyncLoadMessage::ScanProgress(folder_path, current, total) => {
                    if Some(&folder_path) == self.state.current_folder.as_ref() {
                        println!("Scan progress: {}/{} images", current, total);
                    }
                }
                AsyncLoadMessage::FolderLoadCompleted(path, _images) => {
                    if Some(&path) == self.state.current_folder.as_ref() {
                        println!("Progressive loading completed, setting state to Loaded");
                        // Only set to Loaded after completion - images are already available during loading
                        self.state.loading_state = LoadingState::Loaded;
                        ctx.request_repaint();
                    }
                }
                AsyncLoadMessage::FolderLoadFailed(_path, error) => {
                    println!("Setting loading state to Failed: {}", error);
                    self.state.loading_state = LoadingState::Failed(error);
                }
                AsyncLoadMessage::PriorityImageLoad(path) => {
                    self.thumbnail_grid.prioritize_thumbnail_load(path);
                }
                AsyncLoadMessage::ScanCompleted(path, actual_total_count) => {
                    if Some(&path) == self.state.current_folder.as_ref() {
                        println!("ScanCompleted: Setting TreeView expected count to {}", actual_total_count);
                        self.tree_view.update_image_count(&path, actual_total_count);
                        self.state.expected_image_count = Some(actual_total_count);
                    }
                }
            }
            
            // Stop processing if we hit the frame limit, but continue in next frame
            if message_count >= max_messages_per_frame {
                break;
            }
        }
        
        // Always request repaint after processing messages for immediate updates
        if message_count > 0 {
            println!("Processed {} async messages, requesting UI repaint", message_count);
            ctx.request_repaint();
            
            // During loading, request frequent repaints for smooth updates
            if matches!(self.state.loading_state, LoadingState::Loading) {
                ctx.request_repaint();
            }
        }
    }
    
    // Helper method to add images to grid with logging
    fn add_images_to_grid(&mut self, images: &[crate::core::file_scanner::ImageFile], ctx: &egui::Context) {
        if images.is_empty() {
            return;
        }
        println!("Adding {} images in small batch for immediate display", images.len());
        for image_file in images {
            println!("Adding image progressively: {}", image_file.name);
            self.thumbnail_grid.add_image(image_file.clone(), ctx);
        }
    }
}

impl eframe::App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Force immediate and continuous repaints for maximum update frequency
        ctx.request_repaint();
        
        // Enable aggressive continuous mode during loading for real-time updates
        if matches!(self.state.loading_state, LoadingState::Loading) {
            // Force continuous mode with minimal delay
            ctx.request_repaint_after(std::time::Duration::from_millis(16)); // ~60 FPS
        }
        
        // Handle focus navigation FIRST to ensure it always works
        self.handle_focus_navigation(ctx);
        
        // Always request repaint for responsive UI during async operations
        if matches!(self.state.loading_state, LoadingState::Loading) {
            ctx.request_repaint();
        }
        
        // Handle async loading messages with high frequency during loading
        self.handle_async_messages(ctx);
        
        // During loading, process messages more frequently for smooth updates
        if matches!(self.state.loading_state, LoadingState::Loading) {
            self.handle_async_messages(ctx);
            ctx.request_repaint();
        }
        
        // Handle Tab key focus navigation
        self.handle_focus_navigation(ctx);
        
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.show_toolbar(ui);
        });

        egui::SidePanel::left("tree_panel")
            .width_range(200.0..=400.0)
            .default_width(self.state.left_panel_width)
            .show(ctx, |ui| {
                // Ensure TreeView is always interactive regardless of loading state
                ui.set_enabled(true);
                let is_focused = self.state.focus_state == FocusState::TreeView;
                let (selected_folder, was_clicked) = self.tree_view.show(ui, is_focused);
                
                // Change focus to tree view when clicked
                if was_clicked {
                    self.state.focus_state = FocusState::TreeView;
                }
                
                if let Some(folder) = selected_folder {
                    println!("Tree selected folder: {}", folder.display());
                    self.state.current_folder = Some(folder.clone());
                    // Reset image count check state for new folder
                    self.state.expected_image_count = None;
                    self.state.last_check_time = std::time::Instant::now();
                    self.load_folder_async(folder);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // LoadingState::Loading中でもcurrent_imagesを必ず描画（early return禁止）
            ui.set_enabled(true);
            let is_focused = self.state.focus_state == FocusState::ThumbnailGrid;
            let (selected_index, was_clicked, should_open_viewer) = self.thumbnail_grid.show(
                ui, 
                self.state.viewer_size, 
                is_focused, 
                self.state.show_viewer, 
                &self.state.loading_state
            );
            if was_clicked {
                self.state.focus_state = FocusState::ThumbnailGrid;
            }
            if let Some(index) = selected_index {
                self.state.selected_image = Some(index);
            }
            if should_open_viewer {
                self.state.show_viewer = true;
                if let Some(selected_idx) = self.state.selected_image {
                    if let Some(image_path) = self.thumbnail_grid.get_image_path_at_index(selected_idx) {
                        self.request_priority_load(image_path);
                    }
                }
            }
        });

        // Image viewer modal
        if self.state.show_viewer {
            let images = self.thumbnail_grid.get_current_images();
            
            // Handle navigation keys even when viewer is open
            if ctx.input(|i| {
                i.key_pressed(egui::Key::ArrowLeft) || 
                i.key_pressed(egui::Key::ArrowUp)
            }) {
                if let Some(current) = self.state.selected_image {
                    if current > 0 {
                        let new_index = current - 1;
                        self.state.selected_image = Some(new_index);
                        self.thumbnail_grid.set_selected_index(new_index);
                        
                        // Request priority load for the new image
                        if let Some(image_path) = self.thumbnail_grid.get_image_path_at_index(new_index) {
                            self.request_priority_load(image_path);
                        }
                    }
                }
            }
            
            if ctx.input(|i| {
                i.key_pressed(egui::Key::ArrowRight) || 
                i.key_pressed(egui::Key::ArrowDown)
            }) {
                if let Some(current) = self.state.selected_image {
                    let total_images = self.thumbnail_grid.get_image_count();
                    if current + 1 < total_images {
                        let new_index = current + 1;
                        self.state.selected_image = Some(new_index);
                        self.thumbnail_grid.set_selected_index(new_index);
                        
                        // Request priority load for the new image
                        if let Some(image_path) = self.thumbnail_grid.get_image_path_at_index(new_index) {
                            self.request_priority_load(image_path);
                        }
                    }
                }
            }
            
            // Close viewer with Escape or Space
            if ctx.input(|i| {
                i.key_pressed(egui::Key::Escape) || 
                i.key_pressed(egui::Key::Space)
            }) {
                self.state.show_viewer = false;
            }
            
            if let Some(action) = self.image_viewer.show(ctx, &mut self.state, &images) {
                match action {
                    ImageViewerAction::Close => self.state.show_viewer = false,
                    ImageViewerAction::Previous => {
                        if let Some(current) = self.state.selected_image {
                            if current > 0 {
                                let new_index = current - 1;
                                self.state.selected_image = Some(new_index);
                                self.thumbnail_grid.set_selected_index(new_index);
                                
                                // Request priority load for the new image
                                if let Some(image_path) = self.thumbnail_grid.get_image_path_at_index(new_index) {
                                    self.request_priority_load(image_path);
                                }
                            }
                        }
                    }
                    ImageViewerAction::Next => {
                        if let Some(current) = self.state.selected_image {
                            let total_images = self.thumbnail_grid.get_image_count();
                            if current + 1 < total_images {
                                let new_index = current + 1;
                                self.state.selected_image = Some(new_index);
                                self.thumbnail_grid.set_selected_index(new_index);
                                
                                // Request priority load for the new image
                                if let Some(image_path) = self.thumbnail_grid.get_image_path_at_index(new_index) {
                                    self.request_priority_load(image_path);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Increment frame counter
        self.frame_counter += 1;
        
        // Always request repaint to ensure maximum UI responsiveness and input acceptance
        ctx.request_repaint();
        
        // Check for missed images periodically
        self.check_image_count_consistency(ctx);
    }
}

impl ImageViewerApp {
    fn check_image_count_consistency(&mut self, ctx: &egui::Context) {
        // Check every 0.5 seconds for faster detection during loading
        let check_interval = if matches!(self.state.loading_state, LoadingState::Loading) {
            std::time::Duration::from_millis(500) // Faster during loading
        } else {
            std::time::Duration::from_secs(2) // Normal interval when not loading
        };
        
        if self.state.last_check_time.elapsed() < check_interval {
            return;
        }
        
        // Only check when loading is complete
        if !matches!(self.state.loading_state, LoadingState::Loaded) {
            return;
        }
        
        if let (Some(current_folder), Some(expected_count)) = (&self.state.current_folder, self.state.expected_image_count) {
            let tree_count = self.tree_view.get_image_count(current_folder).unwrap_or(0);
            let actual_count = self.thumbnail_grid.get_image_count();
            let (debug_count, sample_names) = self.thumbnail_grid.get_image_debug_info();
            
            println!("=== Image Count Check ===");
            println!("Folder: {}", current_folder.display());
            println!("Expected (Scanned): {}", expected_count);
            println!("TreeView current: {}", tree_count);
            println!("Actual (ThumbnailGrid): {} (debug: {})", actual_count, debug_count);
            if !sample_names.is_empty() {
                println!("Sample images: {:?}", &sample_names[..sample_names.len().min(10)]);
            }
            
            // More aggressive mismatch detection - any count discrepancy triggers reload
            if actual_count != expected_count {
                println!("*** INCONSISTENCY DETECTED! ***");
                println!("Expected: {}, Actual: {}, TreeView: {}", expected_count, actual_count, tree_count);
                println!("Clearing images and reloading folder...");
                
                // Clear current images to prevent duplicates
                self.thumbnail_grid.clear_images();
                // Reset state
                self.state.loading_state = LoadingState::Loading;
                self.state.expected_image_count = None;
                
                // Force immediate reload
                self.load_folder_async(current_folder.clone());
                ctx.request_repaint();
            }
        }
        
        self.state.last_check_time = std::time::Instant::now();
    }
}