# Wave 1: Process and state safety

Current close-to-tray, sound controls, and F11 behavior are described in
[UI_LIFECYCLE.md](UI_LIFECYCLE.md); it supersedes older lifecycle/copy descriptions below.

Changes are applied to the existing v1.1 source tree. No version downgrade or
duplicate application tree was introduced.

## Frontend lock

`src/main.ts` defines `setEncodingState(isEncoding: boolean)`. It disables paths,
title, quality, all Browse buttons, subtitle reset, character buttons,
notifications, batch start, and preview. Mute, UI sound controls, minimize to tray,
and preview close remain utilities. Disabled buttons have reduced opacity and a
`not-allowed` cursor. The card exposes `aria-busy`.

The handler checks `ready`, `busy`, `starting`, `previewLoading`, `closing`, and
`disposed`. It sets `starting` and locks synchronously before its first `await`.
Validation errors never acquire the lock. Preparation, submission, and status
refresh are inside `try/catch/finally`. Failure to start restores controls. If
the backend has already accepted a job but fetching its snapshot fails, the UI
stays locked until a terminal progress event arrives. A terminal event releases
the form unless application shutdown is underway. Old event sequences remain
ignored.

## Child ownership and cleanup

The concrete type is in the independently testable media core,
`src-tauri/crates/encoder-core/src/process_state.rs`:

```rust
use std::sync::Mutex;
use tokio::process::Child;

#[derive(Default)]
pub struct ActiveProcess(pub Mutex<Option<Child>>);
```

`src-tauri/src/lib.rs` re-exports this type and manages the **same** shared slot:

```rust
pub use keroro_encoder_core::ActiveProcess;

// In tauri::Builder:
.manage(Arc::new(ActiveProcess::default()))
```

The core's `run_batch_with_session` receives that `Arc<ActiveProcess>`. FFprobe and
FFmpeg each register their real Tokio `Child` in the slot after spawn and pipe
extraction. A `ProcessLease` owns the registration. Synchronous locks only cover
short handle operations; they are never held across `.await`. `try_wait()` is
polled with asynchronous 25 ms sleeps, allowing both the UI and cancellation
to stay responsive.

Normal completion reaps the child and drops the lease. Error returns, worker
abortion, and panic unwinding drop the lease too. Its destructor requests
termination, then schedules a reaper retaining the slot until the child exits.
Both commands also use `kill_on_drop(true)` as a runtime-teardown fallback.
`wait_until_idle()` completes before output cleanup, the next item, or publishing
the final job result. No second media child can claim an occupied slot.

The Tauri layer reserves a task **before** its first asynchronous operation,
preventing concurrent IPC requests from starting competing jobs. A separate
supervisor joins the worker. If it panics, the supervisor waits for child cleanup
and publishes a sequenced `failed` result, unlocking the frontend. Panic recovery
requires `panic = "unwind"` in the release Cargo profile; `panic = "abort"` skips
Rust destructors. Completed outputs remain intact. A panic can bypass temporary
file removal; the error message explicitly identifies that cleanup may be needed.

## Native close guard

The existing native `WindowEvent::CloseRequested` handler prevents immediate
destruction and calls `request_close`. Active **job** state guards preparation,
probing, gaps between files, encoding, and cleanup, rather than relying solely
on whether a child exists at that instant.

The native dialog says:

> Conversion is currently ongoing! Are you sure you want to cancel and exit?

Only one prompt can be open. Declining leaves the cancellation token unchanged.
Confirming blocks new jobs and signals cancellation. FFmpeg receives `q` through
stdin; stdin writing is bounded to 250 ms, followed by up to two seconds to exit.
A stalled process is then terminated and reaped. The app waits for the worker's
finished signal and preview cleanup before exiting. Tray Exit uses the same guard.
Idle close retains the existing minimize-to-tray behavior.

## Complete source and verification

[V1_1_CODE.md](V1_1_CODE.md) contains the complete current TypeScript, CSS, Rust,
and Cargo configuration blocks, including the new process-state module. Apply
the modules together; a second independent `Child` mutex in Tauri would not track
the child's actual owner.

Tests cover concurrent backend reservations, prompt/exit start rejection, normal
process completion, early errors, actual worker panic/abort cleanup, slot reuse,
live graceful cancellation, and the existing media pipeline. Browser checks use
mocked IPC to exercise rapid clicks, utility exceptions, full form locking,
terminal failure recovery, and accepted-job snapshot failure. Native Windows
confirmation interaction still needs a manual desktop test.

Release packaging continues to use the project's clean-first hook. The previous
portable archive is not a build of these source changes.
