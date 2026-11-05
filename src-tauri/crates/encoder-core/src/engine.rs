use crate::{
    ActiveProcess, Cancellation, CpuMode, Item, ItemUpdate, Settings, TaskKind, Tools, artifacts,
    atomic_file, audio, crop, diagnostics, ffmpeg, file_percent_seconds, hardware, output,
    preflight, probe, process, profile, progress, records, session, subtitles, validation,
};
use std::{path::Path, process::Stdio, sync::Arc, time::Instant};
use tokio::fs;

#[derive(Default)]
struct SourceInfo {
    duration: Option<f64>,
    has_audio: Option<bool>,
    audio_streams: Option<Vec<audio::AudioStream>>,
    subtitle_streams: Option<Vec<probe::SubtitleStream>>,
}
async fn source_info(
    tools: &Tools,
    input: &Path,
    cancellation: &Cancellation,
    processes: &Arc<ActiveProcess>,
) -> SourceInfo {
    let mut command = process(&tools.ffprobe);
    command
        .args([
            "-v",
            "error",
            "-protocol_whitelist",
            "file,pipe",
            "-show_entries",
            "format=duration:stream=index,codec_type,codec_name,channels,channel_layout:stream_tags=language,title",
            "-of",
            "json",
        ])
        .arg(input);
    let text = match hardware::probe(command, cancellation, processes).await {
        Ok(text) => text,
        Err(_) => return SourceInfo::default(),
    };
    let document: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => return SourceInfo::default(),
    };
    SourceInfo {
        audio_streams: audio::parse_streams(&document).ok(),
        subtitle_streams: probe::subtitle_streams(&document).ok(),
        duration: document["format"]["duration"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v > 0.0),
        has_audio: document["streams"]
            .as_array()
            .map(|streams| streams.iter().any(|s| s["codec_type"] == "audio")),
    }
}

async fn finalize_output(
    temp: &Path,
    output: &Path,
    cancellation: &Cancellation,
    checkpoint: Option<(&Arc<session::SessionStore>, usize)>,
) -> Result<(), String> {
    if cancellation.is_cancelled() {
        return Err("Encoding cancelled.".into());
    }
    let source = temp.to_path_buf();
    let destination = output.to_path_buf();
    if let Some((store, index)) = checkpoint {
        let store = store.clone();
        let stop = cancellation.clone();
        let result = session::blocking(move || {
            store.commit_checked(index, &source, &destination, &|| stop.is_cancelled())
        })
        .await;
        if result.is_err() && cancellation.is_cancelled() {
            return Err("Encoding cancelled.".into());
        }
        result.map_err(|error| {
            format!("STATE_PERSISTENCE_FAILED: {error}. Any completed output was preserved.")
        })?;
    } else {
        session::blocking(move || {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&source)
                .map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            drop(file);
            atomic_file::rename(&source, &destination, false)
        })
        .await?;
    }
    Ok(())
}

pub(crate) struct Encoder<'a> {
    pub tools: &'a Tools,
    pub settings: &'a Settings,
    pub kind: TaskKind,
    pub cancellation: &'a Cancellation,
    pub processes: &'a Arc<ActiveProcess>,
}

impl Encoder<'_> {
    pub(crate) async fn encode(
        &self,
        item: &Item,
        encoder: hardware::VideoEncoder,
        checkpoint: Option<(&Arc<session::SessionStore>, usize)>,
        mut update: impl FnMut(ItemUpdate),
    ) -> Result<Option<String>, String> {
        let cancellation = self.cancellation;
        let processes = self.processes;
        if cancellation.is_cancelled() {
            return Err("Encoding cancelled.".into());
        }
        if let Some(error) = &item.validation_error {
            return Err(error.clone());
        }
        if fs::try_exists(&item.output)
            .await
            .map_err(|e| e.to_string())?
        {
            return Err("Output already exists; it was preserved. Choose another output folder or move the existing output.".into());
        }
        let parent = item.output.parent().ok_or("Invalid output directory.")?;
        let destination = parent.to_path_buf();
        let expected_volume = self.settings.target_volume_id.clone();
        session::blocking(move || {
            crate::storage::verify_target(&destination, expected_volume.as_deref())
        })
        .await?;
        if let Some(warning) = preflight::space(parent).await? {
            update(ItemUpdate::Warning(warning));
        }
        let temp = output::partial_path(&item.output)?;
        let mut staged: Option<subtitles::StagedSubtitle> = None;
        let reserved = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .await
            .map_err(|e| format!("Cannot reserve temporary output: {e}"))?;
        drop(reserved);
        if let Err(error) = artifacts::record(&temp).await {
            let _ = fs::remove_file(&temp).await;
            return Err(format!("Cannot journal temporary output: {error}"));
        }
        let result = self
            .encode_reserved(item, encoder, checkpoint, &temp, &mut staged, update)
            .await;
        // An early pipe/read/wait error can drop the lease before the child exits.
        // Finish reaping before deleting files or starting the next batch item.
        processes.wait_until_idle().await;
        let mut cleanup_errors = Vec::new();
        if result.is_err()
            && let Err(error) = fs::remove_file(&temp).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            cleanup_errors.push(format!("temporary output: {error}"));
        }
        if !temp.exists()
            && let Err(error) = artifacts::forget(&temp).await
        {
            cleanup_errors.push(error);
        }
        if let Some(staged) = staged
            && let Err(error) = staged.close()
        {
            cleanup_errors.push(error);
        }
        if !cleanup_errors.is_empty() {
            let details = cleanup_errors.join("; ");
            return match result {
                Ok(_) => Ok(Some(format!(
                    "Output saved; cleanup needs attention: {details}"
                ))),
                Err(error) => Err(format!("{error} Cleanup failed: {details}")),
            };
        }
        result
    }
    async fn encode_reserved(
        &self,
        item: &Item,
        encoder: hardware::VideoEncoder,
        checkpoint: Option<(&Arc<session::SessionStore>, usize)>,
        temp: &Path,
        staged: &mut Option<subtitles::StagedSubtitle>,
        mut update: impl FnMut(ItemUpdate),
    ) -> Result<Option<String>, String> {
        let Self {
            tools,
            settings,
            kind,
            cancellation,
            processes,
        } = *self;
        let parent = item.output.parent().ok_or("Invalid output directory.")?;

        let observed_item = item.clone();
        let observed_settings = settings.clone();
        let request = session::blocking(move || {
            records::Request::capture(&observed_item, &observed_settings)
        })
        .await?;
        let encode_started = Instant::now();
        let source = source_info(tools, &item.input, cancellation, processes).await;
        let duration = source.duration;
        processes.wait_until_idle().await;
        if cancellation.is_cancelled() {
            return Err("Encoding cancelled.".into());
        }
        let duration = if kind == TaskKind::Preview {
            duration.map(|d| d.min(10.0))
        } else {
            duration
        };
        let target = parent.to_path_buf();
        let expected_volume = settings.target_volume_id.clone();
        let volume = session::blocking(move || {
            crate::storage::verify_target(&target, expected_volume.as_deref())
        })
        .await?;
        volume.validate_size(duration)?;
        let crop = if settings.video.auto_crop {
            match crop::detect(tools, &item.input, source.duration, cancellation, processes).await {
                Ok(crop) => {
                    log::info!("Auto-crop for {}: {:?}", item.input.display(), crop);
                    crop
                }
                Err(error) => {
                    if cancellation.is_cancelled() {
                        return Err("Encoding cancelled.".into());
                    }
                    log::warn!("Crop detection unavailable; retaining original frame: {error}");
                    None
                }
            }
        } else {
            None
        };
        let mut filter = settings.video.filter_with_crop(crop)?;
        let selected_subtitle = probe::select_subtitle(
            source.subtitle_streams.as_deref(),
            settings.selected_subtitle_index,
            settings.preferred_sub_lang.as_deref(),
            item.subtitle.is_some(),
        )?;
        if let Some(index) = selected_subtitle {
            filter.push(',');
            filter.push_str(&ffmpeg::embedded_subtitle_filter(&item.input, index)?);
        } else if let Some(source) = &item.subtitle {
            match subtitles::read_scoped(source, item.subtitle_root.as_deref()).await {
                Ok(decoded) => {
                    if cancellation.is_cancelled() {
                        return Err("Encoding cancelled.".into());
                    }
                    if let Some(warning) = decoded.warning {
                        update(ItemUpdate::Warning(warning));
                    }
                    let ext = subtitles::extension(source).ok_or("Invalid subtitle extension.")?;
                    let owned = subtitles::stage(decoded.text, ext).await?;
                    filter.push_str(",subtitles=filename=");
                    filter.push_str(owned.filename()?);
                    filter.push_str(":charenc=UTF-8");
                    *staged = Some(owned);
                }
                Err(subtitles::ReadError::Syntax(reason)) => {
                    return Err(format!("Subtitle syntax error: {reason}"));
                }
                Err(error) => {
                    update(ItemUpdate::Warning(format!(
                        "{error} Encoding without external subtitles."
                    )));
                    update(ItemUpdate::SubtitlesOff);
                }
            }
        }
        let has_subtitles = staged.is_some() || selected_subtitle.is_some();
        if cancellation.is_cancelled() {
            return Err("Encoding cancelled.".into());
        }
        let mut command = process(&tools.ffmpeg);
        #[cfg(windows)]
        if settings.cpu_mode == CpuMode::Balanced {
            command.creation_flags(0x0800_0000 | 0x0000_4000);
        }

        let threads = settings.cpu_mode.threads(num_cpus::get()).to_string();
        command.args(["-threads", &threads, "-filter_threads", &threads]);
        command
            .current_dir(
                staged
                    .as_ref()
                    .map_or(parent, |subtitle| subtitle.directory()),
            )
            .args([
                "-hide_banner",
                "-loglevel",
                "repeat+level+info",
                "-stdin",
                "-y",
                "-progress",
                "pipe:1",
                "-stats_period",
                "0.25",
                "-nostats",
                "-benchmark",
            ]);
        if kind == TaskKind::Preview {
            command.args(["-ss", "00:00:00"]);
        }
        command
            .args(["-protocol_whitelist", "file,pipe"])
            .arg("-i")
            .arg(&item.input);
        if kind == TaskKind::Preview {
            command.args(["-t", "10"]);
        }
        let selected_audio = audio::select_index(
            source.audio_streams.as_deref(),
            settings.selected_audio_index,
            settings.preferred_audio_lang.as_deref(),
        )?;
        profile::configure_with_audio(
            &mut command,
            &filter,
            encoder,
            settings.crf,
            &threads,
            settings.video.max_fps()?,
            selected_audio,
        );
        if volume.is_fat32() {
            command.args(["-fs", &crate::storage::FAT32_GUARD_BYTES.to_string()]);
        }
        let mut child = command
            .arg(temp)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Cannot start FFmpeg: {e}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or("FFmpeg progress pipe is unavailable.")?;
        let stderr = child
            .stderr
            .take()
            .ok_or("FFmpeg diagnostic pipe is unavailable.")?;
        let mut child = processes.track(child)?;
        child.hold_subtitle(staged.take());
        let (duration_updates, fallback_duration) = tokio::sync::watch::channel(None);
        let diagnostics = tokio::spawn(async move {
            diagnostics::monitor(stderr, move |line| {
                if let Some(duration) = progress::stderr_duration(line) {
                    duration_updates.send_replace(Some(duration));
                }
            })
            .await
        });
        let mut final_fps = None;
        let read_result = tokio::select! {
            _ = cancellation.cancelled() => Ok(()),
            result = progress::read(stdout, |metrics| {
                final_fps = metrics.fps;
                let total = duration.or(*fallback_duration.borrow()).map(|duration| {
                    if kind == TaskKind::Preview { duration.min(10.0) } else { duration }
                });
                let percent = metrics.position.and_then(|position| file_percent_seconds(position, total));
                let eta = progress::eta_seconds(metrics.position, total, metrics.speed);
                update(ItemUpdate::Progress(percent, metrics.fps, metrics.speed.map(|speed| format!("{speed}x")), eta, metrics.frame));
            }) => result,
        };
        let stop_result = if cancellation.is_cancelled() || read_result.is_err() {
            child.stop().await
        } else {
            Ok(())
        };
        let status = tokio::select! {
            result = child.wait() => result.map_err(|e| format!("Cannot wait for FFmpeg: {e}")),
            _ = cancellation.cancelled(), if !cancellation.is_cancelled() => {
                child.stop().await?;
                child.wait().await.map_err(|e| e.to_string())
            },
        };
        let diagnostic = diagnostics
            .await
            .map_err(|e| format!("Cannot join FFmpeg diagnostics: {e}"))??;
        stop_result?;
        if cancellation.is_cancelled() {
            return Err("Encoding cancelled.".into());
        }
        if let Err(error) = read_result {
            return Err(format!("Cannot read FFmpeg progress: {error}"));
        }
        if !status?.success() {
            log::error!(
                "FFmpeg failure for {}. Last stderr lines:\n{}",
                item.input.display(),
                diagnostic
            );
            let category = if has_subtitles {
                "FFmpeg subtitle/filter or media parse failure"
            } else {
                "FFmpeg failed"
            };
            return Err(format!("{category}.\n\nFFmpeg stderr:\n{diagnostic}"));
        }
        // Release the exited encoder (and its subtitle staging) before tracking ffprobe.
        drop(child);
        processes.wait_until_idle().await;
        let validated = validation::validate(
            tools,
            temp,
            duration,
            source.has_audio,
            cancellation,
            processes,
            &settings.video,
        )
        .await
        .map_err(|error| format!("OUTPUT_VALIDATION_FAILED: {error}"))?;
        if volume.is_fat32() && validated.bytes >= crate::storage::FAT32_GUARD_BYTES {
            return Err("FAT32_4GB_WARNING: Encoding reached the safe size limit; incomplete output was discarded.".into());
        }
        let target = parent.to_path_buf();
        if session::blocking(move || crate::storage::inspect(&target))
            .await?
            .id
            != volume.id
        {
            return Err("Output volume changed during encoding.".into());
        }
        let verify_request = request.clone();
        let verify_item = item.clone();
        let verify_settings = settings.clone();
        session::blocking(move || verify_request.unchanged(&verify_item, &verify_settings)).await?;
        let mut warnings = Vec::new();
        if let Some(warning) = validation::fat32_warning(validated.bytes) {
            warnings.push(warning);
        }
        finalize_output(temp, &item.output, cancellation, checkpoint).await?;
        let mut measurements = records::Measurements::from_stderr(&diagnostic);
        measurements.encoder = encoder.name().into();
        measurements.subtitle_applied = has_subtitles;
        measurements.cpu_mode = match settings.cpu_mode {
            CpuMode::Balanced => "balanced",
            CpuMode::Maximum => "maximum",
        }
        .into();
        measurements.cpu_threads = settings.cpu_mode.threads(num_cpus::get());
        measurements.media_seconds = validated.duration;
        measurements.wall_seconds = encode_started.elapsed().as_secs_f64();
        measurements.final_fps = final_fps;
        measurements.output_bytes = validated.bytes;
        log::info!(
            "Encoding measurements: {}",
            serde_json::to_string(&measurements).map_err(|e| e.to_string())?
        );
        if let Some((store, _)) = checkpoint {
            let directory = store.records_directory();
            let output = item.output.clone();
            let persisted = measurements.clone();
            if let Err(error) =
                session::blocking(move || records::save(&directory, request, &output, persisted))
                    .await
            {
                log::warn!("Output saved but provenance/statistics could not be recorded: {error}");
                warnings.push("Output saved, but its reuse record could not be written. A later batch will identify it as unverified.".into());
            }
        }
        update(ItemUpdate::Measurements(measurements));
        Ok((!warnings.is_empty()).then(|| warnings.join(" ")))
    }
}
