# v1.3.0 — Hardware & Performance

This release extends the existing TypeScript/Vite interface. It does not introduce React or a second queue implementation.

## Hardware encoders

Open **Settings → Hardware Encoder** to select Auto, x264, NVIDIA NVENC, Intel QuickSync, or AMD AMF. Discovery uses the bundled FFmpeg, checks its encoder listing, and then attempts a real two-frame encode. A compiled-in codec alone is not marked Detected. Results are cached for the app session; each batch independently verifies its requested driver again.

Auto retains the existing NVENC → AMF → QSV → x264 priority. Failed hardware encoding is reaped, its partial output is removed, and the same item is retried once using x264. The remainder of that native batch uses x264. Existing final outputs remain protected.

| Encoder | Profile |
| --- | --- |
| x264 | `-preset medium -crf <selected quality>` |
| NVENC | `-preset p4 -rc vbr -cq <selected quality>` with a bounded bitrate target |
| QSV | `-preset medium -look_ahead 0` with bounded VBR |
| AMF | `-quality quality -rc vbr_peak -enforce_hrd 1` with a bounded bitrate target |

The existing 20/23/26 quality choices remain usable. QSV ICQ (`global_quality`) and AMF CQP are intentionally not used: those modes can bypass the bitrate control needed for the WiiMC profile. All encoder paths retain 8-bit 4:2:0, Main/Level 3.1, geometry-dependent frame-rate limits, 3200k maxrate, 6000k buffer, and AAC stereo 160k/48kHz. Subtitles and scaling stay in software before encoding.

Saving unrelated preferences preserves the Advanced mode encoder override. Changing the encoder default while a queue is active takes effect after that queue settles.

## SD / USB output

The picker beside the output path shows connected drives, filesystem, free capacity, and a usage meter. Selecting a drive creates only `videos/WiiMC` on that volume. The app checks the current volume identity and rejects links/junctions in this directory chain.

Automatic routing is opt-in in Settings. It selects a remembered connected drive or the only available candidate. Multiple unidentified drives require a selection. Targets cannot change during encoding or snapshot generation. Discovery refreshes on window focus and every ten seconds while visible. A disconnected selection is retained and labeled, rather than silently redirected to another drive.

Windows enumeration covers D:–Z:, including USB/SD devices that report themselves as fixed disks. Linux uses mount information and the device tree. macOS lists mounted `/Volumes` candidates. Only the Windows implementation has been compiled and exercised in this development environment; the macOS mount list can include disk images and secondary volumes, so users must choose the intended device.

New checkpoints retain the output volume identity. Resume rejects a replacement volume at the same path. Older checkpoints lacking that optional field remain readable.

Volume checks run before adopting existing outputs and before reserving temporary output files. A rejected resume leaves the saved job unchanged. Ten-second previews use the local cache volume independently of the selected SD/USB destination.

### FAT32 guard

The queue uses probed duration and the profile's video/audio cap, plus 2% container allowance, for a conservative estimate. It is an upper estimate, not a prediction of CRF output size. A **FAT32 4GB Warning** appears at 4,000,000,000 estimated bytes. The backend independently validates the filesystem and rejects such jobs before encoding. Shorten/split the source or choose another destination. There is no automatic splitting feature in this release.

On FAT32, FFmpeg also receives a 4,000,000,000-byte size guard. Output validation rejects incomplete duration or a file reaching that guard, and the existing `.part` rollback removes the unfinished output. Unknown-duration sources retain this runtime guard. The true FAT32 single-file ceiling is 4,294,967,295 bytes; the lower guard leaves room for container finalization. Other filesystems do not receive the FAT32 restriction.

## Frame snapshots

Each queue card offers **Preview Snapshot** while the encoder is idle. The modal has a timestamp slider and 1m/5m/10m jumps. Short clips clamp requests before their end. It uses the current crop, dimensions, aspect ratio, and selected embedded or explicitly paired external subtitles. Anamorphic frames are converted to square pixels for correct display in the image viewer.

Fast input seeking restores the subtitle timeline before libass runs, avoiding the common bug where a five-minute preview burns subtitles from time zero. Temporary JPEGs live in an isolated `tempfile` directory, are read into a bounded binary IPC response, and are removed after completion or ordinary failure. No WebView-wide filesystem asset permission is added. Closing a running snapshot requests cancellation; native process cleanup finishes before another task is admitted. A forced OS termination can leave an isolated temporary directory for normal OS temporary-file cleanup.

## Queue controls and diagnostics

Active cards have **Pause / Resume**, backed by the existing owned process handle and OS suspension implementation. New `pause_encoding` / `resume_encoding` adapters take the native job ID as `fileId`. The original `pause_encode` / `resume_encode` commands remain supported. A single sequential queue needs one process owner; a second PID/child registry would introduce conflicting ownership.

Failed cards display **Failed → View Error Log**. The modal shows the bounded stderr tail, provides clipboard copy with a manual-copy fallback, and lets an idle queue re-queue that item. Retry preserves track choices, re-probes the source, and waits for **Start Platoon Downscale**. Successful final files are never erased by retry. The serialized legacy queue status remains `Error`; `Failed` is its displayed label. `Paused` is an explicit live queue state.

Stderr retains the last 100 lines, capped at 2048 bytes per line. Failure strings carry the tail after `FFmpeg stderr:`; the queue separates the short message from the diagnostic log. The log is local and may contain media paths. Raw logs are not written into browser storage. Existing recovery and updater guards also cover snapshot tasks.

## Files and cleanup

| Area | Main files |
| --- | --- |
| GPU discovery | `src-tauri/src/gpu_probe.rs`, `encoder-core/src/hardware.rs` |
| Drive IPC and validation | `src-tauri/src/drive_target.rs`, `encoder-core/src/storage.rs`, `src/drive-selector.ts` |
| Snapshots | `src-tauri/src/preview.rs`, `encoder-core/src/snapshot.rs`, `src/preview-modal.ts` |
| Process controls | `src-tauri/src/process_control.rs`, existing `process_state.rs` / `state.rs` |
| Diagnostics and retry | `encoder-core/src/diagnostics.rs`, `src/file-queue.ts`, `src/error-log-modal.ts` |
| Preferences and selectors | `src/hardware.ts`, `src/encoder-selector.ts`, `src/settings-modal.ts` |

Cleanup is already applied. Do not manually delete the existing pause commands or the ten-second video preview. Removed/replaced code:

- `validation::projected_fat32_warning`: replaced by filesystem-aware preflight and `storage::estimated_bytes`.
- The duplicated synthetic hardware probe in `hardware::detect_requested`: both discovery and selection use `usable`.
- The 20-line stderr buffer: replaced by the bounded 100-line buffer.
- Hardcoded software `superfast`, QSV `fast`, and AMF's preset alias: replaced by the profiles above.

No installer is published by these changes. Physical FAT32 unplug/replug testing and playback on actual Wii hardware remain release checks; automated tests use boundary values, simulated drive routing, and real FFmpeg media fixtures.

## Validation

The Windows validation pass covers 96 encoder-core tests and 5 native application tests, including real FFmpeg snapshot timing, embedded/external subtitles, Unicode paths, corruption handling, cancellation, OS suspension/resume, and checkpoint volume identity. The synthetic benchmark is intentionally excluded from correctness tests.

The TypeScript/Vite production build, frontend regression scripts, and browser interaction checks also pass. Browser checks cover GPU preferences, deferred setting changes, FAT32 admission, drive disconnection, snapshots, per-item controls, diagnostic logs, retry, and the existing updater and Simple/Advanced layouts. Cargo Clippy is run for all targets in both crates with warnings denied; platform-specific filesystem cache warnings may still be emitted by the compiler.
