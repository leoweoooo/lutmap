use crate::app::AppState;
use eframe::egui::{Align, Id, Layout, Panel, Ui};

pub fn show(ui: &mut Ui, app: &AppState) {
    Panel::bottom(Id::new("bottom_statusbar")).show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(5.0);

            if app.export.in_progress() {
                ui.spinner();
                ui.label(format!(
                    "Exporting {}/{}…",
                    app.export.done, app.export.total
                ));
            } else if app.currently_busy {
                ui.spinner();
                ui.label(format!(
                    "Loading {}/{}…",
                    app.loaded_count,
                    app.images.len()
                ));
            } else if app.loaded_count > 0 {
                ui.label(format!(
                    "Image {}/{}",
                    app.current_image + 1,
                    app.images.len()
                ));
            } else {
                ui.label("Ready");
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(5.0);
                ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
            });
        });
    });
}
