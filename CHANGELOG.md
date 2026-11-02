# Changelog

## Unreleased

- Added persistent multi-file selection/drop queues, pending removal, per-item
  status/errors, sequential native-job completion tracking, overall progress, and
  Clear Queue. Preserved existing encoder parameters and guarded updater handoff.

### v1.1.0 — Simple and Advanced Modes

- Updated the shared batch/preview profile to Main Level 3.1, 8-bit yuv420p, VBV 3200k/6000k, and AAC-LC 160k stereo at 48 kHz.
- Recommended 720x480 anamorphic widescreen, replaced the exposed 854 preset with 848x480, added WiiMC helper badges, and rounded custom sizes to even numbers.
- Added optional bounded black-bar crop detection, safe uncropped fallback, and encoder overrides with CPU fallback.
- Added a persistent header mode switch and a three-step Simple workflow.
- Added CRT, widescreen, and fastest presets using the existing controls and encoding pipeline.
- Added collapsible advanced settings, plain-language tooltips, and a diagnostic-display toggle.
- Preserved native command signatures, existing IPC hooks, file handlers, progress tracking, pause/resume, and theme switching.
- Returning to Simple resets pixel/quality/CPU overrides while preserving paths and output naming.

- Added persistent Auto/4:3/16:9 display aspect and SD resolution controls, SAR-aware scaling, custom even dimensions, geometry-aware output verification/reuse, and conservative frame-rate budgeting.

- Prepared signed background startup checks/downloads with an update-ready restart notice, passive NSIS installation, idle-job guards, and silent failure handling; maintainer key/feed setup is required.

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

The current project version is 1.1.0. See the [user guide](docs/USER_GUIDE.md) for usage and recovery details.
