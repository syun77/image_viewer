use eframe::egui;
use std::path::PathBuf;

use crate::core::{file_scanner::ImageFile, image_loader::ImageLoader};

use super::ThumbnailGrid;

impl ThumbnailGrid {
    pub(super) fn load_thumbnail_async(&mut self, path: &PathBuf, ctx: &egui::Context) {
        if self.loading_thumbnails.contains(path) {
            return;
        }

        self.load_thumbnail_sync(path.clone(), ctx);
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
