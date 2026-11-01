# Wave 2: Data and file resilience

**Superseded subtitle details:** [Wave 3](WAVE_3_SUBTITLE_TRAPS.md) replaces the
16 MiB limit, Windows-only decoder, and staging beside the output with a 50 MB
limit, encoding_rs, SSA/VTT support, and isolated tempfile storage. The sections
below describe the Wave 2 implementation and its completed checks.

The implementation updates the current project. `src-tauri/src/lib.rs` remains
the Tauri command/event adapter; actual per-file transcoding lives in
`src-tauri/crates/encoder-core/src/lib.rs`. This keeps media tests independent
of the webview. [V1_1_CODE.md](V1_1_CODE.md) includes the complete current code
blocks, helper modules, manifests, and frontend changes.

## Per-file recovery and retained diagnostics

`encode_item` returns `Result`, and the batch loop handles each failure without
propagating it through the whole batch. It increments `failed`, records the
filename/reason, publishes a sequenced progress event, and continues:

```rust
Err(reason) => {
    state.failed += 1;
    state.message = format!("[Error] Failed to process {name}: {reason}. Skipping...");
    state.errors.push(ItemFailure { file_name: name, reason });
}
```

Cancellation still stops the queue intentionally. Shared configuration failures
(invalid folders, duplicate destinations, invalid quality, inadequate initial
free space) return before starting a media child. Each later file failure is
isolated. Existing destinations remain untouched; failed partial outputs are
removed after process reaping.

`Progress.warnings` holds `{fileName, reason}` entries separately from failures.
Warnings also travel through the existing `wii-progress` listener and saved
snapshot, so they survive final summaries and frontend reloads. `main.ts` renders
both lists with `textContent`, including `[Error]`/`[Warning]` labels, in the
**Warnings and skipped files** console. Equal-length changed diagnostics update
correctly. Old event sequences remain ignored. Notifications include warning
counts. A subtitle warning does not count as a failed video.

## Native Unicode paths and safe filter construction

All real paths stay as `Path`/`PathBuf`/`OsString` and reach FFmpeg/ffprobe through
individual native arguments. Display text and case-insensitive matching may use
lossy string conversion, but command arguments and filesystem operations do not
use that display conversion for video paths.

```rust
command.arg("-i").arg(&item.input);
// Later, the output is a separate native path argument too:
command.arg(&temp);
```

No shell is used. Original subtitle paths are not interpolated into filtergraphs.
The app decodes a subtitle and writes a temporary UTF-8 copy beside the output,
using an internally generated ASCII basename. FFmpeg runs in that directory:

```text
scale=640:480:...,pad=640:480:...,setsar=1,subtitles=filename=.keroro-subtitles-JOB.srt:charenc=UTF-8
```

Internal job IDs are validated as ASCII letters/digits/hyphens/underscores before
being used in generated filenames. Original subtitles are never rewritten.

## Subtitle encoding and fallback policy

`subtitles.rs` reads at most 16 MiB plus one byte, even when a file grows during
reading. It handles:

| Input | Behavior |
| --- | --- |
| UTF-8, optional BOM | Strict decoding; a corrupt BOM-declared file is rejected |
| UTF-16 LE/BE with BOM | Strict decoding and UTF-8 staging; conversion warning |
| Shift-JIS / Windows-932 | Strict Win32 conversion with Japanese-content heuristic; guess warning |
| Western ANSI / Windows-1252 | Strict Win32 fallback conversion; guess warning |
| Missing, unreadable, binary, oversized, or no recognizable cues | Warning; encode without external subtitles |

Legacy encodings cannot be identified reliably from arbitrary bytes. A BOM-less
file can be valid in multiple code pages. The app reports guesses and asks the
user to verify the preview. Windows-1252 does not represent every Windows ANSI
code page; for other languages or an incorrect guess, save the source as UTF-8.
Undetectable mojibake already present in valid UTF-8 cannot be repaired by this
decoder. Non-Windows builds accept Unicode encodings but skip unsupported legacy
encodings with a warning.

A selected subtitle folder with no match logs a per-file warning. A subtitle
deleted after scanning also becomes a warning, clears the active subtitle label,
and leaves the video encode running without that subtitle. Ambiguous matching
now uses the queue-wide index fallback described in
[UI_LIFECYCLE.md](UI_LIFECYCLE.md), with warnings and no subtitle reuse.
Recognizable but otherwise malformed subtitle syntax can still be rejected by
FFmpeg; that file is reported as failed and the batch continues.

## Output write and free-space preflight

`preflight.rs` validates the output as a directory and creates a unique probe
using `create_new(true)`. It actually writes, flushes, syncs, closes, and removes
the probe. Failure prevents the batch from starting, without overwriting an
existing file. Cleanup failure is reported with the probe's path.

On Windows, `GetDiskFreeSpaceExW` checks **caller-available** space, including
quota restrictions. The Unicode API receives a checked, terminated UTF-16 path.
The potentially blocking query runs through `spawn_blocking`. At least 64 MiB
of free reserve is required initially and before each item. If the space query
is unavailable but the write probe passed, a warning is retained.

This is a minimum-reserve guard, not an output-size estimate or reservation.
CRF output size varies, another program may consume storage, and the FAT32/exFAT
finalization fallback can temporarily require a second output copy. Mid-encode
storage errors are still handled as individual item failures with cleanup.

## Verification

The real-media test queues ten episodes under Japanese directory/filenames with
spaces, apostrophes, brackets, commas, ampersands, and `$()` text. Episode 4 has a
corrupt MKV header; episodes 5–10 still process. Pixel checks prove subtitles
were burned from UTF-8, Shift-JIS, Windows-1252, and UTF-16 inputs. Missing,
deleted-after-scan, and binary subtitle cases produce clean video without text.
Original subtitle bytes and output/temp cleanup are checked.

Other tests cover write-probe cleanup, non-directory rejection, a live Windows
free-space query, deterministic low-space boundaries (without filling a drive),
invalid/truncated encodings, cancellation, and process reaping. Browser checks
use mocked native IPC to verify safe diagnostic rendering, Unicode display,
warnings surviving completion, equal-length diagnostic updates, stale-event
rejection, and form locking. No full-disk destructive test was performed.

Win32 API references:

- [MultiByteToWideChar](https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-multibytetowidechar)
- [GetDiskFreeSpaceExW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdiskfreespaceexw)

Release packaging still uses the existing clean-first hook. The previous release
archive is not a build of these source changes.
