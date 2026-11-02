# Runtime architecture

The desktop bootstrap in `src-tauri/src/lib.rs` registers plugins, managed state,
commands, startup logging, and the tray. The implementation is split into:

| Module | Responsibility |
| --- | --- |
| `state.rs` | Exclusive job reservations, snapshots, preview ownership, and close preferences |
| `commands.rs` | Tauri IPC validation and batch/preview/pause/recovery adapters |
| `execution.rs` | Async task supervision, sidecar resolution, preview cleanup, and progress delivery |
| `lifecycle.rs` | Window/tray behavior, exit confirmation, and waiting for child reaping |
| `encoder-core/src/process_state.rs` | Child ownership, OS suspension, bounded stop, and panic/abort reaping |
| `encoder-core/src/profile.rs` | Shared Wii-oriented video/audio encoding arguments |
| `encoder-core/src/records.rs` | Bounded local reuse records and performance measurements |
| `encoder-core/src/validation.rs` | Post-encode metadata checks and FAT32 warnings |
| `encoder-core/src/hardware.rs` | Encoder enumeration, bounded driver probes, and hardware preference |

Queues contain file paths; they do not pre-spawn media processes. Only one
FFmpeg or ffprobe child occupies the managed slot at a time. Mutex guards stay
out of async waits. Directory operations and pipe reads are asynchronous;
pairing, checksum, and checkpoint work use blocking workers. Cancellation stops
the current child and prevents subsequent queue items from spawning. Exit waits
for both worker cleanup and the process reaper. A paused encoder is resumed
before graceful shutdown; bounded termination and the Windows job object remain
fallbacks for stalled or interrupted workers.

The common preview/batch profile uses the selected SD raster and SAR-aware display
aspect handling, 8-bit 4:2:0 H.264 Main Level 3.1, a calculated frame-rate
ceiling (30 fps maximum; 25 fps for 848x480), a 3.2 Mbps
VBV video rate limit with a 6 Mb buffer, and AAC-LC stereo targeting 160 kbps at
48 kHz. CPU CRF remains selectable within those limits. Hardware rate-control settings use the corresponding quality/bitrate mapping; see [hardware encoding](HARDWARE_ENCODING.md). MP4 fast-start metadata
and explicit MP4 muxing support `.mp4.part` output before atomic finalization.
Lower source frame rates are retained. Wii player compatibility still depends
on the player and content; this is not a hardware playback certification.

UI audio stays in the existing frontend manager so it honors UI sounds and
volume settings without an additional playback dependency or duplicate IPC.
Clicks use the delegated interaction handler. Progress snapshots trigger one
error cue when a new file failure arrives, rather than waiting for batch end;
repeated progress and the final error modal do not replay that cue. Successful
batches play the existing completion chime. Restored snapshots remain silent.

Real-media checks cover profile metadata, 60-to-30 fps conversion, preservation
of 24 fps sources, audio levels/timing, subtitle burn-in, corrupt input skipping,
process suspension with zero CPU-time advance, and paused-child shutdown.

Settings now include a backward-compatible CPU-usage mode (old checkpoints
restore Maximum). The progress payload optionally carries the selected encoder
and measurements. A read-only `get_performance_report` command restores the last
recorded batch measurement; missing or invalid optional reports do not prevent
startup. Probe capture is bounded, cancellable, and shared by encoder detection,
source inspection, and output validation.

## Windows sidecar packaging

The configured application version is `1.1.0`. `bundle.externalBin` includes both
`binaries/ffmpeg` and `binaries/ffprobe`; source executables carry the
`-x86_64-pc-windows-msvc.exe` target suffix. NSIS installs them as `ffmpeg.exe`
and `ffprobe.exe` beside the main executable. The custom OLED template retains
Tauri's external-binary install/uninstall loops.

Production tool lookup uses `app.path().resource_dir()` (the executable directory
on Windows), validates both files, and passes their explicit paths to owned Tokio
processes. Development uses the target-suffixed files under `src-tauri/binaries`.
There is no PATH fallback. This preserves OS pause/resume, stdin cancellation,
process ownership, and child reaping. Tauri 1's `api::process::Command::new_sidecar`
is unavailable in Tauri 2; switching to the shell plugin's child wrapper would
require a separate lifecycle integration and is unnecessary for bundled execution.

`bundle.useLocalToolsDir` caches NSIS build tools under `src-tauri/target/.tauri`
instead of the user AppData cache. It does not alter installation paths or OLED
artwork. Build the final installer with `npm run build:release`; successful source
checks do not substitute for running and testing that installer.
