#[cfg(test)]
mod benchmark;
use serde::{Deserialize, Serialize};
pub mod artifacts;
pub mod atomic_file;
pub mod audio;
mod cancel;
pub mod crop;
mod diagnostics;
pub mod drop_scan;
mod engine;
pub mod environment;
pub mod ffmpeg;
pub mod hardware;
mod job_fence;
pub mod local_log;
pub mod offline;
mod output;
mod pipe_lines;
pub mod power;
pub mod preflight;
pub mod probe;
mod process_state;
mod profile;
pub mod progress;
mod records;
pub mod session;
pub mod snapshot;
pub mod storage;
pub mod subtitles;
mod validation;
pub mod video;
pub use process_state::ActiveProcess;
pub use records::Measurements as EncodingMeasurements;
pub use video::{AspectRatio, ResolutionPreset, VideoSettings};
pub mod media;
pub use cancel::Cancellation;
use std::{
    collections::HashSet,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Instant,
};
use tokio::{fs, process::Command};

/// Container candidates; FFmpeg validates the actual streams and installed decoders.
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm"];

pub fn is_video_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            VIDEO_EXTENSIONS
                .iter()
                .any(|supported| ext.eq_ignore_ascii_case(supported))
        })
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CpuMode {
    Balanced,
    #[default]
    Maximum,
}
impl CpuMode {
    fn threads(self, cores: usize) -> usize {
        match self {
            Self::Balanced => progress::thread_budget(cores),
            Self::Maximum => 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_volume_id: Option<String>,
    pub input_dir: String,
    pub output_dir: String,
    pub crf: u8,
    #[serde(default)]
    pub video: VideoSettings,
    #[serde(default)]
    pub cpu_mode: CpuMode,
    #[serde(default)]
    pub subtitle_dir: Option<String>,
    #[serde(default)]
    pub anime_title: String,
    #[serde(default)]
    pub selected_audio_index: Option<u32>,
    #[serde(default)]
    pub selected_subtitle_index: Option<u32>,
    #[serde(default)]
    pub preferred_sub_lang: Option<String>,
    #[serde(default)]
    pub preferred_audio_lang: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Tools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Item {
    input: PathBuf,
    output: PathBuf,
    subtitle: Option<PathBuf>,
    subtitle_root: Option<PathBuf>,
    validation_error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BatchPlan {
    pub job_id: String,
    settings: Settings,
    items: Vec<Item>,
    pub kind: TaskKind,
    warnings: Vec<ItemFailure>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Batch,
    Preview,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Queued,
    Running,
    Succeeded,
    CompletedWithErrors,
    Failed,
    Cancelled,
}

impl Phase {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::CompletedWithErrors | Self::Failed | Self::Cancelled
        )
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFailure {
    pub file_name: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub kind: TaskKind,
    pub job_id: String,
    pub sequence: u64,
    pub phase: Phase,
    pub paused: bool,
    pub encoder: Option<String>,
    pub measurements: Option<EncodingMeasurements>,
    pub current_file: usize,
    pub total_files: usize,
    pub file_name: Option<String>,
    pub subtitle_name: Option<String>,
    pub file_percent: Option<f64>,
    pub batch_percent: f64,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub elapsed_seconds: f64,
    pub fps: Option<f64>,
    pub frame: Option<u64>,
    pub speed: Option<String>,
    pub eta_seconds: Option<f64>,
    pub message: String,
    pub errors: Vec<ItemFailure>,
    pub warnings: Vec<ItemFailure>,
}

enum ItemUpdate {
    Progress(
        Option<f64>,
        Option<f64>,
        Option<String>,
        Option<f64>,
        Option<u64>,
    ),
    Warning(String),
    SubtitlesOff,
    Measurements(EncodingMeasurements),
}

impl BatchPlan {
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub async fn prepare(job_id: String, settings: Settings) -> Result<Self, String> {
        Self::prepare_inner(job_id, settings, TaskKind::Batch).await
    }

    pub async fn prepare_preview(
        job_id: String,
        mut settings: Settings,
        directory: &Path,
    ) -> Result<Self, String> {
        settings.output_dir = directory.to_string_lossy().into_owned();
        settings.target_volume_id = None;
        Self::prepare_inner(job_id, settings, TaskKind::Preview).await
    }

    async fn prepare_inner(
        job_id: String,
        mut settings: Settings,
        kind: TaskKind,
    ) -> Result<Self, String> {
        settings.video.validate()?;
        if settings
            .selected_subtitle_index
            .is_some_and(|index| index >= 256)
        {
            return Err("Invalid embedded subtitle index.".into());
        }
        if settings.preferred_sub_lang.as_ref().is_some_and(|v| {
            v.len() > 16 || (v != "none" && audio::normalize_language(v) == "und" && v != "und")
        }) {
            return Err("Invalid subtitle language preference.".into());
        }
        if settings.selected_subtitle_index.is_some() {
            settings.subtitle_dir = None;
        }
        if settings
            .selected_audio_index
            .is_some_and(|index| index >= audio::MAX_AUDIO_STREAMS)
            || settings.preferred_audio_lang.as_ref().is_some_and(|value| {
                value.len() > 16 || (audio::normalize_language(value) == "und" && value != "und")
            })
        {
            return Err("Invalid audio track index or preferred language.".into());
        }
        offline::local_path(Path::new(&settings.input_dir))?;
        offline::local_path(Path::new(&settings.output_dir))?;
        if let Some(path) = &settings.subtitle_dir
            && !path.trim().is_empty()
        {
            offline::local_path(Path::new(path))?;
        }
        if job_id.is_empty()
            || job_id.len() > 128
            || !job_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
        {
            return Err("Invalid internal job identifier.".into());
        }
        if !matches!(settings.crf, 20 | 23 | 26) {
            return Err("Choose CRF 20, 23, or 26.".into());
        }
        let raw_title = settings.anime_title.trim();
        let title = media::clean_title(raw_title)?;
        if !raw_title.is_empty() && title.is_empty() {
            return Err("The anime title must contain filename-safe characters.".into());
        }
        settings.anime_title = title;
        let input = fs::canonicalize(&settings.input_dir)
            .await
            .map_err(|e| format!("Cannot open input path: {e}"))?;
        let input_metadata = fs::metadata(&input).await.map_err(|e| e.to_string())?;
        if !input_metadata.is_dir() && !(input_metadata.is_file() && is_video_path(&input)) {
            return Err("Input path must be a supported video file or folder.".into());
        }
        let mut subtitles = Vec::new();
        let mut subtitle_root = None;
        let mut explicit_subtitle = false;
        if let Some(directory) = settings
            .subtitle_dir
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            let mut directory = fs::canonicalize(directory)
                .await
                .map_err(|e| format!("Cannot canonicalize subtitle path: {e}"))?;
            // The frontend's automatic subtitle default follows the exact media input.
            if input_metadata.is_file() && directory == input {
                directory = input
                    .parent()
                    .ok_or("Input file has no parent folder.")?
                    .to_path_buf();
            }
            let metadata = fs::metadata(&directory).await.map_err(|e| e.to_string())?;
            if metadata.is_file() && subtitles::extension(&directory).is_some() {
                explicit_subtitle = true;
                subtitle_root = Some(
                    directory
                        .parent()
                        .ok_or("Subtitle file has no parent folder.")?
                        .to_path_buf(),
                );
                subtitles.push(directory.clone());
            } else if metadata.is_dir() {
                subtitle_root = Some(directory.clone());
            } else {
                return Err("Subtitle path must be a supported subtitle file or folder.".into());
            }
            settings.subtitle_dir = Some(
                directory
                    .to_str()
                    .ok_or("Subtitle path cannot be saved as Unicode.")?
                    .to_owned(),
            );
            if !explicit_subtitle {
                let mut entries = fs::read_dir(&directory)
                    .await
                    .map_err(|e| format!("Cannot read subtitle folder: {e}"))?;
                while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
                    let path = entry.path();
                    if entry
                        .file_type()
                        .await
                        .map_err(|e| e.to_string())?
                        .is_file()
                        && subtitles::extension(&path).is_some()
                    {
                        // Deletion/unreadability after scanning is handled per item during staging.
                        subtitles.push(path);
                    }
                }
            }
        }
        let mut videos = Vec::new();
        if input_metadata.is_file() {
            videos.push(input.clone());
        } else {
            let mut entries = fs::read_dir(&input).await.map_err(|e| e.to_string())?;
            while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
                let path = entry.path();
                if entry
                    .file_type()
                    .await
                    .map_err(|e| e.to_string())?
                    .is_file()
                    && is_video_path(&path)
                {
                    videos.push(path);
                }
            }
        }
        if videos.is_empty() {
            return Err("No supported videos found in the input folder.".into());
        }
        let (mut videos, mut pairing) = session::blocking(move || {
            videos.sort_by(|a, b| media::compare_videos(a, b));
            let mut pairing = media::pair_subtitles(&videos, &subtitles);
            if explicit_subtitle && videos.len() == 1 {
                pairing.matches = vec![subtitles.first().cloned()];
                pairing.warnings.clear();
            }
            Ok((videos, pairing))
        })
        .await?;
        if kind == TaskKind::Preview {
            videos.truncate(1);
            pairing.matches.truncate(1);
        }
        if settings.output_dir.trim().is_empty() {
            return Err("Choose an output folder.".into());
        }
        let destination = PathBuf::from(&settings.output_dir);
        let expected_volume = settings.target_volume_id.clone();
        let volume = session::blocking(move || {
            storage::verify_target(&destination, expected_volume.as_deref())
        })
        .await?;
        settings.target_volume_id = Some(volume.id);
        fs::create_dir_all(&settings.output_dir)
            .await
            .map_err(|e| preflight::access_error("Cannot create output folder", e))?;
        let output = fs::canonicalize(&settings.output_dir)
            .await
            .map_err(|e| preflight::access_error("Cannot open output folder", e))?;
        if input == output {
            return Err(
                "Choose a separate output folder to avoid re-encoding previous outputs.".into(),
            );
        }
        let mut warnings = Vec::new();
        if subtitle_root.is_some() {
            for reason in pairing.warnings {
                log::warn!("Subtitle pairing: {reason}");
                warnings.push(ItemFailure {
                    file_name: "Subtitle pairing".into(),
                    reason,
                });
            }
        }
        if let Some(reason) = preflight::output(&output).await? {
            warnings.push(ItemFailure {
                file_name: "Output folder".into(),
                reason,
            });
        }
        let mut destinations = HashSet::new();
        settings.input_dir = input
            .to_str()
            .ok_or("Input path cannot be saved as Unicode.")?
            .to_owned();
        settings.output_dir = output
            .to_str()
            .ok_or("Output path cannot be saved as Unicode.")?
            .to_owned();
        let mut items = Vec::new();
        for (index, (path, subtitle)) in videos.into_iter().zip(pairing.matches).enumerate() {
            let name = if kind == TaskKind::Preview {
                OsString::from("preview_temp.mp4")
            } else {
                let episode = if input_metadata.is_file() {
                    path.file_stem()
                        .and_then(|stem| media::episode_number(&stem.to_string_lossy()))
                        .map_or(1, |episode| episode.number as usize)
                } else {
                    index + 1
                };
                media::output_name(&path, &settings.anime_title, episode)?
            };
            if !destinations.insert(name.to_string_lossy().to_lowercase()) {
                return Err(format!(
                    "Multiple inputs produce {}. Rename one input before encoding.",
                    name.to_string_lossy()
                ));
            }
            items.push(Item {
                input: path,
                output: output.join(name),
                subtitle,
                subtitle_root: subtitle_root.clone(),
                validation_error: None,
            });
        }
        Ok(Self {
            job_id,
            settings,
            items,
            kind,
            warnings,
        })
    }

    pub fn preview_path(&self) -> Option<PathBuf> {
        if self.kind == TaskKind::Preview {
            self.items.first().map(|item| item.output.clone())
        } else {
            None
        }
    }

    pub fn initial_progress(&self) -> Progress {
        Progress {
            kind: self.kind,
            subtitle_name: None,
            job_id: self.job_id.clone(),
            sequence: 0,
            phase: Phase::Queued,
            paused: false,
            encoder: None,
            measurements: None,
            current_file: 0,
            total_files: self.items.len(),
            file_name: None,
            file_percent: None,
            batch_percent: 0.0,
            succeeded: 0,
            skipped: 0,
            failed: 0,
            elapsed_seconds: 0.0,
            fps: None,
            frame: None,
            speed: None,
            eta_seconds: None,
            message: format!("Queued {} video(s).", self.items.len()),
            errors: Vec::new(),
            warnings: self.warnings.clone(),
        }
    }
}

fn process(path: &Path) -> Command {
    let mut command = Command::new(path);
    command.stdin(Stdio::null()).kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flashing per item.
    command
}

pub fn file_percent(out_time_us: &str, duration: Option<f64>) -> Option<f64> {
    file_percent_seconds(out_time_us.parse::<f64>().ok()? / 1_000_000.0, duration)
}

pub fn file_percent_seconds(seconds: f64, duration: Option<f64>) -> Option<f64> {
    let duration = duration.filter(|d| d.is_finite() && *d > 0.0)?;
    if !seconds.is_finite() {
        return None;
    }
    // 100% is reserved for a successful exit and finalized output.
    Some((seconds / duration * 100.0).clamp(0.0, 99.9))
}

pub async fn run_batch(
    plan: BatchPlan,
    tools: Tools,
    cancellation: Cancellation,
    publish: impl FnMut(Progress),
) -> Progress {
    run_batch_with_session(
        plan,
        tools,
        cancellation,
        Arc::new(ActiveProcess::default()),
        None,
        publish,
    )
    .await
}

pub async fn run_batch_with_session(
    plan: BatchPlan,
    tools: Tools,
    cancellation: Cancellation,
    processes: Arc<ActiveProcess>,
    checkpoint: Option<Arc<session::SessionStore>>,
    mut publish: impl FnMut(Progress),
) -> Progress {
    let start = Instant::now();
    let mut state = plan.initial_progress();
    state.phase = Phase::Running;
    let mut interrupted = false;
    let mut selected_encoder = None;
    for (index, item) in plan.items.iter().enumerate() {
        if cancellation.is_cancelled() {
            state.phase = Phase::Cancelled;
            break;
        }
        let destination = PathBuf::from(&plan.settings.output_dir);
        let expected_volume = plan.settings.target_volume_id.clone();
        if let Err(error) = session::blocking(move || {
            storage::verify_target(&destination, expected_volume.as_deref())
        })
        .await
        {
            state.message = error;
            interrupted = true;
            break;
        }
        let saved = match checkpoint
            .as_ref()
            .map(|store| store.completed(index))
            .transpose()
        {
            Ok(saved) => saved.unwrap_or(false),
            Err(error) => {
                state.message = error;
                interrupted = true;
                break;
            }
        };
        let existing = if plan.kind == TaskKind::Batch && !saved {
            let path = item.output.clone();
            match session::blocking(move || output::existing(&path)).await {
                Ok(existing) => existing,
                Err(error) => {
                    log::warn!("Cannot smart-skip {}: {error}", item.output.display());
                    false
                }
            }
        } else {
            false
        };
        let mut unverified = false;
        if saved || existing {
            if let Some(store) = &checkpoint {
                let was_encoded = match store.was_encoded(index) {
                    Ok(value) => value,
                    Err(reason) => {
                        state.message = reason;
                        interrupted = true;
                        break;
                    }
                };
                if !was_encoded {
                    let directory = store.records_directory();
                    let current = item.clone();
                    let settings = plan.settings.clone();
                    let assessment =
                        session::blocking(move || records::assess(&directory, &current, &settings))
                            .await;
                    let problem = match assessment {
                        Ok(records::Match::Verified) => None,
                        Ok(records::Match::Unknown) => {
                            unverified = true;
                            None
                        }
                        Ok(records::Match::Changed(reason)) | Err(reason) => Some(reason),
                    };
                    if let Some(reason) = problem {
                        let name = item
                            .input
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "Video".into());
                        let reason = format!(
                            "{reason} Existing output was preserved. Choose a different output folder or move the old file before encoding again."
                        );
                        state.current_file = index + 1;
                        state.file_name = Some(name.clone());
                        state.file_percent = None;
                        state.eta_seconds = None;
                        state.failed += 1;
                        state.errors.push(ItemFailure {
                            file_name: name,
                            reason: reason.clone(),
                        });
                        state.batch_percent = (index + 1) as f64 / state.total_files as f64 * 100.0;
                        state.message = format!("[Error] {reason}");
                        state.sequence += 1;
                        publish(state.clone());
                        continue;
                    }
                }
            }
            if existing && let Some(store) = &checkpoint {
                let store = store.clone();
                if let Err(error) = session::blocking(move || store.record_existing(index)).await {
                    log::error!("Cannot checkpoint skipped output: {error}");
                    state.message = error;
                    interrupted = true;
                    break;
                }
            }
            if unverified {
                log::warn!(
                    "Preserved unverified existing output: {}",
                    item.output.display()
                );
                let reason = "Some existing outputs have no provenance records. They were preserved and skipped without verifying that their source/subtitles/quality match.";
                if !state.warnings.iter().any(|w| w.reason == reason) {
                    state.warnings.push(ItemFailure {
                        file_name: "Unverified existing outputs".into(),
                        reason: reason.into(),
                    });
                }
            }
            state.current_file = index + 1;
            state.skipped += 1;
            state.file_name = item
                .input
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            state.subtitle_name = None;
            state.file_percent = Some(100.0);
            state.eta_seconds = Some(0.0);
            state.fps = None;
            state.frame = None;
            state.speed = None;
            state.batch_percent = (index + 1) as f64 / state.total_files as f64 * 100.0;
            state.elapsed_seconds = start.elapsed().as_secs_f64();
            state.message = format!(
                "Skipped episode {}: final output already exists; preserved {}.",
                index + 1,
                item.output.display()
            );
            log::info!("{}", state.message);
            state.sequence += 1;
            publish(state.clone());
            continue;
        }
        let name = item
            .input
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Video".into());
        log::info!("Encoding episode {}: {}", index + 1, item.input.display());
        state.current_file = index + 1;
        state.file_name = Some(name.clone());
        state.subtitle_name = item
            .subtitle
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        state.file_percent = None;
        state.fps = None;
        state.frame = None;
        state.speed = None;
        state.eta_seconds = None;
        state.batch_percent = index as f64 / state.total_files as f64 * 100.0;
        state.message = format!(
            "Encoding file {} of {}: {name}",
            index + 1,
            state.total_files
        );
        state.sequence += 1;
        publish(state.clone());
        if plan
            .settings
            .subtitle_dir
            .as_deref()
            .is_some_and(|dir| !dir.trim().is_empty())
            && item.subtitle.is_none()
            && plan.settings.selected_subtitle_index.is_none()
            && plan.settings.preferred_sub_lang.is_none()
            && item.validation_error.is_none()
        {
            let reason =
                "No matching .srt/.ass/.ssa/.vtt subtitle; encoding without external subtitles."
                    .to_string();
            state.warnings.push(ItemFailure {
                file_name: name.clone(),
                reason: reason.clone(),
            });
            state.message = format!("[Warning] {name}: {reason}");
            state.sequence += 1;
            publish(state.clone());
        }
        if selected_encoder.is_none() {
            selected_encoder = Some(
                hardware::detect_requested(
                    &tools,
                    &cancellation,
                    &processes,
                    plan.settings.video.encoder_override,
                )
                .await,
            );
        }
        let mut encoder = selected_encoder.unwrap_or(hardware::VideoEncoder::Cpu);
        state.encoder = Some(encoder.name().into());
        let result = loop {
            state.encoder = Some(encoder.name().into());
            let mut progress_gate = progress::ProgressGate::default();
            let attempt = engine::Encoder {
                tools: &tools,
                settings: &plan.settings,
                kind: plan.kind,
                cancellation: &cancellation,
                processes: &processes,
            }
            .encode(
                item,
                encoder,
                checkpoint.as_ref().map(|store| (store, index)),
                |update| {
                    match update {
                        ItemUpdate::Progress(percent, fps, speed, eta, frame) => {
                            state.file_percent = percent;
                            state.batch_percent = (index as f64 + percent.unwrap_or(0.0) / 100.0)
                                / state.total_files as f64
                                * 100.0;
                            state.fps = fps;
                            state.frame = frame;
                            state.speed = speed;
                            state.eta_seconds = eta;
                            let percent_changed = progress_gate.should_emit(percent);
                            let eta_changed = progress_gate.should_emit_eta(eta);
                            let metrics_changed =
                                progress_gate.should_emit_metrics(fps, state.speed.as_deref());
                            if !percent_changed && !eta_changed && !metrics_changed {
                                return;
                            }
                        }
                        ItemUpdate::Warning(reason) => {
                            log::warn!("{}: {}", name, reason);
                            state.message = format!("[Warning] {name}: {reason}");
                            state.warnings.push(ItemFailure {
                                file_name: name.clone(),
                                reason,
                            });
                        }
                        ItemUpdate::SubtitlesOff => state.subtitle_name = None,
                        ItemUpdate::Measurements(measurements) => {
                            state.measurements = Some(measurements)
                        }
                    }
                    state.elapsed_seconds = start.elapsed().as_secs_f64();
                    state.sequence += 1;
                    publish(state.clone());
                },
            )
            .await;
            if encoder != hardware::VideoEncoder::Cpu
                && !cancellation.is_cancelled()
                && attempt.as_ref().err().is_some_and(|e| {
                    e.starts_with("FFmpeg failed")
                        || e.starts_with("FFmpeg subtitle/filter")
                        || e.starts_with("OUTPUT_VALIDATION_FAILED:")
                })
                && output::partial_path(&item.output).is_ok_and(|path| !path.exists())
                && !item.output.exists()
            {
                log::warn!(
                    "Hardware encode failed for {}; retrying with libx264. {}",
                    item.input.display(),
                    attempt.as_ref().err().map_or("", String::as_str)
                );
                encoder = hardware::VideoEncoder::Cpu;
                selected_encoder = Some(encoder);
                continue;
            }
            break attempt;
        };
        if cancellation.is_cancelled() {
            state.phase = Phase::Cancelled;
            if result.is_ok() {
                state.succeeded += 1;
                state.file_percent = Some(100.0);
                state.eta_seconds = Some(0.0);
            }
            if let Err(reason) = &result
                && reason != "Encoding cancelled."
            {
                state.errors.push(ItemFailure {
                    file_name: name.clone(),
                    reason: reason.clone(),
                });
            }
            break;
        }
        match result {
            Ok(warning) => {
                state.succeeded += 1;
                state.file_percent = Some(100.0);
                state.eta_seconds = Some(0.0);
                if let Some(reason) = warning {
                    state.message = format!("[Warning] {name}: {reason}");
                    state.warnings.push(ItemFailure {
                        file_name: name.clone(),
                        reason,
                    });
                } else {
                    state.message = format!("Saved {name}.");
                }
            }
            Err(reason) => {
                state.eta_seconds = None;
                log::error!("Failed {}: {}", name, reason);
                interrupted = reason.starts_with("STATE_PERSISTENCE_FAILED:");
                state.failed += 1;
                state.message = format!("[Error] Failed to process {name}: {reason}. Skipping...");
                state.errors.push(ItemFailure {
                    file_name: name,
                    reason,
                });
            }
        }
        state.batch_percent = (index + 1) as f64 / state.total_files as f64 * 100.0;
        state.elapsed_seconds = start.elapsed().as_secs_f64();
        state.sequence += 1;
        publish(state.clone());
        if interrupted {
            break;
        }
    }
    state.phase = if interrupted {
        Phase::Failed
    } else if state.phase == Phase::Cancelled {
        Phase::Cancelled
    } else if state.failed == 0 {
        Phase::Succeeded
    } else if state.succeeded + state.skipped > 0 {
        Phase::CompletedWithErrors
    } else {
        Phase::Failed
    };
    if !interrupted
        && state.phase != Phase::Cancelled
        && let Some(store) = &checkpoint
    {
        let store = store.clone();
        let job_id = plan.job_id.clone();
        if let Err(error) = session::blocking(move || store.finish(&job_id)).await {
            log::error!("Cannot clear finished checkpoint: {error}");
            state.warnings.push(ItemFailure { file_name: "Recovery state".into(), reason: "Completed output was saved, but the finished checkpoint could not be cleared. Review recovery on the next launch.".into() });
        }
    }
    state.message = if interrupted {
        "Encoding paused because recovery state could not be saved. Completed outputs were preserved; review the local log.".into()
    } else if state.phase == Phase::Cancelled {
        format!(
            "Task cancelled. {} completed output(s) preserved; {} skipped.",
            state.succeeded, state.skipped
        )
    } else {
        format!(
            "Batch complete: {} encoded, {} skipped, {} failed, {} total. {} warning(s).",
            state.succeeded,
            state.skipped,
            state.failed,
            state.total_files,
            state.warnings.len()
        )
    };
    state.sequence += 1;
    state.elapsed_seconds = start.elapsed().as_secs_f64();
    publish(state.clone());
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn preview_uses_its_cache_volume_and_batch_rejects_replaced_targets() {
        let root = tempfile::tempdir().expect("fixture");
        let input = root.path().join("Ep03.mkv");
        std::fs::write(&input, []).expect("source");
        let output = root.path().join("not-created");
        let settings = Settings {
            target_volume_id: Some("disconnected-card".into()),
            input_dir: input.to_string_lossy().into_owned(),
            output_dir: output.to_string_lossy().into_owned(),
            crf: 23,
            video: VideoSettings::default(),
            cpu_mode: CpuMode::Maximum,
            subtitle_dir: None,
            anime_title: String::new(),
            selected_audio_index: None,
            selected_subtitle_index: None,
            preferred_sub_lang: None,
            preferred_audio_lang: None,
        };
        let error = BatchPlan::prepare("batch".into(), settings.clone())
            .await
            .expect_err("wrong volume");
        assert!(error.contains("disconnected or replaced"));
        assert!(!output.exists());

        let cache = root.path().join("preview-cache");
        let preview = BatchPlan::prepare_preview("preview".into(), settings, &cache)
            .await
            .expect("preview uses local cache");
        assert_eq!(
            preview.settings.target_volume_id,
            Some(storage::inspect(&cache).expect("cache volume").id)
        );
        assert_eq!(preview.kind, TaskKind::Preview);
    }
    #[tokio::test]
    async fn single_file_queue_keeps_exact_video_and_scopes_subtitle_selection() {
        let root = tempfile::tempdir().expect("fixture");
        let input = root.path().join("input");
        let output = root.path().join("output");
        std::fs::create_dir(&input).expect("input");
        for name in ["Ep01.mkv", "Ep02.mkv", "Ep01.srt", "Ep02.srt", "custom.ass"] {
            std::fs::write(input.join(name), []).expect("fixture file");
        }
        let video = std::fs::canonicalize(input.join("Ep02.mkv")).expect("video");
        for (sub, expected) in [
            (input.clone(), "Ep02.srt"),
            (video.clone(), "Ep02.srt"),
            (input.join("custom.ass"), "custom.ass"),
        ] {
            let plan = BatchPlan::prepare(
                "single-file".into(),
                Settings {
                    target_volume_id: None,
                    video: Default::default(),
                    selected_audio_index: None,
                    selected_subtitle_index: None,
                    preferred_sub_lang: None,
                    preferred_audio_lang: None,
                    input_dir: video.to_string_lossy().into_owned(),
                    output_dir: output.to_string_lossy().into_owned(),
                    subtitle_dir: Some(sub.to_string_lossy().into_owned()),
                    crf: 23,
                    cpu_mode: crate::CpuMode::Maximum,
                    anime_title: String::new(),
                },
            )
            .await
            .expect("single file plan");
            let state_path = root.path().join("recovery-state");
            let store = session::SessionStore::open(&state_path).expect("checkpoint store");
            store.begin(&plan).expect("single-file checkpoint");
            drop(store);
            let store = session::SessionStore::open(&state_path).expect("reopen checkpoint");
            assert!(
                store
                    .info()
                    .expect("recovery info")
                    .expect("saved queue")
                    .can_resume
            );
            let recovered = store
                .resume_checked("single-file", "restored", &|| false)
                .expect("resume single file");
            assert_eq!(recovered.items[0].input, video);
            store.finish("restored").expect("finish checkpoint");
            assert_eq!(plan.items.len(), 1);
            assert_eq!(plan.items[0].input, video);
            assert_eq!(
                plan.items[0].subtitle.as_ref().expect("paired"),
                &std::fs::canonicalize(input.join(expected)).expect("subtitle")
            );
            if expected == "custom.ass" {
                assert_eq!(
                    Path::new(plan.settings.subtitle_dir.as_ref().expect("saved path")),
                    plan.items[0].subtitle.as_ref().expect("subtitle")
                );
            }
        }
        let plan = BatchPlan::prepare(
            "folder".into(),
            Settings {
                target_volume_id: None,
                video: Default::default(),
                selected_audio_index: None,
                selected_subtitle_index: None,
                preferred_sub_lang: None,
                preferred_audio_lang: None,
                input_dir: input.to_string_lossy().into_owned(),
                output_dir: output.to_string_lossy().into_owned(),
                subtitle_dir: None,
                crf: 23,
                cpu_mode: crate::CpuMode::Maximum,
                anime_title: String::new(),
            },
        )
        .await
        .expect("folder plan");
        assert_eq!(plan.items.len(), 2);
    }
    #[tokio::test]
    async fn single_file_names_use_source_episode_while_folders_stay_sequential() {
        let root = tempfile::tempdir().expect("fixture");
        let input = root.path().join("input");
        std::fs::create_dir(&input).expect("input");
        let mut settings = Settings {
            target_volume_id: None,
            video: Default::default(),
            selected_audio_index: None,
            selected_subtitle_index: None,
            preferred_sub_lang: None,
            preferred_audio_lang: None,
            input_dir: String::new(),
            output_dir: root.path().join("output").to_string_lossy().into_owned(),
            subtitle_dir: None,
            crf: 23,
            cpu_mode: CpuMode::Maximum,
            anime_title: "My Anime".into(),
        };
        for (filename, episode) in [
            ("[Group] Anime Name - 03.mkv", 3),
            ("AnimeName_Ep03.mp4", 3),
            ("Anime Name - 3.avi", 3),
            ("Anime S02E07.webm", 7),
            ("Movie 2026 1080.mkv", 1),
            ("Ambiguous 01 02.mkv", 1),
        ] {
            let file = input.join(filename);
            std::fs::write(&file, []).expect("video");
            settings.input_dir = file.to_string_lossy().into_owned();
            let plan = BatchPlan::prepare("file-name".into(), settings.clone())
                .await
                .expect("file plan");
            assert_eq!(plan.items.len(), 1);
            assert_eq!(
                plan.items[0].output.file_name().expect("name"),
                format!("My Anime - Ep{episode:02} [Wii].mp4").as_str()
            );
            let store = session::SessionStore::open(&root.path().join("state")).expect("store");
            store.begin(&plan).expect("checkpoint");
            let restored = store
                .resume_checked("file-name", "resumed", &|| false)
                .expect("resume");
            assert_eq!(restored.items[0].output, plan.items[0].output);
            store.finish("resumed").expect("finish");
        }
        settings.anime_title.clear();
        let plan = BatchPlan::prepare("original-name".into(), settings.clone())
            .await
            .expect("original plan");
        assert_eq!(
            plan.items[0].output.file_name().expect("name"),
            "Ambiguous 01 02_Wii.mp4"
        );
        settings.anime_title = "My Anime".into();
        settings.input_dir = input.to_string_lossy().into_owned();
        let folder = BatchPlan::prepare("folder-names".into(), settings)
            .await
            .expect("folder plan");
        assert_eq!(folder.items.len(), 6);
        for (index, item) in folder.items.iter().enumerate() {
            assert_eq!(
                item.output.file_name().expect("name"),
                format!("My Anime - Ep{:02} [Wii].mp4", index + 1).as_str()
            );
        }
    }

    #[test]
    fn progress_handles_unknown_duration_and_invalid_timestamps() {
        assert_eq!(file_percent("5000000", Some(10.0)), Some(50.0));
        assert_eq!(file_percent("5000000", None), None);
        assert_eq!(file_percent("N/A", Some(10.0)), None);
        assert_eq!(file_percent("NaN", Some(10.0)), None);
        assert_eq!(file_percent("10000000", Some(0.0)), None);
        assert_eq!(file_percent("12000000", Some(10.0)), Some(99.9));
        assert_eq!(file_percent("-1000000", Some(10.0)), Some(0.0));
    }
}
