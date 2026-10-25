use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::model::Project;
use crate::errors::UserError;

pub const EXTENSION: &str = "lss";
const APP_TAG: &str = "luminar-steam-studio";
const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct ProjectFile {
    app: String,
    format: u32,
    project: Project,
}

/// Serialises with asset paths made relative to `base` where possible, so a project
/// folder can be moved or shared together with its images.
pub fn to_json(project: &Project, base: Option<&Path>) -> Result<String, UserError> {
    let mut p = project.clone();
    if let Some(base) = base {
        map_paths(&mut p, |path| path.strip_prefix(base).map(Path::to_path_buf).unwrap_or_else(|_| path.clone()));
    }
    let file = ProjectFile { app: APP_TAG.into(), format: FORMAT_VERSION, project: p };
    serde_json::to_string_pretty(&file)
        .map_err(|e| UserError::new("Couldn't prepare the project file", format!("Unexpected data problem: {e}")))
}

/// Copy meant to travel with exports: file names only, no export folder, so a shared
/// ZIP never reveals the user's name or folder layout. Reopening finds images placed
/// next to the project file.
pub fn to_shareable_json(project: &Project) -> Result<String, UserError> {
    let mut p = project.clone();
    map_paths(&mut p, |path| path.file_name().map(PathBuf::from).unwrap_or_default());
    p.export.folder = None;
    to_json(&p, None)
}

pub fn save(project: &Project, path: &Path) -> Result<(), UserError> {
    let json = to_json(project, path.parent())?;
    // Write beside the target first so a crash mid-write can't corrupt an existing project.
    let tmp = path.with_extension("lss.tmp");
    std::fs::write(&tmp, json).map_err(|e| UserError::from_io("save", path, &e))?;
    std::fs::rename(&tmp, path).map_err(|e| UserError::from_io("save", path, &e))
}

pub fn load(path: &Path) -> Result<Project, UserError> {
    let text = std::fs::read_to_string(path).map_err(|e| UserError::from_io("open", path, &e))?;
    let file: ProjectFile = serde_json::from_str(&text).map_err(|_| {
        UserError::new(
            "This isn't a Luminar Steam Studio project",
            "The file couldn't be read as a project. It may be damaged or from a different program.",
        )
        .hint("Project files end in .lss and are created with File > Save.")
    })?;
    if file.app != APP_TAG {
        return Err(UserError::new(
            "This isn't a Luminar Steam Studio project",
            "The file was made by a different program.",
        ));
    }
    if file.format > FORMAT_VERSION {
        return Err(UserError::new(
            "This project needs a newer version",
            "It was saved by a newer Luminar Steam Studio than the one you're running.",
        )
        .hint("Update the app, then open the project again."));
    }
    let mut project = file.project;
    if let Some(base) = path.parent() {
        let base = base.to_path_buf();
        map_paths(&mut project, |p| relink(p, &base));
    }
    Ok(project)
}

/// Resolves relative paths, and if an absolute path went missing, looks for a
/// file with the same name next to the project.
fn relink(path: &PathBuf, base: &Path) -> PathBuf {
    if path.is_relative() {
        return base.join(path);
    }
    if !path.exists() {
        if let Some(name) = path.file_name() {
            let candidate = base.join(name);
            if candidate.exists() {
                return candidate;
            }
        }
    }
    path.clone()
}

fn map_paths(p: &mut Project, f: impl Fn(&PathBuf) -> PathBuf) {
    if let Some(a) = &mut p.artwork {
        a.source = f(&a.source);
    }
    if let Some(b) = &mut p.background {
        b.source = f(&b.source);
    }
    if let Some(a) = &mut p.avatar {
        a.source = f(&a.source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::ArtworkLayer;

    #[test]
    fn round_trip_keeps_relative_assets() {
        let dir = std::env::temp_dir().join("lss_roundtrip_test");
        std::fs::create_dir_all(&dir).unwrap();
        let mut p = Project::default();
        p.name = "Round trip".into();
        p.artwork = Some(ArtworkLayer {
            source: dir.join("art.png"),
            transform: Default::default(),
            crop: Default::default(),
        });
        let file = dir.join("test.lss");
        save(&p, &file).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"art.png\""), "asset path should be stored relative");
        let loaded = load(&file).unwrap();
        assert_eq!(loaded, p);
    }

    #[test]
    fn shareable_copy_has_no_local_paths() {
        let mut p = Project::default();
        p.artwork = Some(ArtworkLayer {
            source: PathBuf::from("/home/someone/private/art.png"),
            transform: Default::default(),
            crop: Default::default(),
        });
        p.export.folder = Some(PathBuf::from("/home/someone/exports"));
        let json = to_shareable_json(&p).unwrap();
        assert!(!json.contains("someone"), "{json}");
        assert!(json.contains("\"art.png\""));
    }
}
