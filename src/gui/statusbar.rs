use crate::app::AppState;
use eframe::egui::{Align, Button, ComboBox, Layout, Panel, Ui};

pub fn show(ui: &mut Ui, app: &mut AppState) {
    Panel::bottom("bottom_statusbar")
        .resizable(false)
        .show_inside(ui, |ui| {
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
                    ui.separator();

                    let can_save = !app.currently_busy && app.loaded_count > 0;
                    if ui
                        .add_enabled(can_save, Button::new("Save Images…"))
                        .clicked()
                    {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            app.start_export(folder);
                        }
                    }

                    ComboBox::from_id_salt("export_format")
                        .selected_text(app.export_format.clone())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.export_format, "PNG".to_string(), "PNG");
                            ui.selectable_value(&mut app.export_format, "JPEG".to_string(), "JPEG");
                            ui.selectable_value(&mut app.export_format, "TIFF".to_string(), "TIFF");
                        });
                });
            });
        });
}
