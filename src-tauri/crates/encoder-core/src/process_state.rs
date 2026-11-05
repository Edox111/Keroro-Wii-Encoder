use std::{
    process::ExitStatus,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{io::AsyncWriteExt, process::Child};

/// One authoritative FFmpeg/ffprobe child slot, shared with the application.
/// Never hold this synchronous mutex across an await.
#[derive(Default)]
pub struct ActiveProcess(pub Mutex<Option<Child>>, AtomicBool);

pub(crate) struct ProcessLease {
    state: Arc<ActiveProcess>,
    subtitle: Option<crate::subtitles::StagedSubtitle>,
}

impl ActiveProcess {
    pub fn is_paused(&self) -> bool {
        self.1.load(Ordering::Acquire)
    }

    pub fn set_paused(&self, paused: bool) -> Result<(), String> {
        let mut slot = self.0.lock().map_err(|e| e.to_string())?;
        let child = slot
            .as_mut()
            .ok_or("No FFmpeg process is active; wait for encoding to start.")?;
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return Err("The media process has already finished.".into());
        }
        if child.stdin.is_none() {
            return Err(
                "The encoder is preparing or shutting down; try again when encoding starts.".into(),
            );
        }
        if self.is_paused() != paused {
            suspend_child(child, paused)?;
            self.1.store(paused, Ordering::Release);
        }
        Ok(())
    }

    /// Wait for any unwind/cancellation reaper before releasing the job reservation.
    pub async fn wait_until_idle(&self) {
        loop {
            let occupied = match self.0.lock() {
                Ok(slot) => slot.is_some(),
                Err(poisoned) => poisoned.into_inner().is_some(),
            };
            if !occupied {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
    pub(crate) fn track(self: &Arc<Self>, mut child: Child) -> Result<ProcessLease, String> {
        if child.try_wait().map_err(|e| e.to_string())?.is_none()
            && let Err(error) = crate::job_fence::assign(&child)
        {
            // Very short probes can exit in the spawn/assignment interval.
            if child.try_wait().map_err(|e| e.to_string())?.is_none() {
                return Err(error);
            }
        }
        let mut slot = self
            .0
            .lock()
            .map_err(|e| format!("Process state unavailable: {e}"))?;
        if slot.is_some() {
            return Err("Another media process is already active.".into());
        }
        self.1.store(false, Ordering::Release);
        *slot = Some(child);
        Ok(ProcessLease {
            state: self.clone(),
            subtitle: None,
        })
    }
}

// Native NT process operations suspend/resume all threads, including FFmpeg workers.
#[cfg(windows)]
fn suspend_child(child: &Child, paused: bool) -> Result<(), String> {
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtSuspendProcess(process: windows_sys::Win32::Foundation::HANDLE) -> i32;
        fn NtResumeProcess(process: windows_sys::Win32::Foundation::HANDLE) -> i32;
    }
    let handle = child
        .raw_handle()
        .ok_or("FFmpeg process handle is unavailable.")?;
    // SAFETY: the child slot mutex keeps this owned process handle alive throughout
    // the call. Only this state owner changes its suspend count, idempotently.
    let status = unsafe {
        if paused {
            NtSuspendProcess(handle)
        } else {
            NtResumeProcess(handle)
        }
    };
    if status < 0 {
        return Err(format!(
            "Cannot {} FFmpeg (NTSTATUS {status:#010x}).",
            if paused { "pause" } else { "resume" }
        ));
    }
    Ok(())
}
#[cfg(not(windows))]
fn suspend_child(_: &Child, _: bool) -> Result<(), String> {
    Err("OS-level pause/resume is currently supported on Windows only.".into())
}

impl ProcessLease {
    pub(crate) fn hold_subtitle(&mut self, subtitle: Option<crate::subtitles::StagedSubtitle>) {
        self.subtitle = subtitle;
    }
    pub(crate) async fn wait(&self) -> Result<ExitStatus, String> {
        loop {
            let status = {
                let mut slot = self.state.0.lock().map_err(|e| e.to_string())?;
                slot.as_mut()
                    .ok_or("Media process handle is missing.")?
                    .try_wait()
                    .map_err(|e| format!("Cannot wait for media process: {e}"))?
            };
            if let Some(status) = status {
                return Ok(status);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    pub(crate) async fn kill_and_wait(&self) -> Result<(), String> {
        {
            let mut slot = self.state.0.lock().map_err(|e| e.to_string())?;
            let child = slot.as_mut().ok_or("Media process handle is missing.")?;
            if child.try_wait().map_err(|e| e.to_string())?.is_none() {
                child
                    .start_kill()
                    .map_err(|e| format!("Cannot terminate media process: {e}"))?;
            }
        }
        self.wait().await.map(|_| ())
    }

    pub(crate) async fn stop(&self) -> Result<(), String> {
        let stdin = {
            let mut slot = self.state.0.lock().map_err(|e| e.to_string())?;
            let child = slot.as_mut().ok_or("Media process handle is missing.")?;
            if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                return Ok(());
            }
            if self.state.is_paused() {
                suspend_child(child, false)?;
                self.state.1.store(false, Ordering::Release);
            }
            child.stdin.take()
        };
        // A stalled stdin must not prevent the bounded graceful shutdown.
        if let Some(mut stdin) = stdin {
            let _ = tokio::time::timeout(Duration::from_millis(250), stdin.write_all(b"q\n")).await;
        }
        if let Ok(status) = tokio::time::timeout(Duration::from_secs(2), self.wait()).await {
            return status.map(|_| ());
        }
        self.kill_and_wait().await
    }
}

impl Drop for ProcessLease {
    fn drop(&mut self) {
        // An unwinding worker must not delete a subtitle while FFmpeg still reads it.
        let subtitle = self.subtitle.take();
        let mut slot = match self.state.0.lock() {
            Ok(slot) => slot,
            // Cleanup must still run after a panic poisoned the slot.
            Err(poisoned) => poisoned.into_inner(),
        };
        self.state.1.store(false, Ordering::Release);
        let Some(child) = slot.as_mut() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(_)) => {
                slot.take();
                drop(slot);
                if let Some(subtitle) = subtitle
                    && let Err(error) = subtitle.close()
                {
                    log::warn!("{error}");
                }
                return;
            } // Already reaped.
            Ok(None) | Err(_) => {
                if let Err(error) = child.start_kill() {
                    log::error!("Cannot stop dropped media process: {error}");
                }
            }
        }
        // Dropping a worker future or unwinding must never detach a live child.
        // Commands also set kill_on_drop(true) as a fallback without a runtime.
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let state = self.state.clone();
            drop(slot);
            runtime.spawn(async move {
                loop {
                    let done = {
                        let mut slot = match state.0.lock() {
                            Ok(slot) => slot,
                            Err(poisoned) => poisoned.into_inner(),
                        };
                        match slot.as_mut().map(|child| child.try_wait()) {
                            Some(Ok(None)) => false,
                            Some(Err(error)) => {
                                log::error!("Cannot inspect dropped media process: {error}");
                                // Retain ownership and retry termination; never declare a live child idle.
                                if let Some(child) = slot.as_mut() {
                                    let _ = child.start_kill();
                                }
                                false
                            }
                            Some(Ok(Some(_))) | None => true,
                        }
                    };
                    if done {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                if let Some(subtitle) = subtitle
                    && let Err(error) = subtitle.close()
                {
                    log::error!("{error}");
                }
                let mut slot = match state.0.lock() {
                    Ok(slot) => slot,
                    Err(poisoned) => poisoned.into_inner(),
                };
                slot.take();
            });
        } else {
            // Runtime teardown: Child's kill_on_drop is the final fallback.
            slot.take();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;
    use tokio::sync::oneshot;

    fn fixture(live: bool) -> Child {
        let mut command = tokio::process::Command::new(
            std::env::var_os("KERORO_TEST_FFMPEG").expect("FFmpeg fixture path"),
        );
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000);
        if live {
            command.args([
                "-re",
                "-f",
                "lavfi",
                "-i",
                "color=size=64x64:rate=10",
                "-t",
                "60",
                "-f",
                "null",
                "-",
            ]);
        } else {
            command.arg("-version");
        }
        command.spawn().expect("spawn fixture")
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "Requires bundled FFmpeg"]
    async fn pause_freezes_cpu_and_partial_output_then_resumes_same_process() {
        use windows_sys::Win32::{Foundation::FILETIME, System::Threading::GetProcessTimes};
        fn cpu(state: &ActiveProcess) -> u64 {
            let slot = state.0.lock().expect("slot");
            let handle = slot.as_ref().expect("child").raw_handle().expect("handle");
            let (mut creation, mut exit, mut kernel, mut user) = (
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
            );
            // SAFETY: the slot lock retains the child and four writable FILETIME values.
            assert_ne!(
                unsafe {
                    GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user)
                },
                0
            );
            let ticks = |t: FILETIME| ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64;
            ticks(kernel) + ticks(user)
        }
        let root = tempfile::tempdir().expect("fixture");
        let part = root.path().join("episode.mp4.part");
        let mut command =
            tokio::process::Command::new(std::env::var_os("KERORO_TEST_FFMPEG").expect("binary"));
        command
            .creation_flags(0x0800_0000)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-re",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=320x240:rate=25",
                "-t",
                "3",
                "-c:v",
                "libx264",
                "-threads",
                "1",
                "-movflags",
                "frag_keyframe+empty_moov",
                "-f",
                "mp4",
            ])
            .arg(&part)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let state = Arc::new(ActiveProcess::default());
        let lease = state.track(command.spawn().expect("spawn")).expect("track");
        tokio::time::sleep(Duration::from_millis(500)).await;
        let pid = state.0.lock().expect("slot").as_ref().expect("child").id();
        state.set_paused(true).expect("pause");
        state.set_paused(true).expect("idempotent pause");
        let before = cpu(&state);
        let length = std::fs::metadata(&part).expect("partial output").len();
        tokio::time::sleep(Duration::from_millis(400)).await;
        assert_eq!(cpu(&state), before, "Suspended FFmpeg consumed CPU");
        assert_eq!(
            std::fs::metadata(&part).expect("partial retained").len(),
            length
        );
        assert!(state.is_paused());
        state.set_paused(false).expect("resume");
        state.set_paused(false).expect("idempotent resume");
        assert_eq!(
            state.0.lock().expect("slot").as_ref().expect("child").id(),
            pid
        );
        assert!(
            tokio::time::timeout(Duration::from_secs(7), lease.wait())
                .await
                .expect("resumed")
                .expect("wait")
                .success()
        );
        drop(lease);
        assert!(!state.is_paused());
        assert!(std::fs::metadata(part).expect("completed partial").len() > length);
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "Requires bundled FFmpeg"]
    async fn exit_cleanup_resumes_a_paused_child_before_stopping() {
        let mut command =
            tokio::process::Command::new(std::env::var_os("KERORO_TEST_FFMPEG").expect("binary"));
        command
            .creation_flags(0x0800_0000)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-re",
                "-f",
                "lavfi",
                "-i",
                "color=size=64x64:rate=10",
                "-f",
                "null",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let state = Arc::new(ActiveProcess::default());
        let lease = state.track(command.spawn().expect("spawn")).expect("track");
        tokio::time::sleep(Duration::from_millis(300)).await;
        state.set_paused(true).expect("pause");
        tokio::time::timeout(Duration::from_secs(4), lease.stop())
            .await
            .expect("bounded stop")
            .expect("stopped");
        drop(lease);
        assert!(!state.is_paused());
        assert!(state.0.lock().expect("slot").is_none());
    }

    #[tokio::test]
    #[ignore = "Requires bundled FFmpeg"]
    async fn normal_exit_reaps_child_and_clears_slot() {
        let state = Arc::new(ActiveProcess::default());
        let lease = state.track(fixture(false)).expect("track");
        assert!(
            state
                .0
                .try_lock()
                .expect("mutex is not held across awaits")
                .is_some()
        );
        assert!(lease.wait().await.expect("wait").success());
        drop(lease);
        assert!(state.0.lock().expect("state").is_none());
    }

    #[tokio::test]
    #[ignore = "Requires bundled FFmpeg"]
    async fn early_error_stops_reaps_and_clears_child() {
        let state = Arc::new(ActiveProcess::default());
        let result: Result<(), String> = async {
            let _lease = state.track(fixture(true))?;
            Err("Injected pipe/read/setup failure".into())
        }
        .await;
        assert!(result.is_err());
        tokio::time::timeout(Duration::from_secs(3), state.wait_until_idle())
            .await
            .expect("error cleanup reaped child");
        assert!(state.0.lock().expect("state").is_none());
    }

    async fn dropped_worker_is_reaped(panic_worker: bool) {
        let state = Arc::new(ActiveProcess::default());
        let worker_state = state.clone();
        let (registered, registration) = oneshot::channel();
        let (trigger, proceed) = oneshot::channel::<()>();
        let worker = tokio::spawn(async move {
            let mut lease = worker_state.track(fixture(true)).expect("track");
            let staged = crate::subtitles::stage("subtitle fixture".into(), "srt")
                .await
                .expect("stage");
            let paths = (
                staged.path().to_path_buf(),
                staged.directory().to_path_buf(),
            );
            lease.hold_subtitle(Some(staged));
            registered.send(paths).expect("registration receiver");
            let _ = proceed.await;
            if panic_worker {
                panic!("Injected worker panic");
            }
            std::future::pending::<()>().await;
        });
        let (subtitle_path, subtitle_directory) = registration.await.expect("registered");
        assert!(state.0.try_lock().expect("short mutex scope").is_some());
        if panic_worker {
            trigger.send(()).expect("trigger panic");
        } else {
            worker.abort();
        }
        let failure = worker.await.expect_err("worker interrupted");
        assert_eq!(failure.is_panic(), panic_worker);
        tokio::time::timeout(Duration::from_secs(3), state.wait_until_idle())
            .await
            .expect("child reaped promptly");
        assert!(
            !subtitle_path.exists() && !subtitle_directory.exists(),
            "Subtitle resources must outlive FFmpeg, then be removed"
        );
        assert!(state.0.lock().expect("state").is_none());
        // The slot remains reusable after unwinding or dropping a future.
        let lease = state.track(fixture(false)).expect("reuse slot");
        assert!(lease.wait().await.expect("wait").success());
        drop(lease);
        assert!(state.0.lock().expect("state").is_none());
    }

    #[tokio::test]
    #[ignore = "Requires bundled FFmpeg"]
    async fn worker_panic_stops_reaps_and_clears_child() {
        dropped_worker_is_reaped(true).await;
    }

    #[tokio::test]
    #[ignore = "Requires bundled FFmpeg"]
    async fn worker_abort_stops_reaps_and_clears_child() {
        dropped_worker_is_reaped(false).await;
    }
}
