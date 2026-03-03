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
    all_directories: Vec<PathBuf>, // For keyboard navigation
    selected_index: Option<usize>, // For keyboard navigation
    // Add async image counts for each directory
    image_counts: HashMap<PathBuf, usize>,
}

impl TreeView {
    pub fn new(file_scanner: Arc<Mutex<FileScanner>>) -> Self {
        Self {
            file_scanner,
            root_info: None,
            expanded_dirs: HashMap::new(),
            selected_path: None,
            all_directories: Vec::new(),
            selected_index: None,
            image_counts: HashMap::new(),
        }
    }

    pub fn set_root(&mut self, path: PathBuf) {
        let root_info = {
            if let Ok(scanner) = self.file_scanner.lock() {
                scanner.scan_directory(&path).ok()
            } else {
                None
            }
        };
        
        if let Some(root_info) = root_info {
            self.root_info = Some(root_info);
            self.expanded_dirs.clear();
            self.expanded_dirs.insert(path.clone(), true);
            self.selected_path = Some(path);
            self.rebuild_navigation_list();
        }
    }

    // Helper method to rebuild the navigation list for keyboard navigation
    fn rebuild_navigation_list(&mut self) {
        let mut temp_list = Vec::new();
        if let Some(root_info) = self.root_info.clone() {
            self.collect_visible_directories(&root_info, &mut temp_list);
        }
        self.all_directories = temp_list;
        
        // Update selected index based on current selected path
        if let Some(selected_path) = &self.selected_path {
            self.selected_index = self.all_directories.iter().position(|p| p == selected_path);
        }
    }

    // Recursively collect all visible (expanded) directories
    fn collect_visible_directories(&self, dir_info: &DirectoryInfo, list: &mut Vec<PathBuf>) {
        list.push(dir_info.path.clone());
        if *self.expanded_dirs.get(&dir_info.path).unwrap_or(&false) {
            for child in &dir_info.children {
                self.collect_visible_directories(child, list);
            }
        }
    }

    // Handle keyboard navigation
    fn handle_keyboard_navigation(&mut self, ui: &egui::Ui) {
        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            if let Some(current) = self.selected_index {
                if current > 0 {
                    self.selected_index = Some(current - 1);
                    if let Some(path) = self.all_directories.get(current - 1) {
                        self.selected_path = Some(path.clone());
                    }
                }
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            if let Some(current) = self.selected_index {
                if current + 1 < self.all_directories.len() {
                    self.selected_index = Some(current + 1);
                    if let Some(path) = self.all_directories.get(current + 1) {
                        self.selected_path = Some(path.clone());
                    }
                }
            } else if !self.all_directories.is_empty() {
                self.selected_index = Some(0);
                self.selected_path = Some(self.all_directories[0].clone());
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            if let Some(selected_path) = &self.selected_path {
                // Expand current directory
                self.expanded_dirs.insert(selected_path.clone(), true);
                self.rebuild_navigation_list();
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            if let Some(selected_path) = &self.selected_path {
                // Collapse current directory
                self.expanded_dirs.insert(selected_path.clone(), false);
                self.rebuild_navigation_list();
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, is_focused: bool) -> (Option<PathBuf>, bool) {
        let mut selected_folder = None;
        let mut was_clicked = false;

        // Handle keyboard navigation when focused
        if is_focused {
            self.handle_keyboard_navigation(ui);
        }

        // Show focus indicator with darker color
        if is_focused {
            ui.painter().rect_stroke(
                ui.available_rect_before_wrap(),
                egui::Rounding::same(2.0),
                egui::Stroke::new(2.0, egui::Color32::from_rgb(40, 40, 80)), // Darker blue
            );
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(root_info) = self.root_info.clone() {
                let (folder, clicked) = self.show_directory_node(ui, &root_info, 0);
                if let Some(path) = folder {
                    selected_folder = Some(path);
                }
                if clicked {
                    was_clicked = true;
                }
            } else {
                ui.label("No root directory selected");
            }
        });

        (selected_folder, was_clicked)
    }

    fn show_directory_node(
        &mut self, 
        ui: &mut egui::Ui, 
        dir_info: &DirectoryInfo, 
        depth: usize
    ) -> (Option<PathBuf>, bool) {
        let mut selected_folder = None;
        let mut was_clicked = false;
        let indent = depth as f32 * 20.0;
        let is_expanded = self.expanded_dirs.get(&dir_info.path).copied().unwrap_or(false);
        let has_children = !dir_info.children.is_empty();
        
        ui.horizontal(|ui| {
            ui.add_space(indent);
            
            if has_children {
                let expand_icon = if is_expanded { "▼" } else { "▶" };
                if ui.small_button(expand_icon).clicked() {
                    self.expanded_dirs.insert(dir_info.path.clone(), !is_expanded);
                    self.rebuild_navigation_list();
                    was_clicked = true;
                }
            } else {
                ui.add_space(20.0); // Space for alignment
            }
            
            let is_selected = self.selected_path.as_ref() == Some(&dir_info.path);
            let response = ui.selectable_label(is_selected, &dir_info.name);
            
            if response.clicked() {
                self.selected_path = Some(dir_info.path.clone());
                selected_folder = Some(dir_info.path.clone());
                was_clicked = true;
                // Update selected index for keyboard navigation
                self.selected_index = self.all_directories.iter().position(|p| p == &dir_info.path);
            }
            
            // Show image count from async loading instead of sync scan
            if let Some(count) = self.get_image_count(&dir_info.path) {
                if count > 0 {
                    ui.label(format!("({})", count));
                }
            } else if !dir_info.image_files.is_empty() {
                // Fallback to sync count if async count not available yet
                ui.label(format!("({})", dir_info.image_files.len()));
            }
        });
        
        if is_expanded {
            for child in &dir_info.children {
                let (child_folder, child_clicked) = self.show_directory_node(ui, child, depth + 1);
                if let Some(path) = child_folder {
                    selected_folder = Some(path);
                }
                if child_clicked {
                    was_clicked = true;
                }
            }
        }
        
        (selected_folder, was_clicked)
    }

    pub fn get_selected_path(&self) -> Option<&PathBuf> {
        self.selected_path.as_ref()
    }

    // Update image count for a directory
    pub fn update_image_count(&mut self, path: &PathBuf, count: usize) {
        self.image_counts.insert(path.clone(), count);
    }

    // Get image count for a directory
    pub fn get_image_count(&self, path: &PathBuf) -> Option<usize> {
        self.image_counts.get(path).copied()
    }
}