mod controls;
mod menubar;
mod preview;
mod statusbar;

use crate::app::AppState;
use eframe::egui::{CentralPanel, Context, Id, TopBottomPanel};

pub fn draw(ctx: &Context, app: &mut AppState) {
    menubar::show(ctx, app);
    statusbar::show(ctx, app);

    CentralPanel::default().show(ctx, |ui| {
        controls::show_inside(ui, app);
        preview::show(ui, app);
    });
}
