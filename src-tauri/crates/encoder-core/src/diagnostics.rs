use std::collections::VecDeque;
use tokio::io::{AsyncRead, AsyncReadExt};

/// Drain stderr completely without allocating an unbounded String for one hostile line.
pub async fn monitor(
    mut reader: impl AsyncRead + Unpin,
    mut on_line: impl FnMut(&str),
) -> Result<String, String> {
    let mut tail = VecDeque::new();
    let mut line = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut truncated = false;
    let mut previous_cr = false;
    loop {
        let count = reader.read(&mut chunk).await.map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        for byte in &chunk[..count] {
            if *byte == b'\n' || *byte == b'\r' {
                if *byte == b'\n' && previous_cr {
                    previous_cr = false;
                    continue;
                }
                previous_cr = *byte == b'\r';
                push(&mut tail, &line, truncated, &mut on_line);
                line.clear();
                truncated = false;
            } else {
                previous_cr = false;
                if line.len() < 2048 {
                    line.push(*byte);
                } else {
                    truncated = true;
                }
            }
        }
    }
    if !line.is_empty() || truncated {
        push(&mut tail, &line, truncated, &mut on_line);
    }
    Ok(tail.into_iter().collect::<Vec<_>>().join("\n"))
}
fn push(
    tail: &mut VecDeque<String>,
    bytes: &[u8],
    truncated: bool,
    on_line: &mut impl FnMut(&str),
) {
    if tail.len() == 20 {
        tail.pop_front();
    }
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    if !truncated {
        on_line(&text);
    }
    if truncated {
        text.push_str(" [truncated]");
    }
    tail.push_back(text);
}

#[cfg(test)]
async fn tail(reader: impl AsyncRead + Unpin) -> Result<String, String> {
    monitor(reader, |_| {}).await
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn carriage_return_stats_are_parsed_without_crlf_duplicate_lines() {
        let mut parser = crate::progress::StderrProgress::default();
        let input = b"[info] Duration: 00:03:00.00, bitrate: 123\r\ntime=00:00:30.00 speed=2x\rtime=00:01:00.00 speed=1x\r\n";
        let tail = super::monitor(input.as_slice(), |line| {
            parser.update(line);
        })
        .await
        .expect("monitor");
        assert_eq!(tail.lines().count(), 3);
        assert_eq!(parser.duration, Some(180.0));
        assert_eq!(parser.position, Some(60.0));
        assert_eq!(parser.speed, Some(1.0));
    }
    #[tokio::test]
    #[ignore = "Requires KERORO_TEST_FFMPEG pointing to the bundled FFmpeg executable"]
    async fn real_ffmpeg_stderr_provides_duration_time_and_speed() {
        use std::process::Stdio;
        use tokio::process::Command;
        let binary = std::env::var_os("KERORO_TEST_FFMPEG").expect("FFmpeg fixture binary");
        let root = tempfile::tempdir().expect("fixture");
        let input = root.path().join("\u{30b1}\u{30ed}\u{30ed} [test]' .mp4");
        let created = Command::new(&binary)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=64x48:r=10:d=2",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&input)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await
            .expect("make video");
        assert!(
            created.status.success(),
            "{}",
            String::from_utf8_lossy(&created.stderr)
        );
        let mut child = Command::new(&binary)
            .args([
                "-hide_banner",
                "-loglevel",
                "repeat+level+info",
                "-stats",
                "-stats_period",
                "0.1",
                "-i",
            ])
            .arg(&input)
            .args(["-f", "null", "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("run");
        let mut parser = crate::progress::StderrProgress::default();
        let tail = super::monitor(child.stderr.take().expect("stderr"), |line| {
            parser.update(line);
        })
        .await
        .expect("drain");
        assert!(child.wait().await.expect("wait").success(), "{tail}");
        assert_eq!(parser.duration, Some(2.0));
        assert!(parser.position.is_some_and(|position| position >= 1.9));
        assert!(parser.speed.is_some_and(|speed| speed > 0.0));
        assert!(
            crate::progress::eta_seconds(parser.position, parser.duration, parser.speed).is_some()
        );
    }
    #[tokio::test]
    async fn last_twenty_normal_lines_are_preserved_exactly() {
        let input = (1..=25)
            .map(|number| format!("diagnostic {number}\n"))
            .collect::<String>();
        let actual = super::tail(input.as_bytes()).await.expect("tail");
        let expected = (6..=25)
            .map(|number| format!("diagnostic {number}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(actual, expected);
    }
    #[tokio::test]
    async fn diagnostics_cap_long_lines_and_preserve_final_errors() {
        let mut bytes = vec![b'x'; 200_000];
        bytes.extend_from_slice(b"\nSubtitle parser failed\n");
        let text = super::tail(bytes.as_slice()).await.expect("drain");
        assert!(text.len() < 2200);
        assert!(text.contains("[truncated]") && text.contains("Subtitle parser failed"));
    }
}
