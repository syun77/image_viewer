use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use image::DynamicImage;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub size: u64,
}

pub struct ThumbnailCache {
    cache: HashMap<CacheKey, DynamicImage>,
    max_entries: usize,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_entries: 1000,
        }
    }

    pub fn get(&mut self, key: &CacheKey) -> Option<&DynamicImage> {
        self.cache.get(key)
    }

    pub fn put(&mut self, key: CacheKey, thumbnail: DynamicImage) {
        if self.cache.len() >= self.max_entries {
            // Simple eviction - remove first item
            if let Some(first_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&first_key);
            }
        }
        self.cache.insert(key, thumbnail);
    }

    pub fn generate_key(path: &PathBuf, modified: SystemTime, size: u64) -> CacheKey {
        CacheKey {
            path: path.clone(),
            modified,
            size,
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }
}