use eframe::egui;

pub struct KeyboardHandler;

impl KeyboardHandler {
    /// Check if Space key is pressed and should trigger image viewer
    pub fn should_open_viewer(ui: &egui::Ui, has_focus: bool) -> bool {
        has_focus && ui.input(|i| i.key_pressed(egui::Key::Space))
    }

    /// Check if Escape key is pressed to close viewer
    pub fn should_close_viewer(ui: &egui::Ui) -> bool {
        ui.input(|i| i.key_pressed(egui::Key::Escape))
    }

    /// Handle navigation keys
    pub fn get_navigation_direction(ui: &egui::Ui) -> Option<NavigationDirection> {
        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            Some(NavigationDirection::Up)
        } else if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            Some(NavigationDirection::Down)
        } else if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            Some(NavigationDirection::Left)
        } else if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            Some(NavigationDirection::Right)
        } else {
            None
        }
    }

    /// Check for zoom keys
    pub fn get_zoom_action(ui: &egui::Ui) -> Option<ZoomAction> {
        if ui.input(|i| i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
            Some(ZoomAction::ZoomIn)
        } else if ui.input(|i| i.key_pressed(egui::Key::Minus)) {
            Some(ZoomAction::ZoomOut)
        } else if ui.input(|i| i.key_pressed(egui::Key::Num0)) {
            Some(ZoomAction::FitToWindow)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomAction {
    ZoomIn,
    ZoomOut,
    FitToWindow,
}