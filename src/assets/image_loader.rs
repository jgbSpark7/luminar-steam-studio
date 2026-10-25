use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::ColorImage;
use image::RgbaImage;

use super::animation::{self, FrameInfo};
use crate::errors::{display_name, UserError};

pub const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "ico"];

/// Longest edge of the on-screen preview texture. Exports always use full resolution.
const PREVIEW_MAX_EDGE: u32 = 2048;

#[derive(Debug, Clone, PartialEq)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub file_bytes: u64,
    pub frames: FrameInfo,
    pub has_alpha: bool,
}

impl ImageInfo {
    pub fn summary(&self) -> String {
        format!("{} × {} {}", self.width, self.height, self.format)
    }
}

pub struct DecodedImage {
    pub path: PathBuf,
    pub full: Arc<RgbaImage>,
    pub preview: ColorImage,
    pub info: ImageInfo,
}

pub fn is_project_file(path: &Path) -> bool {
    ext_of(path).as_deref() == Some(crate::project::serialization::EXTENSION)
}

fn ext_of(path: &Path) -> Option<String> {
    path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase())
}

/// Rejects obvious non-images early with advice specific to the file type.
pub fn check_extension(path: &Path) -> Result<(), UserError> {
    let name = display_name(path);
    let Some(ext) = ext_of(path) else {
        return Err(UserError::new(format!("Couldn't open {name}"), "The file has no extension, so its type is unknown.")
            .hint("Save it as PNG, JPG, WEBP or GIF."));
    };
    if SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return Ok(());
    }
    let hint = match ext.as_str() {
        "psd" | "psb" => "Photoshop files can't be opened directly. Use File > Export > PNG in Photoshop first.",
        "kra" | "clip" | "sai" | "procreate" | "xcf" => "Export the drawing from your art program as PNG first.",
        "svg" | "ai" | "eps" => "Vector files need to be rendered to PNG at the size you want first.",
        "heic" | "heif" | "avif" => "Convert the photo to JPG or PNG first.",
        "mp4" | "webm" | "mov" => "Video artwork isn't supported yet. Convert it to a GIF for now.",
        _ => "Supported formats are PNG, JPG, WEBP, GIF, BMP and TIFF.",
    };
    Err(UserError::new(format!("Couldn't open {name}"), format!("“.{ext}” files aren't supported.")).hint(hint))
}

/// Reads only the header. Fast enough to call on the UI thread when importing.
pub fn probe_dimensions(path: &Path) -> Result<(u32, u32), UserError> {
    check_extension(path)?;
    image::image_dimensions(path).map_err(|e| UserError::from_image(path, &e))
}

pub fn decode(path: &Path) -> Result<DecodedImage, UserError> {
    check_extension(path)?;
    let bytes = std::fs::read(path).map_err(|e| UserError::from_io("open", path, &e))?;
    let reader = image::ImageReader::new(Cursor::new(bytes.as_slice()))
        .with_guessed_format()
        .map_err(|e| UserError::from_io("open", path, &e))?;
    let format = reader.format();
    let img = reader.decode().map_err(|e| UserError::from_image(path, &e))?;
    let source_has_alpha = img.color().has_alpha();
    let rgba = img.into_rgba8();
    let has_alpha = source_has_alpha && rgba.pixels().any(|p| p[3] < 255);

    let info = ImageInfo {
        width: rgba.width(),
        height: rgba.height(),
        format: format.map(|f| format!("{f:?}").to_uppercase()).unwrap_or_else(|| "IMAGE".into()),
        file_bytes: bytes.len() as u64,
        frames: animation::probe(&bytes, format),
        has_alpha,
    };
    let preview = make_preview(&rgba);
    Ok(DecodedImage { path: path.to_path_buf(), full: Arc::new(rgba), preview, info })
}

fn make_preview(rgba: &RgbaImage) -> ColorImage {
    let (w, h) = rgba.dimensions();
    let longest = w.max(h);
    if longest <= PREVIEW_MAX_EDGE {
        return ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw());
    }
    let k = PREVIEW_MAX_EDGE as f32 / longest as f32;
    let (nw, nh) = (((w as f32 * k).round() as u32).max(1), ((h as f32 * k).round() as u32).max(1));
    let small = image::imageops::resize(rgba, nw, nh, image::imageops::FilterType::Triangle);
    ColorImage::from_rgba_unmultiplied([nw as usize, nh as usize], small.as_raw())
}
