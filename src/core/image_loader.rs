use image::{DynamicImage, GenericImageView};
use std::path::Path;
use anyhow::{Result, anyhow};

pub struct ImageLoader;

impl ImageLoader {
    pub fn load_image(path: &Path) -> Result<DynamicImage> {
        let img = image::open(path)
            .map_err(|e| anyhow!("Failed to load image {}: {}", path.display(), e))?;
        Ok(img)
    }

    pub fn generate_thumbnail(img: &DynamicImage, target_size: u32) -> Result<DynamicImage> {
        let (orig_width, orig_height) = img.dimensions();
        
        // Calculate new dimensions while maintaining aspect ratio
        let (new_width, new_height) = if orig_width > orig_height {
            let ratio = target_size as f32 / orig_width as f32;
            (target_size, (orig_height as f32 * ratio) as u32)
        } else {
            let ratio = target_size as f32 / orig_height as f32;
            ((orig_width as f32 * ratio) as u32, target_size)
        };

        // Use simple resize method
        let resized = img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
        Ok(resized)
    }

    pub fn load_thumbnail(path: &Path, target_size: u32) -> Result<DynamicImage> {
        let img = Self::load_image(path)?;
        Self::generate_thumbnail(&img, target_size)
    }
}