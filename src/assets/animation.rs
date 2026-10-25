//! Animation awareness. The MVP exports still frames, but knows when a source is
//! animated so it can say so. Frame-accurate decoding will plug in here later.

use std::io::Cursor;

use image::{AnimationDecoder, ImageFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameInfo {
    Still,
    Animated { frames: usize },
}

/// Caps decoding work for very long GIFs; we only need a rough count.
const MAX_FRAMES_COUNTED: usize = 600;

pub fn probe(bytes: &[u8], format: Option<ImageFormat>) -> FrameInfo {
    let frames = match format {
        Some(ImageFormat::Gif) => image::codecs::gif::GifDecoder::new(Cursor::new(bytes))
            .ok()
            .map(|d| d.into_frames().take(MAX_FRAMES_COUNTED).filter(Result::is_ok).count()),
        Some(ImageFormat::WebP) => image::codecs::webp::WebPDecoder::new(Cursor::new(bytes))
            .ok()
            .filter(|d| d.has_animation())
            .map(|d| d.into_frames().take(MAX_FRAMES_COUNTED).filter(Result::is_ok).count()),
        _ => None,
    };
    match frames {
        Some(n) if n > 1 => FrameInfo::Animated { frames: n },
        _ => FrameInfo::Still,
    }
}
