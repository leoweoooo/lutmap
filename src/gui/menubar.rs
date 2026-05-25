use crate::app::AppState;
use eframe::egui::{Button, Panel, Ui, ViewportCommand};

pub fn show(ui: &mut Ui, app: &mut AppState) {
    Panel::top("top_menubar").show_inside(ui, |ui| {
        eframe::egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                let not_busy = !app.currently_busy;

                if ui
                    .add_enabled(not_busy, Button::new("Open File(s)"))
                    .clicked()
                {
                    ui.close();
                    if let Some(paths) = rfd::FileDialog::new()
                        .add_filter("Image Files", &["png", "jpg", "jpeg", "tiff", "tif"])
                        .pick_files()
                    {
                        app.open_files(paths);
                    }
                }

                ui.separator();

                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
            });
        });
    });
}
