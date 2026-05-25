use std::time::Duration;

use eframe::egui::{Align, Layout, RichText, Slider, Ui};
use egui_extras::TableBuilder;

use crate::{
    app::{AppState, LoadState},
    processing::{Channel, ChannelState, process_channels},
};

pub fn show_inside(ui: &mut Ui, app: &mut AppState) {
    eframe::egui::TopBottomPanel::bottom("channel_control")
        .resizable(false)
        .min_height(100.0)
        .show_inside(ui, |ui| {
            ui.separator();
            ui.label(RichText::new("Channels").size(16.0));
            ui.add_space(5.0);

            let mut changed_any = false;
            let mut released_any = false;

            TableBuilder::new(ui)
                .striped(false)
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(egui_extras::Column::auto())
                .column(egui_extras::Column::auto())
                .column(egui_extras::Column::remainder())
                .column(egui_extras::Column::remainder())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label("Channel");
                    });
                    header.col(|ui| {
                        ui.label("Color");
                    });
                    header.col(|ui| {
                        ui.label("Brightness");
                    });
                    header.col(|ui| {
                        ui.label("Contrast");
                    });
                })
                .body(|mut body| {
                    let row_h = 28.0;
                    let channels = [
                        ("R", &mut app.channels[Channel::R] as *mut ChannelState),
                        ("G", &mut app.channels[Channel::G] as *mut ChannelState),
                        ("B", &mut app.channels[Channel::B] as *mut ChannelState),
                    ];
                    for (label, state_ptr) in channels {
                        let state = unsafe { &mut *state_ptr };
                        body.row(row_h, |mut row| {
                            row.col(|ui| {
                                ui.label(label);
                            });
                            row.col(|ui| {
                                changed_any |=
                                    ui.color_edit_button_srgba(&mut state.color).changed();
                            });
                            row.col(|ui| {
                                let resp = ui.add(Slider::new(&mut state.brightness, -1.0..=1.0));
                                changed_any |= resp.changed();
                                released_any |= resp.drag_stopped();
                            });
                            row.col(|ui| {
                                let resp = ui.add(Slider::new(&mut state.contrast, 0.0..=3.0));
                                changed_any |= resp.changed();
                                released_any |= resp.drag_stopped();
                            });
                        });
                    }
                });

            if changed_any {
                if let Some((_, LoadState::Loaded { original, display })) =
                    app.images.get_mut(app.current_image)
                {
                    let preview = process_channels(original, &app.channels);
                    display.set(preview, Default::default());
                }
                ui.ctx().request_repaint_after(Duration::from_millis(16));
            }

            if released_any {
                app.reprocess_all();
            }
        });
}
