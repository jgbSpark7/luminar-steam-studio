//! Windows only: embeds the application icon and version details into the .exe.
//! Name and creator come from src/about.rs and the version from Cargo.toml, so
//! there is still a single place to edit each of them.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/about.rs");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows_resources();
    }
}

/// Reads `pub const NAME: &str = "value";` from src/about.rs.
fn about_const(name: &str) -> String {
    let source = std::fs::read_to_string("src/about.rs").expect("src/about.rs is readable");
    let prefix = format!("pub const {name}: &str = \"");
    source
        .lines()
        .find_map(|l| l.trim().strip_prefix(prefix.as_str()))
        .and_then(|rest| rest.split('"').next())
        .map(str::to_owned)
        .unwrap_or_else(|| panic!("{name} not found in src/about.rs"))
}

#[cfg(windows)]
fn windows_resources() {
    let app_name = about_const("APP_NAME");
    let studio = about_const("STUDIO");
    let creator = about_const("CREATOR");

    let mut res = winresource::WindowsResource::new();
    // FileDescription is what Task Manager shows as the process name.
    res.set("ProductName", &app_name);
    res.set("FileDescription", &app_name);
    res.set("CompanyName", &studio);
    res.set("LegalCopyright", &format!("{creator} / {studio}"));
    res.set("OriginalFilename", "luminar-steam-studio.exe");

    let icon = std::path::Path::new("assets/icon.ico");
    if icon.exists() {
        res.set_icon(&icon.to_string_lossy());
    } else {
        println!("cargo:warning=assets/icon.ico not found: the .exe will be built without an icon.");
    }
    if let Err(e) = res.compile() {
        panic!(
            "Couldn't embed Windows resources ({e}). The Windows SDK resource compiler (rc.exe) \
             comes with Visual Studio Build Tools > Desktop development with C++."
        );
    }
}

#[cfg(not(windows))]
fn windows_resources() {
    // Cross-compiling for Windows from another OS isn't set up; skip resources.
    let _ = about_const("APP_NAME");
    println!("cargo:warning=Windows resources (icon, version info) are only embedded when building on Windows.");
}
