# v1.1.0 UI modes and compatibility

This release extends the existing TypeScript/Vite application with a presentation refactor and optional video-formatting settings.
React and Tailwind are not dependencies. The complete entry/layout files are
`index.html`, `src/main.ts`, `src/preferences.ts`, `src/ui-mode.ts`, and
`src/ui-mode.css`; the original `src/styles.css` remains in use.

## Presentation boundary

`src/ui-mode.ts` owns the top-level `uiMode` state, the segmented control, preset
selection, panel visibility, and the diagnostic-display preference. The existing
preferences JSON gains `uiMode`, `simplePreset`, and `showLogs`; old preferences
default to Simple Mode and Classic CRT.

The controller receives two narrow callbacks from `src/main.ts`: apply a preset
through existing field references and `saveForm()`, and read the current selected
settings. It imports no Tauri APIs and makes no IPC calls. It does not select
files, start/cancel jobs, choose an encoder, or render progress events.

All original element IDs are retained exactly once. Original controls remain
mounted when Advanced Mode is hidden, so the same listeners and element
references continue to work. Input/output/subtitle paths and output naming are
never reset by the mode controller.

## Preset mappings

| Preset | Resolution | Aspect | Quality | CPU mode |
| --- | --- | --- | --- | --- |
| Classic CRT | 640x480 | 4:3 | 23 | maximum / Full Throttle |
| Wii Widescreen | 720x480 | 16:9 | 23 | maximum / Full Throttle |
| Fastest Encode | 640x480 | 4:3 | 26 | maximum / Full Throttle |

Every preset retains the existing backend's GPU detection and CPU fallback.
Widescreen fits the source without stretching into an explicit 16:9 canvas. Its frame-rate cap is 30 fps.

Returning to Simple Mode resets manual video/quality/CPU overrides to the last
explicitly selected preset, including rounding custom dimensions to even values and resetting unused custom dimensions, disabling auto-crop, and restoring Auto encoder selection. Names
and paths remain unchanged. An already running job retains its frozen request.
Mode and preset controls are locked alongside settings during active work; an
initial Simple-mode reset is deferred if the UI attaches to an existing job.

Advanced Mode exposes existing fields and a display-only diagnostic toggle.
Statistics/performance visibility changes without stopping collection; warning
and error details remain accessible in both modes. Panel scrolling is separate
from status scrolling so the overall window retains its viewport bounds.

Advanced Mode now also exposes opt-in auto-crop and Auto/NVENC/AMF/QSV/CPU
encoder selection. The two new members travel inside the existing `settings.video`
object; missing members default to `false` / `auto` in both preferences and Rust.
Native command signatures, invoke call sites, progress events, file handlers, and
core file-selection variables remain unchanged. Arbitrary CLI flags are not exposed.

See [crop detection](AUTO_CROP.md) for the bounded pre-pass and filter ordering.

## Frontend presentation polish

Resolution and aspect controls use equal grid columns with labels above the
native selects. The resolution help badge spans the grid instead of becoming
an extra select cell. Native values and event handlers remain unchanged.

Settings help uses a body-level, viewport-clamped tooltip portal. Modern WebViews
place it in the popover top layer so scrolling cards cannot clip it. It supports
keyboard focus and Escape, flips above/below the trigger, and cleans up on disposal.

When no external subtitle source was explicitly chosen, the known optional
missing-match notice renders as neutral status rather than a warning banner or
auto-expanded warning list. Explicit selection failures and other warnings/errors
remain visible. This only changes display: backend warnings and logs are retained.
The current encoder excludes embedded subtitle streams with `-sn`; the UI does
not claim embedded subtitle passthrough or change that encoding behavior.

## Verification for this refactor

- Frontend compilation and browser tests cover both modes, first-launch defaults,
  preset/reset/persistence, preserved DOM nodes/paths, drop routing, active-job
  locks, geometry, progress, pause/resume, themes, sounds, and diagnostics.
- Source comparisons against the pre-refactor workspace preserve every original
  `invoke` call, file/encoding/progress handler, state initializer, and HTML ID.
- Native Tauri adapter source files (`src-tauri/src/*.rs`) are unchanged. Optional crop/encoder functionality lives in encoder-core. Application version metadata is 1.1.0; the internal encoder-core library remains version 1.0.0.
- Native compilation and the existing real FFmpeg aspect/subtitle regression
  pass. The release installer was not rebuilt or published by this UI task.
