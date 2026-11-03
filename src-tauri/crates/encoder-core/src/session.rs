use crate::{BatchPlan, Item, Settings, TaskKind, artifacts};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Stamp {
    length: u64,
    modified: u128,
}
fn stamp(path: &Path) -> Result<Stamp, String> {
    artifacts::no_links(path)?;
    let metadata =
        fs::metadata(path).map_err(|e| format!("Cannot inspect {}: {e}", path.display()))?;
    if !metadata.is_file() {
        return Err("Expected a regular media file.".into());
    }
    let modified = metadata
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    Ok(Stamp {
        length: metadata.len(),
        modified,
    })
}
fn digest(path: &Path, cancelled: &dyn Fn() -> bool) -> Result<[u8; 32], String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let before = stamp(path)?;
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        if cancelled() {
            return Err("Encoding cancelled.".into());
        }
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if stamp(path)? != before {
        return Err("Media changed while computing its recovery checksum.".into());
    }
    Ok(hash.finalize().into())
}
#[derive(Clone, Serialize, Deserialize)]
struct Completed {
    digest: [u8; 32],
    stamp: Stamp,
}
#[derive(Clone, Serialize, Deserialize)]
struct Intent {
    index: usize,
    digest: [u8; 32],
    stamp: Stamp,
}
#[derive(Clone, Serialize, Deserialize)]
struct Checkpoint {
    version: u8,
    job_id: String,
    settings: Settings,
    items: Vec<Item>,
    inputs: Vec<Stamp>,
    completed: BTreeMap<usize, Completed>,
    #[serde(default)]
    skipped: BTreeMap<usize, Stamp>,
    intent: Option<Intent>,
}
struct Contents {
    checkpoint: Option<Checkpoint>,
    fault: Option<String>,
}
pub struct SessionStore {
    path: PathBuf,
    contents: Mutex<Contents>,
    _lock: fs::File,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryInfo {
    pub id: String,
    pub next_episode: Option<usize>,
    pub total_files: usize,
    pub completed_files: usize,
    pub can_resume: bool,
    pub problem: Option<String>,
    pub settings: Option<Settings>,
}

impl SessionStore {
    pub(crate) fn records_directory(&self) -> PathBuf {
        self.path.with_file_name("output_records")
    }
    pub(crate) fn was_encoded(&self, index: usize) -> Result<bool, String> {
        let contents = self.contents.lock().map_err(|e| e.to_string())?;
        Ok(contents
            .checkpoint
            .as_ref()
            .is_some_and(|c| c.completed.contains_key(&index)))
    }
    pub fn last_measurements(&self) -> Result<Option<crate::EncodingMeasurements>, String> {
        crate::records::latest(&self.records_directory())
    }
    pub fn open(directory: &Path) -> Result<Arc<Self>, String> {
        crate::offline::local_path(directory)?;
        fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let directory = fs::canonicalize(directory).map_err(|e| e.to_string())?;
        artifacts::no_links(&directory)?;
        let lock_path = directory.join("runtime.lock");
        if lock_path.exists() {
            artifacts::no_links(&lock_path)?;
        }
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|e| e.to_string())?;
        lock.try_lock()
            .map_err(|e| format!("Another Keroro instance owns recovery state: {e}"))?;
        let path = directory.join("batch_state.json");
        let load = || -> Result<Option<Checkpoint>, String> {
            match fs::metadata(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e.to_string()),
                Ok(meta) if meta.len() > 8 * 1024 * 1024 => {
                    return Err("Checkpoint exceeds its 8 MiB limit.".into());
                }
                _ => {}
            }
            artifacts::no_links(&path)?;
            let checkpoint: Checkpoint =
                serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("Invalid batch checkpoint: {e}"))?;
            Self::validate(&checkpoint)?;
            Ok(Some(checkpoint))
        };
        let (checkpoint, fault) = match load() {
            Ok(state) => (state, None),
            Err(error) => (None, Some(error)),
        };
        let store = Arc::new(Self {
            path,
            contents: Mutex::new(Contents { checkpoint, fault }),
            _lock: lock,
        });
        Ok(store)
    }
    /// Import an earlier roaming-data checkpoint once, without deleting its source.
    pub fn open_with_legacy(directory: &Path, legacy: &Path) -> Result<Arc<Self>, String> {
        let store = Self::open(directory)?;
        if fs::canonicalize(legacy).ok().as_deref() == store.path.parent() {
            return Ok(store);
        }
        let marker = store.path.with_file_name("checkpoint_location.json");
        if marker.exists() {
            artifacts::no_links(&marker)?;
            return Ok(store);
        }
        if !store.path.exists() && legacy.join("batch_state.json").exists() {
            let previous = Self::open(legacy)?;
            let contents = previous.contents.lock().map_err(|e| e.to_string())?;
            if let Some(checkpoint) = &contents.checkpoint {
                crate::atomic_file::json(&store.path, checkpoint)?;
                store.contents.lock().map_err(|e| e.to_string())?.checkpoint =
                    Some(checkpoint.clone());
                log::info!("Imported recovery state into local app data.");
            } else if let Some(error) = &contents.fault {
                // Keep the original corrupt checkpoint for manual inspection.
                log::warn!("Legacy recovery state could not be imported: {error}");
            }
        }
        // Written after copying: an interrupted migration can safely retry.
        // Also prevents an archived legacy queue from returning after completion.
        crate::atomic_file::json(&marker, &serde_json::json!({"localStorage": true}))?;
        Ok(store)
    }

    fn reconcile(checkpoint: &mut Checkpoint, path: &Path) -> Result<(), String> {
        let Some(intent) = checkpoint.intent.clone() else {
            return Ok(());
        };
        let item = checkpoint
            .items
            .get(intent.index)
            .ok_or("Invalid commit index.")?;
        if digest(&item.output, &|| false).as_ref() == Ok(&intent.digest)
            && stamp(&item.output).is_ok_and(|current| current.length == intent.stamp.length)
        {
            let mut candidate = checkpoint.clone();
            candidate.completed.insert(
                intent.index,
                Completed {
                    digest: intent.digest,
                    stamp: intent.stamp,
                },
            );
            candidate.intent = None;
            crate::atomic_file::json(path, &candidate)?;
            *checkpoint = candidate;
        }
        Ok(())
    }
    fn validate(checkpoint: &Checkpoint) -> Result<(), String> {
        checkpoint.settings.video.validate()?;
        if checkpoint.version != 1
            || checkpoint.items.is_empty()
            || checkpoint.items.len() > 50_000
            || checkpoint.inputs.len() != checkpoint.items.len()
            || checkpoint.job_id.is_empty()
            || checkpoint.job_id.len() > 128
            || !checkpoint
                .job_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
        {
            return Err("Unsupported or inconsistent recovery checkpoint.".into());
        }
        let input = Path::new(&checkpoint.settings.input_dir);
        let output = Path::new(&checkpoint.settings.output_dir);
        crate::offline::local_path(input)?;
        crate::offline::local_path(output)?;
        if !input.is_absolute() || !output.is_absolute() || input == output {
            return Err("Invalid checkpoint roots.".into());
        }
        let mut destinations = std::collections::HashSet::new();
        for (index, item) in checkpoint.items.iter().enumerate() {
            // Queue entries are absolute selections, not children of a shared input root.
            for path in [&item.input, &item.output] {
                crate::offline::local_path(path)?;
                if !path.is_absolute()
                    || path.components().any(|part| {
                        matches!(
                            part,
                            std::path::Component::ParentDir | std::path::Component::CurDir
                        )
                    })
                {
                    return Err("Checkpoint media paths must be normalized absolute paths.".into());
                }
            }
            if item.output.parent() != Some(output) || !crate::is_video_path(&item.input) {
                return Err("Invalid checkpoint video or output destination.".into());
            }
            let expected = crate::media::output_name(
                &item.input,
                &checkpoint.settings.anime_title,
                index + 1,
            )?;
            let actual = item
                .output
                .file_name()
                .ok_or("Missing checkpoint output name.")?;
            // Earlier checkpoints keep their frozen E01 names; new batches use Ep01.
            let legacy = format!(
                "{} - E{:02} [Wii].mp4",
                checkpoint.settings.anime_title,
                index + 1
            );
            let legacy_matches = !checkpoint.settings.anime_title.is_empty()
                && actual == std::ffi::OsStr::new(&legacy);
            let source_episode = (checkpoint.items.len() == 1 && input == item.input)
                .then(|| {
                    item.input
                        .file_stem()
                        .and_then(|stem| crate::media::episode_number(&stem.to_string_lossy()))
                })
                .flatten();
            let source_matches = match source_episode {
                Some(episode) => {
                    actual
                        == crate::media::output_name(
                            &item.input,
                            &checkpoint.settings.anime_title,
                            episode.number as usize,
                        )?
                        .as_os_str()
                }
                None => false,
            };
            // Preserve old sequential names while accepting source-numbered single selections.
            if (actual != expected.as_os_str() && !legacy_matches && !source_matches)
                || !destinations.insert(actual.to_string_lossy().to_lowercase())
            {
                return Err("Checkpoint output name differs from the original naming plan.".into());
            }
            if let Some(subtitle) = &item.subtitle {
                crate::offline::local_path(subtitle)?;
                if !subtitle.is_absolute()
                    || subtitle.components().any(|part| {
                        matches!(
                            part,
                            std::path::Component::ParentDir | std::path::Component::CurDir
                        )
                    })
                    || item.subtitle_root.as_deref() != subtitle.parent()
                    || crate::subtitles::extension(subtitle).is_none()
                {
                    return Err("Invalid checkpoint subtitle path.".into());
                }
            }
            // The per-item subtitle parent scopes staging; it need not equal a
            // folder setting, which may itself be an explicitly selected file.
        }
        if checkpoint
            .completed
            .keys()
            .chain(checkpoint.skipped.keys())
            .any(|index| *index >= checkpoint.items.len())
            || checkpoint
                .skipped
                .keys()
                .any(|index| checkpoint.completed.contains_key(index))
            || checkpoint.intent.as_ref().is_some_and(|intent| {
                intent.index >= checkpoint.items.len()
                    || checkpoint.skipped.contains_key(&intent.index)
            })
        {
            return Err("Checkpoint indices are invalid.".into());
        }
        Ok(())
    }
    pub fn begin(&self, plan: &BatchPlan) -> Result<(), String> {
        let mut contents = self.contents.lock().map_err(|e| e.to_string())?;
        if contents.checkpoint.is_some() || contents.fault.is_some() {
            return Err(
                "Resolve the previous recovery checkpoint before starting a new batch.".into(),
            );
        }
        let inputs = plan
            .items
            .iter()
            .map(|item| stamp(&item.input))
            .collect::<Result<Vec<_>, _>>()?;
        let mut skipped = BTreeMap::new();
        for (index, item) in plan.items.iter().enumerate() {
            match crate::output::existing(&item.output) {
                Ok(true) => {
                    match crate::records::assess(&self.records_directory(), item, &plan.settings) {
                        Ok(crate::records::Match::Verified | crate::records::Match::Unknown) => {
                            skipped.insert(index, stamp(&item.output)?);
                        }
                        Ok(crate::records::Match::Changed(reason)) | Err(reason) => {
                            log::warn!("Existing output requires review: {reason}")
                        }
                    }
                }
                Ok(false) => {}
                Err(error) => log::warn!("Existing output will be checked per file: {error}"),
            }
        }
        let checkpoint = Checkpoint {
            version: 1,
            job_id: plan.job_id.clone(),
            settings: plan.settings.clone(),
            items: plan.items.clone(),
            inputs,
            completed: BTreeMap::new(),
            skipped,
            intent: None,
        };
        Self::validate(&checkpoint)?;
        crate::atomic_file::json(&self.path, &checkpoint)?;
        contents.checkpoint = Some(checkpoint);
        Ok(())
    }
    fn check_files(
        checkpoint: &Checkpoint,
        deep: bool,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), String> {
        for (index, item) in checkpoint.items.iter().enumerate() {
            if let Some(skipped) = checkpoint.skipped.get(&index) {
                if stamp(&item.output)? != *skipped {
                    return Err(format!(
                        "Previously skipped output changed: {}. It was preserved.",
                        item.output.display()
                    ));
                }
            } else if let Some(completed) = checkpoint.completed.get(&index) {
                if deep && digest(&item.output, cancelled)? != completed.digest {
                    return Err(format!(
                        "Completed output checksum changed: {}. It was preserved.",
                        item.output.display()
                    ));
                }
                if stamp(&item.output)?.length != completed.stamp.length {
                    return Err(format!(
                        "Completed output changed: {}. It was preserved; review it before resuming.",
                        item.output.display()
                    ));
                }
            } else {
                if crate::output::existing(&item.output)? {
                    if checkpoint
                        .intent
                        .as_ref()
                        .is_some_and(|intent| intent.index == index)
                    {
                        return Err(format!(
                            "Interrupted commit does not match its checksum: {}. It was preserved.",
                            item.output.display()
                        ));
                    }
                    continue;
                }
                if stamp(&item.input)?
                    != *checkpoint
                        .inputs
                        .get(index)
                        .ok_or("Missing source fingerprint.")?
                {
                    return Err(format!("Source changed: {}", item.input.display()));
                }
            }
        }
        Ok(())
    }
    pub fn info(&self) -> Result<Option<RecoveryInfo>, String> {
        let mut contents = self.contents.lock().map_err(|e| e.to_string())?;
        if let Some(problem) = &contents.fault {
            return Ok(Some(RecoveryInfo {
                id: "invalid".into(),
                next_episode: None,
                total_files: 0,
                completed_files: 0,
                can_resume: false,
                problem: Some(problem.clone()),
                settings: None,
            }));
        }
        let reconcile_problem = if let Some(state) = contents.checkpoint.as_mut() {
            Self::reconcile(state, &self.path).err()
        } else {
            None
        };
        let Some(state) = &contents.checkpoint else {
            return Ok(None);
        };
        let next_episode = (0..state.items.len())
            .find(|index| {
                !state.completed.contains_key(index) && !state.skipped.contains_key(index)
            })
            .map(|index| index + 1);
        let problem =
            reconcile_problem.or_else(|| Self::check_files(state, false, &|| false).err());
        Ok(Some(RecoveryInfo {
            id: state.job_id.clone(),
            next_episode,
            total_files: state.items.len(),
            completed_files: state.completed.len() + state.skipped.len(),
            can_resume: problem.is_none() && next_episode.is_some(),
            problem,
            settings: Some(state.settings.clone()),
        }))
    }
    #[cfg(test)]
    fn resume(&self, id: &str, new_job_id: &str) -> Result<BatchPlan, String> {
        self.resume_checked(id, new_job_id, &|| false)
    }
    pub fn resume_checked(
        &self,
        id: &str,
        new_job_id: &str,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<BatchPlan, String> {
        let mut contents = self.contents.lock().map_err(|e| e.to_string())?;
        let state = contents
            .checkpoint
            .as_mut()
            .filter(|state| state.job_id == id)
            .ok_or("Recovery checkpoint changed; reopen the prompt.")?;
        Self::reconcile(state, &self.path)?;
        let mut candidate = state.clone();
        for (index, item) in candidate.items.iter().enumerate() {
            if !candidate.completed.contains_key(&index)
                && !candidate.skipped.contains_key(&index)
                && !candidate
                    .intent
                    .as_ref()
                    .is_some_and(|intent| intent.index == index)
                && crate::output::existing(&item.output)?
            {
                candidate.skipped.insert(index, stamp(&item.output)?);
            }
        }
        Self::validate(&candidate)?;
        Self::check_files(&candidate, true, cancelled)?;
        candidate.job_id = new_job_id.into();
        candidate.intent = None;
        crate::atomic_file::json(&self.path, &candidate)?;
        *state = candidate;
        Ok(BatchPlan {
            job_id: new_job_id.into(),
            settings: state.settings.clone(),
            items: state.items.clone(),
            kind: TaskKind::Batch,
            warnings: Vec::new(),
        })
    }
    pub fn completed(&self, index: usize) -> Result<bool, String> {
        Ok(self
            .contents
            .lock()
            .map_err(|e| e.to_string())?
            .checkpoint
            .as_ref()
            .is_some_and(|state| {
                state.completed.contains_key(&index) || state.skipped.contains_key(&index)
            }))
    }
    pub fn record_existing(&self, index: usize) -> Result<(), String> {
        let mut contents = self.contents.lock().map_err(|e| e.to_string())?;
        let state = contents
            .checkpoint
            .as_mut()
            .ok_or("Missing active checkpoint.")?;
        let item = state.items.get(index).ok_or("Invalid skip index.")?;
        if !crate::output::existing(&item.output)? {
            return Err("Output disappeared before it could be skipped.".into());
        }
        let mut candidate = state.clone();
        if !candidate.completed.contains_key(&index) {
            candidate.skipped.insert(index, stamp(&item.output)?);
            crate::atomic_file::json(&self.path, &candidate)?;
            *state = candidate;
        }
        Ok(())
    }

    #[cfg(test)]
    fn commit(&self, index: usize, temp: &Path, output: &Path) -> Result<(), String> {
        self.commit_checked(index, temp, output, &|| false)
    }
    pub fn commit_checked(
        &self,
        index: usize,
        temp: &Path,
        output: &Path,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), String> {
        let mut contents = self.contents.lock().map_err(|e| e.to_string())?;
        let state = contents
            .checkpoint
            .as_mut()
            .ok_or("Missing active checkpoint.")?;
        if state
            .items
            .get(index)
            .is_none_or(|item| item.output != output)
        {
            return Err("Checkpoint commit does not match its frozen queue.".into());
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(temp)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        let intent = Intent {
            index,
            digest: digest(temp, cancelled)?,
            stamp: stamp(temp)?,
        };
        if intent.stamp.length == 0 {
            return Err("FFmpeg produced an empty output.".into());
        }
        state.intent = Some(intent.clone());
        crate::atomic_file::json(&self.path, state)?;
        crate::atomic_file::rename(temp, output, false)?;
        state.completed.insert(
            index,
            Completed {
                digest: intent.digest,
                stamp: intent.stamp,
            },
        );
        state.intent = None;
        crate::atomic_file::json(&self.path, state)?;
        log::info!(
            "Checkpoint committed episode {}: {}",
            index + 1,
            output.display()
        );
        Ok(())
    }
    pub fn discard(&self, expected_id: &str) -> Result<(), String> {
        let mut contents = self.contents.lock().map_err(|e| e.to_string())?;
        let actual = contents
            .checkpoint
            .as_ref()
            .map(|state| state.job_id.as_str())
            .unwrap_or("invalid");
        if actual != expected_id {
            return Err("Recovery checkpoint changed; reopen the prompt.".into());
        }
        if self.path.exists() {
            if contents.fault.is_some() {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_nanos();
                crate::atomic_file::rename(
                    &self.path,
                    &self
                        .path
                        .with_file_name(format!("batch_state.invalid-{timestamp}.json")),
                    false,
                )?;
            } else {
                fs::remove_file(&self.path).map_err(|e| e.to_string())?;
            }
        }
        contents.checkpoint = None;
        contents.fault = None;
        Ok(())
    }
    pub fn finish(&self, job_id: &str) -> Result<(), String> {
        self.discard(job_id)
    }
}

pub async fn blocking<T: Send + 'static>(
    task: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|e| format!("Local recovery worker failed: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absolute_external_media_and_subtitles_survive_checkpoint_roundtrip() {
        let root = tempfile::tempdir().expect("fixture");
        let mut queue = plan(root.path(), "external-files");
        let elsewhere = root.path().join("separate-selected-files");
        fs::create_dir(&elsewhere).expect("external folder");
        let elsewhere = fs::canonicalize(elsewhere).expect("canonical folder");
        let subtitle = elsewhere.join("translation.ass");
        fs::write(&subtitle, b"subtitle fixture").expect("subtitle");
        let external_video = elsewhere.join("Episode02.mkv");
        fs::write(&external_video, b"external video").expect("video");
        queue.items[1].input = external_video;
        queue.settings.input_dir = queue.items[0].input.to_string_lossy().into_owned();
        queue.settings.subtitle_dir = Some(subtitle.to_string_lossy().into_owned());
        for item in &mut queue.items {
            item.subtitle = Some(subtitle.clone());
            item.subtitle_root = Some(elsewhere.clone());
        }
        let state = root.path().join("local-app-data");
        let store = SessionStore::open(&state).expect("store");
        store.begin(&queue).expect("absolute paths accepted");
        assert!(state.join("batch_state.json").exists());
        assert!(!elsewhere.join("batch_state.json").exists());
        drop(store);
        let store = SessionStore::open(&state).expect("reopen");
        let restored = store.resume("external-files", "resumed").expect("resume");
        assert_eq!(restored.items[1].input, queue.items[1].input);
        assert_eq!(restored.items[0].subtitle.as_ref(), Some(&subtitle));
        let mut invalid = queue.clone();
        invalid.items[0].output = elsewhere.join("Keroro - Ep01 [Wii].mp4");
        store.finish("resumed").expect("finish");
        assert!(
            store.begin(&invalid).is_err(),
            "Output writes must retain their selected destination"
        );
    }

    #[test]
    fn legacy_recovery_migrates_once_to_local_data_without_resurrecting_finished_queue() {
        let root = tempfile::tempdir().expect("fixture");
        let legacy = root.path().join("roaming-data");
        let local = root.path().join("local-data");
        let queue = plan(root.path(), "migration");
        let previous = SessionStore::open(&legacy).expect("legacy store");
        previous.begin(&queue).expect("legacy checkpoint");
        let partial = queue.items[0].output.with_extension("mp4.part");
        fs::write(&partial, b"completed fixture").expect("partial");
        previous
            .commit(0, &partial, &queue.items[0].output)
            .expect("commit");
        drop(previous);
        let store = SessionStore::open_with_legacy(&local, &legacy).expect("migration");
        assert_eq!(
            store.info().expect("info").expect("queue").next_episode,
            Some(2)
        );
        assert!(local.join("batch_state.json").exists());
        assert!(
            legacy.join("batch_state.json").exists(),
            "Keep original recovery backup"
        );
        store.finish("migration").expect("finish");
        drop(store);
        let store = SessionStore::open_with_legacy(&local, &legacy).expect("reopen local");
        assert!(
            store.info().expect("info").is_none(),
            "Completed legacy queue must not reappear"
        );
    }

    #[test]
    fn preexisting_finals_are_skipped_and_recover_without_being_overwritten() {
        let root = tempfile::tempdir().expect("fixture");
        let state = root.path().join("state");
        let plan = plan(root.path(), "skip");
        let final_path = &plan.items[0].output;
        fs::write(final_path, b"user final").expect("final");
        let store = SessionStore::open(&state).expect("open");
        store.begin(&plan).expect("begin");
        assert!(store.completed(0).expect("skip checkpoint"));
        assert_eq!(
            store.info().expect("info").expect("saved").next_episode,
            Some(2)
        );
        drop(store);
        let store = SessionStore::open(&state).expect("reopen");
        store.resume("skip", "again").expect("resume");
        assert_eq!(fs::read(final_path).expect("preserved"), b"user final");
        assert!(store.completed(0).expect("skip retained"));
    }
    #[test]
    fn newly_existing_finals_can_be_adopted_and_changed_skipped_files_are_preserved() {
        let root = tempfile::tempdir().expect("fixture");
        let state = root.path().join("state");
        let plan = plan(root.path(), "adopt");
        let store = SessionStore::open(&state).expect("open");
        store.begin(&plan).expect("begin");
        fs::write(&plan.items[1].output, b"existing after start").expect("new final");
        store.record_existing(1).expect("checkpoint skip");
        assert!(store.completed(1).expect("skip recorded"));
        fs::write(&plan.items[2].output, b"existing after crash").expect("another final");
        store.resume("adopt", "adopted").expect("adopt on resume");
        assert!(store.completed(2).expect("adopted"));
        fs::write(&plan.items[1].output, b"modified user output, preserve it").expect("change");
        assert!(store.resume("adopted", "blocked").is_err());
        assert_eq!(
            fs::read(&plan.items[1].output).expect("preserved"),
            b"modified user output, preserve it"
        );
    }
    #[test]
    fn older_checkpoints_without_skipped_map_still_load() {
        let root = tempfile::tempdir().expect("fixture");
        let state = root.path().join("state");
        let plan = plan(root.path(), "older");
        let store = SessionStore::open(&state).expect("open");
        store.begin(&plan).expect("begin");
        drop(store);
        let path = state.join("batch_state.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read")).expect("json");
        value.as_object_mut().expect("object").remove("skipped");
        value["settings"]
            .as_object_mut()
            .expect("settings")
            .remove("video");
        value["settings"]
            .as_object_mut()
            .expect("settings")
            .remove("cpuMode");
        for field in ["selectedAudioIndex", "preferredAudioLang"] {
            value["settings"].as_object_mut().expect("settings").remove(field);
        }
        crate::atomic_file::json(&path, &value).expect("old schema");
        let store = SessionStore::open(&state).expect("reopen");
        assert!(store.info().expect("info").expect("saved").can_resume);
        assert_eq!(
            store
                .info()
                .expect("info")
                .expect("saved")
                .settings
                .expect("settings")
                .video,
            crate::VideoSettings::default()
        );
        assert_eq!(
            store
                .info()
                .expect("info")
                .expect("saved")
                .settings
                .expect("settings")
                .cpu_mode,
            crate::CpuMode::Maximum
        );
    }
    fn plan(root: &Path, job: &str) -> BatchPlan {
        let input = root.join("input");
        let output = root.join("output");
        fs::create_dir_all(&input).expect("input");
        fs::create_dir_all(&output).expect("output");
        let input = fs::canonicalize(input).expect("canonical");
        let output = fs::canonicalize(output).expect("canonical");
        let items = (1..=3)
            .map(|index| {
                let source = input.join(format!("Episode{index:02}.mkv"));
                fs::write(&source, b"source fixture").expect("source");
                Item {
                    input: source,
                    output: output.join(format!("Keroro - Ep{index:02} [Wii].mp4")),
                    subtitle: None,
                    subtitle_root: None,
                    validation_error: None,
                }
            })
            .collect();
        BatchPlan {
            job_id: job.into(),
            settings: Settings {
                video: Default::default(),
                selected_audio_index: None,
                preferred_audio_lang: None,
                input_dir: input.to_string_lossy().into_owned(),
                output_dir: output.to_string_lossy().into_owned(),
                subtitle_dir: None,
                anime_title: "Keroro".into(),
                crf: 23,
                cpu_mode: crate::CpuMode::Maximum,
            },
            items,
            kind: TaskKind::Batch,
            warnings: Vec::new(),
        }
    }
    #[test]
    fn legacy_episode_prefix_resumes_without_renaming_the_frozen_queue() {
        let root = tempfile::tempdir().expect("fixture");
        let state = root.path().join("state");
        let store = SessionStore::open(&state).expect("open");
        let mut plan = plan(root.path(), "legacy");
        for (index, item) in plan.items.iter_mut().enumerate() {
            item.output = item
                .output
                .with_file_name(format!("Keroro - E{:02} [Wii].mp4", index + 1));
        }
        store.begin(&plan).expect("legacy checkpoint");
        drop(store);
        let store = SessionStore::open(&state).expect("reopen");
        let restored = store.resume("legacy", "resumed").expect("resume");
        assert_eq!(restored.items[0].output, plan.items[0].output);
        assert_eq!(restored.items[2].output, plan.items[2].output);
    }
    #[test]
    fn checkpoints_resume_original_names_and_preserve_completed_outputs() {
        let root = tempfile::tempdir().expect("fixture");
        let state = root.path().join("state");
        let store = SessionStore::open(&state).expect("open");
        let plan = plan(root.path(), "original");
        store.begin(&plan).expect("checkpoint");
        let item = plan.items.first().expect("episode");
        let temp = item.output.with_file_name(".episode.original.part.mp4");
        fs::write(&temp, b"complete MP4 fixture").expect("temp");
        store.commit(0, &temp, &item.output).expect("commit");
        fs::write(
            std::path::Path::new(&plan.settings.input_dir).join("Episode00.mkv"),
            b"new source after checkpoint",
        )
        .expect("new source must not renumber original queue");
        assert_eq!(
            store.info().expect("info").expect("recovery").next_episode,
            Some(2)
        );
        assert!(
            SessionStore::open(&state).is_err(),
            "second instance must not sweep an active queue"
        );
        drop(store);
        let store = SessionStore::open(&state).expect("reopen");
        let restored = store.resume("original", "resumed").expect("resume");
        assert_eq!(restored.items.len(), 3);
        assert_eq!(restored.items[1].output, plan.items[1].output);
        assert!(store.completed(0).expect("completed"));
        assert!(!store.completed(1).expect("pending"));
        assert_eq!(
            fs::read(&item.output).expect("original output"),
            b"complete MP4 fixture"
        );
    }
    #[test]
    fn replaced_output_or_changed_pending_source_blocks_resume_without_deletion() {
        let root = tempfile::tempdir().expect("fixture");
        let store = SessionStore::open(&root.path().join("state")).expect("open");
        let plan = plan(root.path(), "test");
        store.begin(&plan).expect("begin");
        fs::write(&plan.items[1].input, b"changed source").expect("modify");
        assert!(!store.info().expect("info").expect("checkpoint").can_resume);
        assert!(store.resume("test", "retry").is_err());
        assert!(plan.items[1].input.exists());
    }
    #[test]
    fn corrupt_checkpoint_can_be_archived_without_erasing_media() {
        let root = tempfile::tempdir().expect("fixture");
        fs::write(root.path().join("batch_state.json"), b"{broken").expect("corrupt");
        let store = SessionStore::open(root.path()).expect("open with fault");
        assert!(!store.info().expect("info").expect("fault").can_resume);
        store.discard("invalid").expect("archive");
        assert!(store.info().expect("clear").is_none());
        assert!(
            fs::read_dir(root.path())
                .expect("files")
                .filter_map(Result::ok)
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("batch_state.invalid-"))
        );
    }
    #[test]
    fn rename_to_checkpoint_crash_window_is_reconciled() {
        let root = tempfile::tempdir().expect("fixture");
        let state_dir = root.path().join("state");
        let store = SessionStore::open(&state_dir).expect("open");
        let plan = plan(root.path(), "commit-window");
        store.begin(&plan).expect("begin");
        let item = plan.items.first().expect("item");
        let temp = item.output.with_file_name(".crash.part.mp4");
        fs::write(&temp, b"complete").expect("temp");
        {
            let mut contents = store.contents.lock().expect("lock");
            let checkpoint = contents.checkpoint.as_mut().expect("checkpoint");
            checkpoint.intent = Some(Intent {
                index: 0,
                digest: digest(&temp, &|| false).expect("digest"),
                stamp: stamp(&temp).expect("stamp"),
            });
            crate::atomic_file::json(&store.path, checkpoint).expect("intent");
        }
        crate::atomic_file::rename(&temp, &item.output, false)
            .expect("commit before simulated crash");
        drop(store);
        let reopened = SessionStore::open(&state_dir).expect("recover");
        reopened.info().expect("reconcile in recovery worker");
        assert!(reopened.completed(0).expect("reconciled"));
        assert_eq!(
            reopened
                .info()
                .expect("info")
                .expect("checkpoint")
                .next_episode,
            Some(2)
        );
    }

    #[test]
    fn same_length_output_changes_are_detected_by_checksum_and_preserved() {
        let root = tempfile::tempdir().expect("fixture");
        let store = SessionStore::open(&root.path().join("state")).expect("open");
        let plan = plan(root.path(), "checksum");
        store.begin(&plan).expect("begin");
        let item = &plan.items[0];
        let temp = item.output.with_file_name(".checksum.part.mp4");
        fs::write(&temp, b"complete").expect("temp");
        store.commit(0, &temp, &item.output).expect("commit");
        fs::write(&item.output, b"modified").expect("same length replacement");
        assert!(
            store
                .resume("checksum", "retry")
                .err()
                .expect("reject")
                .contains("checksum")
        );
        assert_eq!(fs::read(&item.output).expect("preserved"), b"modified");
    }
}
