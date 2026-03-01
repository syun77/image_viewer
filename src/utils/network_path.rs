use std::path::{Path, PathBuf};

pub struct NetworkPath;

impl NetworkPath {
    /// Check if a path is a UNC path (\\server\share\path)
    pub fn is_unc_path(path: &str) -> bool {
        path.starts_with("\\\\")
    }

    /// Check if a path is a mapped network drive (like Z:\)
    pub fn is_mapped_drive(_path: &Path) -> bool {
        #[cfg(windows)]
        {
            if let Some(prefix) = _path.components().next() {
                if let std::path::Component::Prefix(prefix_component) = prefix {
                    use std::path::Prefix;
                    matches!(
                        prefix_component.kind(),
                        Prefix::Disk(_) | Prefix::UNC(_, _) | Prefix::DeviceNS(_)
                    )
                } else {
                    false
                }
            } else {
                false
            }
        }
        #[cfg(not(windows))]
        {
            // On non-Windows systems, assume it's a regular path
            false
        }
    }

    /// Normalize a path for cross-platform compatibility
    pub fn normalize_path(path: &str) -> PathBuf {
        #[cfg(windows)]
        {
            if Self::is_unc_path(path) {
                // Convert forward slashes to backslashes for UNC paths
                PathBuf::from(path.replace('/', "\\"))
            } else {
                PathBuf::from(path)
            }
        }
        #[cfg(not(windows))]
        {
            PathBuf::from(path)
        }
    }

    /// Check if a path is accessible (with timeout)
    pub fn is_accessible(path: &Path) -> bool {
        path.exists() && path.is_dir()
    }

    /// Convert a path to a display-friendly string
    pub fn to_display_string(path: &Path) -> String {
        path.to_string_lossy().to_string()
    }
}