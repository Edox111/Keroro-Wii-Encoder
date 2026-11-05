//! Bounded, cancellable crop detection. Only validated integers enter filters.
use crate::{ActiveProcess, Cancellation, Tools, diagnostics, process};
use std::{path::Path, process::Stdio, sync::Arc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CropRect {
    width: u32,
    height: u32,
    x: u32,
    y: u32,
}
impl CropRect {
    fn parse(value: &str, frame: (u32, u32)) -> Option<Self> {
        let numbers = value
            .split(':')
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        let [width, height, x, y] = numbers.as_slice() else {
            return None;
        };
        let rect = Self {
            width: *width,
            height: *height,
            x: *x,
            y: *y,
        };
        if rect.width < 16
            || rect.height < 16
            || [rect.width, rect.height, rect.x, rect.y]
                .iter()
                .any(|v| v % 2 != 0)
            || rect.x.checked_add(rect.width)? > frame.0
            || rect.y.checked_add(rect.height)? > frame.1
            || u64::from(rect.width) * u64::from(rect.height)
                < u64::from(frame.0) * u64::from(frame.1) / 4
            || (frame.0 - rect.width < 16 && frame.1 - rect.height < 16)
        {
            return None;
        }
        Some(rect)
    }
    pub fn filter(self) -> String {
        format!("crop={}:{}:{}:{}", self.width, self.height, self.x, self.y)
    }
}

#[derive(Default)]
struct Detection {
    frame: Option<(u32, u32)>,
    candidate: Option<CropRect>,
    reports: u32,
}
impl Detection {
    fn update(&mut self, line: &str) {
        if line.contains("Parsed_showinfo_") {
            let frame = line
                .split_whitespace()
                .find_map(|part| part.strip_prefix("s:"))
                .and_then(|size| size.split_once('x'))
                .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)));
            if let Some(frame) = frame {
                if self.frame != Some(frame) {
                    self.candidate = None;
                    self.reports = 0;
                }
                self.frame = Some(frame);
            }
        }
        if line.contains("Parsed_cropdetect_")
            && let Some(value) = line
                .split_whitespace()
                .find_map(|part| part.strip_prefix("crop="))
        {
            self.candidate = self.frame.and_then(|frame| CropRect::parse(value, frame));
            self.reports = self.reports.saturating_add(1);
        }
    }
    fn finish(self) -> Option<CropRect> {
        (self.reports >= 3).then_some(self.candidate).flatten()
    }
}

pub async fn detect(
    tools: &Tools,
    input: &Path,
    duration: Option<f64>,
    cancel: &Cancellation,
    processes: &Arc<ActiveProcess>,
) -> Result<Option<CropRect>, String> {
    if cancel.is_cancelled() {
        return Err("Crop detection cancelled.".into());
    }
    // Short clips have no frame at minute one. Analyze their opening instead.
    let seek = if duration.is_some_and(|d| d.is_finite() && d < 62.0) {
        "0"
    } else {
        "60"
    };
    let mut command = process(&tools.ffmpeg);
    command
        .args([
            "-hide_banner",
            "-nostdin",
            "-nostats",
            "-loglevel",
            "info",
            "-ss",
            seek,
            "-threads",
            "1",
            "-filter_threads",
            "1",
            "-protocol_whitelist",
            "file,pipe",
            "-i",
        ])
        .arg(input)
        .args([
            "-t",
            "2",
            "-map",
            "0:v:0",
            "-an",
            "-sn",
            "-dn",
            "-vf",
            "showinfo,cropdetect=limit=24:round=16:reset=0",
            "-f",
            "null",
            "-",
        ]);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stderr = child.stderr.take().ok_or("Missing crop detector stderr.")?;
    let lease = processes.track(child)?;
    let reader = tokio::spawn(async move {
        let mut detection = Detection::default();
        let tail = diagnostics::monitor(stderr, |line| detection.update(line)).await?;
        Ok::<_, String>((detection.finish(), tail))
    });
    let status = tokio::select! {
        _=cancel.cancelled()=>Err("Crop detection cancelled.".to_string()),
        result=tokio::time::timeout(Duration::from_secs(15),lease.wait())=>
            result.map_err(|_|"Crop detection timed out.".to_string()).and_then(|status|status),
    };
    if status.is_err() {
        let _ = lease.kill_and_wait().await;
    }
    drop(lease);
    processes.wait_until_idle().await;
    let (crop, tail) = reader.await.map_err(|e| e.to_string())??;
    if !status?.success() {
        return Err(format!("Crop pre-pass failed: {tail}"));
    }
    Ok(crop)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parser_rejects_invalid_bounds_injection_and_alignment_only_crops() {
        assert_eq!(
            CropRect::parse("640:352:0:64", (640, 480))
                .expect("crop")
                .filter(),
            "crop=640:352:0:64"
        );
        for text in [
            "640:352:0:64,drawtext=x",
            "640:352:-1:64",
            "854:352:2:64",
            "854:480:0:0",
            "848:480:0:0",
            "16:16:0:0",
            "640:353:0:64",
            "4294967295:352:2:64",
        ] {
            assert!(CropRect::parse(text, (854, 480)).is_none(), "{text}");
        }
        let mut d = Detection::default();
        d.update("[Parsed_showinfo_0] n:0 s:640x480");
        for _ in 0..3 {
            d.update("[Parsed_cropdetect_1] crop=640:352:0:64");
        }
        assert!(d.candidate.is_some());
        d.update("[Parsed_cropdetect_1] crop=640:480:0:0");
        assert!(
            d.finish().is_none(),
            "A later full-frame report cancels an earlier crop"
        );
    }
}
