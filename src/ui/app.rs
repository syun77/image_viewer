mod async_ops;
mod state;
mod viewer_ops;

use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use crate::core::{file_scanner::FileScanner, thumbnail_cache::ThumbnailCache};
use super::{image_viewer::ImageViewer, thumbnail_grid::ThumbnailGrid, tree_view::TreeView};

pub use state::{AppState, AsyncLoadMessage, FocusState, ImageViewerAction, LoadingState};

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
        cc.egui_ctx
            .set_pixels_per_point(cc.egui_ctx.pixels_per_point());

        let file_scanner = Arc::new(Mutex::new(FileScanner::new()));
        let thumbnail_cache = Arc::new(Mutex::new(ThumbnailCache::new()));
        let async_runtime = Arc::new(Runtime::new().expect("Failed to create tokio runtime"));

        let (load_sender, load_receiver) = mpsc::sync_channel(200);

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
        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
            self.state.focus_state = match self.state.focus_state {
                FocusState::TreeView => FocusState::ThumbnailGrid,
                FocusState::ThumbnailGrid => FocusState::TreeView,
            };
        }
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("ルートパス:");

            ui.add_enabled(true, egui::TextEdit::singleline(&mut self.root_path));

            if ui.add_enabled(true, egui::Button::new("参照")).clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.root_path = path.to_string_lossy().to_string();
                    self.set_root_path(path);
                }
            }

            if ui.add_enabled(true, egui::Button::new("設定")).clicked() {
                let path = PathBuf::from(&self.root_path);
                if path.exists() {
                    self.set_root_path(path);
                }
            }

            ui.label("テスト: 日本語表示");
        });
    }

    fn set_root_path(&mut self, path: PathBuf) {
        println!("Setting root path: {}", path.display());
        self.state.current_folder = Some(path.clone());
        self.tree_view.set_root(path.clone());
        self.load_folder_async(path);
    }
}

impl eframe::App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        if matches!(self.state.loading_state, LoadingState::Loading) {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        self.handle_focus_navigation(ctx);

        if matches!(self.state.loading_state, LoadingState::Loading) {
            ctx.request_repaint();
        }

        self.handle_async_messages(ctx);

        if matches!(self.state.loading_state, LoadingState::Loading) {
            self.handle_async_messages(ctx);
            ctx.request_repaint();
        }

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
                    println!("Tree selected folder: {}", folder.display());
                    self.state.current_folder = Some(folder.clone());
                    self.state.expected_image_count = None;
                    self.state.last_check_time = std::time::Instant::now();
                    self.load_folder_async(folder);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let is_focused = self.state.focus_state == FocusState::ThumbnailGrid;
            let (selected_index, was_clicked, should_open_viewer) = self.thumbnail_grid.show(
                ui,
                self.state.viewer_size,
                is_focused,
                self.state.show_viewer,
                &self.state.loading_state,
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

        self.handle_viewer_modal(ctx);

        self.frame_counter += 1;

        ctx.request_repaint();

        self.check_image_count_consistency(ctx);
    }
}

impl ImageViewerApp {
    fn check_image_count_consistency(&mut self, ctx: &egui::Context) {
        let check_interval = if matches!(self.state.loading_state, LoadingState::Loading) {
            std::time::Duration::from_millis(500)
        } else {
            std::time::Duration::from_secs(2)
        };

        if self.state.last_check_time.elapsed() < check_interval {
            return;
        }

        if !matches!(self.state.loading_state, LoadingState::Loaded) {
            return;
        }

        if let (Some(current_folder), Some(expected_count)) =
            (&self.state.current_folder, self.state.expected_image_count)
        {
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

            if actual_count != expected_count {
                println!("*** INCONSISTENCY DETECTED! ***");
                println!("Expected: {}, Actual: {}, TreeView: {}", expected_count, actual_count, tree_count);
                println!("Clearing images and reloading folder...");

                self.thumbnail_grid.clear_images();
                self.state.loading_state = LoadingState::Loading;
                self.state.expected_image_count = None;

                self.load_folder_async(current_folder.clone());
                ctx.request_repaint();
            }
        }

        self.state.last_check_time = std::time::Instant::now();
    }
}