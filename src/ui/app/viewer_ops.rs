use eframe::egui;

use super::{ImageViewerAction, ImageViewerApp};

impl ImageViewerApp {
    pub(super) fn handle_viewer_modal(&mut self, ctx: &egui::Context) {
        if !self.state.show_viewer {
            return;
        }

        let images = self.thumbnail_grid.get_current_images();

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowUp)) {
            self.move_viewer_selection(-1);
        }

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowDown)) {
            self.move_viewer_selection(1);
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Space)) {
            self.state.show_viewer = false;
        }

        if let Some(action) = self.image_viewer.show(ctx, &mut self.state, &images) {
            match action {
                ImageViewerAction::Close => self.state.show_viewer = false,
                ImageViewerAction::Previous => self.move_viewer_selection(-1),
                ImageViewerAction::Next => self.move_viewer_selection(1),
            }
        }
    }

    fn move_viewer_selection(&mut self, delta: isize) {
        let total_images = self.thumbnail_grid.get_image_count();
        if total_images == 0 {
            return;
        }

        let current = self.state.selected_image.unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, total_images.saturating_sub(1) as isize) as usize;

        self.state.selected_image = Some(next);
        self.thumbnail_grid.set_selected_index(next);

        if let Some(image_path) = self.thumbnail_grid.get_image_path_at_index(next) {
            self.request_priority_load(image_path);
        }
    }
}
