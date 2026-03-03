use eframe::egui;
use std::path::PathBuf;

use crate::core::{
    file_scanner::ImageFile,
    image_loader::ImageLoader,
    thumbnail_cache::ThumbnailCache,
};

use super::ThumbnailGrid;

impl ThumbnailGrid {
    pub(super) fn process_thumbnail_results(&mut self, ctx: &egui::Context) {
        let mut updated = 0;

        while let Ok((path, thumbnail)) = self.thumbnail_result_receiver.try_recv() {
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [thumbnail.width() as usize, thumbnail.height() as usize],
                &thumbnail.to_rgba8(),
            );

            let texture_id = format!("thumbnail_{}", path.display());
            let texture = ctx.load_texture(texture_id, color_image, egui::TextureOptions::LINEAR);

            self.thumbnails.insert(path.clone(), texture);
            self.loading_thumbnails.remove(&path);
            self.loading_started_at.remove(&path);
            updated += 1;

            if updated >= 64 {
                break;
            }
        }

        if updated > 0 {
            ctx.request_repaint();
        }
    }

    pub(super) fn load_thumbnail_async(&mut self, path: &PathBuf, ctx: &egui::Context) {
        if self.loading_thumbnails.contains(path) {
            return;
        }

        self.loading_thumbnails.insert(path.clone());
        self.loading_started_at
            .insert(path.clone(), std::time::Instant::now());

        let path_clone = path.clone();
        let cache_arc = self.thumbnail_cache.clone();
        let sender = self.thumbnail_result_sender.clone();
        let repaint_ctx = ctx.clone();

        std::thread::spawn(move || {
            let metadata = std::fs::metadata(&path_clone).ok();
            let cache_key = metadata.as_ref().map(|m| {
                let modified = m.modified().unwrap_or(std::time::UNIX_EPOCH);
                ThumbnailCache::generate_key(&path_clone, modified, m.len())
            });

            let mut cached_thumbnail: Option<image::DynamicImage> = None;
            if let Some(key) = &cache_key {
                if let Ok(mut cache) = cache_arc.lock() {
                    cached_thumbnail = cache.get(key).cloned();
                }
            }

            let thumbnail = if let Some(img) = cached_thumbnail {
                img
            } else {
                let generated = ImageLoader::load_image(&path_clone)
                    .and_then(|img| ImageLoader::generate_thumbnail(&img, 160))
                    .unwrap_or_else(|_| image::DynamicImage::new_rgba8(160, 160));

                if let Some(key) = cache_key {
                    if let Ok(mut cache) = cache_arc.lock() {
                        cache.put(key, generated.clone());
                    }
                }

                generated
            };

            let _ = sender.send((path_clone, thumbnail));
            repaint_ctx.request_repaint();
        });
    }

    fn load_thumbnail_sync(&mut self, path: PathBuf, ctx: &egui::Context) {
        let image_file = self
            .current_images
            .iter()
            .find(|img| img.path == path)
            .cloned();

        if let Some(image_file) = image_file {
            self.load_thumbnail(ctx, image_file);
        }
    }

    fn load_thumbnail(&mut self, ctx: &egui::Context, image_file: ImageFile) {
        self.loading_thumbnails.insert(image_file.path.clone());

        let cache_key = crate::core::thumbnail_cache::ThumbnailCache::generate_key(
            &image_file.path,
            image_file.modified,
            image_file.size,
        );

        if let Ok(mut cache) = self.thumbnail_cache.lock() {
            if let Some(thumbnail) = cache.get(&cache_key) {
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [thumbnail.width() as usize, thumbnail.height() as usize],
                    &thumbnail.to_rgba8(),
                );

                let texture_id = format!(
                    "thumbnail_{}_{}",
                    image_file.path.file_name().unwrap_or_default().to_string_lossy(),
                    image_file
                        .modified
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                );

                let texture = ctx.load_texture(texture_id, color_image, egui::TextureOptions::LINEAR);
                self.thumbnails.insert(image_file.path.clone(), texture);
                self.loading_thumbnails.remove(&image_file.path);
                return;
            }
        }

        if let Ok(mut cache) = self.thumbnail_cache.lock() {
            match ImageLoader::load_thumbnail(&image_file.path, 160, &mut *cache) {
                Ok(thumbnail) => {
                    cache.put(cache_key, thumbnail.clone());

                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        [thumbnail.width() as usize, thumbnail.height() as usize],
                        &thumbnail.to_rgba8(),
                    );

                    let texture_id = format!(
                        "thumbnail_{}_{}",
                        image_file.path.file_name().unwrap_or_default().to_string_lossy(),
                        image_file
                            .modified
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    );

                    let texture = ctx.load_texture(texture_id, color_image, egui::TextureOptions::LINEAR);
                    self.thumbnails.insert(image_file.path.clone(), texture);
                    self.loading_thumbnails.remove(&image_file.path);
                }
                Err(err) => {
                    eprintln!("Failed to load thumbnail: {}", err);
                    self.loading_thumbnails.remove(&image_file.path);
                }
            }
        } else {
            self.loading_thumbnails.remove(&image_file.path);
        }
    }
}
