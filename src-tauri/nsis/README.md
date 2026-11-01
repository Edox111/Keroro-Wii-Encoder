# OLED installer theme

`installer.nsi` is the upstream installer template embedded in Tauri CLI 2.12.4,
with page-show callbacks added for the OLED palette. It retains the upstream
Handlebars variables, elevation, WebView2 installation, upgrade handling,
resources, and uninstall behavior. Its upstream license is Apache-2.0 OR MIT.

`oled-theme.nsh` supplies black backgrounds, white text, and recursive control
coloring for installer and uninstaller pages, including the reinstall dialog.
Windows owns the UAC prompt and native message boxes; they retain OS styling.
A dark native title bar is requested when Windows supports it.

The template enables `XPStyle on`, uses Segoe UI at 10 points via `SetFont`,
and blanks the footer with `BrandingText " "`. Native visual styles are retained
on the frame and navigation buttons. Checkboxes and radio buttons alone disable
themed text rendering so their white labels remain readable on the black pages.
The finish page explicitly colors its title, body, launch checkbox, and desktop
shortcut checkbox. The uninstall app-data checkbox receives the same treatment.
The uninstall option reads **Delete user preferences and app data** and follows
the measured install-path field with a 12-pixel gap at 96 DPI, scaled with the
display DPI. It remains unchecked by default and keeps its existing deletion
behavior.
The plain-text license has a dedicated show callback that sets its RichEdit
foreground/background to white/black. It also applies `EM_SETCHARFORMAT` to
all license text, clearing RichEdit's automatic text color after loading.

Tauri configuration uses `headerImage` and `sidebarImage`. `installerHeader`
is not a supported Tauri 2 option. The artwork files are preserved as supplied.
The custom uninstaller uses `uninstallerIcon: "icons/uninstaller.ico"`, which
Tauri resolves into the template's existing `MUI_UNICON` definition. Loose source
PNGs live in the root `assets/` directory; build-ready icons and bitmaps stay in
`src-tauri/icons/`.
Setup uses the thumbs-up Keroro icon through `installerIcon: "icons/installer.ico"`.
The app/window/tray icon bundle uses the red-triangle artwork in `assets/Icon.png`.

When updating the Tauri CLI, compare this template with the new upstream
template and reapply the OLED callbacks before building an installer. Keep the
configured release cleanup hook enabled; build with `npm run build:windows`.

Validation: configuration keys, BMP headers, asset paths, and callback coverage
were checked. A minimal wizard compilation was attempted, but the installed
NSIS compiler could not start because Windows reported Illegal System DLL
Relocation. A compiled wizard and its appearance still require verification
on a Windows environment where that compiler runs.
