use std::path::PathBuf;

#[derive(Debug)]
pub enum ImageViewerAction {
    Close,
    Previous,
    Next,
}

#[derive(Debug)]
pub enum AsyncLoadMessage {
    FolderLoadStarted(PathBuf),
    FolderLoadCompleted(PathBuf, Vec<crate::core::file_scanner::ImageFile>),
    FolderLoadFailed(PathBuf, String),
    PriorityImageLoad(PathBuf),
    ImageFound(PathBuf, crate::core::file_scanner::ImageFile),
    ScanProgress(PathBuf, usize, usize),
    ScanCompleted(PathBuf, usize),
}

#[derive(Default, PartialEq)]
pub enum FocusState {
    #[default]
    TreeView,
    ThumbnailGrid,
}

#[derive(Default, PartialEq)]
pub enum LoadingState {
    #[default]
    Idle,
    Loading,
    Loaded,
    Failed(String),
}

pub struct AppState {
    pub current_folder: Option<PathBuf>,
    pub selected_image: Option<usize>,
    pub show_viewer: bool,
    pub left_panel_width: f32,
    pub viewer_size: f32,
    pub focus_state: FocusState,
    pub loading_state: LoadingState,
    pub priority_load_path: Option<PathBuf>,
    pub last_check_time: std::time::Instant,
    pub expected_image_count: Option<usize>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_folder: None,
            selected_image: None,
            show_viewer: false,
            left_panel_width: 250.0,
            viewer_size: 160.0,
            focus_state: FocusState::TreeView,
            loading_state: LoadingState::Idle,
            priority_load_path: None,
            last_check_time: std::time::Instant::now(),
            expected_image_count: None,
        }
    }
}
