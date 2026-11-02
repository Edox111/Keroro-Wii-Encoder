# WiiMC encoding profile

The shared builder `profile::configure()` in
`src-tauri/crates/encoder-core/src/profile.rs` applies to batch, preview, CPU,
and GPU output. Crop detection and subtitle filters stay in software; final
encoding uses the selected encoder and the existing CPU fallback.

```text
-profile:v main -level:v 3.1 -pix_fmt yuv420p
-maxrate 3200k -bufsize 6000k -bf 0
-c:a aac -profile:a aac_low -b:a 160k -ac 2 -ar 48000
-movflags +faststart -f mp4
```

Intel QSV accepts numeric `-level:v 31`, equivalent to Level 3.1. Output validation
requires Main/31, yuv420p, the chosen dimensions/display shape, the frame-rate
ceiling, and AAC-LC stereo at 48 kHz when audio is present. Video-only sources
remain supported. The FAT32 size projection uses the new rate cap.

## Resolution selections

| Preset | Default DAR | SAR at that DAR | Maximum frame rate |
| --- | --- | --- | --- |
| 720x480 — Widescreen (recommended) | 16:9 | 32:27 | 30 fps |
| 640x480 — Standard CRT | 4:3 | 1:1 | 30 fps |
| 848x480 — Mod-16 wide raster | 16:9 | 160:159 | 25 fps |
| Custom | Current aspect setting | Calculated | At most 30 fps |

Simple Widescreen selects 720x480 and 16:9. Advanced resolution changes select
the associated DAR; users can override aspect afterward. Auto aspect remains
available. Custom numbers round to the nearest even integer, ties upward, on
field commit and when preparing settings. Rust independently applies the same
rounding. Bounds remain 2–854 wide and 2–480 high for existing custom settings.
Custom sizes are not promised to be Mod-16; the fixed presets are Mod-16.

848/480 is not exactly 16/9: exact 16:9 requires slightly non-square pixels.
The UI therefore labels this as near-square pixels. The filter builder retains
SAR-aware fitting, because simply fitting into 720x480 and then changing DAR can
distort a source. The order remains validated crop → fitted scale → pad → DAR
→ yuv420p → external subtitles, all in one native `-vf` argument.

The application deliberately retains its conservative decoding workload budget
of 40,500 macroblocks/s and a 30 fps ceiling. It does not increase frame rate
merely because Level 3.1 permits a larger workload.

Legacy `854x480` preset values are accepted and migrated to `848x480` in Rust,
saved preferences, and restored settings. Existing file selections, IPC commands,
events, and pause/resume ownership are unchanged. The output provenance pipeline
identifier changes, so old-profile files are preserved and reported as conflicts
rather than certified as new-profile output. Completed checkpoint files remain
preserved; use a fresh output directory to re-encode everything with this profile.

## Compatibility limits

These are application encoding choices, not a WiiMC certification or a promise
of flawless playback. WiiMC recommends keeping video at 480p. Mod-16 alignment
can reduce padded macroblock work, but decoder complexity, references, entropy
coding, frame rate, storage, and WiiMC settings still matter. Main Level 3.1
signaling alone does not reduce decoding cost; Main can use CABAC, unlike
Baseline. VBV caps constrain rate control rather than guaranteeing every
instantaneous I/O burst. 720x480 is also an NTSC DVD raster, but H.264 MP4 output
is not DVD-Video. Actual Wii/player playback must be tested before release.

References: [WiiMC FAQ](https://www.wiimc.org/documentation/frequently-asked-questions/),
[WiiMC settings](https://www.wiimc.org/documentation/settings/), and
[FFmpeg aspect filters](https://ffmpeg.org/ffmpeg-filters.html#setdar_002c-setsar).

Real-FFmpeg regression coverage includes Hi10P input plus 5.1 PCM audio,
CPU/available-GPU encoding to 8-bit Main/31 at 720x480 with 32:27 SAR and
48 kHz stereo AAC, as well as existing subtitle, crop, cancellation, and
checkpoint tests. It does not replace hardware Wii playback testing.
