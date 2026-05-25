use eframe::egui::{Align, Layout, Panel, RichText, Slider, Ui};
use egui_extras::TableBuilder;

use crate::{app::AppState, processing::Channel};

pub(super) fn show(ui: &mut Ui, app: &mut AppState) {
    Panel::bottom("channel_control")
        .resizable(false)
        .min_size(100.0)
        .show_inside(ui, |ui| {
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
                    for ch in Channel::ALL {
                        body.row(row_h, |mut row| {
                            let state = &mut app.channels[ch];
                            row.col(|ui| {
                                ui.label(ch.label());
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
                app.update_preview();
            }
        });
}
