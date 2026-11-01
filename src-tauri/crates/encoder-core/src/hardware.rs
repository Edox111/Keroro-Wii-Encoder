//! Probe compiled encoders and real driver availability, in preferred GPU order.
use crate::{ActiveProcess, Cancellation, Tools, diagnostics, process, profile};
use std::{process::Stdio, sync::Arc, time::Duration};
use tokio::{io::AsyncReadExt, process::Command};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoEncoder {
    Nvenc,
    Amf,
    Qsv,
    Cpu,
}
impl VideoEncoder {
    pub fn name(self) -> &'static str {
        match self {
            Self::Nvenc => "h264_nvenc",
            Self::Amf => "h264_amf",
            Self::Qsv => "h264_qsv",
            Self::Cpu => "libx264",
        }
    }
}
fn candidates(listing: &str) -> Vec<VideoEncoder> {
    [VideoEncoder::Nvenc, VideoEncoder::Amf, VideoEncoder::Qsv]
        .into_iter()
        .filter(|encoder| {
            listing.lines().any(|line| {
                let mut fields = line.split_whitespace();
                fields
                    .next()
                    .is_some_and(|flags| flags.len() == 6 && flags.starts_with('V'))
                    && fields.next() == Some(encoder.name())
            })
        })
        .collect()
}

pub(crate) async fn probe(
    mut command: Command,
    cancel: &Cancellation,
    processes: &Arc<ActiveProcess>,
) -> Result<String, String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or("Missing encoder probe stdout")?;
    let stderr = child.stderr.take().ok_or("Missing encoder probe stderr")?;
    let lease = processes.track(child)?;
    let output = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stdout
            .take(1024 * 1024)
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(String::from_utf8_lossy(&bytes).into_owned())
    });
    let diagnostics = tokio::spawn(async move { diagnostics::monitor(stderr, |_| {}).await });
    let status = tokio::select! {
        _ = cancel.cancelled() => Err("Encoder detection cancelled.".to_string()),
        status = tokio::time::timeout(Duration::from_secs(8), lease.wait()) =>
            status.map_err(|_| "Encoder probe timed out.".to_string()).and_then(|status| status),
    };
    if status.is_err() {
        let _ = lease.kill_and_wait().await;
    }
    drop(lease);
    processes.wait_until_idle().await;
    let output = output.await.map_err(|e| e.to_string())??;
    let tail = diagnostics.await.map_err(|e| e.to_string())??;
    if !status?.success() {
        return Err(tail);
    }
    Ok(output)
}

pub async fn detect(
    tools: &Tools,
    cancel: &Cancellation,
    processes: &Arc<ActiveProcess>,
) -> VideoEncoder {
    if cancel.is_cancelled() {
        return VideoEncoder::Cpu;
    }
    let mut command = process(&tools.ffmpeg);
    command.args(["-hide_banner", "-encoders"]);
    let listing = match probe(command, cancel, processes).await {
        Ok(listing) => listing,
        Err(error) => {
            log::warn!("Encoder enumeration unavailable; using CPU: {error}");
            return VideoEncoder::Cpu;
        }
    };
    for encoder in candidates(&listing) {
        if cancel.is_cancelled() {
            break;
        }
        let mut command = process(&tools.ffmpeg);
        command.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=640x480:r=30:d=0.1",
        ]);
        profile::configure(&mut command, profile::VIDEO_FILTER, encoder, 23, "0");
        command.args(["-frames:v", "2", "-an", "-f", "null", "-"]);
        match probe(command, cancel, processes).await {
            Ok(_) => {
                log::info!("Selected usable video encoder: {}", encoder.name());
                return encoder;
            }
            Err(error) => log::info!(
                "{} is listed but unusable; trying next encoder: {error}",
                encoder.name()
            ),
        }
    }
    log::info!("Selected CPU encoder: libx264 superfast / automatic threads");
    VideoEncoder::Cpu
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "Requires bundled FFmpeg/ffprobe paths"]
    async fn real_encoder_detection_checks_drivers_and_reaps_every_probe() {
        let tools = Tools {
            ffmpeg: std::env::var_os("KERORO_TEST_FFMPEG")
                .expect("FFmpeg")
                .into(),
            ffprobe: std::env::var_os("KERORO_TEST_FFPROBE")
                .expect("ffprobe")
                .into(),
        };
        let processes = Arc::new(ActiveProcess::default());
        let cancel = Cancellation::new();
        let mut command = process(&tools.ffmpeg);
        command.args(["-hide_banner", "-encoders"]);
        let listing = probe(command, &cancel, &processes)
            .await
            .expect("encoder listing");
        assert!(
            !candidates(&listing).is_empty(),
            "Bundled FFmpeg must list hardware candidates"
        );
        let mut cpu = process(&tools.ffmpeg);
        cpu.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=640x480:r=30:d=0.1",
        ]);
        profile::configure(&mut cpu, profile::VIDEO_FILTER, VideoEncoder::Cpu, 23, "0");
        cpu.args(["-frames:v", "2", "-an", "-f", "null", "-"]);
        probe(cpu, &cancel, &processes)
            .await
            .expect("CPU superfast/automatic-thread fallback profile");
        let selected = detect(&tools, &cancel, &processes).await;
        assert!(selected == VideoEncoder::Cpu || candidates(&listing).contains(&selected));
        assert!(processes.0.lock().expect("slot").is_none());
        assert!(!processes.is_paused());
        // Test output documents which physical encoding path this environment exercised.
        eprintln!(
            "Usable encoder selected by runtime probe: {}",
            selected.name()
        );
    }

    #[tokio::test]
    async fn cancelled_detection_does_not_spawn_a_child() {
        let cancel = Cancellation::new();
        cancel.cancel();
        let processes = Arc::new(ActiveProcess::default());
        let tools = Tools {
            ffmpeg: "must-not-run.exe".into(),
            ffprobe: "unused.exe".into(),
        };
        assert_eq!(detect(&tools, &cancel, &processes).await, VideoEncoder::Cpu);
        assert!(processes.0.lock().expect("slot").is_none());
    }

    #[test]
    fn listing_matches_exact_video_encoder_names_in_priority_order() {
        let listing = " V..... h264_qsv Intel\n V....D h264_amf AMD\n V....D h264_nvenc NVIDIA\n A..... h264_nvenc_fake fake\n V..... libx264 CPU\n";
        assert_eq!(
            candidates(listing),
            vec![VideoEncoder::Nvenc, VideoEncoder::Amf, VideoEncoder::Qsv]
        );
        assert!(candidates("text h264_nvenc\n A..... h264_amf audio").is_empty());
    }
}
