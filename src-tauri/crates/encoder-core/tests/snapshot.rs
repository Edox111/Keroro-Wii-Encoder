use keroro_encoder_core::{ActiveProcess, Cancellation, Tools, VideoSettings, snapshot};
use std::sync::Arc;

#[tokio::test]
#[ignore = "Requires bundled FFmpeg/ffprobe paths"]
async fn snapshots_preserve_subtitle_timing_after_fast_seek_and_reap_children() {
    let tools = Tools {
        ffmpeg: std::env::var_os("KERORO_TEST_FFMPEG")
            .expect("ffmpeg")
            .into(),
        ffprobe: std::env::var_os("KERORO_TEST_FFPROBE")
            .expect("ffprobe")
            .into(),
    };
    let root = tempfile::tempdir().expect("fixture");
    let subs = root.path().join("字幕.srt");
    std::fs::write(
        &subs,
        "1\n00:00:04,000 --> 00:00:06,000\nVISIBLE AT FIVE SECONDS\n",
    )
    .expect("subtitles");
    let input = root.path().join("ケロロ [team], Bob's Ep03.mkv");
    let result = tokio::process::Command::new(&tools.ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=black:size=320x240:rate=24:duration=8",
            "-i",
        ])
        .arg(&subs)
        .args([
            "-map",
            "0:v:0",
            "-map",
            "1:s:0",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-threads",
            "1",
            "-c:s",
            "srt",
        ])
        .arg(&input)
        .output()
        .await
        .expect("fixture");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let processes = Arc::new(ActiveProcess::default());
    let mut bright = Vec::new();
    for (seconds, embedded, external) in [
        (0, Some(0), None),
        (5, Some(0), None),
        (5, None, Some(subs.as_path())),
        (300, Some(0), None),
    ] {
        let jpeg = snapshot::generate(
            &tools,
            &input,
            seconds,
            embedded,
            external,
            &VideoSettings::default(),
            &Cancellation::new(),
            &processes,
        )
        .await
        .expect("snapshot");
        assert!(jpeg.starts_with(&[0xff, 0xd8]));
        let image = root.path().join("check.jpg");
        std::fs::write(&image, jpeg).expect("image");
        let decoded = tokio::process::Command::new(&tools.ffmpeg)
            .args(["-v", "error", "-i"])
            .arg(image)
            .args([
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray",
                "pipe:1",
            ])
            .output()
            .await
            .expect("decode");
        assert!(decoded.status.success());
        bright.push(decoded.stdout.iter().filter(|v| **v > 128).count());
        assert!(processes.0.lock().expect("slot").is_none());
    }
    assert_eq!(bright[0], 0);
    assert!(
        bright[1] > 100,
        "Subtitle was not aligned to the fast seek: {bright:?}"
    );
    assert_eq!(bright[1], bright[2], "External and embedded text differ");
    assert_eq!(
        bright[3], 0,
        "Past-end seek must clamp and render the final frame"
    );
}
