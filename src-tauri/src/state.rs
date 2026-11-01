//! Shared job reservation and desktop state; mutex guards never cross awaits.
use keroro_encoder_core::{Cancellation, Progress, TaskKind};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::watch;

#[derive(Clone)]
pub(super) struct ActiveTask {
    pub(super) job_id: String,
    pub(super) cancellation: Cancellation,
    pub(super) finished: watch::Receiver<bool>,
}
#[derive(Clone)]
pub(super) struct PreviewFile {
    pub(super) job_id: String,
    pub(super) path: PathBuf,
}
pub(super) struct RuntimeState {
    pub(super) active: Option<ActiveTask>,
    pub(super) progress: Option<Progress>,
    pub(super) preview: Option<PreviewFile>,
    pub(super) close_prompt_open: bool,
    pub(super) exiting: bool,
    pub(super) minimize_on_close: bool,
    pub(super) tray_percent: Option<u8>,
}
impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            active: None,
            progress: None,
            preview: None,
            close_prompt_open: false,
            exiting: false,
            minimize_on_close: true,
            tray_percent: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClosePreferences {
    pub(super) minimize_on_close: bool,
}

#[derive(Clone, Default)]
pub(super) struct BatchManager(pub(super) Arc<Mutex<RuntimeState>>);
pub(super) struct JobReservation {
    manager: BatchManager,
    job_id: String,
    finished: watch::Sender<bool>,
}
impl BatchManager {
    pub(super) fn control_pause(
        &self,
        job_id: &str,
        paused: bool,
        control: impl FnOnce() -> Result<(), String>,
    ) -> Result<Progress, String> {
        let mut state = self.0.lock().map_err(|e| e.to_string())?;
        let active = state
            .active
            .as_ref()
            .filter(|active| active.job_id == job_id)
            .ok_or("The active job changed; refresh its status.")?;
        if state.exiting || active.cancellation.is_cancelled() {
            return Err("The encoder is shutting down.".into());
        }
        if !state.progress.as_ref().is_some_and(|p| {
            p.job_id == job_id && p.kind == TaskKind::Batch && !p.phase.is_terminal()
        }) {
            return Err("No active batch encode is available.".into());
        }
        control()?;
        // Mutate only pause-related fields while serialized with worker updates.
        let snapshot = state
            .progress
            .as_mut()
            .ok_or("Active progress is unavailable.")?;
        snapshot.paused = paused;
        snapshot.sequence += 1;
        snapshot.message = if paused {
            "Encoding paused. Select Resume to continue."
        } else {
            "Encoding resumed."
        }
        .into();
        Ok(snapshot.clone())
    }

    pub(super) fn reserve(
        &self,
        job_id: String,
        cancellation: Cancellation,
    ) -> Result<JobReservation, String> {
        let (finished, receiver) = watch::channel(false);
        {
            let mut state = self.0.lock().map_err(|e| e.to_string())?;
            if state.active.is_some() || state.close_prompt_open || state.exiting {
                return Err("An encoding task or close confirmation is already active.".into());
            }
            state.active = Some(ActiveTask {
                job_id: job_id.clone(),
                cancellation,
                finished: receiver,
            });
        }
        Ok(JobReservation {
            manager: self.clone(),
            job_id,
            finished,
        })
    }
}

impl Drop for JobReservation {
    fn drop(&mut self) {
        if let Ok(mut state) = self.manager.0.lock() {
            if state
                .active
                .as_ref()
                .is_some_and(|active| active.job_id == self.job_id)
            {
                state.active = None;
            }
        }
        self.finished.send_replace(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;

    fn progress() -> Progress {
        Progress {
            kind: TaskKind::Batch,
            job_id: "active".into(),
            sequence: 7,
            phase: keroro_encoder_core::Phase::Running,
            paused: false,
            encoder: Some("libx264".into()),
            measurements: None,
            current_file: 3,
            total_files: 10,
            file_name: Some("Ep03.mkv".into()),
            subtitle_name: None,
            file_percent: Some(45.0),
            batch_percent: 24.5,
            succeeded: 1,
            skipped: 0,
            failed: 1,
            elapsed_seconds: 12.0,
            fps: Some(60.0),
            speed: Some("2x".into()),
            eta_seconds: Some(5.0),
            message: "Encoding".into(),
            errors: vec![keroro_encoder_core::ItemFailure {
                file_name: "Ep02.mkv".into(),
                reason: "Invalid input".into(),
            }],
            warnings: vec![],
        }
    }

    #[test]
    fn pause_controls_preserve_latest_progress_and_reject_stale_or_terminal_jobs() {
        let manager = BatchManager::default();
        let cancellation = Cancellation::new();
        let _reservation = manager
            .reserve("active".into(), cancellation.clone())
            .expect("reserve");
        manager.0.lock().expect("state").progress = Some(progress());
        let paused = manager
            .control_pause("active", true, || Ok(()))
            .expect("pause");
        assert!(paused.paused);
        assert_eq!(paused.sequence, 8);
        // A worker update arrives between the user's pause and resume requests.
        {
            let mut state = manager.0.lock().expect("state");
            let current = state.progress.as_mut().expect("progress");
            current.sequence = 9;
            current.file_percent = Some(55.0);
            current.failed = 2;
            current.measurements = Some(keroro_encoder_core::EncodingMeasurements {
                output_bytes: 1234,
                ..Default::default()
            });
        }
        let resumed = manager
            .control_pause("active", false, || Ok(()))
            .expect("resume");
        assert!(!resumed.paused);
        assert_eq!(resumed.sequence, 10);
        assert_eq!(resumed.file_percent, Some(55.0));
        assert_eq!(resumed.failed, 2);
        assert_eq!(resumed.errors.len(), 1);
        assert_eq!(
            resumed.measurements.expect("measurements").output_bytes,
            1234
        );
        assert!(
            manager
                .control_pause("stale", true, || panic!("must not touch process"))
                .is_err()
        );
        assert!(
            manager
                .control_pause("active", true, || Err("no live process".into()))
                .is_err()
        );
        assert_eq!(
            manager
                .0
                .lock()
                .expect("state")
                .progress
                .as_ref()
                .expect("snapshot")
                .sequence,
            10
        );
        manager
            .0
            .lock()
            .expect("state")
            .progress
            .as_mut()
            .expect("progress")
            .phase = keroro_encoder_core::Phase::Succeeded;
        assert!(
            manager
                .control_pause("active", true, || panic!("terminal process"))
                .is_err()
        );
        manager.0.lock().expect("state").progress = Some(progress());
        cancellation.cancel();
        assert!(
            manager
                .control_pause("active", true, || panic!("cancelled process"))
                .is_err()
        );
    }

    #[test]
    fn simultaneous_starts_reserve_exactly_one_job() {
        let manager = BatchManager::default();
        let barrier = Arc::new(Barrier::new(2));
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..2)
                .map(|number| {
                    let manager = manager.clone();
                    let barrier = barrier.clone();
                    scope.spawn(move || {
                        barrier.wait();
                        let reservation =
                            manager.reserve(format!("job-{number}"), Cancellation::new());
                        barrier.wait(); // Keep the winner reserved until both attempts finish.
                        reservation.is_ok()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("join starter"))
                .collect::<Vec<_>>()
        });
        assert_eq!(results.into_iter().filter(|success| *success).count(), 1);
        assert!(manager.0.lock().expect("state").active.is_none());
        let reservation = manager
            .reserve("retry".into(), Cancellation::new())
            .expect("reusable");
        let finished = manager
            .0
            .lock()
            .expect("state")
            .active
            .as_ref()
            .expect("active")
            .finished
            .clone();
        drop(reservation);
        assert!(*finished.borrow());
    }

    #[test]
    fn prompt_and_exit_block_new_jobs_and_decline_preserves_cancellation() {
        let manager = BatchManager::default();
        let cancellation = Cancellation::new();
        let reservation = manager
            .reserve("active".into(), cancellation.clone())
            .expect("reserve");
        manager.0.lock().expect("state").close_prompt_open = true;
        assert!(
            manager
                .reserve("blocked".into(), Cancellation::new())
                .is_err()
        );
        manager.0.lock().expect("state").close_prompt_open = false;
        assert!(!cancellation.is_cancelled());
        drop(reservation);
        manager.0.lock().expect("state").exiting = true;
        assert!(
            manager
                .reserve("during-exit".into(), Cancellation::new())
                .is_err()
        );
    }
}
