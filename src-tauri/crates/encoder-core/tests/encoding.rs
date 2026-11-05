use keroro_encoder_core::{BatchPlan, Cancellation, Phase, Settings, Tools, run_batch};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

fn tools() -> Tools {
    Tools {
        ffmpeg: PathBuf::from(
            std::env::var_os("KERORO_TEST_FFMPEG").expect("Set KERORO_TEST_FFMPEG"),
        ),
        ffprobe: PathBuf::from(
            std::env::var_os("KERORO_TEST_FFPROBE").expect("Set KERORO_TEST_FFPROBE"),
        ),
    }
}

async fn fixture_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("keroro-{label}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&path)
        .await
        .expect("Create fixture folder");
    path
}

fn settings(input: &Path, output: &Path) -> Settings {
    Settings {
        target_volume_id: None,
        video: Default::default(),
        selected_audio_index: None,
        selected_subtitle_index: None,
        preferred_sub_lang: None,
        preferred_audio_lang: None,
        input_dir: input.to_string_lossy().into_owned(),
        output_dir: output.to_string_lossy().into_owned(),
        crf: 20,
        cpu_mode: keroro_encoder_core::CpuMode::Maximum,
        subtitle_dir: None,
        anime_title: String::new(),
    }
}

fn mean_volume_db(tools: &Tools, path: &Path) -> f64 {
    let measurement = Command::new(&tools.ffmpeg)
        .args(["-hide_banner", "-nostats", "-i"])
        .arg(path)
        .args([
            "-vn",
            "-af",
            "aformat=channel_layouts=stereo,volumedetect",
            "-f",
            "null",
            "-",
        ])
        .output()
        .expect("Measure audio level");
    assert!(measurement.status.success());
    let diagnostics = String::from_utf8_lossy(&measurement.stderr);
    diagnostics
        .lines()
        .find_map(|line| {
            line.split_once("mean_volume:")
                .and_then(|(_, value)| value.split_whitespace().next())
                .and_then(|value| value.parse::<f64>().ok())
        })
        .expect("FFmpeg reported a finite mean audio level")
}

#[tokio::test]
async fn rejects_ambiguous_outputs_and_same_directory() {
    let root = fixture_dir("validation").await;
    let input = root.join("input");
    let output = root.join("output");
    fs::create_dir_all(&input).await.expect("Create inputs");
    fs::write(input.join("Episode.mp4"), b"fixture")
        .await
        .expect("Write fixture");
    fs::write(input.join("episode.mkv"), b"fixture")
        .await
        .expect("Write fixture");
    let collision = BatchPlan::prepare("collision".into(), settings(&input, &output))
        .await
        .expect_err("Reject filename collision");
    assert!(collision.contains("Multiple inputs"));
    let same = BatchPlan::prepare("same-dir".into(), settings(&input, &input))
        .await
        .expect_err("Reject same directory");
    assert!(same.contains("separate output"));
    let mut invalid = settings(&input, &output);
    invalid.crf = 0;
    assert!(BatchPlan::prepare("invalid".into(), invalid).await.is_err());
    fs::remove_dir_all(root).await.expect("Clean test fixtures");
}

#[tokio::test]
#[ignore = "Requires KERORO_TEST_FFMPEG and KERORO_TEST_FFPROBE; run with --include-ignored"]
async fn real_batch_reports_progress_continues_after_failure_and_preserves_outputs() {
    let tools = tools();
    let root = fixture_dir("encoding").await;
    let input = root.join("input");
    let output = root.join("output");
    fs::create_dir_all(&input).await.expect("Create input");
    fs::create_dir_all(&output).await.expect("Create output");
    let source = input.join("a_valid.mp4");
    let fixture = Command::new(&tools.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=640x360:rate=24:duration=6",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=6",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-threads",
            "1",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&source)
        .status()
        .expect("Generate video fixture");
    assert!(fixture.success());
    fs::write(
        input.join("b_broken.mkv"),
        b"This is deliberately not a video.",
    )
    .await
    .expect("Corrupt fixture");
    fs::copy(&source, input.join("c_existing.mp4"))
        .await
        .expect("Copy valid fixture");
    let existing = output.join("c_existing_Wii.mp4");
    let original = b"Existing output must remain byte-for-byte intact.";
    fs::write(&existing, original)
        .await
        .expect("Existing output fixture");
    let plan = BatchPlan::prepare("real-test".into(), settings(&input, &output))
        .await
        .expect("Prepare batch");
    let mut events = Vec::new();
    let result = run_batch(plan, tools.clone(), Cancellation::new(), |progress| {
        events.push(progress)
    })
    .await;
    assert_eq!(result.phase, Phase::CompletedWithErrors);
    assert_eq!(
        (
            result.succeeded,
            result.skipped,
            result.failed,
            result.total_files
        ),
        (1, 1, 1, 3)
    );
    assert_eq!(result.batch_percent, 100.0);
    assert_eq!(
        fs::read(&existing).await.expect("Read preserved output"),
        original
    );
    assert!(
        events
            .iter()
            .any(|e| e.file_percent.is_some_and(|p| p > 0.0 && p < 100.0))
    );
    assert!(
        events
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence
                && pair[0].batch_percent <= pair[1].batch_percent)
    );
    assert!(
        events
            .iter()
            .any(|event| event.skipped == 1 && event.message.contains("Skipped episode"))
    );
    assert!(
        result
            .errors
            .iter()
            .any(|error| error.reason.contains("FFmpeg failed"))
    );
    let probe = Command::new(&tools.ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_name,width,height,pix_fmt,channels,sample_rate,start_time,duration,bit_rate,profile,level,avg_frame_rate",
            "-of",
            "default=noprint_wrappers=1",
        ])
        .arg(output.join("a_valid_Wii.mp4"))
        .output()
        .expect("Inspect encoded video");
    assert!(probe.status.success());
    let metadata = String::from_utf8_lossy(&probe.stdout);
    for expected in [
        "codec_name=h264",
        "level=31",
        "avg_frame_rate=24/1",
        "width=640",
        "height=480",
        "pix_fmt=yuv420p",
        "codec_name=aac",
        "profile=LC",
        "channels=2",
        "sample_rate=48000",
    ] {
        assert!(
            metadata.contains(expected),
            "Missing {expected}: {metadata}"
        );
    }
    assert!(
        metadata.contains("profile=Constrained Baseline")
            || metadata.contains("profile=Baseline")
            || metadata.contains("profile=Main")
    );
    let values = |key: &str| -> Vec<f64> {
        metadata
            .lines()
            .filter_map(|line| line.strip_prefix(key)?.parse().ok())
            .collect()
    };
    let starts = values("start_time=");
    let durations = values("duration=");
    assert_eq!(starts.len(), 2);
    assert_eq!(durations.len(), 2);
    assert!(
        (starts[0] - starts[1]).abs() < 0.05,
        "A/V start mismatch: {starts:?}"
    );
    assert!(
        (durations[0] - durations[1]).abs() < 0.06,
        "A/V duration mismatch: {durations:?}"
    );
    // AAC bitrate is a target; measured average varies with content and duration.
    let bitrates = values("bit_rate=");
    assert!(
        bitrates
            .last()
            .is_some_and(|rate| (140_000.0..180_000.0).contains(rate)),
        "Audio bitrate: {bitrates:?}"
    );
    let gain_db =
        mean_volume_db(&tools, &output.join("a_valid_Wii.mp4")) - mean_volume_db(&tools, &source);
    assert!(
        gain_db.abs() < 0.5,
        "Audio level must remain unchanged, got {gain_db} dB gain"
    );
    let mut files = fs::read_dir(&output).await.expect("Read outputs");
    while let Some(entry) = files.next_entry().await.expect("Read output entry") {
        assert!(!entry.file_name().to_string_lossy().ends_with(".mp4.part"));
    }
    // A fully successful batch must have the success phase used for sound/toast behavior.
    fs::remove_file(input.join("b_broken.mkv"))
        .await
        .expect("Remove corrupt fixture");
    fs::remove_file(input.join("c_existing.mp4"))
        .await
        .expect("Remove collision fixture");
    let clean_output = root.join("clean-output");
    let clean_plan = BatchPlan::prepare("success-test".into(), settings(&input, &clean_output))
        .await
        .expect("Prepare clean batch");
    let clean = run_batch(clean_plan, tools.clone(), Cancellation::new(), |_| {}).await;
    assert_eq!(clean.phase, Phase::Succeeded);
    assert_eq!((clean.succeeded, clean.skipped, clean.failed), (1, 0, 0));
    let rerun_plan = BatchPlan::prepare("skip-test".into(), settings(&input, &clean_output))
        .await
        .expect("Prepare rerun");
    let mut unavailable_tools = tools;
    unavailable_tools.ffmpeg = root.join("must-not-start-ffmpeg.exe");
    unavailable_tools.ffprobe = root.join("must-not-start-ffprobe.exe");
    let rerun = run_batch(rerun_plan, unavailable_tools, Cancellation::new(), |_| {}).await;
    assert_eq!(rerun.phase, Phase::Succeeded);
    assert_eq!((rerun.succeeded, rerun.skipped, rerun.failed), (0, 1, 0));
    fs::remove_dir_all(root)
        .await
        .expect("Clean real video fixtures");
}
