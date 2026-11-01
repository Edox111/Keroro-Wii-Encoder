# Keroro Wii Encoder 1.1.0

- Wave 5 source changes: local rotating logs and forced panic backtraces, owned
  artifact cleanup, private-output rollback, last-20-line stderr diagnostics,
  Windows child job fencing, and atomic frozen-queue recovery with output
  checksums and Resume/Later/Discard dialogs. See the Wave 5 guide for verification
  limits; the existing release binaries do not contain these changes.

- Wave 4 source changes: safe CPU-thread budgeting, native idle-sleep requests,
  integer progress-event throttling, zero-byte output canaries, and startup
  FFmpeg/ffprobe presence checks. Full build remains pending dependency fetching.

- New application and tray icon using the supplied red emblem.
- Coordinated character accent colors and glowing selected buttons across all five themes.
- Per-file FFmpeg progress, overall batch progress, elapsed time, FPS, speed, and result counts in the status panel.
- Original click and completion chimes with a mute preference and volume capped at 0.4.
- Persistent close-to-tray preference, duplicate-launch activation, F11 full screen, and integer batch-progress tray tooltips.
- Background encoding through a system tray icon, restore controls, and an exit option available when idle.
- Bundled FFmpeg and ffprobe; no separate PATH setup for the packaged app.
- Working CRF presets with fixed AAC stereo audio targeting 128 kbps at 44.1 kHz; volume boost removed.
- Preserved existing outputs, private temporary encoding files, safe finalization, per-item error summaries, and continued processing after an individual failure.
- Persistent theme selection, accessible progress indicators, and responsive 26×26 character badges.
- Default subtitle-folder synchronization with persistent manual override and SRT/ASS burn-in.
- Custom anime titles with sequential episode names and a live filename example.
- A 10-second encoded preview with a glass video player and cache cleanup.
- Active-task close confirmation with graceful FFmpeg shutdown, bounded forced termination, and partial-output cleanup.
- Case-insensitive MP4/MKV/AVI/MOV/WMV/FLV/WebM input scanning with uniform MP4 output.
- Wave 1 runtime safety: synchronous form locking, atomic IPC job reservations,
  shared child tracking, panic/abort reaping, and supervised worker failure events.
- Wave 2 file resilience: retained per-file warnings/errors, UTF-8/UTF-16 and
  Windows legacy subtitle decoding, missing/unreadable subtitle fallback,
  output write validation, and quota-aware minimum free-space checks.
- Wave 3 source changes: SRT/ASS/SSA/VTT, a 50 MB pre-read limit, encoding_rs,
  CSPRNG-prefixed isolated tempfile storage retained by the child lease,
  structural timing checks, and bounded stderr. Full Rust verification is
  pending the encoding_rs dependency download; see the Wave 3 guide.
- Character-specific MP3 lookup with click fallback and a single UI sounds checkbox.
- Automatic restoration of quality, folder paths, title, theme, mute, and subtitle-follow/override mode.

The package targets Windows x64 and requires WebView2. Native window and tray behavior should be verified using the installed application. Wii playback and installed Windows tray/window interactions still require manual validation.
