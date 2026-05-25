use std::borrow::Cow;

use eframe::{
    egui::{Color32, ColorImage},
    wgpu::{self, PipelineCompilationOptions},
};
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
    pub const ALL: [Channel; 3] = [Channel::R, Channel::G, Channel::B];
    pub fn label(self) -> &'static str {
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

pub trait ProcessingPipeline: Send + Sync {
    fn process(&self, original: &ColorImage, settings: &ChannelSettings) -> ColorImage;
}

pub struct CpuPipeline;

impl CpuPipeline {
    fn build_lut(brightness: f32, contrast: f32) -> [u8; 256] {
        let mut lut = [0u8; 256];

        for (idx, entry) in lut.iter_mut().enumerate() {
            let x = idx as f32 / 255.0;
            let y = ((x - 0.5) * contrast + 0.5 + brightness).clamp(0.0, 1.0);
            *entry = (y * 255.0).round() as u8;
        }

        lut
    }
}

impl ProcessingPipeline for CpuPipeline {
    fn process(&self, original: &ColorImage, settings: &ChannelSettings) -> ColorImage {
        let luts: [_; 3] = std::array::from_fn(|i| {
            CpuPipeline::build_lut(
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
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuChannelState {
    pub color: [f32; 4],
    pub brightness: f32,
    pub contrast: f32,
    pub _padding: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuSettings {
    pub channels: [GpuChannelState; 3],
}

pub struct GpuPipeline {
    device: wgpu::Device,
    queue: wgpu::Queue,
    compute_pipeline: wgpu::ComputePipeline,
    bindgroup_layout: wgpu::BindGroupLayout,
}

impl GpuPipeline {
    pub async fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok()?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .ok()?;

        let bindgroup_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_bindgroup_layout"),
            entries: &[
                // input image
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // output image
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // settings (uniform buffer)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let shader_src = "
            struct ChannelState {
                color: vec4<f32>,
                brightness: f32,
                contrast: f32,
                _padding: vec2<f32>,
            }

            struct Settings {
                channels: array<ChannelState, 3>,
            }

            @group(0) @binding(0) var<storage, read> input_buffer: array<u32>;
            @group(0) @binding(1) var<storage, read_write> output_buffer: array<u32>;
            @group(0) @binding(2) var<uniform> settings: Settings;

            fn apply_bc(v: f32, brightness: f32, contrast: f32) -> f32 {
                return clamp((v - 0.5) * contrast + 0.5 + brightness, 0.0, 1.0);
            }

            @compute @workgroup_size(64)
            fn main(
                @builtin(global_invocation_id) global_id: vec3<u32>,
                @builtin(workgroup_id) workgroup_id: vec3<u32>
            ) {
                let MAX_X_DISPATCH = 65535u;
                let index = (workgroup_id.y * MAX_X_DISPATCH * 64u) + global_id.x;

                if (index >= arrayLength(&input_buffer)) {
                    return;
                }

                let raw_pixel = input_buffer[index];
                let r_u = (raw_pixel      ) & 0xFFu;
                let g_u = (raw_pixel >>  8u) & 0xFFu;
                let b_u = (raw_pixel >> 16u) & 0xFFu;
                let a_u = (raw_pixel >> 24u) & 0xFFu;
                let color = vec3<f32>(f32(r_u), f32(g_u), f32(b_u)) / 255.0;

                let r_bc = apply_bc(color.r, settings.channels[0].brightness, settings.channels[0].contrast);
                let g_bc = apply_bc(color.g, settings.channels[1].brightness, settings.channels[1].contrast);
                let b_bc = apply_bc(color.b, settings.channels[2].brightness, settings.channels[2].contrast);
                let out_color =
                    r_bc * settings.channels[0].color.rgb +
                    g_bc * settings.channels[1].color.rgb +
                    b_bc * settings.channels[2].color.rgb;

                let final_rgb = clamp(out_color, vec3<f32>(0.0), vec3<f32>(1.0));
                let final_r = u32(final_rgb.r * 255.0) & 0xFFu;
                let final_g = u32(final_rgb.g * 255.0) & 0xFFu;
                let final_b = u32(final_rgb.b * 255.0) & 0xFFu;

                let packed = (a_u << 24u) | (final_b << 16u) | (final_g << 8u) | final_r;

                output_buffer[index] = packed;
            }
        ";

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_src)),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compute_pipeline_layout"),
            bind_group_layouts: &[Some(&bindgroup_layout)],
            immediate_size: 0,
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        });

        Some(Self {
            device,
            queue,
            compute_pipeline,
            bindgroup_layout,
        })
    }
}

impl ProcessingPipeline for GpuPipeline {
    fn process(&self, original: &ColorImage, settings: &ChannelSettings) -> ColorImage {
        let gpu_settings = GpuSettings {
            channels: std::array::from_fn(|i| {
                let state = &settings.channels[i];
                GpuChannelState {
                    color: [
                        state.color.r() as f32 / 255.0,
                        state.color.g() as f32 / 255.0,
                        state.color.b() as f32 / 255.0,
                        1.0,
                    ],
                    brightness: state.brightness,
                    contrast: state.contrast,
                    _padding: [0.0; 2],
                }
            }),
        };

        let settings_array = [gpu_settings];
        let settings_bytes: &[u8] = bytemuck::cast_slice(&settings_array);
        let pixel_bytes: &[u8] = bytemuck::cast_slice(original.pixels.as_slice());
        let buffer_size = pixel_bytes.len() as wgpu::BufferAddress;

        let input_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Input Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: settings_bytes.len() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.queue.write_buffer(&input_buffer, 0, pixel_bytes);
        self.queue.write_buffer(&uniform_buffer, 0, settings_bytes);

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute_bind_group"),
            layout: &self.bindgroup_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("compute_encoder"),
            });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute_pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);

            let total_pixels = original.pixels.len() as u32;
            let total_workgroups = (total_pixels + 63) / 64;

            let max_dispatch = 65535;
            let workgroups_x = total_workgroups.min(max_dispatch);
            let workgroups_y = (total_workgroups + max_dispatch - 1) / max_dispatch;

            cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }
        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, buffer_size);
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
            sender.send(v).unwrap();
        });

        let _ = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });

        if receiver
            .recv()
            .expect("Failed to receive map result")
            .is_ok()
        {
            let data = buffer_slice.get_mapped_range();
            let result_pixels: &[Color32] = bytemuck::cast_slice(&data);
            let result_image = ColorImage::new(original.size, result_pixels.to_vec());

            // the GPU is not allowed to modify the shared memory while the CPU is reading from it
            // (with `get_mapped_range()` in this case).
            // so, the `data` variable needs to be dropped to signal to the GPU that we've taken what we need,
            // and it is free to work with it again.
            drop(data);
            staging_buffer.unmap();

            result_image
        } else {
            eprintln!("Failed to map GPU memory. Returning unedited image.");
            original.clone()
        }
    }
}

pub fn find_pipeline() -> Box<dyn ProcessingPipeline> {
    if let Some(gpu_pipeline) = pollster::block_on(GpuPipeline::new()) {
        println!("wgpu acceleration is available.");
        Box::new(gpu_pipeline)
    } else {
        println!("wgpu acceleration was not available. falling back to cpu");
        Box::new(CpuPipeline)
    }
}
