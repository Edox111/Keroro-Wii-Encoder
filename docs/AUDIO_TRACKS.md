# Audio tracks and language preferences

Add videos with **Add Files** or drag-and-drop. Each queued video is probed with the bundled ffprobe. The audio selector lists language, source channel layout, codec, and any track title. A single-track file shows a disabled selector; a silent video shows **No audio stream**. Track choices lock while the queue runs. Preview uses the first pending item's selected audio.

Japanese is the default audio preference; English is the default subtitle preference. Previously saved choices are preserved. `en`/`eng` and `ja`/`jpn` are normalized; a matching language wins, otherwise the first audio track is used. Missing metadata is shown as **Unknown language**. Manual choices take priority over preferences and survive restarts; metadata is re-probed rather than cached permanently. Probe failures leave the video available for encoding with native preference/first-track fallback. A retained manual index is still validated by the encoder.

The current application uses TypeScript/Vite, not React. No frontend framework or dependency migration is required for this feature.

## Native contract

```ts
const media = await invoke<MediaInfo>("probe_media_streams", { path });
const streams = media.audioStreams; // media.subtitleStreams contains embedded subtitle tracks.
// AudioStream: { index, streamIndex, codecName, channels, channelLayout, language, title }
```

`index` is the **zero-based audio ordinal**. `streamIndex` is the absolute source index for diagnostics. For example, video stream 0 and audio streams 1/3 produce audio ordinals 0/1. Selecting Audio 2 sends `settings.selectedAudioIndex: 1` and maps `-map 0:v:0 -map 0:a:1`. Do not send the absolute stream index.

The existing batch/preview commands accept additive, optional `selectedAudioIndex`, `preferredAudioLang`, `selectedSubtitleIndex`, and `preferredSubLang` settings. Old requests and checkpoints lacking them retain optional first-audio mapping (`0:a:0?`). New preference-only folder jobs resolve the preferred language separately for each source. Video-only input remains supported. Explicit invalid indices produce a per-file error instead of silently encoding the wrong language.

The selected audio is converted using the existing WiiMC profile: AAC-LC, 160 kbps, stereo, 48 kHz. Video constraints, software subtitle filters, GPU fallback, pause/resume, and sequential admission remain in place. Checkpoints preserve the audio policy, and output provenance detects a changed track or preference before reusing a finished file.

Probes use local file paths and explicit native arguments, separate tracked/reaped children, an eight-second timeout, bounded output, and at most two concurrent UI probes. Start, Preview, and updater installation wait for outstanding probes; selecting/removing files remains available.

## Preferences UI and store

The header **Settings** button opens the preferences modal. `src/language-preferences.ts` exposes its stable external store:

```ts
import {
  getLanguagePreferences,
  subscribeLanguagePreferences,
  updateLanguagePreferences,
} from "./language-preferences";

getLanguagePreferences(); // { preferredAudioLang: "jpn", preferredSubLang: "eng", defaultOutputDir: "", playSoundOnComplete: true, openFolderOnComplete: true }
updateLanguagePreferences({ preferredAudioLang: "jpn", preferredSubLang: "eng" });
const unsubscribe = subscribeLanguagePreferences(() => {
  // Re-render future Settings controls from getLanguagePreferences().
});
unsubscribe();
```

Both values live in the existing `keroro.preferences.v1` localStorage record. Changing the audio preference updates automatic pending choices, leaving manual selections intact. A future React component can bind to the same store without changing IPC:

```tsx
import { useSyncExternalStore } from "react";

export function useSettings() {
  const settings = useSyncExternalStore(
    subscribeLanguagePreferences,
    getLanguagePreferences,
    getLanguagePreferences,
  );
  return { ...settings, updateSettings: updateLanguagePreferences };
}
```

`subtitleSuggestionForAudio(stream, preferredSubLang)` in `src/audio.ts` returns a language and `recommendSubtitles` boolean. This is an informational hook for future settings, not an automatic change to embedded subtitle behavior or existing external-subtitle pairing.

## Verification

```powershell
npm run build
node scripts/test-audio.mjs
node scripts/test-queue.mjs
cargo test --manifest-path src-tauri/crates/encoder-core/Cargo.toml
```

The ignored `tests/audio.rs` integration test additionally requires `KERORO_TEST_FFMPEG` and `KERORO_TEST_FFPROBE` pointing at local binaries. It generates Japanese/English tracks with distinct tones, checks probing and checkpoint serialization, encodes automatic/manual/fallback choices, and verifies the actual decoded audio plus AAC stereo output.

See [Media settings](MEDIA_SETTINGS.md) for embedded subtitle selection, external language tags, and live performance statistics.
