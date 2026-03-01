use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::core::{file_scanner::{FileScanner, DirectoryInfo}};

pub struct TreeView {
    file_scanner: Arc<Mutex<FileScanner>>,
    root_info: Option<DirectoryInfo>,
    expanded_dirs: HashMap<PathBuf, bool>,
    selected_path: Option<PathBuf>,
}

impl TreeView {
    pub fn new(file_scanner: Arc<Mutex<FileScanner>>) -> Self {
        Self {
            file_scanner,
            root_info: None,
            expanded_dirs: HashMap::new(),
            selected_path: None,
        }
    }

    pub fn set_root(&mut self, path: PathBuf) {
        if let Ok(scanner) = self.file_scanner.lock() {
            if let Ok(root_info) = scanner.scan_directory(&path) {
                self.root_info = Some(root_info);
                self.expanded_dirs.clear();
                self.expanded_dirs.insert(path.clone(), true);
                self.selected_path = Some(path);
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let mut selected_folder = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(root_info) = self.root_info.clone() {
                if let Some(path) = self.show_directory_node(ui, &root_info, 0) {
                    selected_folder = Some(path);
                }
            } else {
                ui.label("No root directory selected");
            }
        });

        selected_folder
    }

    fn show_directory_node(
        &mut self, 
        ui: &mut egui::Ui, 
        dir_info: &DirectoryInfo, 
        depth: usize
    ) -> Option<PathBuf> {
        let mut selected_folder = None;
        let indent = depth as f32 * 20.0;
        let is_expanded = self.expanded_dirs.get(&dir_info.path).copied().unwrap_or(false);
        let has_children = !dir_info.children.is_empty();
        
        ui.horizontal(|ui| {
            ui.add_space(indent);
            
            if has_children {
                let expand_icon = if is_expanded { "▼" } else { "▶" };
                if ui.small_button(expand_icon).clicked() {
                    self.expanded_dirs.insert(dir_info.path.clone(), !is_expanded);
                }
            } else {
                ui.add_space(20.0); // Space for alignment
            }
            
            let is_selected = self.selected_path.as_ref() == Some(&dir_info.path);
            let response = ui.selectable_label(is_selected, &dir_info.name);
            
            if response.clicked() {
                self.selected_path = Some(dir_info.path.clone());
                selected_folder = Some(dir_info.path.clone());
            }
            
            // Show image count
            if !dir_info.image_files.is_empty() {
                ui.label(format!("({})", dir_info.image_files.len()));
            }
        });
        
        if is_expanded {
            for child in &dir_info.children {
                if let Some(path) = self.show_directory_node(ui, child, depth + 1) {
                    selected_folder = Some(path);
                }
            }
        }
        
        selected_folder
    }

    pub fn get_selected_path(&self) -> Option<&PathBuf> {
        self.selected_path.as_ref()
    }
}