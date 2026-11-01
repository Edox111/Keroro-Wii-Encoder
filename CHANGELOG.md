# Changelog

## Unreleased

- Preserve detected source episode numbers for single-file output names and previews; folder batches remain sequential.

- Fixed first-launch startup failure when the cleanup ledger's AppData directory does not exist yet.
- Fixed a pause/resume race that could replace newer progress with an older snapshot.
- Restored video-only fallback when selected subtitles become unreadable after queue creation.
- Prevented unchanged performance updates from interrupting report text selection.

- Added settings-aware reuse records that preserve conflicting existing outputs.
- Added pre-finalization output validation and FAT32 size warnings.
- Added persistent encoding measurements and an expandable performance report.
- Added Balanced/Maximum CPU modes with compatible preference/checkpoint defaults.
- Added a repeatable synthetic CPU/NVENC/subtitle benchmark command and report.

## Initial Definitive Release

- Windows x64 desktop application built with Tauri 2, Rust, and TypeScript.
- Batch conversion of common video formats to 640 × 480 H.264/AAC MP4 with aspect-ratio-preserving padding.
- Smart drag-and-drop routing for media, subtitle, and empty output folders.
- Subtitle burn-in with filename and episode-number matching, plus natural-order fallback.
- Per-file progress and ETA, Windows process-level pause/resume, `.part` outputs, existing-file skipping, and interrupted-batch recovery.
- Ten-second preview, sequential output naming, saved preferences, and five character themes.
- System tray operation, single-instance handling, local diagnostic logs, and bundled FFmpeg tools.
- NSIS setup configuration and a portable release extraction workflow.

The current project version is 1.0.0. See the [user guide](docs/USER_GUIDE.md) for usage and recovery details.
