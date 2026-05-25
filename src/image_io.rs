use std::path::Path;

use eframe::egui::ColorImage;
use exif::{In, Reader, Tag};
use image::{ImageBuffer, ImageFormat, Rgba};

pub fn load_image(path: &Path) -> Result<ColorImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let mut img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;

    // if there is any exif data about orientation, we try to apply it to the image.
    if let Ok(exif) = Reader::new().read_from_container(&mut std::io::Cursor::new(&bytes)) {
        if let Some(orientation) = exif.get_field(Tag::Orientation, In::PRIMARY) {
            match orientation.value.get_uint(0) {
                Some(2) => img = img.fliph(),
                Some(3) => img = img.rotate180(),
                Some(4) => img = img.flipv(),
                Some(5) => {
                    img = img.rotate90();
                    img = img.fliph();
                }
                Some(6) => img = img.rotate90(),
                Some(7) => {
                    img = img.rotate270();
                    img = img.fliph();
                }
                Some(8) => img = img.rotate270(),
                _ => {}
            }
        }
    }

    let image_buffer = img.into_rgba8();
    let size = [
        image_buffer.width() as usize,
        image_buffer.height() as usize,
    ];
    let pixels = image_buffer.into_raw();

    Ok(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
}

pub fn colorimage_to_imagebuffer(image: &ColorImage) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let [w, h] = image.size;
    let buf: Vec<u8> = bytemuck::cast_slice(image.pixels.as_slice()).to_vec();
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
