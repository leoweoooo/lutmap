use std::{fs::File, io::BufReader, path::Path};

use eframe::egui::ColorImage;
use image::{ImageBuffer, ImageFormat, Rgba};

pub fn load_image_from(path: &Path) -> Result<ColorImage, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let image = image::ImageReader::new(reader)
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;

    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.to_rgba8().into_raw();
    Ok(ColorImage::from_rgba_unmultiplied(size, &pixels))
}

pub fn color_image_to_rgba(image: &ColorImage) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let [w, h] = image.size;
    let mut buf: Vec<u8> = Vec::with_capacity(w * h * 4);
    for px in &image.pixels {
        buf.extend_from_slice(&[px.r(), px.g(), px.b(), px.a()]);
    }
    ImageBuffer::<Rgba<u8>, _>::from_vec(w as u32, h as u32, buf)
        .expect("pixel buffer size must match image dimensions")
}

pub fn format_from_str(format: &str) -> (ImageFormat, &'static str) {
    match format.to_uppercase().as_str() {
        "PNG" => (ImageFormat::Png, "png"),
        "JPG" | "JPEG" => (ImageFormat::Jpeg, "jpg"),
        "TIFF" | "TIF" => (ImageFormat::Tiff, "tif"),
        other => {
            eprintln!("Unknown export format '{}', defaulting to PNG", other);
            (ImageFormat::Png, "png")
        }
    }
}

pub fn is_supported_extension(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "tiff" | "tif"
    )
}
