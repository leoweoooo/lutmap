use crate::gui;
use crate::image_io::format_from_str;
use crate::processing::ChannelSettings;
use crate::worker::{BackgroundWorkers, WorkerEvents};

use eframe::{
    self,
    egui::{ColorImage, Context, TextureHandle},
};
use num_cpus;
use threadpool::ThreadPool;

use std::path::PathBuf;
use std::sync::Arc;

pub enum LoadState {
    NotLoaded,
    Processing(Arc<ColorImage>),
    Loaded {
        original: Arc<ColorImage>,
        display: TextureHandle,
    },
    Failed(String),
}

pub struct AppState {
    pub currently_busy: bool,
    pub export_format: String,
    pub current_image: usize,
    pub images: Vec<(PathBuf, LoadState)>,
    pub loaded_count: usize,
    pub channels: ChannelSettings,
    pool: ThreadPool,
    workers: BackgroundWorkers,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            currently_busy: false,
            export_format: "PNG".to_string(),
            current_image: 0,
            images: Vec::new(),
            loaded_count: 0,
            channels: ChannelSettings::default(),
            pool: ThreadPool::new((num_cpus::get() / 2).max(1)),
            workers: BackgroundWorkers::new(),
        }
    }
}

impl eframe::App for AppState {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_workers(ui.ctx());
        gui::draw(ui, self);
    }
}

impl AppState {
    pub fn open_files(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }

        self.reset_workspace();
        self.images = paths
            .into_iter()
            .map(|p| (p, LoadState::NotLoaded))
            .collect();

        for (index, (path, _)) in self.images.iter().enumerate() {
            self.workers.submit_load(index, path.clone(), &self.pool);
        }
    }

    pub fn update_preview(&mut self) {
        if let Some((_, LoadState::Loaded { original, .. })) = self.images.get(self.current_image) {
            self.workers.submit_preview(
                self.current_image,
                original.clone(),
                self.channels.clone(),
                &self.pool,
            );
        }
    }

    pub fn start_export(&mut self, out_path: PathBuf) {
        if let Some((_, LoadState::Loaded { original, .. })) = self.images.get(self.current_image) {
            let (fmt, _) = format_from_str(&self.export_format);
            self.currently_busy = true;
            self.workers.submit_export(
                original.clone(),
                out_path,
                self.channels.clone(),
                fmt,
                &self.pool,
            );
        }
    }

    pub fn go_next(&mut self) {
        if self.current_image + 1 < self.images.len() {
            self.current_image += 1;
        }
    }

    pub fn go_prev(&mut self) {
        if self.current_image > 0 {
            self.current_image -= 1;
        }
    }

    fn reset_workspace(&mut self) {
        self.images.clear();
        self.current_image = 0;
        self.loaded_count = 0;
        self.currently_busy = true;
    }

    fn poll_workers(&mut self, ctx: &Context) {
        while let Some(event) = self.workers.poll() {
            match event {
                WorkerEvents::ImageLoaded { idx, result } => match result {
                    Ok(original) => {
                        let original_arc = Arc::new(original);
                        self.workers.submit_preview(
                            idx,
                            original_arc.clone(),
                            self.channels.clone(),
                            &self.pool,
                        );
                        self.images[idx].1 = LoadState::Processing(original_arc);
                    }
                    Err(e) => {
                        eprintln!("Failed to load image [{}]: {}", idx, e);
                        self.images[idx].1 = LoadState::Failed(e);
                        self.loaded_count += 1;
                        if self.loaded_count >= self.images.len() {
                            self.currently_busy = false;
                        }
                    }
                },

                WorkerEvents::PreviewReady { idx, result } => {
                    if let Some((path, state)) = self.images.get_mut(idx) {
                        match state {
                            LoadState::Processing(original_arc) => {
                                let texture = ctx.load_texture(
                                    path.to_string_lossy(),
                                    result,
                                    Default::default(),
                                );

                                *state = LoadState::Loaded {
                                    original: original_arc.clone(),
                                    display: texture,
                                };

                                self.loaded_count += 1;
                                if self.loaded_count >= self.images.len() {
                                    self.currently_busy = false;
                                }
                                ctx.request_repaint();
                            }

                            LoadState::Loaded { display, .. } => {
                                display.set(result, Default::default());
                                ctx.request_repaint();
                            }

                            _ => {}
                        }
                    }
                }

                WorkerEvents::ExportFinished(result) => {
                    self.currently_busy = false;
                    match result {
                        Ok(_) => println!("Successfully exported image!"),
                        Err(e) => eprintln!("Export error: {}", e),
                    }
                }
            }
        }
    }
}
