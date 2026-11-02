# Keroro Wii Encoder v1.1.0

## Progressive disclosure

- Simple Mode is the default: Input File, Pick Wii Preset, Start Encoding.
- A segmented header control remembers the selected Simple/Advanced Mode.
- Advanced Mode provides the original granular controls in a collapsible panel.
- CRT, widescreen, and fastest presets update the existing encoding fields.
- Returning to Simple restores safe preset values, preserving paths and naming.
- OLED cards, high-contrast borders, keyboard-accessible info tooltips, and a
  diagnostic-display preference keep the workflow readable.

## Compatibility

All original controls remain mounted with their original IDs. Tauri `invoke`
calls, Rust command signatures, file-selection/drop handlers, core encoding
state initializers, and progress handlers are preserved. Existing automatic GPU
selection, subtitles, geometry handling, output reuse, preview, pause/resume,
recovery, tray behavior, UI sounds, and automatic-update guards remain in place.

The native Browse button continues to select folders; individual videos can be
dropped or entered by path. Advanced Mode adds opt-in black-bar detection and
Auto/NVENC/AMF/QSV/CPU selection with tested CPU fallback. Widescreen now uses recommended 720x480 anamorphic 16:9 DAR.
The optional wide raster is 848x480; the shared output profile is Main Level 3.1,
yuv420p, VBV 3200k/6000k, and 48 kHz stereo AAC at 160k. Existing settings/checkpoints default the new fields safely.

## Verification

Frontend compilation and browser regressions cover mode/preset persistence,
reset behavior, path and DOM-node preservation, active-job locks, naming,
geometry, progress, pause/resume, themes, sounds, and responsive layouts. Rust
source fingerprints and frontend AST comparisons verify the protected runtime
contract for this UI change. No production installer has been rebuilt or
published as part of the UI refactor.
