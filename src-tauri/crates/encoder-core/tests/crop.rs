use keroro_encoder_core::{
    ActiveProcess, BatchPlan, Cancellation, CpuMode, Phase, Settings, Tools, VideoSettings, crop,
    run_batch, video::EncoderPreference,
};
use std::sync::Arc;

#[tokio::test]
#[ignore = "Requires bundled FFmpeg/ffprobe paths"]
async fn crop_prepass_and_cpu_pipeline_preserve_subtitles_and_finalize_outputs() {
    let tools = Tools {
        ffmpeg: std::env::var_os("KERORO_TEST_FFMPEG")
            .expect("ffmpeg")
            .into(),
        ffprobe: std::env::var_os("KERORO_TEST_FFPROBE")
            .expect("ffprobe")
            .into(),
    };
    let root = tempfile::tempdir().expect("fixture");
    let input = root.path().join("ケロロ - Ep03.mkv");
    let output = root.path().join("output");
    let fixture = tokio::process::Command::new(&tools.ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:size=640x352:rate=24:duration=2,pad=640:480:0:64:black",
            "-c:v",
            "libx264",
            "-threads",
            "1",
            "-an",
        ])
        .arg(&input)
        .output()
        .await
        .expect("fixture process");
    assert!(
        fixture.status.success(),
        "{}",
        String::from_utf8_lossy(&fixture.stderr)
    );
    let cancel = Cancellation::new();
    let processes = Arc::new(ActiveProcess::default());
    let detected = crop::detect(&tools, &input, Some(2.0), &cancel, &processes)
        .await
        .expect("detection")
        .expect("letterbox crop");
    assert_eq!(detected.filter(), "crop=640:352:0:64");
    assert!(processes.0.lock().expect("slot").is_none());

    let subtitle = root.path().join("Ep03.srt");
    tokio::fs::write(
        &subtitle,
        "1\n00:00:00,000 --> 00:00:02,000\nVISIBLE SUBTITLE\n",
    )
    .await
    .expect("subtitle");
    let geometry = VideoSettings {
        auto_crop: true,
        encoder_override: EncoderPreference::Cpu,
        aspect_ratio: keroro_encoder_core::AspectRatio::Wide,
        resolution_preset: keroro_encoder_core::ResolutionPreset::Wide,
        ..Default::default()
    };
    let chain = geometry.filter_with_crop(Some(detected)).expect("filter");
    assert!(chain.starts_with("crop=640:352:0:64,scale="));
    let settings = Settings {
        input_dir: input.to_string_lossy().into_owned(),
        output_dir: output.to_string_lossy().into_owned(),
        subtitle_dir: Some(subtitle.to_string_lossy().into_owned()),
        anime_title: "Keroro".into(),
        crf: 23,
        cpu_mode: CpuMode::Maximum,
        video: geometry,
    };
    let plan = BatchPlan::prepare("crop-regression".into(), settings)
        .await
        .expect("queue");
    let result = run_batch(plan, tools.clone(), cancel, |_| {}).await;
    assert_eq!(result.phase, Phase::Succeeded, "{:?}", result.errors);
    assert_eq!(result.succeeded, 1);
    let movie = output.join("Keroro - Ep03 [Wii].mp4");
    assert!(movie.is_file());
    assert!(!output.join("Keroro - Ep03 [Wii].mp4.part").exists());
    let frame = tokio::process::Command::new(&tools.ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(&movie)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "format=gray",
            "-f",
            "rawvideo",
            "-",
        ])
        .output()
        .await
        .expect("decode");
    assert!(frame.status.success());
    assert_eq!(frame.stdout.len(), 848 * 480);
    assert!(
        frame.stdout[50 * 848 + 424] > 40,
        "Original top black bar must be removed"
    );
    assert!(
        frame.stdout[280 * 848..]
            .iter()
            .filter(|pixel| **pixel > 200)
            .count()
            > 100,
        "Subtitles must remain visible after crop/resize"
    );

    // Full-frame content must not be cropped merely to align dimensions to 16.
    // All-black content has no confident bounds and must also remain unchanged.
    for color in ["red", "black"] {
        let plain = root.path().join(format!("plain-{color}.mkv"));
        let fixture = tokio::process::Command::new(&tools.ffmpeg)
            .args(["-v", "error", "-f", "lavfi", "-i"])
            .arg(format!("color=c={color}:size=854x480:rate=24:duration=1"))
            .args(["-c:v", "libx264", "-threads", "1", "-an"])
            .arg(&plain)
            .output()
            .await
            .expect("full-frame fixture");
        assert!(fixture.status.success());
        assert!(
            crop::detect(&tools, &plain, Some(1.0), &Cancellation::new(), &processes)
                .await
                .expect("full frame detection")
                .is_none()
        );
    }
    let cancelled = Cancellation::new();
    cancelled.cancel();
    assert!(
        crop::detect(&tools, &input, Some(2.0), &cancelled, &processes)
            .await
            .is_err()
    );
    assert!(processes.0.lock().expect("slot").is_none());
    let missing = Tools {
        ffmpeg: root.path().join("missing.exe"),
        ffprobe: tools.ffprobe.clone(),
    };
    assert!(
        crop::detect(&missing, &input, None, &Cancellation::new(), &processes)
            .await
            .is_err()
    );
    assert!(processes.0.lock().expect("slot").is_none());
}
