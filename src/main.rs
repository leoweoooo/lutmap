// hides the console window in Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod gui;
mod image_io;
mod processing;

use eframe;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder {
            // min_inner_size: Some(eframe::egui::Vec2::new(600.0, 900.0)),
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "lut-mapper",
        native_options,
        Box::new(|_cc| Ok(Box::new(app::AppState::default()))),
    )
}
