# Encoding engine and frontend cleanup

This pass preserves the existing command names, event names, settings, checkpoint format, GPU fallback, and WiiMC output contract. The repository currently uses TypeScript/Vite and DOM components, not React; no frontend framework was added.

## Changes

| Area | Implementation | Effect |
| --- | --- | --- |
| Encoder orchestration | `src-tauri/crates/encoder-core/src/engine.rs` | Separates per-file encoding, partial-file cleanup, and finalization from batch scheduling. `Encoder` borrows shared settings and process state rather than passing nine arguments. |
| Pipe reading | `encoder-core/src/pipe_lines.rs` | Reuses an 8 KiB read buffer and bounded record storage. Handles LF, CR, CRLF, split reads, invalid UTF-8, and oversized lines without retaining an unbounded string. |
| Progress | `encoder-core/src/progress.rs` | Parses structured stdout records with a 256-byte line limit. Unknown metrics reset at each record instead of carrying stale values forward. |
| Diagnostics | `encoder-core/src/diagnostics.rs` | Retains the last 20 diagnostic records, each limited to 2,048 input bytes. stderr supplies a duration fallback and failure/benchmark diagnostics. |
| FFmpeg output | `engine.rs` | Uses `-progress pipe:1 -stats_period 0.25 -nostats`, removing duplicate human-readable progress generation and parsing. Existing integer progress/ETA and metric throttling remain. |
| IPC | `src/ipc.ts`, `src-tauri/src/execution.rs` | Central command argument/result types and a directly serialized Rust progress struct replace scattered return-type assertions and an intermediate JSON object. Wire names stay unchanged. |
| Queue ordering | `src/file-queue.ts` | Tracks the stats channel's own sequence, ignores duplicates/out-of-order stats, and prevents older full snapshots from overwriting newer metrics. Identical stats do not trigger another render. |
| Queue UI | `src/queue-view.ts` | Coalesces live progress into one paint per animation frame. Queue membership/status changes remain immediate. Selecting tracks preserves existing option nodes. |
| Preview reads | `src-tauri/src/commands.rs` | Opens the preview once and caps the read to 32 MiB plus one byte, including files that grow after the metadata check. |
| Cleanup | Rust and TypeScript sources | Removed redundant imports/state, simplified lint findings, deduplicated settings snapshots, and formatted Rust. |

## Encoding settings retained

Full Throttle already configures CPU encoding with `libx264 -preset superfast -threads 0`, with automatic decoder/filter threads. Background Mode retains its reduced thread budget and Windows priority. NVENC/AMF/QSV retain their tested presets and real driver availability checks.

All encoders retain H.264 Main Level 3.1, 8-bit `yuv420p`, `-maxrate 3200k -bufsize 6000k`, resolution-dependent frame-rate limits, and AAC-LC stereo at 160 kbps / 48 kHz. Software subtitle filters and audio/subtitle selection are unchanged.

No larger media buffers, aggressive probe limits, or `ultrafast` quality tradeoffs were added without evidence. These changes reduce application-side work; they do not establish a measured percentage improvement in video encoding throughput. Physical WiiMC playback remains a separate release check.

FFmpeg documents `-nostats` and structured progress independently: <https://ffmpeg.org/ffmpeg.html>.

## Exact old code removed or replaced

In the table below, `encoder-core/` means `src-tauri/crates/encoder-core/`.

These edits are already applied. **There is nothing to delete manually in this checkout.** When porting the changes to another branch, replace the corresponding implementation rather than keeping both versions.

| Original file | Remove/replace this old implementation | Replacement |
| --- | --- | --- |
| `encoder-core/src/lib.rs` | `SourceInfo`, `source_info`, `finalize_output`, and the nine-argument `encode_item` | `engine.rs`: `SourceInfo`, `source_info`, `finalize_output`, and `Encoder::{encode, encode_reserved}` |
| `encoder-core/src/lib.rs` | `AsyncBufReadExt` / `BufReader` import and the `BufReader::lines()` stdout loop | `progress::read` and `pipe_lines::read` |
| Former encoder command builder in `lib.rs` | `-stats` and the combined stderr timing watch state | `engine.rs`: `-nostats` and a duration-only fallback |
| `encoder-core/src/progress.rs` | `StderrProgress` and `StderrProgress::update` | Structured `Metrics`, `read`, and `stderr_duration` |
| `encoder-core/src/diagnostics.rs` | Its duplicate chunk-scanning loop and `push` helper | Shared `pipe_lines::read`; bounded diagnostic tail retained |
| `src/main.ts` | Local `DropAnalysis` / `RecoveryInfo` declarations and raw `invoke<T>` return assertions | Central definitions and typed `invoke` in `src/ipc.ts` |
| `src/updates.ts` | Direct generic `invoke<boolean>` calls | The same unchanged updater commands through `src/ipc.ts` |
| `src/queue-view.ts` | Unused cached `name` reference, JSON-serialized selection keys, option replacement on every selection change | Stable options and direct selection updates; coalesced progress rendering |
| `src/language-preferences.ts` | Repeated `getPreferences()` calls and duplicate snapshot construction | `settingsSnapshot` |
| `src-tauri/src/execution.rs` | `serde_json::json!` progress payload construction | Typed `EncodingProgress` serializer |
| `src-tauri/src/commands.rs` | Unbounded `tokio::fs::read` for previews | A bounded read from the already-open file |

Production sources contain no `console.log`, `dbg!`, or `println!` debugging statements. Test/benchmark reporting stays in test tools; runtime diagnostics continue through the local `log` integration. Public compatibility helpers and deliberately exposed settings utilities were retained.

## Verification

Validation covers strict TypeScript compilation and Vite production output, Rust formatting and Clippy for both crates, frontend regression scripts, headless-browser UI/lifecycle checks, and real FFmpeg fixtures. The fixtures cover audio and embedded/external subtitles, unusual Windows filenames, corrupted-input isolation, crop/aspect handling, output validation, cancellation, OS pause/resume, and checkpoint recovery.

New regression checks cover oversized/split pipe records, resetting unknown metrics, stale cross-channel stats, duplicate-render suppression, stable dropdown options, and cancelling scheduled UI work during disposal.

No installer was rebuilt or published during this review. Existing source changes from the Media & Preferences milestone remain in place.

The checkpoint regression fixture now changes file length explicitly instead of relying on the filesystem clock advancing between same-size writes. Pending source identity remains size plus modification timestamp; same-size edits preserving that timestamp are outside this check. Completed-output checksums remain intact. This pass does not add full source-video hashing to every batch startup.
