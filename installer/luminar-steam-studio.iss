; Installer for Luminar Steam Studio. Requires Inno Setup 6.3 or newer.
; Build the app first (cargo build --release), then compile this script:
;   "%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe" installer\luminar-steam-studio.iss
; The installer is written to dist\.
;
; Name, version and publisher are read from the built .exe (embedded by build.rs
; from src/about.rs and Cargo.toml), so there is nothing to keep in sync here.

#define ExePath AddBackslash(SourcePath) + "..\target\release\luminar-steam-studio.exe"
#define IconPath AddBackslash(SourcePath) + "..\assets\icon.ico"

#if !FileExists(ExePath)
  #error "target\release\luminar-steam-studio.exe not found. Run: cargo build --release"
#endif

#define AppName GetStringFileInfo(ExePath, "ProductName")
#define AppVersion GetStringFileInfo(ExePath, "ProductVersion")
#define AppPublisher GetStringFileInfo(ExePath, "CompanyName")

#if AppName == "" || AppVersion == ""
  #error "The .exe has no version details. Build it on Windows so build.rs can embed them."
#endif

[Setup]
; Keep this AppId the same across versions so updates replace the old install.
AppId={{6F3C2A9E-4B1D-4E7A-9C55-1A2B3C4D5E6F}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL=https://github.com/jgbSpark7
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
; Installs for the current user without admin rights; users can choose all-users.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=LuminarSteamStudio-{#AppVersion}-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\luminar-steam-studio.exe
UninstallDisplayName={#AppName}
#if FileExists(IconPath)
SetupIconFile={#IconPath}
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#ExePath}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\luminar-steam-studio.exe"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\luminar-steam-studio.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\luminar-steam-studio.exe"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent
