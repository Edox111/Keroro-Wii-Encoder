# UI layout, F11, and tray lifecycle

Current verification for v1.1.0 covers the frontend build, browser mode/preset
and lifecycle regressions, native compilation, and a real-media aspect/subtitle
pipeline regression. Protected Rust source fingerprints and frontend AST checks
confirm the UI refactor preserved the existing runtime hooks. No final release
installer has been rebuilt or published for this UI change.

These changes are applied to the existing source tree. The Vite entry point is
root `index.html`, not `src/index.html`. The public release version is now
1.1.0, synchronized across npm, Tauri, Cargo, and their lockfiles.

## Controls and layout

The header sound button has been removed. UI sounds below the action row is the
single sound setting, including character clips and completion/error feedback.
The native HTML5 Audio manager is now in `src/main.ts`; add click.mp3, success.mp3,
and error.mp3 under `src/assets/` and rebuild. Missing clips or rejected playback
never affect encoding. Volume is capped at 0.4, players are reused, and muting or
disabling UI sounds stops UI audio. Delegated event handlers cover buttons and
selection controls, including keyboard activation. Completion sounds are deduplicated
by job; errors displayed by the frontend use the error hook. The
main button reads **Start Platoon Downscale** and changes while encoding.
Desktop notification controls, frontend imports, native plugin registration,
notification events, dependencies, and permission capabilities were removed.

Both `html` and `body` have 100vw/100vh bounds and hidden overflow. The centered
container is a height-constrained flex column with a 760px maximum width. The
card is another shrinking flex column (`min-height: 0`). Form rows and actions
keep their space; the status panel grows into the remaining space and scrolls.
Notices and persistent warning lists live inside that panel, avoiding nested log
scrollbars. Dialogs retain independent scrolling for accessible long messages.
Compact rules below 740px height keep the recovery utility and controls visible.

The configured window starts at 800x900 with minimum bounds of 650x800.
The initial status panel is checked at an 800x860 content viewport to allow for
native window chrome; long logs retain internal scrolling.

## F11

`src/main.ts` listens for F11 and calls Tauri's `getCurrentWindow().isFullscreen()`
and `setFullscreen()`. The listener prevents the WebView default, ignores held-key
repeats, and serializes pending toggles. It remains available during encoding;
failures are displayed without altering the task. The handler is removed during
page teardown/HMR. Plain browser previews do not invoke native window methods.
The main-window capability adds only `allow-is-fullscreen` and
`allow-set-fullscreen` beyond the existing permissions.

## Close and tray behavior

| Action | Result |
| --- | --- |
| Close with Minimize to System Tray checked | Hide the window; encoding continues without a cancel prompt |
| Close with the option unchecked, idle | Clean up the saved preview and exit |
| Close with the option unchecked, encoding | Confirm cancellation; decline keeps encoding, confirm stops/reaps FFmpeg and exits |
| Tray menu Exit | Exit with the same active-job confirmation, regardless of the toggle |
| Tray click, Show, or a second app launch | Show, un-minimize, and request focus for the primary window |

The standalone Minimize to tray button and its unused IPC command were removed.
The tray option defaults to enabled. It is saved as `minimizeToTrayOnClose` in the
existing localStorage preferences object. The frontend synchronizes it through
`set_close_preferences`; failed saves revert the checkbox and show an error.
Startup persists normalized preferences (including the first-launch true default)
and removes the deprecated keroro.notifications, keroro.uiSounds, and keroro.theme
keys only after their migrated values have been saved successfully.
The backend also stores a small local `window_preferences.json` using the existing
atomic JSON writer, restoring the close behavior during setup before frontend
hydration. No network preference service is involved. Native close is intercepted
for both modes so orderly cancellation and cleanup can finish before exit.

## Instance guard and progress tooltip

The Rust `tauri-plugin-single-instance` dependency is initialized before every
other plugin and before setup opens the existing recovery file lock. Its callback
restores the primary window using the same Show action as the tray. The plugin
rejects the secondary process; launch arguments are not executed or forwarded to
FFmpeg. This plugin has no JavaScript package or frontend command permissions.
The recovery file lock remains a separate safeguard against concurrent state use.

`publish()` derives a finite, clamped, integer batch percentage from existing
progress events and remembers the last tray value. Lifecycle/warning events with
an unchanged integer do not call the OS tray API. Active batch tooltips use
`Keroro Wii Encoder - 45%`; terminal states reset to `Keroro Wii Encoder`.
Previews retain the base tooltip. There is no timer or per-stderr-line refresh.

## Verification

- `npm run build`: passed TypeScript and Vite.
- Headless Chrome with Tauri IPC mocks: passed F11 enter/exit, repeated-key guard,
  saved tray preferences and failure rollback, strict viewport geometry at
  800x750, 800x710, 650x660, and 1920x1080, and long-log scrolling confined to the
  status panel. Checks include a visible wrapped recovery button and long notices.
- First-launch default, deprecated-key migration, restored false preferences, and
  the absence of an initial status scrollbar were checked with native API mocks.
- Existing mocked batch checks also passed: job locks, progress, stale events,
  diagnostics, recovery actions, permission errors, and missing tools.
- Rust formatting: checked. Native compilation and actual multi-process/tray/F11
  interactions are not certified. `cargo fetch` failed because this environment
  cannot resolve `index.crates.io`; the new single-instance plugin and earlier
  encoding_rs/num_cpus dependencies still require fetching. Cargo lockfiles must
  be resolved when connectivity returns. No installer was rebuilt.

After dependencies are available, run `cargo check --manifest-path src-tauri/Cargo.toml`,
then `npm run tauri -- dev`. Test both close modes while idle and encoding, tray
Exit/Show, a duplicate launch while hidden/minimized, tooltip integer transitions,
and F11 at the minimum and maximum window sizes. The canonical release build is
`npm run build:windows`; its configured cleanup hook remains mandatory.

Official API references:

- https://v2.tauri.app/plugin/single-instance/
- https://v2.tauri.app/reference/javascript/api/namespacewindow/

## Naming and asynchronous start safety

New custom-title outputs and the live hint use `Title - Ep01 [Wii].mp4`.
Blank titles retain the original `_Wii.mp4` fallback. Interrupted queues created
by older builds preserve their frozen E01 names when resumed. New root audio
assets are optional and discovered by Vite when rebuilding; the old sounds.ts
module has been removed to keep one audio/mute manager.

Start locks synchronously before any asynchronous work; pending-start state also
keeps controls disabled. Accepted tasks unlock on terminal backend state, while
rejected preflight requests restore controls and play the error hook. Delayed-IPC
browser checks verify repeated clicks do not dispatch overlapping starts.

Starting batches/previews, resume, recovery access, preview reads, and close
preference writes remain async Tauri commands. Canary/write checks use async
filesystem operations, disk queries use spawn_blocking, and checkpoint hashing and
persistence use blocking workers. The synchronous get_batch_state command only
reads the small managed progress snapshot.

Audio feedback browser checks (with mocked HTML5 playback) and TypeScript/Vite
build passed. All 27 cached-dependency Rust component tests passed, including Ep
naming and legacy queue recovery. Full native compilation still requires fetching
the uncached dependencies described above; no installer was rebuilt or listening
test performed with user-provided sound files.

## Smart native drag and drop

Drop one folder or a media/subtitle pair onto the main window. The overlay says
Drop Media Here, Drop Subtitle Here for subtitle filenames, or Drop Media &
Subtitles for multiple paths. Native Tauri 2 drag-enter/drag-drop/drag-leave events
are handled; the requested drop and drag-cancel aliases are also supported. All
listeners are removed at teardown. Browser file-drop APIs are not used to infer
native file paths.

The async `analyze_dropped_paths` command runs `drop_scan::analyze` on a blocking
worker. It returns media_path/sub_path/output_path as local folder strings or null.
Extended Windows drive prefixes are stripped from incoming/UI path strings;
canonical native paths remain available internally for filesystem work.
The scanner accepts up to 64 paths and inspects at most 512 entries per directory,
without recursion or reading file contents. Video matching uses the existing
MP4/MKV/AVI/MOV/WMV/FLV/WebM list; subtitles support SRT/ASS/SSA/VTT, ignoring case.
The first recognized media folder wins; dedicated subtitle folders take precedence
over mixed media/subtitle folders. Individual files select their parent folder;
this does not restrict the batch to only the dropped file. No network shares or
symbolic-link/junction roots are accepted.

The overlay disappears immediately on drop, leave, or cancellation. Fields and
start actions lock during analysis, and both returned folders are applied in one
step and saved to preferences. An explicit subtitle choice survives a subsequent
media-only drop, including a mixed folder. A separate subtitle drop overrides it;
Use anime folder restores automatic following. Unsupported or unreadable drops
preserve the form and display a recoverable error. Drops are ignored while a task,
a folder dialog, or a modal is active, and on shutdown. Drag-and-drop never starts encoding automatically. A completely empty directory
populates Output; a nonempty folder containing only unrelated files or nested
folders is not classified as an output folder. A single-path hover check updates
the overlay to Drop Folder to set Output for an empty folder. Hover does not
change the form, and version checks ignore stale results after leaving, canceling,
starting a task, or dropping. Drop execution rescans before applying its result.

Verification: the TypeScript/Vite build and mocked native drag-event browser
checks passed, including paired folders, overlay copy/cancellation, duplicate-scan
locking, subtitle overrides, persisted paths, error recovery, and rejected drops
during encoding. All 30 available Rust component tests passed, covering scanner
format detection, Unicode, file-parent selection, and shallow scanning. The native
app build and real Windows drag/drop smoke test remain pending uncached Rust
dependencies; no release executable was rebuilt.

## Queue-wide smart subtitle pairing

`encoder-core/src/media.rs` reuses the existing regex dependency to extract
confident episode numbers. For example, [Subs] Keroro 01 (1080p).mkv matches an
otherwise differently named episode01.ass; episode 01 cannot match episode 101.
Exact case-insensitive stems disambiguate before numeric matching. Generic numbers
that look like resolutions/years and adjacent competing episode tokens are not
considered confident. Subtitle episode/stem indexes are built once per queue.

Unique primary matches are reserved globally. Remaining videos use the subtitle
at the same absolute position in independently natural-sorted filename lists
(case-insensitive, numeric 2 before 10). The fallback does not compact leftovers,
overwrite primary selections, or reuse one subtitle for multiple videos. If its
absolute-index subtitle is already reserved or absent, the video stays unpaired.
Fallback can choose a subtitle without a reliable episode match, so verify ordering
with a preview when filenames do not identify episodes clearly.

Queue sorting and pairing run on a blocking worker through the existing async
preparation path. Preview computes the full pairing before truncating to its first
item. The checkpoint freezes selected subtitle paths for subsequent recovery.
Fallback, ambiguity, unused subtitles, and orphaned videos are surfaced through
existing status warnings and log::warn! to keroro-encoder.log. Mismatched file
counts do not abort encoding; unpaired videos retain the video-only fallback.

FFmpeg arguments remain native Path/OsStr arguments with no shell. Original
subtitle paths are not embedded in the filter expression: normalization stages
UTF-8 into the existing isolated temporary directory, and only an ASCII-safe
basename is passed to the filter. No shell quotes are added to native paths.

All 35 available Rust component tests passed, covering number matching across
names, numeric boundaries, adjacent ambiguous numbers, natural index fallback,
non-reuse, competing claims, and count mismatches alongside prior recovery and
process checks. Full native app compilation and media integration tests remain
pending the uncached dependencies; no executable was rebuilt for this change.

### Extended paths and empty output folders

Dropped Windows drive paths such as \\?\D:\Anime are normalized to D:\Anime in
the UI response. UNC and device namespaces remain subject to the existing offline
rejection guard. Unicode content and canonical filesystem paths are preserved.
The first completely empty folder in a drop becomes output_path. Every encountered
entry counts toward non-emptiness, including hidden files and subdirectories;
checking emptiness does not require traversing a large directory in full.

Media, subtitle, and output results can be applied in one drop, with output-only
drops preserving the other fields. The Output path is persisted through the
existing preferences schema. A hover result is only a hint: the authoritative scan
on drop detects changes made since entering the window.

Verification: TypeScript/Vite build and mocked native-event browser checks passed
for the output hover hint, persisted output-only drops, three-folder routing,
stale/canceled hover responses, and prior job locks. All 37 available Rust component
tests passed, including extended drive-prefix handling, hidden-file/subdirectory
non-emptiness, and combined media/subtitle/output detection. No native executable
was rebuilt; full native verification remains blocked by uncached dependencies.

## Live current-file progress and ETA

A hidden-by-default progress strip below Start Platoon Downscale appears while a
batch is starting/running. It displays Encoding Ep01... 45% (ETA: 02m 15s), tracks
the current file, and resets/hides on completion, failure, cancellation, or teardown.
The status panel retains the batch overview. Preview generation does not show the
batch-only strip. A failed preflight start also hides it.

The FFmpeg child retains piped stdout and stderr. ffprobe duration and structured
out_time_us/out_time/speed remain authoritative. A bounded asynchronous stderr
monitor now reads Duration, time=, and speed= from info-level output and carriage-
return status updates as fallback telemetry. FFmpeg emits stats every 0.25 seconds.
Only the latest stderr metrics are retained through a watch channel; the diagnostic
tail still holds at most 20 lines of 2048 bytes. Actual warning/error lines are
logged immediately so later informational stats cannot erase early diagnostics.
Informational output does not generate false subtitle warnings.

ETA is max(duration - processed_time, 0) / speed. Missing duration, nonpositive or
invalid speed, and nonfinite values produce an unknown ETA; no elapsed-time guess
is fabricated. Preview duration is capped at ten seconds. The displayed estimate
is for the current file, not the entire queue, and excludes final hashing/muxing
work. File percentage stays below 100 until a successful process exit and final
output commit.

The native publisher emits encoding-progress alongside the existing wii-progress
snapshot. Its payload contains file, percentage (number or null), eta, and jobId,
sequence, and phase for stale-event rejection. Progress snapshots include
etaSeconds for reconnect/reload rendering. The frontend uses textContent and
clamped percentage values. Old jobs and out-of-order terminal events cannot
replace the active label or clear the strip. Native progress events are suppressed
unless the integer percentage or ETA second changes (at most about four timing
updates per second); tray tooltip refreshes remain integer-percentage-only.

Verification: frontend build and automated browser checks passed for live ETA,
unknown-duration mode, stale/wrong-job events, terminal reset, and active-window
layout. All 41 cached-dependency Rust component tests passed, including a real
bundled-FFmpeg test reading duration/time/speed from stderr with a Unicode and
special-character fixture path. The component harness does not compile the whole
Tauri app; full native compilation remains blocked by uncached dependencies. No
installer or final executable was rebuilt.

## Cancel button, partial outputs, and smart skip

Once a batch is accepted, Start Platoon Downscale becomes an enabled red
Cancel Downscale button. Paths, quality, and theme controls stay locked. During
start handshakes and cancellation cleanup the action is temporarily disabled;
Canceling Downscale... remains until the backend confirms process cleanup. Failed
cancellation requests do not unlock another Start while a task is still running.
Progress bars reset immediately, and late progress cannot refill them during cancel.
Natural completion restores the normal Start state.

`cancel_downscale` targets the exact job ID in managed BatchManager/ActiveProcess
state. It signals the existing cooperative cancellation flag, waits for the worker
and process reaper (bounded graceful q followed by forced kill if needed), then
clears the intentionally canceled queue checkpoint. A cancellation reservation
blocks new backend jobs during that cleanup interval. No second child owner or
shared_child dependency is introduced. Cancel-and-exit and crash recovery retain
their frozen checkpoints; explicit Cancel allows a fresh Start to rescan and skip.

The temporary output is now final-name.mp4.part, exclusively reserved before
FFmpeg starts. `-f mp4` is required because FFmpeg cannot infer the muxer from .part.
Successful output is synchronized and finalized through the existing same-directory
no-overwrite rename, including checkpoint commit intents and checksums. Failure or
cancellation removes the owned partial after the process has been reaped. Unknown
preexisting .part files are preserved; owned orphan parts remain eligible for the
identity-checked startup sweeper. Legacy .part.mp4 ledger entries remain supported.

Before encoding each batch episode, an existing regular final MP4 is skipped
without ffprobe, subtitle decoding, or FFmpeg execution. Existing files are never
validated by decoding or overwritten by this feature: presence is the requested
skip rule. Skipped files have a separate count in the UI and final summary. Partial
files alone do not cause a skip; a directory/link at a final destination is rejected.

The checkpoint stores a backward-compatible skipped-index map with size/modified
stamps for preexisting finals. This avoids hashing the entire output folder just
to skip files, while still detecting changed skipped files on recovery. Newly
encoded outputs retain their SHA-256 checksums. Finals that appear after the initial
checkpoint can be adopted without renumbering the frozen queue. Older checkpoints
without a skipped map still load. Only completed MP4 names are used for smart skip.

Verification: frontend build and mocked browser checks passed for Cancel toggling,
duplicate cancel clicks, waiting for cleanup, late progress, cancellation failures,
restored Start state, and skipped counts. All 46 cached-dependency Rust component
tests passed, including existing-file preservation, legacy checkpoints, skip-map
recovery, and real FFmpeg MP4-part muxing/rename/decoding. The updated full media
integration expectations require a native dependency-resolved build to run. Full
Tauri compilation remains blocked by uncached dependencies; no executable was rebuilt.

### Latest native validation

The previously unavailable Rust dependencies are now cached and both application
and core Cargo lockfiles are resolved. `cargo check --offline --locked` (including
tests) passed, followed by the real core test suite and Tauri library tests. The
rerun integration test substitutes nonexistent FFmpeg/ffprobe paths and still
succeeds with one skipped final, proving no media tool is started on the skip
path. Process cancellation and removal of actual partial outputs passed in the
live encoder test. The old test assertion that existing files must fail was
updated to the newly requested skip behavior; existing bytes remain preserved.

Native tray/button interactions in an installed GUI and final release packaging
still require manual release testing; these Rust tests do not replace that check.
