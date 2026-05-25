use crate::app::AppState;
use eframe::egui::{Button, ComboBox, Context, Id, MenuBar, TopBottomPanel, ViewportCommand};

pub fn show(ctx: &Context, app: &mut AppState) {
    TopBottomPanel::top(Id::new("top_menubar")).show(ctx, |ui| {
        MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                let not_busy = !app.currently_busy;

                if ui.add_enabled(not_busy, Button::new("Open File")).clicked() {
                    ui.close_menu();
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Image Files", &["png", "jpg", "jpeg", "tiff", "tif"])
                        .pick_file()
                    {
                        app.open_file(path);
                    }
                }

                if ui
                    .add_enabled(not_busy, Button::new("Open Folder"))
                    .clicked()
                {
                    ui.close_menu();
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        app.open_folder(folder);
                    }
                }

                ui.separator();

                let can_export = not_busy && app.loaded_count > 0;
                if ui
                    .add_enabled(can_export, Button::new("Save Images…"))
                    .clicked()
                {
                    ui.close_menu();
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        app.start_export(folder);
                    }
                }

                ComboBox::from_label("Format")
                    .selected_text(app.export_format.clone())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.export_format, "PNG".to_string(), "PNG");
                        ui.selectable_value(&mut app.export_format, "JPEG".to_string(), "JPEG");
                        ui.selectable_value(&mut app.export_format, "TIFF".to_string(), "TIFF");
                    });

                ui.separator();

                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            });
        });
    });
}
