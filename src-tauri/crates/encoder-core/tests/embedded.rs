use keroro_encoder_core::{
    ActiveProcess, BatchPlan, Cancellation, CpuMode, Phase, Settings, Tools, VideoSettings, probe,
    run_batch, video::EncoderPreference,
};
use std::sync::Arc;
#[tokio::test]
#[ignore = "Requires bundled FFmpeg/ffprobe paths"]
async fn embedded_selection_burns_chosen_text_with_escaped_unicode_windows_paths() {
    let tools = Tools {
        ffmpeg: std::env::var_os("KERORO_TEST_FFMPEG")
            .expect("ffmpeg")
            .into(),
        ffprobe: std::env::var_os("KERORO_TEST_FFPROBE")
            .expect("ffprobe")
            .into(),
    };
    let root = tempfile::tempdir().expect("fixture");
    let input = root.path().join("日本 [group], Bob's Ep03.mkv");
    let english = root.path().join("english.srt");
    let japanese = root.path().join("japanese.srt");
    std::fs::write(
        &english,
        "1\n00:00:00,000 --> 00:00:01,000\nENGLISH LONG SUBTITLE\n",
    )
    .expect("English");
    std::fs::write(&japanese, "1\n00:00:00,000 --> 00:00:01,000\nJP\n").expect("Japanese");
    let fixture = tokio::process::Command::new(&tools.ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=black:size=320x240:rate=24:duration=1",
            "-i",
        ])
        .arg(&english)
        .arg("-i")
        .arg(&japanese)
        .args([
            "-map",
            "0:v:0",
            "-map",
            "1:s:0",
            "-map",
            "2:s:0",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-threads",
            "1",
            "-c:s",
            "srt",
            "-metadata:s:s:0",
            "language=eng",
            "-metadata:s:s:1",
            "language=jpn",
            "-metadata:s:s:0",
            "title=English Full",
        ])
        .arg(&input)
        .output()
        .await
        .expect("fixture");
    assert!(
        fixture.status.success(),
        "{}",
        String::from_utf8_lossy(&fixture.stderr)
    );
    let media = probe::media_streams(
        &tools,
        &input,
        &Cancellation::new(),
        &Arc::new(ActiveProcess::default()),
    )
    .await
    .expect("probe");
    assert!(media.audio_streams.is_empty());
    assert_eq!(media.subtitle_streams.len(), 2);
    assert_eq!(media.subtitle_streams[1].index, 1);
    assert_eq!(media.subtitle_streams[1].stream_index, 2);
    assert_eq!(
        media.subtitle_streams[0].title.as_deref(),
        Some("English Full")
    );
    let mut counts = Vec::new();
    for (name, selected, preferred) in [
        ("english", Some(0), None),
        ("japanese", Some(1), None),
        ("automatic", None, Some("eng")),
        ("none", None, Some("none")),
    ] {
        let output = root.path().join(name);
        let settings = Settings {
            target_volume_id: None,
            input_dir: input.to_string_lossy().into_owned(),
            output_dir: output.to_string_lossy().into_owned(),
            subtitle_dir: None,
            anime_title: "Embedded".into(),
            crf: 23,
            cpu_mode: CpuMode::Maximum,
            selected_audio_index: None,
            preferred_audio_lang: Some("jpn".into()),
            selected_subtitle_index: selected,
            preferred_sub_lang: preferred.map(str::to_owned),
            video: VideoSettings {
                encoder_override: EncoderPreference::Cpu,
                ..Default::default()
            },
        };
        let queue = BatchPlan::prepare(name.into(), settings)
            .await
            .expect("queue");
        let mut progress_seen = false;
        let result = run_batch(queue, tools.clone(), Cancellation::new(), |p| {
            if p.frame.is_some() && p.fps.is_some() {
                progress_seen = true;
            }
        })
        .await;
        assert_eq!(
            result.phase,
            Phase::Succeeded,
            "{name}: {:?}",
            result.errors
        );
        assert!(progress_seen, "No parsed frame/FPS telemetry");
        let frame = tokio::process::Command::new(&tools.ffmpeg)
            .args(["-v", "error", "-ss", "0.5", "-i"])
            .arg(output.join("Embedded - Ep03 [Wii].mp4"))
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
        assert!(frame.status.success());
        counts.push(frame.stdout.iter().filter(|v| **v > 128).count());
        let encoded = probe::media_streams(
            &tools,
            &output.join("Embedded - Ep03 [Wii].mp4"),
            &Cancellation::new(),
            &Arc::new(ActiveProcess::default()),
        )
        .await
        .expect("output probe");
        assert!(
            encoded.subtitle_streams.is_empty(),
            "Subtitles must be burned, not soft-muxed"
        );
    }
    assert!(
        counts[0] > counts[1] && counts[1] > 100,
        "Wrong burned tracks: {counts:?}"
    );
    assert_eq!(counts[0], counts[2], "Preference failed to select English");
    assert_eq!(counts[3], 0, "None must not burn embedded subtitles");
}
