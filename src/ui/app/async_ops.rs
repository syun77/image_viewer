use eframe::egui;
use std::path::PathBuf;

use super::{AsyncLoadMessage, ImageViewerApp, LoadingState};

impl ImageViewerApp {
    pub(super) fn load_folder_async(&mut self, path: PathBuf) {
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
            })
            .await
            .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking failed: {}", e)));

            match image_paths {
                Ok(paths) => {
                    let total_count = paths.len();
                    for (idx, img_path) in paths.iter().enumerate() {
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
                        })
                        .await
                        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking failed: {}", e)));

                        match image_file {
                            Ok(img) => {
                                let _ = sender.send(AsyncLoadMessage::ImageFound(path_clone.clone(), img));
                                let _ = sender.send(AsyncLoadMessage::ScanProgress(
                                    path_clone.clone(),
                                    idx + 1,
                                    total_count,
                                ));
                            }
                            Err(e) => {
                                println!("Failed to load image file: {}", e);
                            }
                        }

                        tokio::time::sleep(std::time::Duration::from_millis(0)).await;
                    }

                    let _ = sender.send(AsyncLoadMessage::FolderLoadCompleted(path_clone.clone(), vec![]));
                    let _ = sender.send(AsyncLoadMessage::ScanCompleted(path_clone.clone(), total_count));
                }
                Err(e) => {
                    println!("Scan failed for {}: {}", path_clone.display(), e);
                    let _ = sender.send(AsyncLoadMessage::FolderLoadFailed(
                        path_clone.clone(),
                        e.to_string(),
                    ));
                    let _ = sender.send(AsyncLoadMessage::ScanCompleted(path_clone.clone(), 0));
                }
            }
        });
    }

    pub(super) fn request_priority_load(&mut self, path: PathBuf) {
        self.state.priority_load_path = Some(path.clone());
        let sender = self.load_sender.clone();
        let _ = sender.send(AsyncLoadMessage::PriorityImageLoad(path));
    }

    pub(super) fn handle_async_messages(&mut self, ctx: &egui::Context) {
        let mut message_count = 0;
        let max_messages_per_frame = 200;

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
                        self.add_images_to_grid(&[image_file], ctx);
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
                        println!(
                            "ScanCompleted: Setting TreeView expected count to {}",
                            actual_total_count
                        );
                        self.tree_view.update_image_count(&path, actual_total_count);
                        self.state.expected_image_count = Some(actual_total_count);
                    }
                }
            }

            if message_count >= max_messages_per_frame {
                break;
            }
        }

        if message_count > 0 {
            println!("Processed {} async messages, requesting UI repaint", message_count);
            ctx.request_repaint();

            if matches!(self.state.loading_state, LoadingState::Loading) {
                ctx.request_repaint();
            }
        }
    }

    fn add_images_to_grid(
        &mut self,
        images: &[crate::core::file_scanner::ImageFile],
        ctx: &egui::Context,
    ) {
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
