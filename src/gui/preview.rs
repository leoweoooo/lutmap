use eframe::egui::{Color32, Frame, Image, Key, Margin, RichText, Stroke, Ui};

use crate::app::{AppState, LoadState};

pub(super) fn show(ui: &mut Ui, app: &mut AppState) {
    ui.heading("Preview");
    ui.add_space(5.0);

    Frame::canvas(ui.style())
        .fill(Color32::DARK_GRAY)
        .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
        .inner_margin(Margin::same(0))
        .show(ui, |ui| {
            let available_size = ui.available_size();
            ui.set_min_size(available_size);

            match app.images.get(app.current_image) {
                Some((_, LoadState::Loaded { display, .. })) => {
                    ui.centered_and_justified(|ui| {
                        ui.add(Image::new(display).max_size(available_size));
                    });
                }
                Some((_, LoadState::Failed(msg))) => {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new(format!("Failed to load: {}", msg)).color(Color32::RED),
                        );
                    });
                }
                Some((_, LoadState::NotLoaded)) | Some((_, LoadState::Processing(_))) => {
                    ui.centered_and_justified(|ui| {
                        ui.label(RichText::new("Loading…").italics().color(Color32::GRAY));
                    });
                }
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("No image loaded")
                                .italics()
                                .color(Color32::LIGHT_GRAY),
                        );
                    });
                }
            }
        });
    handle_navigation(ui, app);
}

fn handle_navigation(ui: &Ui, app: &mut AppState) {
    let ctx = ui.ctx();

    if ctx.input(|i| i.key_pressed(Key::ArrowRight)) {
        app.go_next();
    }
    if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
        app.go_prev();
    }

    let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
    if scroll > 0.0 {
        app.go_prev();
    } else if scroll < 0.0 {
        app.go_next();
    }
}
