# Keroro Wii Encoder User Guide

## Simple and Advanced Mode

Version 1.1.0 opens in Simple Mode unless you previously chose Advanced Mode.
Use the header switch; the app remembers your choice.

Simple Mode presents three steps: **Input File**, **Pick Wii Preset**, and
**Start Encoding**. Drop a single video or an input folder into the app; the
existing Browse button selects folders. Choose an output folder separately.

- **Classic CRT (4:3):** 640x480, 4:3 display aspect, quality 23, Full Throttle.
- **Wii Widescreen (16:9):** 720x480 anamorphic, 16:9 display aspect, quality 23,
  Full Throttle. This recommended widescreen raster allows up to 30 fps.
- **Fastest Encode (Hardware Accelerated):** 640x480, 4:3, quality 26,
  Full Throttle. Available GPUs are detected by the original backend when a job
  starts; a compatible GPU is optional because CPU fallback remains available.

Advanced Mode reveals a collapsible panel under Step 2 with the original
subtitle, naming, quality, CPU usage, aspect, resolution, and custom-size fields.
The diagnostic-display checkbox shows statistics and performance details;
warnings and errors remain accessible regardless of that checkbox. Collection
and local logging continue when details are hidden.

Returning to Simple Mode resets video dimensions, aspect, quality, and CPU
usage to the last explicitly selected preset. Input/output/subtitle paths and
anime-title naming are preserved. Mode and preset changes are locked while an
encode, preview, or update installation is active; theme controls stay available.

Advanced Mode offers an optional **Auto-Crop Black Bars** checkbox and a video
encoder selector. Auto tries NVIDIA, AMD, Intel, then CPU. A manually selected GPU
is tested before use and falls back to CPU if unavailable. CPU selects libx264.
Crop detection samples two seconds at minute one (the opening for shorter clips),
then removes detected borders before resizing and external subtitle burn-in.
If detection is inconclusive or fails, the original frame is retained. Use a
preview to check the result; one sample cannot represent every scene.
Returning to Simple Mode disables crop and restores automatic encoder selection.
See [auto-crop details](AUTO_CROP.md).

## Starting a conversion

1. Select an **Input Anime Folder** and an **Output Wii Folder**. Use different folders.
2. The subtitle folder follows the anime folder by default. Choose another folder to override it, or use **Use anime folder** to restore the default.
3. Optionally enter an anime title and select a quality setting. A title produces names such as `Sgt Keroro - Ep01 [Wii].mp4`; an empty title retains the original filename with `_Wii.mp4` appended to its stem.
4. Use **Preview First 10 Seconds** to inspect the result, then select **Start Platoon Downscale**. Check the status panel for progress, warnings, and the final summary.

With a custom title, selecting a single video preserves a confident source episode number: `[Group] Anime - 03.mkv` becomes `Your Title - Ep03 [Wii].mp4`. Ambiguous or unnumbered files default to `Ep01`. Folder batches retain sequential numbering from `Ep01`, and saved recovery queues keep their original output names.

## Drag-and-drop routing

Drop up to 64 local files or folders onto the window. The scanner inspects up to the first 512 entries in each folder, without searching subfolders. Its first eligible media folder fills the input field; its first completely empty folder fills the output field. A dedicated subtitle folder is preferred over a mixed media/subtitle folder; otherwise the first eligible subtitle folder is used. Dropping an individual video or subtitle retains that exact file path. A single video input encodes only that file; a subtitle file input uses that file, while a subtitle folder is searched for a match. A folder containing even a hidden file or subfolder is not empty. Drops fill fields but do not start encoding.

The scanner supports MP4, MKV, AVI, MOV, WMV, FLV, and WebM video, and SRT, ASS, SSA, and VTT subtitles. If an entry is not found within the scan limit, select the folder manually. Your explicit subtitle-folder override is preserved when only a media folder is dropped.

## Aspect ratio and resolution

Select the stored resolution and display aspect independently. Existing preferences
default to 640x480 and 4:3. Auto preserves the source display shape; explicit
4:3/16:9 fits it with padding. Custom dimensions round to even values (ties upward), within the SD limits.
See the [aspect and resolution guide](ASPECT_RATIO_AND_RESOLUTION.md), including
anamorphic 720x480 and the conservative 25 fps cap for 848x480.

## Subtitle pairing

The encoder first attempts an exact filename-stem match, then a confident episode-number match such as `Ep01` or `S01E01`. When a confident match is unavailable, it uses the corresponding position in the naturally sorted video and subtitle lists, without reusing a subtitle already assigned to another video. Ambiguous matches and file-count mismatches can produce warnings; review the log and preview the result, especially when filenames differ. A missing subtitle is reported and the video proceeds without one.

Subtitle files are normalized to UTF-8 when possible. Files larger than 50 MB or unreadable subtitle data are rejected for safety; check the status panel for individual failures.

## Pause, resume, and batch recovery

During a batch, the main button becomes **Pause**. Pause suspends all threads of the active FFmpeg process on Windows and preserves its open `.mp4.part` output. **Resume** continues that same process from where it stopped. Progress remains frozen while paused, and input/settings controls remain locked. If the encoder is still preparing a file, wait for encoding to start before pausing. Theme switching remains available during encoding and pauses.

A pause lasts only while the application and child process remain alive. Exiting is a separate action: the active-task guard confirms shutdown and resumes a suspended child before stopping it safely. To continue later without exiting, leave the app running or minimize it to the tray.

Each encode writes to an `.mp4.part` file and is renamed to its final `.mp4` name only after successful completion. An interrupted file must be encoded again from the beginning. If the application closes unexpectedly, it can offer to resume its saved batch on the next launch; choose Resume, Later, or Discard. Previously completed files are retained. Closing to the system tray leaves an active batch running; exiting during a batch asks for confirmation.

New batch outputs have local reuse records. Matching recorded sources, subtitles, quality, and output metadata allow a skip. A known mismatch preserves the old file and reports a conflict; select a fresh output folder or move the old output to encode again. Older outputs without records remain preserved and skipped with an unverified-output warning. The status panel distinguishes encoded, skipped, and failed items. Keep source files and review completed output before deleting originals.

## Recovery storage

Recovery checkpoints and their runtime lock live in Tauri's local app-data
folder (`%LOCALAPPDATA%\com.ironcladplatoon.keroroencoder` on Windows),
independently of the media and subtitle locations. Selected absolute video and
subtitle paths can come from different local directories. Previous roaming-data
checkpoints are imported once; the original is retained as a backup and is not
re-imported after the queue finishes. Output files still stay in the selected
output directory, with naming and checksum validation retained.

## CPU usage and performance reports

**Maximum** preserves automatic full threading. **Balanced** uses a smaller
software thread budget and below-normal FFmpeg process priority on Windows.
The selected mode is saved and is locked during a job. It changes resource use,
not the output format or quality target.

Expand **Last encode performance** in the status panel to inspect the encoder,
processing rate, wall time, CPU times, peak memory when available, and output
size. Wall time includes pauses and verification, so it is not active encode
CPU time. The last recorded batch measurement restores after restarting the app;
preview measurements are also shown during the current session and logged.
Persistent records are under the local app-data `output_records` folder.

New outputs are checked before finalization for the Wii SD profile, MP4
container, expected audio presence, and plausible duration. This is a metadata
check rather than a complete decode or a Wii hardware test. Long encodes may
produce a projected FAT32-size warning; a completed file larger than FAT32's
4 GiB minus 1 byte limit also receives a warning.

See [the playback checklist](WII_PLAYBACK_CHECKLIST.md) before a public release.
