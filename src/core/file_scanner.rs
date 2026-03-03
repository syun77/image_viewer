use std::path::{Path, PathBuf};
use std::fs;
use std::time::SystemTime;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct ImageFile {
    pub path: PathBuf,
    pub name: String,
    pub modified: SystemTime,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct DirectoryInfo {
    pub path: PathBuf,
    pub name: String,
    pub children: Vec<DirectoryInfo>,
    pub image_files: Vec<ImageFile>,
}

pub struct FileScanner {
    current_root: Option<PathBuf>,
    supported_extensions: Vec<String>,
}

impl FileScanner {
    pub fn new() -> Self {
        Self {
            current_root: None,
            supported_extensions: vec![
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
                "bmp".to_string(),
                "gif".to_string(),
                "webp".to_string(),
                "tiff".to_string(),
            ],
        }
    }

    pub fn set_root(&mut self, path: PathBuf) {
        self.current_root = Some(path);
    }

    pub fn scan_directory(&self, path: &Path) -> Result<DirectoryInfo> {
        let mut dir_info = DirectoryInfo {
            path: path.to_path_buf(),
            name: path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            children: Vec::new(),
            image_files: Vec::new(),
        };

        if !path.is_dir() {
            return Ok(dir_info);
        }

        let entries = fs::read_dir(path)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Ok(child_info) = self.scan_directory(&path) {
                    dir_info.children.push(child_info);
                }
            } else if self.is_supported_image(&path) {
                // Skip hidden files (starting with ._)
                if let Some(filename) = path.file_name() {
                    if filename.to_string_lossy().starts_with("._") {
                        continue;
                    }
                }
                
                if let Ok(metadata) = entry.metadata() {
                    let image_file = ImageFile {
                        path: path.clone(),
                        name: path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Unknown".to_string()),
                        modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                        size: metadata.len(),
                    };
                    dir_info.image_files.push(image_file);
                }
            }
        }

        // Sort by name using natural ordering for better Japanese support
        dir_info.children.sort_by(|a, b| natord::compare(&a.name, &b.name));
        dir_info.image_files.sort_by(|a, b| natord::compare(&a.name, &b.name));

        Ok(dir_info)
    }

    pub fn scan_images_in_directory(&self, path: &Path) -> Result<Vec<ImageFile>> {
        let dir_info = self.scan_directory(path)?;
        Ok(dir_info.image_files)
    }
    
    /// 逐次ロード用: ディレクトリ内の画像パス一覧を返す（イテレータ的）
    pub fn get_image_paths_in_directory(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut image_paths = Vec::new();
        if !path.is_dir() {
            return Ok(image_paths);
        }
        let entries = fs::read_dir(path)?;
        for entry in entries {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_file() && self.is_supported_image(&entry_path) {
                if let Some(filename) = entry_path.file_name() {
                    if filename.to_string_lossy().starts_with("._") {
                        continue;
                    }
                }
                image_paths.push(entry_path);
            }
        }
        // ソート（日本語対応）
        image_paths.sort_by(|a, b| natord::compare(&a.to_string_lossy(), &b.to_string_lossy()));
        Ok(image_paths)
    }

    /// 逐次ロード用: 画像パスからImageFileを生成
    pub fn load_image_file(&self, path: &Path) -> Result<ImageFile> {
        let metadata = std::fs::metadata(path)?;
        Ok(ImageFile {
            path: path.to_path_buf(),
            name: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Unknown".to_string()),
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            size: metadata.len(),
        })
    }

    fn is_supported_image(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            self.supported_extensions.contains(&ext)
        } else {
            false
        }
    }

    pub fn get_root(&self) -> Option<&PathBuf> {
        self.current_root.as_ref()
    }
}