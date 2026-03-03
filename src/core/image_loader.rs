use image::{DynamicImage, GenericImageView};
use std::path::Path;
use anyhow::{Result, anyhow};
use log::info;
use crate::core::thumbnail_cache::ThumbnailCache;

pub struct ImageLoader;

impl ImageLoader {
    pub fn load_image(path: &Path) -> Result<DynamicImage> {
        let start_time = std::time::Instant::now();
        let img = image::open(path)
            .map_err(|e| anyhow!("Failed to load image {}: {}", path.display(), e))?;
        info!("Loaded image {} in {:?}", path.display(), start_time.elapsed());
        Ok(img)
    }

    pub fn generate_thumbnail(img: &DynamicImage, target_size: u32) -> Result<DynamicImage> {
        let start_time = std::time::Instant::now();
        let (orig_width, orig_height) = img.dimensions();
        
        // Calculate new dimensions while maintaining aspect ratio
        let (new_width, new_height) = if orig_width > orig_height {
            let ratio = target_size as f32 / orig_width as f32;
            (target_size, (orig_height as f32 * ratio) as u32)
        } else {
            let ratio = target_size as f32 / orig_height as f32;
            ((orig_width as f32 * ratio) as u32, target_size)
        };

        let new_width = new_width.max(1);
        let new_height = new_height.max(1);

        // Use simple resize method
        let resized = img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
        info!("Generated thumbnail in {:?}", start_time.elapsed());
        Ok(resized)
    }

    pub fn load_thumbnail(path: &Path, target_size: u32, cache: &mut ThumbnailCache) -> Result<DynamicImage> {
        match Self::load_image(path) {
            Ok(img) => {
                match Self::generate_thumbnail(&img, target_size) {
                    Ok(thumbnail) => {
                        if let Ok(metadata) = std::fs::metadata(path) {
                            let modified = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
                            let cache_key = ThumbnailCache::generate_key(
                                &path.to_path_buf(),
                                modified,
                                metadata.len(),
                            );
                            cache.put(cache_key, thumbnail.clone());
                        }
                        Ok(thumbnail)
                    }
                    Err(e) => {
                        log::error!("Failed to generate thumbnail for {}: {}", path.display(), e);
                        // Return a placeholder image on failure
                        Ok(DynamicImage::new_rgba8(target_size, target_size))
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to load image {}: {}", path.display(), e);
                // Return a placeholder image on failure
                Ok(DynamicImage::new_rgba8(target_size, target_size))
            }
        }
    }
}