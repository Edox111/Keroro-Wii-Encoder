# v1.2.0 Media & Preferences

Open **Settings** in the header to choose Japanese, English, or another supported language, **Any (first track)** audio, **None** for automatic embedded subtitles, and an optional default output folder. New installations default to Japanese audio / English subtitles. Existing saved choices are kept. Completion sound and opening the output folder both default to enabled and can be switched independently. Values live in the existing `keroro.preferences.v1` localStorage object. The fallback folder is used only when the current output path is empty.

## Queue tracks

Add files or drop videos into the queue. Bundled ffprobe returns:

```ts
interface MediaInfo {
  audioStreams: AudioStream[];
  subtitleStreams: Array<{
    index: number;       // subtitle-only ordinal: 0, 1, ...
    streamIndex: number; // absolute source index, for diagnostics
    codecName: string;
    language: string;
    title: string | null;
    supported: boolean;
  }>;
}
const media = await invoke<MediaInfo>("probe_media_streams", { path });
```

The Audio and Subtitle selectors appear together on each queue row. Matching language wins; otherwise the first audio / supported text subtitle is chosen. `en`/`eng` and `ja`/`jpn` are equivalent. Unsupported bitmap tracks (PGS/VobSub) remain visible but cannot be selected; these require a separate image-overlay pipeline. No embedded streams means normal video-only/external-subtitle behavior.

**None / External Subtitle File** disables embedded burn-in for that item and keeps existing external subtitle routing. Explicitly dropped external files take priority during automatic selection. Naming such as `Ep01.eng.srt`, `Ep01.jpn.ass`, or `Ep01 [English].srt` helps choose the preferred language. Files without a language tag retain exact-basename / sorted fallback matching; the app cannot infer their language from text reliably. Manual embedded selection overrides external files. Manual choices, including None, survive preference changes and restart. Stream metadata is re-probed on restart.

A selected embedded track passes `settings.selectedSubtitleIndex` to the existing batch/preview command. This is the subtitle-only ordinal used by `subtitles=filename=...:si=N`, not the absolute stream index. The native filter builder escapes option values and filtergraph syntax separately, including Windows drive colons, apostrophes, brackets, commas, and spaces. FFmpeg is always launched with native arguments, without a shell. Existing scale/crop/pad/DAR and WiiMC codec settings remain in place. Embedded text is burned into the output video; it is not soft-muxed.

The optional `preferredSubLang` setting supports native folder batches. An explicit selected index wins; otherwise a matched external file wins over the language preference. `none` disables automatic embedded selection. Old callers omitting both fields retain their prior external-only behavior. Saved checkpoints and output provenance include the subtitle policy so a different track cannot silently reuse an old output.

## Performance events

The existing `encoding-progress` event retains `jobId`, `sequence`, `phase`, `paused`, `file`, `percentage`, and formatted `eta`. Additive fields are:

```ts
interface LiveMetrics {
  fileId: string; // jobId:one-based-native-item-number
  progressPercent: number | null;
  frame: number | null;
  fps: number | null;
  speed: number | null; // numeric multiplier, e.g. 1.85
  etaSeconds: number | null;
}
```

Structured FFmpeg stdout (`-progress pipe:1`) supplies frames, FPS, position, and speed. stderr remains a bounded diagnostic stream with a duration fallback; duplicate human-readable progress is disabled with `-nostats`. ETA is `(duration - position) / speed`. Unknown/non-finite metrics display `--`, not invented values. Integer percent / ETA updates are throttled; changing FPS/speed still refreshes at most once per second when duration is unknown. Existing native job identity/sequence guards discard stale events.

Queue rows and the active progress line show percentage, speed, FPS, and ETA. Pause displays **Paused**; resume continues the same process. Metrics disappear from a row after completion. No new polling timer, process, or progress event name was introduced.

## Code map and checks

- `encoder-core/src/probe.rs`: unified metadata and subtitle selection.
- `encoder-core/src/ffmpeg.rs`: embedded subtitle filter escaping.
- `encoder-core/src/progress.rs`: timing and emission throttling.
- `src/media-streams.ts`: typed parsing, labels, automatic selection.
- `src/settings-modal.ts`: native HTML dialog using the existing TypeScript/Vite UI.
- `src/language-preferences.ts`: persistent external store, usable by a future React `useSyncExternalStore` adapter.

The production app currently uses TypeScript/Vite, so this implementation preserves that architecture rather than introducing React dependencies. Tests cover preference persistence, manual None, external language matching, exact real audio selection, text burn-in with special/Unicode Windows paths, checkpoint compatibility, pause/resume, and progress. Physical Wii playback and an installer rebuild are separate release checks.

## Completion actions

The Settings modal contains **Play completion sound** and **Open output folder**, both enabled on first launch. Explicit `false` values persist across restarts. The central settings store exposes `playSoundOnComplete` and `openFolderOnComplete` alongside language preferences and `defaultOutputDir`.

Place a sound at `public/sounds/completion.mp3` or `public/sounds/completion.wav`. The app tries MP3, then WAV, at volume 0.4. Missing assets or autoplay denial are silently ignored. This cue is independent of **UI sounds**, which controls click/character feedback. No invalid placeholder audio is shipped.

Actions run once after the whole admitted queue finishes, including isolated per-file errors, rather than after every episode. Native folder batches run actions after success or completion with errors. Previews, paused tasks, cancelled queues, lost-state safety stops, and historical completed snapshots never trigger them. A frozen destination from the batch start prevents later edits from opening a different folder. Optional action failures do not change encoding results; an opener failure produces a small status notice.

The narrow async `open_output_folder({ path })` command validates an existing absolute local directory, rejects URLs/files/network paths, and dispatches Explorer using Windows' Unicode shell API with the folder-specific `explore` verb and no command-line parameters. COM initialization is balanced on the background worker. No shell-execution plugin or broad opener capability is added.

`src/completion.ts` owns cue playback, bounded duplicate protection, independent actions, and disposal. `src-tauri/src/completion.rs` owns OS folder opening. The release extractor derives its executable filename from the current package version (`Keroro-Wii-Encoder-v1.2.0.exe`). This milestone changes application metadata to 1.2.0 but does not publish updates or build an installer.
