//! Runs a full export on a worker thread and reports progress back to the UI.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use eframe::egui;
use eframe::emath::vec2;
use image::{codecs::jpeg::JpegEncoder, codecs::png, ImageEncoder, RgbaImage};

use super::renderer::{coverage, darken, flatten, render_layer, Placement};
use super::slicer;
use crate::errors::{display_name, UserError};
use crate::project::{ExportFormat, Project};
use crate::steam::profile::ProfileGeometry;
use crate::steam::validation::Severity;
use crate::steam::LayoutSpec;

pub struct ExportJob {
    pub project: Project,
    pub layout: LayoutSpec,
    pub profile: ProfileGeometry,
    pub avatar_size: u32,
    pub size_warning_bytes: u64,
    pub artwork: Arc<RgbaImage>,
    pub background: Option<Arc<RgbaImage>>,
    pub avatar: Option<Arc<RgbaImage>>,
    /// Final destination folder (created if needed).
    pub folder: PathBuf,
    pub project_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExportedFile {
    pub name: String,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ReportLine {
    pub severity: Severity,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ExportReport {
    pub folder: PathBuf,
    pub zip: Option<PathBuf>,
    pub files: Vec<ExportedFile>,
    pub lines: Vec<ReportLine>,
    pub scale: u32,
}

impl ExportReport {
    pub fn worst(&self) -> Severity {
        self.lines.iter().map(|l| l.severity).max().unwrap_or(Severity::Ok)
    }
}

pub enum ExportEvent {
    Progress { fraction: f32, label: String },
    Finished(Result<ExportReport, UserError>),
}

pub fn spawn(job: ExportJob, ctx: egui::Context) -> Receiver<ExportEvent> {
    let (tx, rx) = channel();
    std::thread::Builder::new()
        .name("export".into())
        .spawn(move || {
            let progress = Progress { tx: tx.clone(), ctx: ctx.clone() };
            let result = run(&job, &progress);
            let _ = tx.send(ExportEvent::Finished(result));
            ctx.request_repaint();
        })
        .expect("failed to spawn export thread");
    rx
}

struct Progress {
    tx: Sender<ExportEvent>,
    ctx: egui::Context,
}

impl Progress {
    fn report(&self, fraction: f32, label: impl Into<String>) {
        let _ = self.tx.send(ExportEvent::Progress { fraction, label: label.into() });
        self.ctx.request_repaint();
    }
}

fn run(job: &ExportJob, progress: &Progress) -> Result<ExportReport, UserError> {
    let p = &job.project;
    let art = p.artwork.as_ref().ok_or_else(|| {
        UserError::new("Nothing to export yet", "Add artwork before exporting.")
    })?;
    std::fs::create_dir_all(&job.folder).map_err(|e| UserError::from_io("create", &job.folder, &e))?;

    let scale = p.export.scale.resolve(Some(art.transform.scale));
    let ext = p.export.format.extension();
    let mut files = Vec::new();
    let mut lines = Vec::new();
    let mut expected: Vec<(PathBuf, u32, u32)> = Vec::new();
    let flatten_color = p.style.canvas_fill;
    let must_flatten = p.export.fill_transparent || p.export.format == ExportFormat::Jpeg;

    let layer = Placement { image: &job.artwork, transform: art.transform, crop: art.crop };
    let plans = slicer::plan(&job.layout, p.art_height, scale);
    let mut pieces: Vec<RgbaImage> = Vec::new();
    let steps = plans.len() as f32 + 4.0;

    for (i, plan) in plans.iter().enumerate() {
        progress.report(i as f32 / steps, format!("Rendering {}", plan.region.label.to_lowercase()));
        let mut img = slicer::render(plan, scale, Some(&layer));
        let cov = coverage(&img);
        if cov < 0.995 {
            lines.push(ReportLine {
                severity: Severity::Warning,
                text: format!("{} is {:.0}% covered; empty areas were {}.", plan.region.label, cov * 100.0,
                    if must_flatten { "filled" } else { "left transparent" }),
            });
        }
        if must_flatten {
            flatten(&mut img, flatten_color);
        }
        let name = plan.file_name(ext);
        let path = job.folder.join(&name);
        write_image(&img, &path, p.export.format, p.export.jpeg_quality)?;
        expected.push((path.clone(), plan.pixel_width, plan.pixel_height));
        lines.push(ReportLine {
            severity: Severity::Ok,
            text: format!(
                "{} generated: {} × {} px, shows at {} × {}",
                plan.region.label, plan.pixel_width, plan.pixel_height,
                plan.display.width() as u32, plan.display.height() as u32
            ),
        });
        pieces.push(img);
    }
    if plans.len() > 1 {
        lines.push(ReportLine {
            severity: Severity::Ok,
            text: format!("All {} pieces share a {} px height, so they line up on Steam", plans.len(), plans[0].pixel_height),
        });
    }

    let base_step = plans.len() as f32;
    if p.export.include_avatar {
        if let (Some(av_layer), Some(av_img)) = (&p.avatar, &job.avatar) {
            progress.report(base_step / steps, "Preparing avatar");
            let size = vec2(av_img.width() as f32, av_img.height() as f32);
            let r = av_layer.crop_rect(size);
            let cropped = image::imageops::crop_imm(
                av_img.as_ref(),
                r.min.x.max(0.0) as u32,
                r.min.y.max(0.0) as u32,
                (r.width() as u32).max(1),
                (r.height() as u32).max(1),
            )
            .to_image();
            let mut out = image::imageops::resize(&cropped, job.avatar_size, job.avatar_size, image::imageops::FilterType::Lanczos3);
            flatten(&mut out, flatten_color);
            let path = job.folder.join(format!("Avatar.{ext}"));
            write_image(&out, &path, p.export.format, p.export.jpeg_quality)?;
            expected.push((path, job.avatar_size, job.avatar_size));
        }
    }

    if p.export.include_background {
        if let Some(bg) = &job.background {
            progress.report((base_step + 1.0) / steps, "Writing background reference");
            let path = job.folder.join("Background.png");
            write_image(bg, &path, ExportFormat::Png, 100)?;
            expected.push((path, bg.width(), bg.height()));
        }
    }

    if p.export.include_preview {
        progress.report((base_step + 2.0) / steps, "Composing preview");
        let preview = compose_preview(job, &plans, &pieces, scale);
        let path = job.folder.join("Preview.png");
        write_image(&preview, &path, ExportFormat::Png, 100)?;
        expected.push((path, preview.width(), preview.height()));
    }

    if let (true, Some(json)) = (p.export.include_project, &job.project_json) {
        let path = job.folder.join("Project.lss");
        std::fs::write(&path, json).map_err(|e| UserError::from_io("write", &path, &e))?;
        files.push(file_entry(&path, 0, 0));
    }

    progress.report((base_step + 3.0) / steps, "Verifying files");
    let mut all_valid = true;
    for (path, w, h) in &expected {
        match image::image_dimensions(path) {
            Ok((rw, rh)) if rw == *w && rh == *h => {}
            Ok((rw, rh)) => {
                all_valid = false;
                lines.push(ReportLine {
                    severity: Severity::Error,
                    text: format!("{} is {rw} × {rh} px; expected {w} × {h}.", display_name(path)),
                });
            }
            Err(_) => {
                all_valid = false;
                lines.push(ReportLine { severity: Severity::Error, text: format!("{} couldn't be read back.", display_name(path)) });
            }
        }
        let entry = file_entry(path, *w, *h);
        if entry.bytes > job.size_warning_bytes {
            lines.push(ReportLine {
                severity: Severity::Warning,
                text: format!(
                    "{} is {:.1} MB, which may upload slowly. Try 1× resolution or JPEG.",
                    entry.name,
                    entry.bytes as f64 / 1_048_576.0
                ),
            });
        }
        files.push(entry);
    }
    if all_valid {
        lines.push(ReportLine { severity: Severity::Ok, text: "Dimensions verified on disk".into() });
    }
    lines.push(ReportLine { severity: Severity::Ok, text: format!("{} files exported, no missing assets", files.len()) });

    let zip = if p.export.make_zip {
        progress.report((base_step + 3.5) / steps, "Creating ZIP");
        let zip_path = job.folder.with_extension("zip");
        write_zip(&zip_path, &files)?;
        lines.push(ReportLine { severity: Severity::Ok, text: format!("ZIP created: {}", display_name(&zip_path)) });
        Some(zip_path)
    } else {
        None
    };

    progress.report(1.0, "Done");
    Ok(ExportReport { folder: job.folder.clone(), zip, files, lines, scale })
}

/// The showcase strip as it sits on the profile: background behind, pieces on top.
/// Useful for sharing and for eyeballing alignment across the gap.
fn compose_preview(job: &ExportJob, plans: &[slicer::SlicePlan], pieces: &[RgbaImage], scale: u32) -> RgbaImage {
    let p = &job.project;
    let comp = job.layout.size(p.art_height.round());
    let (w, h) = ((comp.x as u32) * scale, (comp.y as u32) * scale);
    let [r, g, b] = p.style.backdrop;
    let mut out = RgbaImage::from_pixel(w, h, image::Rgba([r, g, b, 255]));
    if let (Some(bg_layer), Some(bg)) = (&p.background, &job.background) {
        let arranged = job.profile.arrange(comp);
        let size = vec2(bg.width() as f32, bg.height() as f32);
        let placement = Placement { image: bg, transform: bg_layer.transform(size), crop: Default::default() };
        render_layer(&mut out, arranged.comp_origin, scale as f32, &placement);
        darken(&mut out, bg_layer.dim);
    }
    for (plan, piece) in plans.iter().zip(pieces) {
        let x = (plan.display.min.x as u32 * scale) as i64;
        image::imageops::overlay(&mut out, piece, x, 0);
    }
    out
}

fn write_image(img: &RgbaImage, path: &Path, format: ExportFormat, quality: u8) -> Result<(), UserError> {
    let file = File::create(path).map_err(|e| UserError::from_io("write", path, &e))?;
    let mut w = BufWriter::new(file);
    let (iw, ih) = img.dimensions();
    let result = match format {
        ExportFormat::Png => png::PngEncoder::new_with_quality(&mut w, png::CompressionType::Default, png::FilterType::Adaptive)
            .write_image(img.as_raw(), iw, ih, image::ExtendedColorType::Rgba8),
        ExportFormat::Jpeg => {
            let rgb = image::DynamicImage::ImageRgba8(img.clone()).into_rgb8();
            JpegEncoder::new_with_quality(&mut w, quality.clamp(1, 100))
                .write_image(rgb.as_raw(), iw, ih, image::ExtendedColorType::Rgb8)
        }
    };
    result.map_err(|e| UserError::from_image(path, &e))?;
    w.flush().map_err(|e| UserError::from_io("write", path, &e))
}

fn file_entry(path: &Path, width: u32, height: u32) -> ExportedFile {
    ExportedFile {
        name: display_name(path),
        path: path.to_path_buf(),
        width,
        height,
        bytes: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
    }
}

fn write_zip(zip_path: &Path, files: &[ExportedFile]) -> Result<(), UserError> {
    let fail = |e: &dyn std::fmt::Display| {
        UserError::new(format!("Couldn't create {}", display_name(zip_path)), "The ZIP archive couldn't be written.")
            .hint(format!("Your exported files are still in the folder. Technical detail: {e}"))
    };
    let file = File::create(zip_path).map_err(|e| UserError::from_io("create", zip_path, &e))?;
    let mut zip = zip::ZipWriter::new(BufWriter::new(file));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for f in files {
        let data = std::fs::read(&f.path).map_err(|e| UserError::from_io("read", &f.path, &e))?;
        zip.start_file(f.name.as_str(), options).map_err(|e| fail(&e))?;
        zip.write_all(&data).map_err(|e| fail(&e))?;
    }
    zip.finish().map_err(|e| fail(&e))?;
    Ok(())
}

pub fn sanitize_folder_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').to_string();
    if trimmed.is_empty() { "LuminarSteamProfile".into() } else { trimmed }
}
