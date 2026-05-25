use crate::gui;
use crate::image_io::{
    color_image_to_rgba, format_from_str, is_supported_extension, load_image_from,
};
use crate::processing::{ChannelSettings, process_channels};
use eframe::{
    self,
    egui::{ColorImage, Context, TextureHandle},
};
use num_cpus;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use threadpool::ThreadPool;

pub enum LoadState {
    NotLoaded,
    Loaded {
        original: ColorImage,
        display: TextureHandle,
    },
    Failed(String),
}

pub struct ExportState {
    pub total: usize,
    pub done: usize,
}

impl ExportState {
    fn reset(&mut self, total: usize) {
        self.total = total;
        self.done = 0;
    }

    pub fn in_progress(&self) -> bool {
        self.total > 0 && self.done < self.total
    }
}

struct BackgroundWorkers {
    // background image loading
    load_tx: Sender<(usize, Result<ColorImage, String>)>,
    load_rx: Receiver<(usize, Result<ColorImage, String>)>,

    // background processing
    proc_tx: Sender<(usize, ColorImage)>,
    proc_rx: Receiver<(usize, ColorImage)>,

    // background export
    export_tx: Sender<usize>,
    export_rx: Receiver<usize>,
}

impl BackgroundWorkers {
    fn new() -> Self {
        let (load_tx, load_rx) = mpsc::channel();
        let (proc_tx, proc_rx) = mpsc::channel();
        let (export_tx, export_rx) = mpsc::channel();

        Self {
            load_tx,
            load_rx,
            proc_tx,
            proc_rx,
            export_tx,
            export_rx,
        }
    }
}

pub struct AppState {
    pub currently_busy: bool,
    pub export_format: String,
    pub current_image: usize,
    pub images: Vec<(PathBuf, LoadState)>,
    pub loaded_count: usize,
    pub export: ExportState,
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
            export: ExportState { total: 0, done: 0 },
            channels: ChannelSettings::default(),
            // use half of the logical cores available (at least 1)
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
    pub fn open_file(&mut self, path: PathBuf) {
        self.reset_images();
        self.images.push((path.clone(), LoadState::NotLoaded));
        let tx = self.workers.load_tx.clone();
        self.pool.execute(move || {
            let result = load_image_from(&path);
            let _ = tx.send((0, result));
        });
    }

    pub fn open_folder(&mut self, folder: PathBuf) {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            return;
        };

        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.is_file() {
                    let ext = path.extension()?.to_str()?;
                    is_supported_extension(ext).then_some(path)
                } else {
                    None
                }
            })
            .collect();
        paths.sort();

        if paths.is_empty() {
            return;
        }

        self.reset_images();
        self.images = paths
            .into_iter()
            .map(|p| (p, LoadState::NotLoaded))
            .collect();

        for (index, (path, _)) in self.images.iter().enumerate() {
            let tx = self.workers.load_tx.clone();
            let path = path.clone();
            self.pool.execute(move || {
                let result = load_image_from(&path);
                let _ = tx.send((index, result));
            });
        }
    }

    pub fn reprocess_all(&mut self) {
        let settings = self.channels.clone();
        let tx = self.workers.proc_tx.clone();

        for (index, (_, state)) in self.images.iter().enumerate() {
            if let LoadState::Loaded { original, .. } = state {
                let original = original.clone();
                let settings = settings.clone();
                let tx = tx.clone();
                self.pool.execute(move || {
                    let img = process_channels(&original, &settings);
                    let _ = tx.send((index, img));
                });
            }
        }
    }

    pub fn start_export(&mut self, folder: PathBuf) {
        if self.images.is_empty() || self.loaded_count == 0 {
            return;
        }

        let (fmt, ext) = format_from_str(&self.export_format);
        let paths: Vec<PathBuf> = self.images.iter().map(|(p, _)| p.clone()).collect();
        let settings = self.channels.clone();
        let done_tx = self.workers.export_tx.clone();

        self.export.reset(self.images.len());
        self.currently_busy = true;

        for in_path in paths {
            let folder = folder.clone();
            let tx = done_tx.clone();
            let settings = settings.clone();
            let ext = ext.to_string();

            self.pool.execute(move || {
                let result = load_image_from(&in_path).map(|img| process_channels(&img, &settings));
                match result {
                    Ok(processed) => {
                        let mut out_name = in_path
                            .file_stem()
                            .map(|s| s.to_os_string())
                            .unwrap_or_default();
                        out_name.push(format!(".{}", ext));
                        let out_path = folder.join(out_name);

                        let dynimg =
                            image::DynamicImage::ImageRgba8(color_image_to_rgba(&processed));
                        if let Err(e) = dynimg.save_with_format(&out_path, fmt) {
                            eprintln!("Failed to save {}: {}", out_path.display(), e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to process {}: {}", in_path.display(), e);
                    }
                }
                let _ = tx.send(1);
            });
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

    fn reset_images(&mut self) {
        self.images.clear();
        self.current_image = 0;
        self.loaded_count = 0;
        self.export = ExportState { total: 0, done: 0 };
        self.currently_busy = true;
    }

    fn poll_workers(&mut self, ctx: &Context) {
        while let Ok((index, result)) = self.workers.load_rx.try_recv() {
            match result {
                Ok(original) => {
                    let processed = process_channels(&original, &self.channels);
                    let texture = ctx.load_texture(
                        self.images[index].0.to_string_lossy(),
                        processed,
                        Default::default(),
                    );
                    self.images[index].1 = LoadState::Loaded {
                        original,
                        display: texture,
                    };
                }
                Err(e) => {
                    eprintln!("Failed to load image [{}]: {}", index, e);
                    self.images[index].1 = LoadState::Failed(e);
                }
            }
            self.loaded_count += 1;
            if self.loaded_count >= self.images.len() {
                self.currently_busy = false;
            }
        }

        while let Ok((index, processed)) = self.workers.proc_rx.try_recv() {
            if let Some((_, LoadState::Loaded { display, .. })) = self.images.get_mut(index) {
                display.set(processed, Default::default());
            }
        }

        while let Ok(done) = self.workers.export_rx.try_recv() {
            self.export.done += done;
            if !self.export.in_progress() {
                self.currently_busy = false;
            }
        }
    }
}
