# Encoding safety and performance

## Output reuse

New batch files have a bounded, atomic JSON record under
`%LOCALAPPDATA%/com.ironcladplatoon.keroroencoder/output_records`.
Records compare the source and selected subtitle paths, sizes, modification
times, quality value, encoding-profile revision, and final output metadata.
Selected subtitle content is also hashed within the existing 50 MB size guard,
so same-size edits with unchanged/coarse timestamps are detected. A video-only
subtitle fallback is never verified as a successful external-subtitle burn-in.
Changing CPU usage alone does not invalidate an output. Completed recovery
outputs retain their existing SHA-256 checks.

A matching record allows reuse. A known mismatch leaves the output untouched
and reports why it cannot be reused; choose another destination or move the old
file before encoding again. Unrecorded legacy files remain preserved and skipped,
with one batch warning and individual entries in the local log. Missing records
after a crash do not cause finished episodes to be overwritten.

## Validation and rollback

After the encoder exits successfully, the managed child is reaped before a
bounded ffprobe subprocess inspects the `.part` file. Checks cover MP4, H.264
Baseline/Main Level 3.0 or lower, 640x480 yuv420p video at no more than 30 fps,
AAC-LC stereo/44.1 kHz audio when present, expected source-audio presence, a
positive duration, and substantial duration discrepancies. The source and
subtitle fingerprints are checked again before committing.

Validation failure rejects the partial output. A hardware encode or validation
failure can retry once on CPU after its process and owned partial are cleaned up.
Cancellation, changed sources, persistence errors, and existing destination files
retain their separate safeguards. Metadata checks do not replace a complete
decode or physical Wii playback testing.

Projected FAT32 warnings are advisory: a bitrate cap is not an exact size
prediction. Actual files exceeding 4 GiB minus 1 byte are preserved with a
compatibility warning rather than silently split or truncated.

## Measurements and CPU usage

`-benchmark` measurements are parsed locally without sending diagnostic noise to
the UI. The expandable performance section shows the last successful encode's
encoder, resource mode, wall time, processing rate, CPU time, peak memory when
available, and size. Per-output records retain measurements, and
`latest-statistics.json` restores the most recent recorded batch measurement.
Wall time includes pauses, probing, and verification. Preview measurements are
shown for the session and logged; persistent batch records remain independent
of disposable preview files.

Maximum remains the default and uses automatic software threading. Balanced
uses `max(1, logical_cores - 2)` threads for software work and below-normal
FFmpeg priority on Windows. Neither option changes the Wii format requirements,
quality selection, or native pause/resume handling.

## Repeatable benchmark

From the project root, run:

```powershell
npm run benchmark:encoder
```

The opt-in benchmark generates private temporary test media, exercises the
production encoding/filter/validation path with no subtitles, SRT, and styled
ASS, compares available GPU and CPU encoding in both resource modes, and writes
[PERFORMANCE_BENCHMARK.md](PERFORMANCE_BENCHMARK.md). Temporary media is removed
afterward. Tests require the bundled Windows FFmpeg/ffprobe tools or the
`KERORO_TEST_FFMPEG` and `KERORO_TEST_FFPROBE` environment variables.

This is a synthetic development-harness measurement, not a speed guarantee or
an anime quality assessment. Use repeat runs and representative episodes before
changing defaults. The validated FFmpeg binaries remain unchanged; a smaller
FFmpeg distribution requires a separate codec/filter/driver compatibility build.
