mod controls;
mod menubar;
mod preview;
mod statusbar;

use crate::app::AppState;
use eframe::egui::{CentralPanel, Ui};

pub fn draw(ui: &mut Ui, app: &mut AppState) {
    statusbar::show(ui, app);
    menubar::show(ui, app);
    CentralPanel::default().show_inside(ui, |ui| {
        controls::show(ui, app);
        preview::show(ui, app);
    });
}
