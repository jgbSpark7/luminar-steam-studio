# Release and safety checklist

A working checklist for shipping Luminar Steam Studio. It records what was reviewed, what's
true of the app today, and what to revisit before or after a public release.

> **This is not legal advice.** Ticking these items does not make the app legally compliant and
> does not protect against claims. Laws differ by country and change. Anything marked
> **Legal review** should be checked by a qualified professional before relying on it.

Each item is tagged with when it applies:

| Tag | Meaning |
| --- | --- |
| **Now** | Applies to the current free, offline desktop app. |
| **Future** | Only applies if online, account, payment, email or commercial features are added. |
| **Legal review** | Needs a qualified professional, whatever the app's state. |

## Current privacy and security posture (v0.1.0)

Reviewed against the source and the full dependency tree (`cargo tree`, `cargo metadata`).

- **Networking:** none. No HTTP client is compiled into the desktop build. `tokio` appears in the
  lock file only through `wasm-bindgen-futures`, which is web-target only (`cargo tree -i tokio`
  prints nothing for the desktop target).
- **Outbound actions:** the app opens links in the default browser, and folders in the file
  manager, only when the user clicks. This goes through `Action::OpenUrl` and `Action::OpenPath`
  (the `open` crate), plus egui's own link handling (`webbrowser`). `OpenUrl` refuses anything
  that isn't `https://`, because URLs can come from the user-editable `steam_config.json`.
- **Accounts and credentials:** none. The app never asks for, sees or stores Steam credentials.
  Uploads happen on Steam's own website, signed in by the user.
- **Telemetry, analytics, ads, crash reporting:** none.
- **Artwork:** decoded, edited and exported entirely on the user's machine.
- **Data stored locally** by eframe's persistence in `app.ron` (window state plus `Settings`:
  recent project paths, last-used folders, UI scale, snapping and guide toggles):
  - Windows: `%APPDATA%\luminar-steam-studio\data\`
  - Linux: `~/.local/share/luminar-steam-studio/`
  - macOS: `~/Library/Application Support/luminar-steam-studio/`

  Settings → Privacy → *Clear recent projects and folders* removes the path history.
- **Files read:** images the user picks or drops, `.lss` projects, and the optional
  `steam_config.json` next to the executable.
- **Files written:** projects the user saves; exports into the folder the user picks; settings.
- **Project files are data only.** `.lss` is JSON parsed by `serde` into fixed structs. Nothing in
  it is executed, and its image paths are only ever decoded as images.
- **Exported `Project.lss` is path-free.** It stores image file names only and drops the export
  folder, so a shared ZIP doesn't reveal usernames or folder layouts
  (`serialization::to_shareable_json`).
- **Export names are constrained.** The export folder name is sanitised (letters, digits, space,
  `-`, `_`). Piece file names from `steam_config.json` are rejected unless they are plain names,
  so a tampered config can't write outside the export folder.
- **Image decoding** uses the `image` crate's default allocation limits, so oversized images fail
  with a readable error instead of exhausting memory.
- **Console snippet:** the upload guide shows the snippet's full text before offering to copy it.
  It is a community workaround from `steam_config.json`. Anyone editing that file should keep it
  to the documented one-liner.

## The 20-item checklist

Status key: **Done** = reviewed and true today. **To do** = action needed. **N/A today** = doesn't
apply until the app changes.

| # | Item | Applies | Status | Notes |
| -: | --- | --- | --- | --- |
| 1 | Privacy policy | Now, Legal review | To do | The app collects nothing, but download sites and stores often require a privacy statement. The Info tab's privacy section (`about::PRIVACY_FACTS`) is an accurate starting point. A full policy becomes essential if any data is ever collected. |
| 2 | Terms of service | Future, Legal review | N/A today | A free offline tool mainly needs a **software licence**. None is set: `Cargo.toml` has no `license` field. The owner should choose one (open source or a free-to-use EULA) before distributing. ToS matters once there are accounts or online services. |
| 3 | Refund policy | Future | N/A today | Free, with no purchases. Required if paid features or donations-with-rewards are introduced. |
| 4 | Cookie policy | Future | N/A today | Native app: no web pages, no cookies. Applies to any future website or web version. |
| 5 | Cookie consent banner | Future, Legal review | N/A today | Same as item 4. Note that non-essential cookies often need prior consent. |
| 6 | Check form consents | Future | N/A today | The app has no forms that collect data. Steam's upload form is Steam's, filled in by the user in their browser. Revisit for feedback forms, newsletters or sign-ups. |
| 7 | No unnecessary data | Now | Done | Only local settings and recent paths are stored, and both can be cleared. The exported project file is path-free. Re-check whenever a new stored field is added. |
| 8 | Audit third-party SDKs | Now | Done for v0.1.0 | 7 direct dependencies, no analytics, ad, crash-reporting or network SDKs. Re-run before each release: `cargo tree --target all`, and ideally `cargo deny check` for licences and advisories. |
| 9 | Remove dark patterns | Now | Done | No upsells, nags, confirm-shaming or pre-ticked opt-ins. The unsaved-changes dialog offers *Save / Don't save / Cancel* in plain words. Keep it that way. |
| 10 | Remove hidden fees | Now | Done | The app is free with nothing to buy. The Info tab states it plainly. |
| 11 | Remove fake reviews | Now, Future | Done in app | The app shows no reviews or ratings. The preview's group names and member counts are clearly placeholder mock-up content. Applies to store pages and any website. |
| 12 | Remove unsupported claims | Now | Done, keep reviewing | Alignment claims are backed by tests (`pieces_align_across_the_gap`) and a pixel measurement of a real export. Estimates are labelled as estimates: the 4 px gap is "not documented by Steam", the preview is "styling is approximate". The Valve non-affiliation notice is in the Info tab. Check store and website copy against this. |
| 13 | Accessibility / alt text | Now | Partly done | eframe's default AccessKit integration exposes widgets to screen readers. Sidebar icons always have text labels, and link text is the destination address, not a repeated "Website". The editing canvas is inherently visual. **To do:** test with Narrator or NVDA on Windows. |
| 14 | Colour contrast | Now | Done for main surfaces | Measured against WCAG AA (4.5:1 for normal text) on the three main backgrounds (`VOID`, `PANEL`, `RAISED`): `TEXT` 9.4–13.0, `MUTED` 6.4–8.9, `FAINT` 4.7–6.5, `LUMEN` 4.7–6.5, status colours 4.6–7.9, primary button text 6.8, selected-row text 4.6. **Remaining:** on the `HOVER` highlight, `FAINT`, `LUMEN` and `ERR` drop to 3.8–3.9. That only affects secondary text on a row while the pointer is over it, but re-measure whenever `ui/theme.rs` changes. |
| 15 | Keyboard navigation | Now | Partly done | Global shortcuts (Help → Keyboard shortcuts), Tab or Shift+Tab focus through widgets, Enter or Space to activate, Esc to dismiss the unsaved-changes and error dialogs. **Limitation:** canvas arrow-key nudging needs the pointer over the canvas. **To do:** give the canvas keyboard focus, and do a full keyboard-only pass on Windows. |
| 16 | Business details if commercial | Future, Legal review | N/A today | Free and non-commercial today. Many jurisdictions require trader or contact details once sales, ads or sponsorship are involved. |
| 17 | Age / children's data | Now (low), Future, Legal review | Done for today | No personal data is collected, so children's-data rules aren't triggered by the app itself. Steam has its own age requirements. Revisit with legal review before adding accounts, online features or analytics. |
| 18 | Email unsubscribe | Future | N/A today | No email. Any future mailing list needs consent and a one-click unsubscribe. |
| 19 | Licence fonts / images / assets | Now | Partly done | Credits are listed in the Info tab (`about::LIBRARIES`, `about::FONTS`). Fonts come from egui's bundled set: Ubuntu Light (UFL 1.0), Hack (MIT + Bitstream Vera), Noto Emoji (OFL 1.1), emoji-icon-font (MIT). The app icon is generated in code, templates are original colour presets, and `examples/demo` images were generated for this project. **To do:** MIT and Apache-2.0 require licence and copyright notices to ship with binaries. Generate `THIRD_PARTY_LICENSES` (for example with `cargo about generate`) and ship it plus the font licence texts with each release. |
| 20 | Data deletion requests | Future, Legal review | N/A today | Nothing is held off-device. Users delete their own data by clearing recents in Settings, deleting the settings folder above, or deleting their project and export files. A deletion process becomes necessary once any cloud or user data exists. |

## Also before a public release

- [ ] Choose and add a licence for Luminar Steam Studio itself (item 2).
- [ ] Generate and bundle third-party licence texts (item 19).
- [ ] Do a keyboard-only and screen-reader pass (items 13, 15). Re-measure contrast if the theme changes (item 14).
- [ ] Bump `version` in `Cargo.toml`. The Info tab and window read it automatically.
- [ ] Consider code-signing the Windows executable, so SmartScreen doesn't flag downloads.
- [ ] If anything in "Current posture" stops being true, update this file **and**
      `about::PRIVACY_FACTS` in the same change.
