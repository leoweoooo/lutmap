use std::{
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
};

use eframe::egui::ColorImage;
use threadpool::ThreadPool;

use crate::{
    image_io::{colorimage_to_imagebuffer, load_image},
    processing::{ChannelSettings, ProcessingPipeline, find_pipeline},
};

struct LoadWorker {
    tx: Sender<(usize, Result<ColorImage, String>)>,
    rx: Receiver<(usize, Result<ColorImage, String>)>,
}

impl Default for LoadWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadWorker {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }

    fn submit(&self, index: usize, in_path: PathBuf, pool: &ThreadPool) {
        let tx = self.tx.clone();
        pool.execute(move || {
            let result = load_image(&in_path);
            let _ = tx.send((index, result));
        });
    }

    fn poll(&self) -> Option<(usize, Result<ColorImage, String>)> {
        self.rx.try_recv().ok()
    }
}

struct ExportWorker {
    tx: Sender<Result<(), String>>,
    rx: Receiver<Result<(), String>>,
}

impl Default for ExportWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportWorker {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }

    fn submit(
        &self,
        original: Arc<ColorImage>,
        out_path: PathBuf,
        settings: ChannelSettings,
        format: image::ImageFormat,
        pool: &ThreadPool,
        pipeline: Arc<dyn ProcessingPipeline>,
    ) {
        let tx = self.tx.clone();
        pool.execute(move || {
            let processed = pipeline.process(&original, &settings);
            let rgba = colorimage_to_imagebuffer(&processed);
            let dynimg = image::DynamicImage::ImageRgba8(rgba);

            if let Err(e) = dynimg.save_with_format(&out_path, format) {
                let _ = tx.send(Err(format!("Failed to save image: {}", e)));
            } else {
                let _ = tx.send(Ok(()));
            }
        });
    }

    fn poll(&self) -> Option<Result<(), String>> {
        self.rx.try_recv().ok()
    }
}

struct PreviewWorker {
    tx: Sender<(usize, ColorImage)>,
    rx: Receiver<(usize, ColorImage)>,
}

impl PreviewWorker {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }

    fn submit(
        &self,
        index: usize,
        original_image: Arc<ColorImage>,
        settings: ChannelSettings,
        pool: &ThreadPool,
        pipeline: Arc<dyn ProcessingPipeline>,
    ) {
        let tx = self.tx.clone();
        pool.execute(move || {
            let preview = pipeline.process(&original_image, &settings);
            let _ = tx.send((index, preview));
        });
    }

    fn poll(&self) -> Option<(usize, ColorImage)> {
        self.rx.try_recv().ok()
    }
}

pub(crate) enum WorkerEvents {
    ImageLoaded {
        idx: usize,
        result: Result<ColorImage, String>,
    },
    PreviewReady {
        idx: usize,
        result: ColorImage,
    },
    ExportFinished(Result<(), String>),
}

pub(crate) struct BackgroundWorkers {
    load: LoadWorker,
    export: ExportWorker,
    preview: PreviewWorker,
    pipeline: Arc<dyn ProcessingPipeline>,
}

impl BackgroundWorkers {
    pub fn new() -> Self {
        let pipeline: Arc<dyn ProcessingPipeline> = Arc::from(find_pipeline());

        Self {
            load: LoadWorker::new(),
            export: ExportWorker::new(),
            preview: PreviewWorker::new(),
            pipeline,
        }
    }

    pub fn submit_load(&self, idx: usize, in_path: PathBuf, pool: &ThreadPool) {
        self.load.submit(idx, in_path, pool);
    }

    pub fn submit_export(
        &self,
        original: Arc<ColorImage>,
        out_path: PathBuf,
        settings: ChannelSettings,
        format: image::ImageFormat,
        pool: &ThreadPool,
    ) {
        self.export.submit(
            original,
            out_path,
            settings,
            format,
            pool,
            self.pipeline.clone(),
        );
    }

    pub fn submit_preview(
        &self,
        idx: usize,
        original: Arc<ColorImage>,
        settings: ChannelSettings,
        pool: &ThreadPool,
    ) {
        self.preview
            .submit(idx, original, settings, pool, self.pipeline.clone());
    }

    pub fn poll(&self) -> Option<WorkerEvents> {
        if let Some((idx, result)) = self.load.poll() {
            return Some(WorkerEvents::ImageLoaded { idx, result });
        }
        if let Some((idx, result)) = self.preview.poll() {
            return Some(WorkerEvents::PreviewReady { idx, result });
        }
        if let Some(result) = self.export.poll() {
            return Some(WorkerEvents::ExportFinished(result));
        }
        None
    }
}
