# Source artwork

- `installer.png`: original installer artwork, moved here unchanged.
- `Icon.png`: original application artwork, also used by the frontend favicon.
- `installer-transparent.png`: preserved thumbs-up Keroro artwork from the
  previous app-icon bundle; setup uses the matching `installer.ico`.

Build-ready ICO and BMP files remain in `src-tauri/icons/`. The NSIS configuration
uses `icons/installer.ico` for the Keroro setup icon. The application icon bundle
is generated from the red triangle in `Icon.png`.
The NSIS configuration
references `icons/uninstaller.ico` through `uninstallerIcon`; the template maps
Tauri's resolved path into `MUI_UNICON` so compilation does not depend on the
generated NSIS script's working directory.

The original `uninstaller.png` was not present when the artwork was organized.
Keep that original here if it is recovered; `uninstaller.ico` remains available
for builds.
