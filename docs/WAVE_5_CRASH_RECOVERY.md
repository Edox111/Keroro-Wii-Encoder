# Wave 5: Crash recovery and local diagnostics

Current dependency resolution and native compilation now pass. See the latest
verification in [UI_LIFECYCLE.md](UI_LIFECYCLE.md); older blocker notes below are
historical, and no final release installer has been rebuilt.

The implementation is in the existing project. Tauri adapters and setup are in
`src-tauri/src/lib.rs`; media, persistence, and cleanup live in the independently
testable `src-tauri/crates/encoder-core/src/` modules. The frontend is
`src/main.ts`, with dialogs in the root `index.html` and `src/styles.css`.
Complete code listings are in [V1_1_CODE.md](V1_1_CODE.md).

## Local logging and panic handling

The first setup operations resolve Tauri's native `app_log_dir()` and install the
`log` sink plus a global panic hook. Info, warning, and error records append to
`keroro-encoder.log`, with epoch-millisecond timestamps, severity, and target.
The hook captures `Backtrace::force_capture()` and synchronizes the log. Log
rotation normally retains the current file and one 16 MiB backup. A panic-safe
fallback avoids waiting for an already-held logger mutex. Rotation warnings are written directly through the local sink without recursive
logging. Disk failures can prevent reliable logging; no console fallback is used.

There is no network logging sink or telemetry client. Paths and diagnostics can
contain private filenames; the log and recovery JSON remain local. Explicit UNC,
Windows device, and mapped network-drive inputs are rejected, and FFmpeg/ffprobe
receive `-protocol_whitelist file,pipe`. This constrains supported media access;
it is not a general operating-system sandbox or a guarantee against codec bugs.

A panic hook records an unwinding panic; it cannot make a fatal process survive,
catch all native access violations, handle every OOM condition, or log after a
power outage. The existing worker supervisor handles unwind failures and publishes
a terminal frontend state after process reaping.

## Startup cleanup and output rollback

`artifacts.rs` maintains an atomically saved, bounded `artifacts.json` ownership
ledger in the native app data directory. An exclusively created output/probe/
subtitle temporary file is registered before content is written. Startup checks
only these recorded paths in the cache workspace and output directories. It
requires the expected private name and recorded file identity, rejects symlinks
and junctions, and avoids deleting a file still open by a surviving encoder.

This deliberately does not delete arbitrary files named `preview_temp.mp4`,
`.keroro_access_test.tmp`, or `*.tmp` in user folders. Older or unrecorded files
are preserved. New canaries use unique names instead of a shared predictable
name. A crash between file creation and registration can leave an unrecorded
empty artifact; automatic deletion never guesses ownership. Locked or replaced
files produce warnings in the local log.

FFmpeg writes to a reserved `.mp4.part` beside the intended destination; the MP4 muxer is selected explicitly. On a
nonzero exit, cancellation, or stream/wait error, the process is stopped/reaped
before its partial output and subtitle files are removed. Cleanup errors are
reported and retained in the ledger for a later startup attempt. Existing final
media is never removed as rollback. Successful output is synchronized and moved
with a same-directory, no-overwrite rename. There is no direct-copy fallback
that exposes a partially copied final MP4 on FAT/exFAT. Unsupported or denied
renames fail safely instead of weakening this rule.

Windows FFmpeg and ffprobe children are assigned to a non-inheritable job object
with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. The OS terminates assigned children if
the parent dies without running Rust destructors. Normal cancellation still uses
the existing cooperative stop and bounded forced termination. Assignment has a
small spawn/assignment interval; it is not an absolute orphan-proof guarantee.

## Diagnostic tail and frontend errors

`diagnostics.rs` retains the last 20 stderr lines. Normal valid UTF-8 lines are
preserved; each line has a 2048-byte bound so hostile unbounded lines cannot grow
memory indefinitely. Overlong lines are marked truncated and invalid UTF-8 is
rendered lossily. On FFmpeg failure the tail is written to the local error log.
The UI receives a concise failure reason and log reference rather than the raw
stderr block. Per-file failures continue to the next independent episode.

Tauri commands return `Result<_, String>` and map filesystem, worker-join, and
serialization failures into rejected IPC calls. `main.ts` handles these failures
with a native HTML dialog, using `textContent` for backend strings. Controls are
restored for rejected starts; an already accepted task stays locked until its
terminal state arrives. Result mapping does not itself impose an IPC timeout or
recover a completely dead native process.

## Atomic batch checkpoints and resume

`session.rs` keeps `batch_state.json` in Tauri's native app data directory. It
stores a version, job ID, settings, the complete original ordered queue, source
size/modified stamps, completed indices, output SHA-256 checksums, and an optional
commit intent. A native exclusive `runtime.lock` prevents two app instances from
owning this recovery state simultaneously. State JSON is capped at 8 MiB, written
to a securely created file in the same directory, flushed, synchronized, then
atomically replaced. A failed or oversized write preserves the prior checkpoint.

Each successful item follows this sequence:

1. Synchronize and hash its private completed MP4.
2. Persist an intent containing the original index, output size, and SHA-256.
3. Rename the private MP4 to its final name without overwriting an existing file.
4. Persist the completed index and clear the intent.

If the app dies after step 3, startup recovery checks the output against the
intent and marks it complete. A completed destination is never deleted because
a later checkpoint write failed. On Resume, completed files are checked by hash;
pending sources are checked by metadata. Changes, missing files, and unexpected
existing destinations block resume and preserve the files for review. Source
fingerprints are metadata checks, not full source-content hashes. Checksum work
runs on a blocking worker with cancellation checks between 64 KiB reads.

The queue is frozen: newly added files do not change original episode numbering.
Resume skips completed entries and starts remaining episodes from the beginning;
it does not continue encoding a damaged partial MP4. Natural completion clears
the checkpoint, including a completed batch with individual failures. Cancel-and-exit, worker panic, and checkpoint persistence failures retain it.
The explicit Cancel Downscale command clears its saved queue after successful
process cleanup; a later Start uses presence-based skipping. See UI_LIFECYCLE.md.

At launch the frontend asks: "An interrupted batch queue was detected. Would you
like to resume from episode X?" Resume restores the saved form values and starts
the frozen backend queue. Later retains the checkpoint and exposes a Review
interrupted batch button; a new batch remains blocked until recovery is resolved.
Discard removes only recovery state and preserves media. Malformed checkpoint
JSON is retained and can be archived through Discard instead of silently erased.

## Verification and limits

- Frontend: TypeScript/Vite build and automated Chrome checks for recovery prompt,
  Resume, Later, Discard, safe error text, form locks, stale events, permission
  errors, and missing tools.
- All 26 cached-dependency Rust component tests passed. The harness compiles the actual persistence, cleanup,
  diagnostics, process, power, preflight, and environment modules. Tests cover
  commit reconciliation, frozen naming, same-length output tampering, source
  changes, corrupt state, exclusive locks, bounded JSON, and preservation of
  user files. This harness does not replace compilation of the full app.
- Separate real-process smoke checks capture a worker panic/backtrace locally and
  verify Windows terminates assigned FFmpeg after an abrupt parent exit.
- Full application `cargo check --offline` remains blocked by the uncached
  `encoding_rs` dependency; `num_cpus` also requires fetching. Network DNS was
  unavailable in this environment. No new installer was built or certified.

When dependency access is restored:

```powershell
cargo fetch --manifest-path .\src-tauri\Cargo.toml
cargo fetch --manifest-path .\src-tauri\crates\encoder-core\Cargo.toml
cargo check --manifest-path .\src-tauri\Cargo.toml
npm run build
npm run test:encoder
npm run build:windows
```

Commit the dependency-resolved lockfiles after successful checks. The canonical
Windows build invokes the existing cleanup hook; do not bypass it. Exit any
running app through its tray menu if cleanup reports a locked executable.

Atomic rename, flushes, and checksums improve recovery but cannot guarantee data
survival across storage-controller failure, filesystem corruption, or all power
loss scenarios. FFmpeg success and nonempty output are checked; every decoded
frame is not independently verified. Installed-app tray/dialog behavior and real
power-failure testing remain manual validation work.
