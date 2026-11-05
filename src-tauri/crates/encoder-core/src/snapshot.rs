use crate::{
    ActiveProcess, Cancellation, Tools, VideoSettings, crop, ffmpeg, hardware, probe, process,
    subtitles,
};
use std::{path::Path, sync::Arc, time::Duration};

#[allow(clippy::too_many_arguments)]
pub async fn generate(
    tools: &Tools,
    path: &Path,
    seconds: u64,
    subtitle_index: Option<u32>,
    external: Option<&Path>,
    video: &VideoSettings,
    cancel: &Cancellation,
    processes: &Arc<ActiveProcess>,
) -> Result<Vec<u8>, String> {
    video.validate()?;
    crate::offline::local_path(path)?;
    let path = tokio::fs::canonicalize(path)
        .await
        .map_err(|e| format!("Cannot open source: {e}"))?;
    let info = probe::media_streams(tools, &path, cancel, processes).await?;
    probe::select_subtitle(Some(&info.subtitle_streams), subtitle_index, None, false)?;
    let seconds = seek_time(seconds, info.duration_seconds);
    let detected = if video.auto_crop {
        match crop::detect(tools, &path, info.duration_seconds, cancel, processes).await {
            Ok(crop) => crop,
            Err(error) if cancel.is_cancelled() => return Err(error),
            Err(error) => {
                log::warn!(
                    "Snapshot crop detection unavailable; retaining original frame: {error}"
                );
                None
            }
        }
    } else {
        None
    };
    let mut filter = video.filter_with_crop(detected)?;
    let mut staged = None;
    if let Some(index) = subtitle_index {
        filter.push_str(&format!(
            ",setpts=PTS+{seconds}/TB,{},setpts=PTS-STARTPTS",
            ffmpeg::embedded_subtitle_filter(&path, index)?
        ));
    } else if let Some(source) = external {
        let source = tokio::fs::canonicalize(source)
            .await
            .map_err(|e| e.to_string())?;
        let decoded = subtitles::read_scoped(&source, source.parent())
            .await
            .map_err(|e| e.to_string())?;
        let ext = subtitles::extension(&source).ok_or("Unsupported external subtitle format.")?;
        let subtitle = subtitles::stage(decoded.text, ext).await?;
        filter.push_str(&format!(
            ",setpts=PTS+{seconds}/TB,subtitles=filename={}:charenc=UTF-8,setpts=PTS-STARTPTS",
            subtitle.filename()?
        ));
        staged = Some(subtitle);
    }
    // JPEG viewers ignore anamorphic SAR; display the same DAR using square pixels.
    filter.push_str(",scale=w=min(854\\,max(2\\,trunc(ih*dar/2)*2)):h=ih,setsar=1");
    let directory = tempfile::Builder::new()
        .prefix("keroro-snapshot-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let output = directory.path().join("frame.jpg");
    let mut command = process(&tools.ffmpeg);
    command.current_dir(staged.as_ref().map_or(directory.path(), |s| s.directory()));
    command
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-y",
            "-ss",
            &seconds.to_string(),
            "-protocol_whitelist",
            "file,pipe",
            "-i",
        ])
        .arg(&path)
        .args([
            "-map",
            "0:v:0",
            "-an",
            "-sn",
            "-dn",
            "-vf",
            &filter,
            "-frames:v",
            "1",
            "-q:v",
            "2",
            "-update",
            "1",
        ])
        .arg(&output);
    let result = hardware::probe_timeout(command, cancel, processes, Duration::from_secs(45)).await;
    processes.wait_until_idle().await;
    result.map_err(|e| format!("Snapshot failed: {e}"))?;
    let metadata = tokio::fs::metadata(&output).await.map_err(|_| {
        "No frame was available at that timestamp. Try an earlier time.".to_string()
    })?;
    if metadata.len() == 0 || metadata.len() > 8 * 1024 * 1024 {
        return Err("Invalid snapshot size.".into());
    }
    let bytes = tokio::fs::read(&output).await.map_err(|e| e.to_string())?;
    if let Some(staged) = staged {
        staged.close()?;
    }
    directory.close().map_err(|e| e.to_string())?;
    Ok(bytes)
}
pub fn seek_time(seconds: u64, duration: Option<f64>) -> f64 {
    (seconds.min(7 * 24 * 60 * 60) as f64)
        .min(duration.map(|d| (d - 0.2).max(0.0)).unwrap_or(f64::MAX))
}

#[cfg(test)]
mod tests {
    #[test]
    fn short_sources_seek_before_the_end() {
        assert_eq!(super::seek_time(300, Some(10.0)), 9.8);
        assert_eq!(super::seek_time(300, Some(0.1)), 0.0);
        assert_eq!(super::seek_time(60, Some(3600.0)), 60.0);
    }
}
