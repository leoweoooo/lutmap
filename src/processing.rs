use eframe::egui::{Color32, ColorImage};
use rayon::prelude::*;

#[derive(Clone)]
pub struct ChannelState {
    pub color: Color32,
    pub brightness: f32,
    pub contrast: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    R = 0,
    G = 1,
    B = 2,
}

impl Channel {
    pub const _ALL: [Channel; 3] = [Channel::R, Channel::G, Channel::B];
    pub fn _label(self) -> &'static str {
        match self {
            Channel::R => "R",
            Channel::G => "G",
            Channel::B => "B",
        }
    }
}

#[derive(Clone)]
pub struct ChannelSettings {
    pub channels: [ChannelState; 3],
}

impl Default for ChannelSettings {
    fn default() -> Self {
        Self {
            channels: [
                ChannelState {
                    // default color is red -> magenta
                    color: Color32::from_rgb(255, 0, 255),
                    brightness: 0.0,
                    contrast: 1.0,
                },
                ChannelState {
                    // default color is green -> yellow
                    color: Color32::from_rgb(255, 255, 0),
                    brightness: 0.0,
                    contrast: 1.0,
                },
                ChannelState {
                    // default color is blue -> cyan
                    color: Color32::from_rgb(0, 255, 255),
                    brightness: 0.0,
                    contrast: 1.0,
                },
            ],
        }
    }
}

impl std::ops::Index<Channel> for ChannelSettings {
    type Output = ChannelState;
    fn index(&self, c: Channel) -> &ChannelState {
        &self.channels[c as usize]
    }
}

impl std::ops::IndexMut<Channel> for ChannelSettings {
    fn index_mut(&mut self, c: Channel) -> &mut ChannelState {
        &mut self.channels[c as usize]
    }
}

fn build_lut(brightness: f32, contrast: f32) -> [u8; 256] {
    let mut lut = [0u8; 256];

    for (idx, entry) in lut.iter_mut().enumerate() {
        let x = idx as f32 / 255.0;
        let y = ((x - 0.5) * contrast + 0.5 + brightness).clamp(0.0, 1.0);
        *entry = (y * 255.0).round() as u8;
    }

    lut
}

pub fn process_channels(original: &ColorImage, settings: &ChannelSettings) -> ColorImage {
    let luts: [_; 3] = std::array::from_fn(|i| {
        build_lut(
            settings.channels[i].brightness,
            settings.channels[i].contrast,
        )
    });

    let r_color = settings[Channel::R].color;
    let g_color = settings[Channel::G].color;
    let b_color = settings[Channel::B].color;

    let pixels: Vec<Color32> = original
        .pixels
        .par_iter()
        .map(|&px| {
            let r_mapped = luts[0][px.r() as usize] as u32;
            let g_mapped = luts[1][px.g() as usize] as u32;
            let b_mapped = luts[2][px.b() as usize] as u32;

            let out_r = (r_mapped * r_color.r() as u32
                + g_mapped * g_color.r() as u32
                + b_mapped * b_color.r() as u32)
                .min(255 * 255)
                / 255;
            let out_g = (r_mapped * r_color.g() as u32
                + g_mapped * g_color.g() as u32
                + b_mapped * b_color.g() as u32)
                .min(255 * 255)
                / 255;
            let out_b = (r_mapped * r_color.b() as u32
                + g_mapped * g_color.b() as u32
                + b_mapped * b_color.b() as u32)
                .min(255 * 255)
                / 255;

            Color32::from_rgba_unmultiplied(out_r as u8, out_g as u8, out_b as u8, px.a())
        })
        .collect();

    ColorImage::new(original.size, pixels)
}
