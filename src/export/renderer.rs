//! Full-resolution CPU renderer for export. Uses the exact same transform math as
//! the editor, so what you positioned is what you get.

use std::borrow::Cow;

use eframe::emath::{pos2, vec2, Pos2};
use image::{imageops::FilterType, RgbaImage};

use crate::project::transform::{bounds, quad, visible_rect};
use crate::project::{Crop, Transform};

pub struct Placement<'a> {
    pub image: &'a RgbaImage,
    pub transform: Transform,
    pub crop: Crop,
}

/// Composites `layer` over `target`. Target pixel (x, y) samples the space point
/// `origin + (x + 0.5, y + 0.5) / px_per_unit`.
pub fn render_layer(target: &mut RgbaImage, origin: Pos2, px_per_unit: f32, layer: &Placement) {
    let (tw, th) = target.dimensions();
    let (sw, sh) = layer.image.dimensions();
    if tw == 0 || th == 0 || sw == 0 || sh == 0 {
        return;
    }
    let src_size = vec2(sw as f32, sh as f32);
    let t = layer.transform;
    let vis = visible_rect(src_size, &layer.crop);

    // Heavy minification with bilinear sampling aliases badly, so pre-shrink the
    // source with a proper filter until roughly 1.25 source px per output px remain.
    let out_per_src = t.scale * px_per_unit;
    let (src, kx, ky): (Cow<RgbaImage>, f32, f32) = if out_per_src < 0.5 {
        let k = out_per_src * 1.25;
        let nw = ((sw as f32 * k).ceil() as u32).max(1);
        let nh = ((sh as f32 * k).ceil() as u32).max(1);
        let small = image::imageops::resize(layer.image, nw, nh, FilterType::Lanczos3);
        (Cow::Owned(small), nw as f32 / sw as f32, nh as f32 / sh as f32)
    } else {
        (Cow::Borrowed(layer.image), 1.0, 1.0)
    };

    // Only touch target pixels that the layer can reach.
    let b = bounds(&quad(&t, src_size, &layer.crop));
    let to_px = |v: f32, o: f32| (v - o) * px_per_unit;
    let x0 = to_px(b.min.x, origin.x).floor().clamp(0.0, tw as f32) as usize;
    let x1 = to_px(b.max.x, origin.x).ceil().clamp(0.0, tw as f32) as usize;
    let y0 = to_px(b.min.y, origin.y).floor().clamp(0.0, th as f32) as usize;
    let y1 = to_px(b.max.y, origin.y).ceil().clamp(0.0, th as f32) as usize;
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let lo = pos2(vis.min.x * kx, vis.min.y * ky);
    let hi = pos2(vis.max.x * kx - 1.0, vis.max.y * ky - 1.0);
    let row_bytes = tw as usize * 4;
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 16);
    let rows_per_chunk = (y1 - y0).div_ceil(threads).max(1);
    let buf: &mut [u8] = target;
    let region = &mut buf[y0 * row_bytes..y1 * row_bytes];
    let src: &RgbaImage = &src;

    std::thread::scope(|scope| {
        for (ci, chunk) in region.chunks_mut(rows_per_chunk * row_bytes).enumerate() {
            scope.spawn(move || {
                for (ri, row) in chunk.chunks_mut(row_bytes).enumerate() {
                    let py = y0 + ci * rows_per_chunk + ri;
                    let wy = origin.y + (py as f32 + 0.5) / px_per_unit;
                    for px in x0..x1 {
                        let wx = origin.x + (px as f32 + 0.5) / px_per_unit;
                        let s = t.comp_to_source(src_size, pos2(wx, wy));
                        if s.x < vis.min.x || s.y < vis.min.y || s.x >= vis.max.x || s.y >= vis.max.y {
                            continue;
                        }
                        let sample = bilinear(src, s.x * kx - 0.5, s.y * ky - 0.5, lo, hi);
                        let i = px * 4;
                        composite_over(&mut row[i..i + 4], sample);
                    }
                }
            });
        }
    });
}

/// Returns premultiplied RGBA in 0..1.
fn bilinear(img: &RgbaImage, x: f32, y: f32, lo: Pos2, hi: Pos2) -> [f32; 4] {
    let x = x.clamp(lo.x, hi.x.max(lo.x));
    let y = y.clamp(lo.y, hi.y.max(lo.y));
    let (w, h) = (img.width() as i32, img.height() as i32);
    let x0 = (x.floor() as i32).clamp(0, w - 1);
    let y0 = (y.floor() as i32).clamp(0, h - 1);
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let raw = img.as_raw();
    let px = |xx: i32, yy: i32| -> [f32; 4] {
        let i = (yy as usize * w as usize + xx as usize) * 4;
        let a = raw[i + 3] as f32 / 255.0;
        [raw[i] as f32 / 255.0 * a, raw[i + 1] as f32 / 255.0 * a, raw[i + 2] as f32 / 255.0 * a, a]
    };
    let (p00, p10, p01, p11) = (px(x0, y0), px(x1, y0), px(x0, y1), px(x1, y1));
    let mut out = [0.0; 4];
    for c in 0..4 {
        let top = p00[c] + (p10[c] - p00[c]) * fx;
        let bot = p01[c] + (p11[c] - p01[c]) * fx;
        out[c] = top + (bot - top) * fy;
    }
    out
}

/// Source-over onto a straight-alpha RGBA pixel.
fn composite_over(dst: &mut [u8], src: [f32; 4]) {
    let sa = src[3];
    if sa <= 0.0 {
        return;
    }
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    for c in 0..3 {
        let d = dst[c] as f32 / 255.0 * da;
        let v = (src[c] + d * (1.0 - sa)) / out_a.max(1e-6);
        dst[c] = (v * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dst[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

/// Fraction of pixels that are (nearly) opaque.
pub fn coverage(img: &RgbaImage) -> f32 {
    let total = (img.width() as u64 * img.height() as u64).max(1);
    let opaque = img.pixels().filter(|p| p[3] >= 250).count() as u64;
    opaque as f32 / total as f32
}

/// Flattens onto a solid colour, removing transparency.
pub fn flatten(img: &mut RgbaImage, rgb: [u8; 3]) {
    for p in img.pixels_mut() {
        let a = p[3] as f32 / 255.0;
        for c in 0..3 {
            p[c] = (p[c] as f32 * a + rgb[c] as f32 * (1.0 - a)).round() as u8;
        }
        p[3] = 255;
    }
}

pub fn darken(img: &mut RgbaImage, amount: f32) {
    let k = 1.0 - amount.clamp(0.0, 1.0);
    for p in img.pixels_mut() {
        for c in 0..3 {
            p[c] = (p[c] as f32 * k).round() as u8;
        }
    }
}
