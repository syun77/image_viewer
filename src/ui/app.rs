use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::core::{file_scanner::FileScanner, thumbnail_cache::ThumbnailCache};
use super::{tree_view::TreeView, thumbnail_grid::ThumbnailGrid, image_viewer::ImageViewer};

#[derive(Debug)]
pub enum ImageViewerAction {
    Close,
    Previous,
    Next,
}

#[derive(Default, PartialEq)]
pub enum FocusState {
    #[default]
    TreeView,
    ThumbnailGrid,
}

#[derive(Default)]
pub struct AppState {
    pub current_folder: Option<PathBuf>,
    pub selected_image: Option<usize>,
    pub show_viewer: bool,
    pub left_panel_width: f32,
    pub viewer_size: f32,
    pub focus_state: FocusState,
}

pub struct ImageViewerApp {
    state: AppState,
    file_scanner: Arc<Mutex<FileScanner>>,
    thumbnail_cache: Arc<Mutex<ThumbnailCache>>,
    tree_view: TreeView,
    thumbnail_grid: ThumbnailGrid,
    image_viewer: ImageViewer,
    root_path: String,
}

impl ImageViewerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let file_scanner = Arc::new(Mutex::new(FileScanner::new()));
        let thumbnail_cache = Arc::new(Mutex::new(ThumbnailCache::new()));
        
        Self {
            state: AppState {
                left_panel_width: 250.0,
                viewer_size: 160.0,
                ..Default::default()
            },
            file_scanner: file_scanner.clone(),
            thumbnail_cache: thumbnail_cache.clone(),
            tree_view: TreeView::new(file_scanner.clone()),
            thumbnail_grid: ThumbnailGrid::new(file_scanner.clone(), thumbnail_cache.clone()),
            image_viewer: ImageViewer::new(),
            root_path: String::new(),
        }
    }

    fn handle_focus_navigation(&mut self, ctx: &egui::Context) {
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
            ui.text_edit_singleline(&mut self.root_path);
            
            if ui.button("参照").clicked() {  // Japanese "Browse" button
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.root_path = path.to_string_lossy().to_string();
                    self.set_root_path(path);
                }
            }
            
            if ui.button("設定").clicked() {  // Japanese "Set" button
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
        self.state.current_folder = Some(path.clone());
        self.tree_view.set_root(path.clone());
        self.thumbnail_grid.load_folder(path);
    }
}

impl eframe::App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle Tab key focus navigation
        self.handle_focus_navigation(ctx);
        
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.show_toolbar(ui);
        });

        egui::SidePanel::left("tree_panel")
            .width_range(200.0..=400.0)
            .default_width(self.state.left_panel_width)
            .show(ctx, |ui| {
                let is_focused = self.state.focus_state == FocusState::TreeView;
                let (selected_folder, was_clicked) = self.tree_view.show(ui, is_focused);
                
                // Change focus to tree view when clicked
                if was_clicked {
                    self.state.focus_state = FocusState::TreeView;
                }
                
                if let Some(folder) = selected_folder {
                    self.thumbnail_grid.load_folder(folder);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let is_focused = self.state.focus_state == FocusState::ThumbnailGrid;
            let (selected_index, was_clicked, should_open_viewer) = self.thumbnail_grid.show(ui, self.state.viewer_size, is_focused, self.state.show_viewer);
            
            // Change focus to thumbnail grid when clicked
            if was_clicked {
                self.state.focus_state = FocusState::ThumbnailGrid;
            }
            
            if let Some(index) = selected_index {
                self.state.selected_image = Some(index);
            }
            
            // Open viewer on Space/Enter key, double-click, or explicit viewer open request
            if should_open_viewer {
                self.state.show_viewer = true;
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
                        self.state.selected_image = Some(current - 1);
                        self.thumbnail_grid.set_selected_index(current - 1);
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
                        self.state.selected_image = Some(current + 1);
                        self.thumbnail_grid.set_selected_index(current + 1);
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
                                self.state.selected_image = Some(current - 1);
                                self.thumbnail_grid.set_selected_index(current - 1);
                            }
                        }
                    }
                    ImageViewerAction::Next => {
                        if let Some(current) = self.state.selected_image {
                            let total_images = self.thumbnail_grid.get_image_count();
                            if current + 1 < total_images {
                                self.state.selected_image = Some(current + 1);
                                self.thumbnail_grid.set_selected_index(current + 1);
                            }
                        }
                    }
                }
            }
        }
    }
}