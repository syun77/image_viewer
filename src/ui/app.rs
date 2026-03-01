use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::core::{file_scanner::FileScanner, thumbnail_cache::ThumbnailCache};
use super::{tree_view::TreeView, thumbnail_grid::ThumbnailGrid, image_viewer::ImageViewer};

#[derive(Default)]
pub struct AppState {
    pub current_folder: Option<PathBuf>,
    pub selected_image: Option<usize>,
    pub show_viewer: bool,
    pub left_panel_width: f32,
    pub viewer_size: f32,
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

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Root Path:");
            ui.text_edit_singleline(&mut self.root_path);
            
            if ui.button("Browse").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.root_path = path.to_string_lossy().to_string();
                    self.set_root_path(path);
                }
            }
            
            if ui.button("Set").clicked() {
                let path = PathBuf::from(&self.root_path);
                if path.exists() {
                    self.set_root_path(path);
                }
            }
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
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.show_toolbar(ui);
        });

        egui::SidePanel::left("tree_panel")
            .width_range(200.0..=400.0)
            .default_width(self.state.left_panel_width)
            .show(ctx, |ui| {
                if let Some(selected_folder) = self.tree_view.show(ui) {
                    self.thumbnail_grid.load_folder(selected_folder);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(selected_index) = self.thumbnail_grid.show(ui, self.state.viewer_size) {
                self.state.selected_image = Some(selected_index);
                
                // Handle keyboard input for Space key
                if ui.input(|i| i.key_pressed(egui::Key::Space)) {
                    self.state.show_viewer = !self.state.show_viewer;
                }
            }
        });

        // Image viewer modal
        if self.state.show_viewer {
            if let Some(action) = self.image_viewer.show(ctx, &mut self.state) {
                match action {
                    ImageViewerAction::Close => self.state.show_viewer = false,
                    ImageViewerAction::Previous => {
                        if let Some(current) = self.state.selected_image {
                            if current > 0 {
                                self.state.selected_image = Some(current - 1);
                            }
                        }
                    }
                    ImageViewerAction::Next => {
                        if let Some(current) = self.state.selected_image {
                            let total_images = self.thumbnail_grid.get_image_count();
                            if current + 1 < total_images {
                                self.state.selected_image = Some(current + 1);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub enum ImageViewerAction {
    Close,
    Previous,
    Next,
}