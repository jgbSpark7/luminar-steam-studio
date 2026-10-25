use eframe::emath::Vec2;

use super::config::SteamConfig;
use super::layouts::LayoutSpec;
use crate::project::transform::covers;
use crate::project::Project;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Ok,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickFix {
    ImportArtwork,
    CoverRegions,
    ClampHeight,
    ChooseExportFolder,
    UseOneX,
}

impl QuickFix {
    pub fn label(self) -> &'static str {
        match self {
            Self::ImportArtwork => "Import artwork",
            Self::CoverRegions => "Fill all pieces",
            Self::ClampHeight => "Fix height",
            Self::ChooseExportFolder => "Choose folder",
            Self::UseOneX => "Export at 1×",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    pub fix: Option<QuickFix>,
}

impl Check {
    fn new(severity: Severity, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { severity, title: title.into(), detail: detail.into(), fix: None }
    }
    fn fix(mut self, f: QuickFix) -> Self {
        self.fix = Some(f);
        self
    }
}

pub enum ArtworkState {
    Missing,
    Loading,
    Failed,
    Ready { size: Vec2, summary: String, animated: Option<usize>, has_alpha: bool },
}

pub struct Facts {
    pub artwork: ArtworkState,
    pub background_loaded: bool,
    pub avatar_loaded: bool,
}

pub fn run(project: &Project, layout: &LayoutSpec, _cfg: &SteamConfig, facts: &Facts) -> Vec<Check> {
    let mut out = Vec::new();
    let h = project.art_height;

    out.push(Check::new(
        Severity::Ok,
        layout.name.clone(),
        format!(
            "{} {}: {} wide, {} px tall.",
            layout.regions.len(),
            if layout.regions.len() == 1 { "piece" } else { "pieces" },
            layout.widths_summary(),
            h as u32
        ),
    ));

    if h < layout.min_height || h > layout.max_height || h.fract() != 0.0 {
        out.push(
            Check::new(
                Severity::Warning,
                "Showcase height out of range",
                format!(
                    "This layout supports {}–{} px. Very tall artwork can be slow to load on Steam.",
                    layout.min_height as u32, layout.max_height as u32
                ),
            )
            .fix(QuickFix::ClampHeight),
        );
    }

    match &facts.artwork {
        ArtworkState::Missing => out.push(
            Check::new(Severity::Error, "No artwork yet", "Import an image to start designing.")
                .fix(QuickFix::ImportArtwork),
        ),
        ArtworkState::Loading => out.push(Check::new(Severity::Info, "Loading artwork…", "This takes a moment for large images.")),
        ArtworkState::Failed => out.push(
            Check::new(Severity::Error, "Artwork couldn't be loaded", "The file may have moved or be damaged.")
                .fix(QuickFix::ImportArtwork),
        ),
        ArtworkState::Ready { size, summary, animated, has_alpha } => {
            out.push(Check::new(Severity::Ok, "Artwork ready", summary.clone()));
            let art = project.artwork.as_ref().expect("ready artwork implies a layer");

            let uncovered: Vec<&str> = layout
                .region_rects(h)
                .filter(|(_, rect)| !covers(&art.transform, *size, &art.crop, *rect))
                .map(|(r, _)| r.label.as_str())
                .collect();
            if uncovered.is_empty() {
                out.push(Check::new(Severity::Ok, "Every piece is fully covered", "No empty edges will show on your profile."));
            } else {
                let fill = if project.export.fill_transparent { "filled with your canvas colour" } else { "left transparent" };
                out.push(
                    Check::new(
                        Severity::Warning,
                        format!("{} {} empty areas", join_names(&uncovered), if uncovered.len() == 1 { "has" } else { "have" }),
                        format!("Part of the layout isn't covered by your image. It will be {fill} on export."),
                    )
                    .fix(QuickFix::CoverRegions),
                );
            }

            let scale = art.transform.scale;
            let factor = project.export.scale.resolve(Some(scale));
            if scale > 1.0 {
                out.push(Check::new(
                    Severity::Warning,
                    format!("Artwork is enlarged to {:.0}%", scale * 100.0),
                    "Your image is smaller than the space it fills, so it will look soft. Use a larger image or scale it down.",
                ));
            } else if scale * factor as f32 > 1.05 {
                out.push(
                    Check::new(
                        Severity::Info,
                        format!("{factor}× export will upscale slightly"),
                        format!(
                            "Your image supplies about {:.1}× detail. Exporting higher won't add sharpness.",
                            1.0 / scale
                        ),
                    )
                    .fix(QuickFix::UseOneX),
                );
            } else {
                out.push(Check::new(
                    Severity::Ok,
                    format!("Full detail at {factor}× export"),
                    "Pieces stay crisp on high-DPI screens.",
                ));
            }

            if let Some(frames) = animated {
                out.push(Check::new(
                    Severity::Warning,
                    "Animated image: first frame only",
                    format!("Your file has {frames} frames. This version exports still images; animation support is planned."),
                ));
            }
            if *has_alpha {
                let what = if project.export.fill_transparent {
                    "Transparent areas will be filled with your canvas colour."
                } else {
                    "Transparent areas will show the profile behind them."
                };
                out.push(Check::new(Severity::Info, "Artwork has transparency", what));
            }
        }
    }

    if facts.background_loaded {
        out.push(Check::new(
            Severity::Info,
            "Background is for preview only",
            "Steam backgrounds come from the Points Shop or Market and can't be uploaded. Use the one you own here to line up your art.",
        ));
    }
    if !facts.avatar_loaded {
        out.push(Check::new(Severity::Info, "No avatar", "The preview uses a placeholder. Avatars are optional."));
    }
    if project.export.folder.is_none() {
        out.push(
            Check::new(Severity::Info, "No export folder yet", "You'll be asked where to save when you export.")
                .fix(QuickFix::ChooseExportFolder),
        );
    }
    out
}

fn join_names(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {}", rest.join(", "), last.to_lowercase()),
    }
}
