//! Errors shown to people. Every failure the user can see goes through
//! [`UserError`] so raw library messages never reach the UI.

use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct UserError {
    pub title: String,
    pub detail: String,
    pub hint: Option<String>,
}

impl UserError {
    pub fn new(title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { title: title.into(), detail: detail.into(), hint: None }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// `verb` completes "Couldn't … <file>", e.g. "open", "save", "write".
    pub fn from_io(verb: &str, path: &Path, err: &std::io::Error) -> Self {
        use std::io::ErrorKind as K;
        let name = display_name(path);
        match err.kind() {
            K::NotFound => Self::new(
                format!("Couldn't find {name}"),
                "The file or folder may have been moved, renamed or deleted.",
            )
            .hint("Check the location, or choose it again."),
            K::PermissionDenied => Self::new(
                format!("Couldn't {verb} {name}"),
                "Your system denied access to this location.",
            )
            .hint("Choose a folder you own, such as Documents or Pictures, and close any program using the file."),
            K::StorageFull => Self::new(format!("Couldn't {verb} {name}"), "The disk is full.")
                .hint("Free up some space and try again."),
            _ => Self::new(
                format!("Couldn't {verb} {name}"),
                "Something went wrong while accessing the file.",
            )
            .hint(format!("Technical detail: {err}")),
        }
    }

    pub fn from_image(path: &Path, err: &image::ImageError) -> Self {
        use image::ImageError as E;
        let name = display_name(path);
        match err {
            E::IoError(e) => Self::from_io("open", path, e),
            E::Unsupported(_) => Self::new(
                format!("Couldn't open {name}"),
                "This image uses a format or feature that isn't supported.",
            )
            .hint("Try exporting it as PNG or JPG."),
            E::Limits(_) => Self::new(
                format!("{name} is too large to open"),
                "The image needs more memory than is safe to use.",
            )
            .hint("Resize it below about 12,000 px on its longest side and try again."),
            E::Encoding(_) => Self::new(format!("Couldn't write {name}"), "The image couldn't be encoded.")
                .hint("Try PNG instead of JPEG in the export settings."),
            _ => Self::new(
                format!("Couldn't open {name}"),
                "The file may be corrupted or use an unsupported format.",
            )
            .hint("Try exporting it as PNG or JPG."),
        }
    }
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.title, self.detail)
    }
}

pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
