# Building Luminar Steam Studio for Windows

Luminar Steam Studio is a native Rust application (egui/eframe 0.29, OpenGL renderer) built with
Cargo. There is no Node, Electron or Tauri layer. The release build is a single `.exe` with
everything embedded: the Steam config, fonts, and (once added) the icon.

## One-time setup

- Rust with the **MSVC** toolchain (`rustup default stable-x86_64-pc-windows-msvc`).
- Visual Studio 2022 Build Tools with **Desktop development with C++**. This provides the linker
  and `rc.exe`, which embeds the icon.
- For the installer only: **Inno Setup 6.3+** (`winget install JRSoftware.InnoSetup`).

## Build the .exe

Run from the project folder in Command Prompt:

```bat
rustup show
cargo build --release
```

Output: `target\release\luminar-steam-studio.exe`. That one file is the whole app.

What the build configuration does:

| File | Purpose |
| --- | --- |
| `.cargo/config.toml` | Statically links Microsoft's C runtime (`+crt-static`), so users don't need the Visual C++ Redistributable. |
| `build.rs` | On Windows, embeds `assets/icon.ico` and version details (name, version, publisher). Name, publisher and creator come from `src/about.rs`; the version comes from `Cargo.toml`. |
| `Cargo.toml` → `[target.'cfg(windows)'.build-dependencies]` | `winresource`, used only at build time and only on Windows. |
| `src/main.rs` | `windows_subsystem = "windows"` in release builds, so no console window appears. |

## Check the .exe

1. **Runs outside the project:** copy the `.exe` to another folder (for example
   `Desktop\LSS-test`) and double-click it. Nothing else needs to come with it.
2. **No runtime dependencies:** in *x64 Native Tools Command Prompt for VS 2022*:
   ```bat
   dumpbin /dependents target\release\luminar-steam-studio.exe
   ```
   The list should contain only Windows system DLLs (such as `KERNEL32.dll`, `USER32.dll`,
   `GDI32.dll`, `OPENGL32.dll`). It must **not** list `VCRUNTIME140.dll` or `api-ms-win-crt-*`.
3. **Version details:** right-click the `.exe` → Properties → Details shows Luminar Steam
   Studio, the version and LuminarCore.
4. **On another PC** (the real test): copy the `.exe` over and run it.

## What other PCs need

- Windows 10 or 11, 64-bit.
- A graphics driver with **OpenGL 2.0 or newer**. Practically every PC with its normal graphics
  driver has this. Virtual machines without 3D acceleration, some Remote Desktop sessions and
  Windows Sandbox may only offer OpenGL 1.1, and the app won't open there. If that matters,
  eframe can use its Direct3D-based `wgpu` renderer instead (a change to `Cargo.toml` features).
- **No** Rust, Visual Studio, Visual C++ Redistributable or other runtime is needed.

Expect these on first launch:

- **SmartScreen** shows "Windows protected your PC" for new, unsigned programs. Users click
  *More info → Run anyway*. Signing the `.exe` and installer with a code-signing certificate
  removes this over time.
- A few antivirus products are suspicious of brand-new unsigned executables. Signing helps here
  too.

## Build the installer (optional, recommended for sharing)

`installer/luminar-steam-studio.iss` creates a standard installer:

- Installs per-user without admin rights by default; users can choose an all-users install.
- Adds a Start Menu shortcut and an optional desktop shortcut (ticked by default).
- Registers an uninstaller in *Settings → Apps*.
- Reads the name, version and publisher from the built `.exe`, so nothing needs editing per release.

```bat
cargo build --release
"%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe" installer\luminar-steam-studio.iss
```

If Inno Setup was installed per-user with winget, `ISCC.exe` is in
`%LOCALAPPDATA%\Programs\Inno Setup 6\` instead.

Output: `dist\LuminarSteamStudio-<version>-Setup.exe`.

Uninstalling leaves the user's settings in `%APPDATA%\luminar-steam-studio\`, which is the
normal Windows behaviour for user data.

## Releasing a new version

1. Change `version` in `Cargo.toml`. The app, the `.exe` details and the installer name all
   follow it.
2. `cargo build --release`, then compile the installer.
3. Keep the `AppId` in the `.iss` file unchanged, so the new installer updates existing installs.

## Build record

| Date | Step | Result |
| --- | --- | --- |
| Earlier | First `cargo run --release` on Windows | **Failed:** `error calling dlltool 'dlltool.exe': program not found`. Rust was on the GNU toolchain without MinGW. **Fixed** by installing VS Build Tools and switching to `stable-x86_64-pc-windows-msvc`. The app then ran. |
| This change | Added `.cargo/config.toml`, `build.rs`, the `winresource` build dependency and the installer script | Verified off-Windows: project builds, 10/10 tests pass, `build.rs` reads `APP_NAME`, `STUDIO` and `CREATOR` correctly, and `winresource` 0.1.34's API matches the calls used. **Not yet verified on Windows:** release build, embedded resources, installer compile. |
| This change | Application icon | Created `assets/icon.ico` with 10 sizes (16–256 px) from the provided `corven_editor_icon_blue_minimal_256.png`, resized only (the supplied `.ico` held a single 32×32 image). The 256 px frame is pixel-identical to the PNG. `build.rs` and the installer pick it up automatically. **Not yet changed:** the runtime window/taskbar icon and the in-app logo still use the drawn ring. |
