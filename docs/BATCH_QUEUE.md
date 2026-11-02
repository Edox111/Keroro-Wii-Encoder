# Video file queue (v1.1.0)

Drop multiple videos or choose **Add video files** in Step 1. Supported formats
match the existing encoder: MP4, MKV, AVI, MOV, WMV, FLV, and WebM. Files are
queued in selection/drop order; repeated paths are ignored. Folder browsing and
the existing mixed media/subtitle/output-folder drop routing remain available.
Folder batches still use the native folder scanner; explicit video files use
the new frontend queue. The visible queue takes precedence over the input field
until it has finished and is cleared.

Rows display **Pending**, **Encoding**, **Done**, or **Error**, with error details
kept on the affected row. **Remove** only removes pending items, including while
another item encodes. **Clear Queue** appears when all retained rows are settled.
It does not delete source videos, completed output, or native checkpoints.

The Start button submits one file through the unchanged `start_batch` command.
It waits for that job's terminal `wii-progress` payload before submitting another.
An acknowledged job ID is not considered completion. Events carry job identity
and sequence; early completion is cached and stale/other-job events cannot advance
the queue. A snapshot poll supplements events. A brief, bounded retry handles
the existing native reservation's terminal-event/release handoff.

The native child remains the only encoder. Pause/Resume controls it through the
existing commands. A failed media item records its error and the queue proceeds.
Cancellation, loss of native state, or a recovery-persistence failure stops further
submissions so a possibly active encoder or recoverable checkpoint is preserved.
The frontend does not kill or suspend processes itself.

Step 3 shows **Processing X of Y** and overall queue progress beneath the action
buttons. Overall percentage weights each file equally; the original file progress
and ETA remain native FFmpeg measurements. Encoding settings and UI mode remain
locked across file boundaries. Themes remain interactive. The updater can still
check/download, but its install predicate blocks restart throughout the running
queue, including the gaps between native jobs.

Existing WiiMC profile/filter arguments are unchanged. Every submission uses the
selected video/audio settings. Automatic subtitle selection searches each queued
video's parent directory; an explicitly selected subtitle file/folder or cleared
subtitle field is respected for all items. The main input/subtitle form fields
are not rewritten to iterate through the queue.

When subtitle files are selected/dropped as a list, `matchSubtitles()` first
reserves exact basenames (extension ignored, case-insensitive NFC comparison).
It then alphabetically pairs only the remaining videos and subtitles by index.
The visible video order is preserved, each subtitle is consumed at most once,
and an unmatched item receives an explicit null subtitle instead of reusing the
scanner's first selection. Subtitle-only drops can match an existing pending
queue. Assignments and unused subtitle paths are saved locally; legacy saved
queue arrays still load. Changing the subtitle field, browsing another subtitle
source, or choosing Use anime folder clears pending explicit assignments.

The log container has a 150px preferred flex basis, no maximum height cap, and
internal vertical scrolling. It grows with available space and can shrink on
short windows without pushing controls outside the fixed viewport.

Naming uses the existing source-episode extraction and output naming rules.
Two items that resolve to the same output name are reported as a collision rather
than silently adopting one another's output. With a custom anime title, use files
with distinct episode numbers, or clear the title to retain distinct original
names. No new Rust output-name parameter is invented.

Queue rows are saved locally under `keroro.queue.v1`; no media is uploaded.
Interrupted Encoding rows restore as Pending, and nothing starts automatically.
Resume or discard the existing native checkpoint first. When restarting the
frontend queue, the backend's normal output verification/skip rules preserve
finished videos. Limits of 500 files and bounded saved-path/message sizes prevent
unbounded UI/storage growth. Storage restrictions leave the in-memory queue usable.

Implementation: `src/file-queue.ts` owns sequencing and state, `src/queue-view.ts`
updates the DOM, and `src/queue.css` styles the bounded list. The existing
`src/main.ts` connects these to current IPC and lifecycle guards; no React or
Tailwind dependencies are added. Unit/browser tests cover sequencing, errors,
early/stale events, removal, pause/resume, persistence, cleanup, and legacy UI.
