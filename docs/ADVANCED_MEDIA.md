# Subtitle burn-in, naming, preview, and close guard

Current close-to-tray, sound controls, and F11 behavior are described in
[UI_LIFECYCLE.md](UI_LIFECYCLE.md); it supersedes older lifecycle/copy descriptions below.

For current subtitle formats, size limits, normalization, and secure staging,
see [Wave 3](WAVE_3_SUBTITLE_TRAPS.md), which supersedes the earlier subtitle
staging details in this guide.

The implementation is applied to the existing source tree. The Vite entry point in this repository is **root `index.html`**, not `src/index.html`.

## Current layout and subtitle defaults

The primary card orders fields as Anime folder, Subtitle folder (Default), Output
folder, then Anime Title. The audio boost control and its IPC/backend setting
have been removed. Batch and preview share these output arguments:

```rust
.args(["-c:a", "aac", "-b:a", "160k", "-ac", "2", "-ar", "48000", "-movflags", "+faststart"])
```

160 kbps is the AAC encoder target, not a promise that every file's measured
average bitrate is exactly 160000 bits/second. No gain filter is applied.
Timestamps remain under FFmpeg's normal input handling; the application does
not reset the audio timeline independently. Lossy audio conversion cannot be
lossless, guarantee no clipping for arbitrary source material, or repair every
source timing problem. Tests check unchanged audio level and aligned A/V stream
start/duration on a known fixture, plus 48 kHz stereo AAC for preview and batch.

`subtitleFolderOverridden` persists intent separately from paths. Input browsing
and typing update subtitles only in automatic mode. Successful subtitle browsing
or manual text input enables override mode, even when the field is cleared or
the selected folder happens to equal the current anime folder. Cancelling Browse
does not change mode. **Use anime folder** resets the mode and immediately copies
the current input path. Older preferences with a distinct nonempty subtitle path
retain that override; otherwise they migrate to following the anime folder.

## Release additions: formats, preferences, and character audio

The authoritative container list is `VIDEO_EXTENSIONS` in the encoder core:

```rust
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm"];

pub fn is_video_path(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| {
        VIDEO_EXTENSIONS.iter().any(|supported| ext.eq_ignore_ascii_case(supported))
    })
}
```

The existing scan also requires a regular file, sorts naturally, and routes every
candidate through the same encoder. Corrupt or unsupported streams fail per item;
an extension alone cannot guarantee decodability. Directory selection uses
`open({ directory: true, multiple: false, defaultPath: target.value || undefined })`.
File-dialog extension filters do not constrain directory contents, so the UI
documents supported containers and Rust owns filtering.

The header uses this exact new markup:

```html
<div class="header-heading">
  <h1>Keroro Wii Encoder</h1>
  <button type="button" id="muteBtn" class="secondary-btn mute-btn"
          aria-label="Mute UI sounds" aria-pressed="false" title="Mute UI sounds">🔊</button>
</div>
```

```css
.header-heading { display: flex; align-items: center; justify-content: center; gap: 12px; }
.header-heading h1 { min-width: 0; }
.header-heading .mute-btn { flex: 0 0 44px; width: 44px; height: 44px; padding: 0; font-size: 1.2rem; }
.mute-btn[aria-pressed="true"] { border-color: var(--accent-color); }
```

`src/preferences.ts` stores this schema under `keroro.preferences.v1`:

```typescript
interface Preferences {
  selectedTheme: "keroro" | "tamama" | "giroro" | "kururu" | "dororo";
  isMuted: boolean;
  defaultCrf: 20 | 23 | 26;
  subtitleFolderOverridden: boolean;
  lastInputFolder: string;
  lastOutputFolder: string;
  lastSubtitleFolder: string;
  animeTitle: string;
}
```

Loading validates every field, tolerates malformed JSON/storage exceptions, and
migrates the old theme/sound keys. `main.ts` restores values before native event
initialization, updates the quality label and filename hint, and saves on text
input, quality changes, folder selection, and job submission. Restoration
never starts a job or plays a sound. Theme and audio modules patch this same
settings object, so changes cannot overwrite unrelated preferences.

`sounds.ts` discovers character MP3 URLs using Vite's eager `import.meta.glob`.
Supply `keroro.mp3`, `tamama.mp3`, `giroro.mp3`, `kururu.mp3`, and `dororo.mp3`
in `src/assets/sfx/`, then rebuild. No character recordings are included yet;
missing clips use the original click tone. Clicking even the active character
replays its sound. A single clip plays at a time, mute stops existing clips, and
the completion chime obeys the same global state. Preview media audio remains
under the video player's controls.

The complete current HTML/CSS/TypeScript/Rust listings are in
[V1_1_CODE.md](V1_1_CODE.md); source files are authoritative for later edits.

## 1. HTML controls and CSS

`index.html` orders the first four fields exactly as follows:

```html
        <!-- Input Folder Selection -->
        <div class="input-group">
          <label for="inputFolder">Input Anime Folder:</label>
          <div class="input-row">
            <input type="text" id="inputFolder" placeholder="C:\Users\YourName\Videos\RawAnime" />
            <button id="browseInputBtn" class="secondary-btn">Browse</button>
          </div>
          <p class="field-hint">Supported video containers: MP4, MKV, AVI, MOV, WMV, FLV, WebM. Output: MP4.</p>
        </div>

        <div class="input-group">
          <label for="subtitleFolder">Input Subtitle Folder (Default):</label>
          <div class="input-row">
            <input type="text" id="subtitleFolder" placeholder="Follows the anime folder unless overridden" aria-describedby="subtitleHint" />
            <button type="button" id="browseSubtitleBtn" class="secondary-btn">Browse</button>
          </div>
          <div class="subtitle-options">
            <p id="subtitleHint" class="field-hint">Matches .srt or .ass by filename or episode. Clear to skip subtitles.</p>
            <button type="button" id="resetSubtitleBtn" class="secondary-btn">Use anime folder</button>
          </div>
        </div>

        <div class="input-group">
          <label for="outputFolder">Output Wii Folder:</label>
          <div class="input-row">
            <input type="text" id="outputFolder" placeholder="C:\Users\YourName\Videos\WiiOutput" />
            <button id="browseOutputBtn" class="secondary-btn">Browse</button>
          </div>
        </div>

        <div class="input-group">
          <label for="animeTitle">Anime Title / Output Name Pattern:</label>
          <input type="text" id="animeTitle" placeholder="Sgt Keroro" aria-describedby="outputNameHint" />
          <p id="outputNameHint" class="field-hint" aria-live="polite">Original filename_Wii.mp4</p>
        </div>
```

The action row includes `previewBtn` beside `convertBtn`. The `previewModal` HTML dialog contains `previewVideo`, a close button, descriptive text, and playback error feedback.

`src/styles.css` adds a responsive action grid, field hints, and a glass dialog with an independently blurred backdrop. The video uses `object-fit: contain`, constrained viewport dimensions, and native playback controls. The dialog supplies modal focus handling and Escape-to-close behavior.

## 2. TypeScript handlers

`src/naming.ts` sanitizes Windows filename characters using a Unicode-aware regex and provides the live output name example. The backend validates and sanitizes again; the frontend is not the authority on filesystem names.

```ts
export function cleanAnimeTitle(value: string): string {
  return value.replace(/[\p{Cc}<>:"/\\|?*]/gu, " ")
    .replace(/\s+/gu, " ").trim().replace(/[. ]+$/u, "");
}

export function outputNameExample(value: string, episode = 1): string {
  const title = cleanAnimeTitle(value);
  return title
    ? `${title} - E${String(episode).padStart(2, "0")} [Wii].mp4`
    : "Original filename_Wii.mp4";
}
```

`src/main.ts` connects the subtitle Browse button to the existing native directory dialog, validates the optional title, and submits:

```ts
const settings = {
  inputDir,
  outputDir,
  crf,
  subtitleDir: subtitles.value.trim() || null,
  animeTitle: cleanAnimeTitle(animeTitle.value),
};

// Full batch:
await invoke<string>("start_batch", {
  settings,
  notifyOnCompletion: notifications.checked,
});

// Preview; outputDir may be blank because Rust uses its own private cache:
await invoke<string>("start_preview", { settings });
```

Both commands return a job ID. The frontend buffers events arriving before the command response and ignores older sequence numbers. Batch and preview controls cannot start competing FFmpeg jobs.

On a successful preview, `read_preview` returns a binary `ArrayBuffer`. The frontend creates an MP4 `Blob`, loads an object URL into the video, and opens the dialog. Closing via the button or Escape stops playback, unloads the video, revokes the URL, and calls `release_preview`. A failed preview reports an error instead of opening an empty player.

The `task-cancelling` listener locks task controls and displays cleanup status. Rust owns native close interception and confirmation; no competing JavaScript close-confirmation handler is registered.

## 3. Rust media and naming logic

`src-tauri/src/lib.rs` provides narrow IPC adapters, native close handling, and private preview access. Reusable media logic remains in `src-tauri/crates/encoder-core/`:

| File | Responsibility |
| --- | --- |
| `src/lib.rs` | Plan preparation, FFmpeg arguments, preview duration, progress, output finalization, cleanup |
| `src/media.rs` | Filename sanitization, episode parsing, natural ordering, and subtitle matching |
| `src/cancel.rs` | Cloneable cancellation signal that cannot miss an earlier cancellation |

Subtitle pairing first reserves unique case-insensitive exact stems or confident
episode matches across the complete queue. Regex recognizes common S02E03, E03,
Episode 03, Anime - 03, and numeric episode forms, ignoring resolution/year tokens
in the generic numeric form. Primary season matches must agree when both are
specified. If a unique match is unavailable, both lists are independently sorted
by natural alphanumeric filename order and paired at the same absolute index.
Fallback never shifts leftover indices or reuses a subtitle reserved by a primary
match. Ambiguity, fallback use, count mismatches, and unresolved orphans generate
local log/UI warnings. A still-unpaired input encodes without external subtitles.
Index fallback is a heuristic; filenames without reliable episode tokens require
checking the ordering. Preview uses the full batch pairing before selecting its
first video, and recovery keeps the original frozen subtitle paths.

Before invoking FFmpeg, the matched subtitle is copied to a private, exclusively created ASCII staging filename beside the temporary output. FFmpeg runs with that directory as its working directory. The filter graph contains only the generated staging name, so original Windows drive letters, apostrophes, commas, brackets, and Unicode path characters cannot alter the filter syntax. The subtitle is rendered **after** scaling/padding, onto the final 640×480 canvas. Both SRT and ASS are supported; external subtitle files larger than 16 MiB are rejected.

For a custom title, naturally ordered files receive sequential names starting at one:

```text
Sgt Keroro - Ep01 [Wii].mp4
Sgt Keroro - Ep02 [Wii].mp4
```

This is a new sequence, not preservation of original episode numbers. Without a title, the original stem is retained with `_Wii.mp4`. Existing output files remain protected.

Preview processing selects only the first video in the same order used for batches. It uses the same scaling, CRF, encoder preset, audio settings, and subtitle selection as the batch, adding `-ss 00:00:00` and `-t 10`. The private cache layout is:

```text
<application cache>/previews/<job ID>/preview_temp.mp4
```

Preview access accepts a job ID rather than an arbitrary filesystem path. Binary IPC is limited to 32 MiB, and no broad asset-protocol or filesystem permission was added.

## 4. Native close confirmation and cancellation

`WindowEvent::CloseRequested` always prevents immediate window destruction. During preparation, probing, batch encoding, or preview generation, Rust displays:

> Conversion is currently ongoing! Are you sure you want to cancel and exit?

The native dialog offers **Cancel and exit** and **Keep encoding**. Only one confirmation can be open. Declining changes neither the worker nor its cancellation token.

Confirmation signals the worker and waits for its completion notification. FFmpeg first receives `q` through its dedicated stdin pipe. After two seconds, a process that has not stopped is killed and reaped. Progress and diagnostic readers are joined, incomplete outputs and staged subtitles are cleaned, and only then does the app exit. Already completed batch outputs are retained. The same guard is used by the tray Exit action.

When idle, closing the window retains the existing hide-to-tray behavior. The explicit Minimize to tray button always keeps the task running. Saved preview cache files are also cleaned on explicit exit.

## Verification

The media tests cover visible SRT and ASS pixels, paths containing apostrophes/brackets/commas/Unicode, 10-second preview duration, sequential naming, natural episode order, live-process cancellation, partial-file removal, preserved existing outputs, and continued processing after an individual failure.

The browser checks cover title hints, subtitle browsing, preview IPC settings, actual MP4 playback in the modal, button/Escape cleanup, object URL revocation, stale progress events, and UI locking during cancellation. Native Windows confirmation interaction still requires a manual desktop check.

Build using the existing clean-first workflow:

```powershell
npm run build:windows
```

For a release executable without an installer:

```powershell
npm run tauri -- build --no-bundle
```
