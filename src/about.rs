//! Application identity and credits. Edit creator/studio details here; the version
//! comes from `Cargo.toml` so the app and the build can never disagree.

pub const APP_NAME: &str = "Luminar Steam Studio";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const TAGLINE: &str = "Free desktop application for designing and preparing custom Steam profile artwork.";

pub const CREATOR: &str = "JGB_SPARK7";
pub const STUDIO: &str = "LuminarCore";
pub const GITHUB_URL: &str = "https://github.com/jgbSpark7";

pub const PRICING: &str = "Free application";
pub const PRICING_DETAIL: &str =
    "Luminar Steam Studio is free to use. There are no subscriptions, paid features, ads or upgrade prompts.";

pub const TRADEMARK_NOTICE: &str =
    "Steam is a trademark of Valve Corporation. Luminar Steam Studio is an independent tool and is not affiliated with or endorsed by Valve.";

/// Plain statements about how the app treats user data. Keep these true:
/// update them if networking, accounts or telemetry are ever introduced.
pub const PRIVACY_FACTS: &[&str] = &[
    "Your artwork is processed and exported on this computer. It is never uploaded by the app.",
    "No account is needed, and the app never asks for your Steam password.",
    "No analytics, telemetry or tracking. The app makes no network requests of its own.",
    "Links open in your browser only when you click them.",
    "Settings and your recent-project list are stored locally. Clear them in Settings.",
];

pub struct Credit {
    pub name: &'static str,
    pub purpose: &'static str,
    pub license: &'static str,
    pub url: &'static str,
}

/// Direct dependencies, licences as declared in each crate's own metadata.
pub const LIBRARIES: &[Credit] = &[
    Credit { name: "egui / eframe", purpose: "User interface and window", license: "MIT OR Apache-2.0", url: "https://github.com/emilk/egui" },
    Credit { name: "image", purpose: "Image decoding and encoding", license: "MIT OR Apache-2.0", url: "https://github.com/image-rs/image" },
    Credit { name: "serde", purpose: "Project and settings data", license: "MIT OR Apache-2.0", url: "https://github.com/serde-rs/serde" },
    Credit { name: "serde_json", purpose: "Project and config files", license: "MIT OR Apache-2.0", url: "https://github.com/serde-rs/json" },
    Credit { name: "rfd", purpose: "Native file dialogs", license: "MIT", url: "https://github.com/PolyMeilex/rfd" },
    Credit { name: "zip", purpose: "ZIP export", license: "MIT", url: "https://github.com/zip-rs/zip2" },
    Credit { name: "open", purpose: "Opening folders and links", license: "MIT", url: "https://github.com/Byron/open-rs" },
];

/// Fonts bundled through egui's default font set (epaint_default_fonts).
pub const FONTS: &[Credit] = &[
    Credit { name: "Ubuntu Light", purpose: "Interface text", license: "Ubuntu Font Licence 1.0", url: "https://design.ubuntu.com/font" },
    Credit { name: "Hack", purpose: "Monospace text", license: "MIT and Bitstream Vera Licence", url: "https://github.com/source-foundry/Hack" },
    Credit { name: "Noto Emoji", purpose: "Emoji symbols", license: "SIL Open Font Licence 1.1", url: "https://github.com/googlefonts/noto-emoji" },
    Credit { name: "emoji-icon-font", purpose: "Interface symbols", license: "MIT", url: "https://github.com/jslegers/emoji-icon-font" },
];

pub const TRANSITIVE_NOTE: &str = "These libraries rely on further open-source packages, almost all under permissive licences such as MIT, Apache-2.0, BSD, Zlib and Unicode-3.0. All copyrights belong to their respective authors.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_https() {
        assert!(GITHUB_URL.starts_with("https://"));
        for c in LIBRARIES.iter().chain(FONTS) {
            assert!(c.url.starts_with("https://"), "{} must use https", c.name);
        }
    }

    #[test]
    fn version_matches_cargo() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
        assert!(!VERSION.is_empty());
    }
}
