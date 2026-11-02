# Optional black-bar detection (v1.1.0)

Enable **Auto-Crop Black Bars** in Advanced Mode. It defaults to off for existing
preferences, old checkpoints, and Simple presets. The checkbox and encoder
selection use the existing `settings.video` object; no command or event is renamed.

## Pipeline

1. Probe the input duration using the existing managed ffprobe process.
2. If enabled, run a two-second CPU crop-detection sample at 60 seconds. Inputs
   shorter than 62 seconds are sampled from their opening instead.
3. Validate the detector's last reported rectangle against decoded frame dimensions.
4. Build one video filter argument: crop, SAR-aware scale, pad, DAR, yuv420p, then
   the existing external subtitle filter. Auto aspect keeps source DAR through SAR.
5. Encode through the selected GPU or CPU path and retain normal output validation,
   `.part` finalization, and checkpoint updates.

The actual pre-pass adds `showinfo` to obtain decoded frame dimensions, including
autorotation, and disables audio/subtitle/data output:

```text
ffmpeg -hide_banner -nostdin -nostats -loglevel info -ss 60
  -threads 1 -filter_threads 1 -protocol_whitelist file,pipe -i INPUT
  -t 2 -map 0:v:0 -an -sn -dn
  -vf showinfo,cropdetect=limit=24:round=16:reset=0 -f null -
```

This is descriptive formatting, not a shell command constructed by the app.
Rust passes every argument directly to the bundled executable. Detector output
cannot supply arbitrary filter text: `CropRect` has private fields, is not
deserializable from IPC, and formats only validated unsigned integers.

Rectangles must have even dimensions/offsets, stay inside the decoded frame, retain
at least a quarter of the frame area, and remove at least 16 pixels in one axis.
At least three reports are required. This deliberately conservative rule avoids
cropping solely to align a full-frame raster such as 854 pixels to 16 pixels.
Full-frame, all-black, malformed, oversized, or out-of-bounds reports yield no crop.

The managed child is hidden on Windows, has a 15-second timeout, uses the existing
process ownership/cancellation system, and drains stderr through the bounded
diagnostic reader. A failed pre-pass logs locally and retains the original frame.
Cancellation kills/reaps the pre-pass and cancels the job, rather than starting an
uncropped encode. Pause remains available once the actual encoding child starts;
the existing preparation guard applies during probes and crop detection.

## Helpers and integration

- `src-tauri/crates/encoder-core/src/crop.rs`: `detect(...)` and trusted rectangle parser.
- `src-tauri/crates/encoder-core/src/video.rs`: `VideoSettings::filter_with_crop(...)`,
  which prepends a validated crop to the existing non-stretching `filter()` builder.
- `src-tauri/crates/encoder-core/src/lib.rs`: optional detection in `encode_item`,
  before external subtitles are appended to the same `-vf` value.
- `src-tauri/crates/encoder-core/src/hardware.rs`: requested encoder probing with
  CPU fallback; the original `detect(...)` API remains available.

```rust
// Inside the existing managed encode path:
let crop = crop::detect(tools, &item.input, source.duration, cancellation, processes).await?;
let mut filter = settings.video.filter_with_crop(crop)?;
// Append the existing securely staged subtitle filter next.
```

The production integration additionally gates this call on `auto_crop` and catches
non-cancellation errors to retain an uncropped frame. It does not pass user paths
through a shell or insert untrusted strings from cropdetect into a filter graph.

## Limits and verification

A two-second sample is a heuristic: changing borders or dark scenes can make a
crop unsuitable elsewhere. Preview before committing a long batch. The preview
uses the same crop sample and settings as the batch, even though its output is
the first ten seconds. Embedded subtitles or signs already baked into the video
can be cropped; external subtitles are rendered after resizing.

Real-FFmpeg regression coverage checks Japanese input paths, detected letterboxing,
848x480/16:9 output validation, visible external subtitles, finalization, full-frame
and all-black fallbacks, cancellation, and missing tools. Browser regressions cover
new-field persistence, legacy migration, Simple resets, and active-job locks.
Physical Wii/player playback and variable-scene sources still require user testing.
