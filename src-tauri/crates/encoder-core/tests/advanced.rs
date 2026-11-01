use keroro_encoder_core::{BatchPlan, Cancellation, Phase, Settings, Tools, run_batch};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::fs;

#[test]
fn video_candidates_are_case_insensitive_and_reject_other_files() {
    for ext in keroro_encoder_core::VIDEO_EXTENSIONS {
        assert!(keroro_encoder_core::is_video_path(Path::new(&format!(
            "Episode.{}",
            ext.to_uppercase()
        ))));
    }
    for name in [
        "notes.txt",
        "Episode.mp4.part",
        "Episode.srt",
        "no-extension",
    ] {
        assert!(!keroro_encoder_core::is_video_path(Path::new(name)));
    }
}

#[tokio::test]
#[ignore = "Requires bundled FFmpeg/ffprobe paths"]
async fn all_supported_containers_transcode_to_wii_mp4() {
    let tools = tools();
    let root = directory().await;
    let input = root.join("input");
    let output = root.join("output");
    fs::create_dir_all(&input).await.expect("input");
    // Generate actual containers, rather than renaming an MP4 seven times.
    for (index, (ext, codec)) in [
        ("mp4", "libx264"),
        ("mkv", "libx264"),
        ("avi", "mpeg4"),
        ("mov", "libx264"),
        ("WMV", "wmv2"),
        ("FLV", "flv"),
        ("webm", "libvpx-vp9"),
    ]
    .iter()
    .enumerate()
    {
        let fixture = Command::new(&tools.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:size=320x240:rate=60:duration=0.4",
                "-an",
                "-c:v",
                codec,
                "-threads",
                "1",
            ])
            .arg(input.join(format!("Episode{:02}.{ext}", index + 1)))
            .output()
            .expect("fixture process");
        assert!(
            fixture.status.success(),
            "{}",
            String::from_utf8_lossy(&fixture.stderr)
        );
    }
    fs::write(input.join("ignore.txt"), "not a video")
        .await
        .expect("non-video");
    fs::create_dir(input.join("directory.mp4"))
        .await
        .expect("non-file");
    let plan = BatchPlan::prepare("containers".into(), settings(&input, &output, None))
        .await
        .expect("plan");
    let result = run_batch(plan, tools.clone(), Cancellation::new(), |_| {}).await;
    assert_eq!(result.phase, Phase::Succeeded, "{:?}", result.errors);
    assert_eq!(result.total_files, 7);
    assert_eq!(result.succeeded, 7);
    for episode in 1..=7 {
        let probe = Command::new(&tools.ffprobe)
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=codec_name,width,height,pix_fmt,profile,level,avg_frame_rate",
                "-of",
                "json",
            ])
            .arg(output.join(format!("Sgt Keroro - Ep{episode:02} [Wii].mp4")))
            .output()
            .expect("probe");
        assert!(probe.status.success());
        let metadata: serde_json::Value =
            serde_json::from_slice(&probe.stdout).expect("probe JSON");
        let video = &metadata["streams"][0];
        assert_eq!(video["codec_name"], "h264");
        assert!(matches!(
            video["profile"].as_str(),
            Some("Constrained Baseline" | "Baseline" | "Main")
        ));
        assert_eq!(video["level"], 30);
        assert_eq!(video["width"], 640);
        assert_eq!(video["height"], 480);
        assert_eq!(video["pix_fmt"], "yuv420p");
        let (num, den) = video["avg_frame_rate"]
            .as_str()
            .expect("frame rate")
            .split_once('/')
            .expect("fraction");
        let fps = num.parse::<f64>().expect("numerator") / den.parse::<f64>().expect("denominator");
        assert!(
            fps > 0.0 && fps <= 30.0,
            "Output exceeds SD frame rate: {fps}"
        );
    }
    fs::remove_dir_all(root).await.expect("clean fixture");
}

fn tools() -> Tools {
    Tools {
        ffmpeg: std::env::var_os("KERORO_TEST_FFMPEG")
            .expect("FFmpeg path")
            .into(),
        ffprobe: std::env::var_os("KERORO_TEST_FFPROBE")
            .expect("ffprobe path")
            .into(),
    }
}
async fn directory() -> PathBuf {
    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let number = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "keroro-advanced-{}-{stamp}-{number}",
        std::process::id()
    ));
    fs::create_dir(&path).await.expect("create unique fixture");
    path
}
fn settings(input: &Path, output: &Path, subtitles: Option<&Path>) -> Settings {
    Settings {
        input_dir: input.to_string_lossy().into_owned(),
        output_dir: output.to_string_lossy().into_owned(),
        crf: 23,
        cpu_mode: keroro_encoder_core::CpuMode::Maximum,
        subtitle_dir: subtitles.map(|p| p.to_string_lossy().into_owned()),
        anime_title: "Sgt Keroro".into(),
    }
}
fn make_video(tools: &Tools, path: &Path, source: &str) {
    assert!(
        Command::new(&tools.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                source,
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=12",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-threads",
                "1",
                "-c:a",
                "aac",
                "-shortest"
            ])
            .arg(path)
            .status()
            .expect("create video")
            .success()
    );
}
fn white_pixels(tools: &Tools, path: &Path) -> usize {
    let frame = Command::new(&tools.ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-ss", "1", "-i"])
        .arg(path)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "crop=640:160:0:320,format=gray",
            "-f",
            "rawvideo",
            "pipe:1",
        ])
        .output()
        .expect("decode frame");
    assert!(frame.status.success());
    frame
        .stdout
        .into_iter()
        .filter(|pixel| *pixel > 200)
        .count()
}

#[tokio::test]
#[ignore = "Requires bundled FFmpeg/ffprobe paths"]
async fn preview_and_batch_burn_srt_ass_and_use_episode_order() {
    let tools = tools();
    let root = directory().await;
    let input = root.join("input");
    let subtitles = root.join("subs [日本], O'Brien");
    let batch_output = root.join("output [x], O'Brien");
    let preview_output = root.join("preview [x], O'Brien");
    fs::create_dir_all(&input).await.expect("input");
    fs::create_dir_all(&subtitles).await.expect("subs");
    let first = input.join("[Group] Keroro - 02 [1080p].mp4");
    make_video(
        &tools,
        &first,
        "color=c=black:size=640x360:rate=24:duration=12",
    );
    fs::copy(&first, input.join("[Group] Keroro - 10 [1080p].mp4"))
        .await
        .expect("second episode");
    fs::write(
        subtitles.join("Translated E02.srt"),
        "1\n00:00:00,000 --> 00:00:09,500\nKERORO SUBTITLE TEST\n",
    )
    .await
    .expect("SRT");
    fs::write(subtitles.join("Translated E10.ass"), "[Script Info]\nScriptType: v4.00+\nPlayResX: 640\nPlayResY: 480\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,36,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,20,20,30,1\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:00.00,0:00:10.00,Default,,0,0,0,,ASS SUBTITLE TEST\n").await.expect("ASS");
    let options = settings(&input, &batch_output, Some(&subtitles));
    let preview =
        BatchPlan::prepare_preview("preview-test".into(), options.clone(), &preview_output)
            .await
            .expect("preview plan");
    let result = run_batch(preview, tools.clone(), Cancellation::new(), |_| {}).await;
    assert_eq!(result.phase, Phase::Succeeded, "{:#?}", result.errors);
    assert_eq!(result.total_files, 1);
    assert!(
        !batch_output.exists(),
        "Preview must not create batch outputs"
    );
    let clip = preview_output.join("preview_temp.mp4");
    let audio = Command::new(&tools.ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=codec_name,sample_rate,channels",
            "-of",
            "csv=p=0",
        ])
        .arg(&clip)
        .output()
        .expect("preview audio");
    assert!(audio.status.success());
    assert_eq!(String::from_utf8_lossy(&audio.stdout).trim(), "aac,44100,2");
    let probe = Command::new(&tools.ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(&clip)
        .output()
        .expect("duration");
    let duration = String::from_utf8_lossy(&probe.stdout)
        .trim()
        .parse::<f64>()
        .expect("duration seconds");
    assert!(
        (9.8..=10.15).contains(&duration),
        "Preview duration: {duration}"
    );
    assert!(
        white_pixels(&tools, &clip) > 50,
        "SRT must be visible in encoded pixels"
    );
    let plan = BatchPlan::prepare("subtitle-batch".into(), options)
        .await
        .expect("batch plan");
    let mut matched = Vec::new();
    let result = run_batch(plan, tools.clone(), Cancellation::new(), |event| {
        if let Some(name) = event.subtitle_name {
            matched.push(name);
        }
    })
    .await;
    assert_eq!(result.phase, Phase::Succeeded, "{:#?}", result.errors);
    assert_eq!(result.succeeded, 2);
    assert!(
        result.warnings.is_empty(),
        "Successful subtitle conversion generated warnings: {:?}",
        result.warnings
    );
    assert!(matched.iter().any(|name| name.ends_with("E02.srt")));
    assert!(matched.iter().any(|name| name.ends_with("E10.ass")));
    for name in ["Sgt Keroro - Ep01 [Wii].mp4", "Sgt Keroro - Ep02 [Wii].mp4"] {
        assert!(
            white_pixels(&tools, &batch_output.join(name)) > 50,
            "Burned subtitle missing from {name}"
        );
    }
    for folder in [&batch_output, &preview_output] {
        let mut entries = fs::read_dir(folder).await.expect("outputs");
        while let Some(entry) = entries.next_entry().await.expect("entry") {
            let name = entry.file_name().to_string_lossy().into_owned();
            assert!(
                !name.ends_with(".mp4.part") && !name.starts_with(".keroro-subtitles-"),
                "Staging file leaked: {name}"
            );
        }
    }
    // Exercise the actual checkpointed pipeline for a single video plus a
    // separately selected subtitle file, not just queue preparation.
    let single_output = root.join("single-output");
    let queue = BatchPlan::prepare(
        "single-with-checkpoint".into(),
        settings(
            &first,
            &single_output,
            Some(&subtitles.join("Translated E02.srt")),
        ),
    )
    .await
    .expect("single-file plan");
    let saved_queue = queue.clone();
    let store = keroro_encoder_core::session::SessionStore::open(&root.join("local-state"))
        .expect("central checkpoint store");
    store
        .begin(&queue)
        .expect("checkpoint single video and subtitle");
    let result = keroro_encoder_core::run_batch_with_session(
        queue,
        tools.clone(),
        Cancellation::new(),
        std::sync::Arc::new(keroro_encoder_core::ActiveProcess::default()),
        Some(store.clone()),
        |_| {},
    )
    .await;
    assert_eq!(result.phase, Phase::Succeeded, "{:?}", result.errors);
    assert_eq!(result.total_files, 1);
    assert!(white_pixels(&tools, &single_output.join("Sgt Keroro - Ep02 [Wii].mp4")) > 50);
    assert!(store.info().expect("completed state").is_none());
    let measured = store
        .last_measurements()
        .expect("persisted statistics")
        .expect("measurements");
    assert_eq!(measured.cpu_mode, "maximum");
    assert!(
        measured.media_seconds > 0.0 && measured.wall_seconds > 0.0 && measured.output_bytes > 0
    );
    let encoded_file = single_output.join("Sgt Keroro - Ep02 [Wii].mp4");
    let original = fs::read(&encoded_file).await.expect("original output");
    store.begin(&saved_queue).expect("repeat queue");
    let reuse = keroro_encoder_core::run_batch_with_session(
        saved_queue,
        tools.clone(),
        Cancellation::new(),
        std::sync::Arc::new(keroro_encoder_core::ActiveProcess::default()),
        Some(store.clone()),
        |_| {},
    )
    .await;
    assert_eq!((reuse.succeeded, reuse.skipped, reuse.failed), (0, 1, 0));
    assert!(
        reuse.warnings.is_empty(),
        "Verified output should skip without legacy warning"
    );
    for change_subtitle in [false, true] {
        let mut changed = settings(
            &first,
            &single_output,
            Some(&subtitles.join("Translated E02.srt")),
        );
        if change_subtitle {
            fs::write(
                subtitles.join("Translated E02.srt"),
                "1\n00:00:00,000 --> 00:00:09,500\nCHANGED SUBTITLE CONTENT\n",
            )
            .await
            .expect("change subtitles");
        } else {
            changed.crf = 26;
        }
        let queue = BatchPlan::prepare("changed-request".into(), changed)
            .await
            .expect("changed queue");
        store.begin(&queue).expect("checkpoint changed queue");
        let blocked = keroro_encoder_core::run_batch_with_session(
            queue,
            tools.clone(),
            Cancellation::new(),
            std::sync::Arc::new(keroro_encoder_core::ActiveProcess::default()),
            Some(store.clone()),
            |_| {},
        )
        .await;
        assert_eq!(
            (blocked.succeeded, blocked.skipped, blocked.failed),
            (0, 0, 1)
        );
        assert!(blocked.errors[0].reason.contains("preserved"));
        assert_eq!(
            fs::read(&encoded_file).await.expect("preserved output"),
            original
        );
    }
    let rejected_output = root.join("verification-rejection");
    let queue = BatchPlan::prepare(
        "bad-verifier".into(),
        settings(&first, &rejected_output, None),
    )
    .await
    .expect("queue");
    let mut bad_tools = tools.clone();
    bad_tools.ffprobe = tools.ffmpeg.clone();
    let rejected = run_batch(queue, bad_tools, Cancellation::new(), |_| {}).await;
    assert_eq!(rejected.phase, Phase::Failed);
    assert!(!rejected_output.join("Sgt Keroro - Ep02 [Wii].mp4").exists());
    assert!(
        !rejected_output
            .join("Sgt Keroro - Ep02 [Wii].mp4.part")
            .exists()
    );
    // Optional subtitles can disappear or be replaced after queue preparation.
    let subtitle_path = subtitles.join("Translated E02.srt");
    let fallback_output = root.join("subtitle-fallback");
    let queue = BatchPlan::prepare(
        "subtitle-replaced".into(),
        settings(&first, &fallback_output, Some(&subtitle_path)),
    )
    .await
    .expect("fallback queue");
    fs::remove_file(&subtitle_path)
        .await
        .expect("remove subtitle");
    fs::create_dir(&subtitle_path)
        .await
        .expect("replace subtitle with directory");
    store.begin(&queue).expect("checkpoint fallback");
    let fallback = keroro_encoder_core::run_batch_with_session(
        queue,
        tools.clone(),
        Cancellation::new(),
        std::sync::Arc::new(keroro_encoder_core::ActiveProcess::default()),
        Some(store.clone()),
        |_| {},
    )
    .await;
    assert_eq!(fallback.phase, Phase::Succeeded, "{:?}", fallback.errors);
    assert_eq!(fallback.succeeded, 1);
    assert!(
        !fallback.warnings.is_empty(),
        "Video-only fallback must be visible"
    );
    assert!(
        !fallback
            .measurements
            .expect("fallback measurements")
            .subtitle_applied
    );
    assert!(
        fallback_output
            .join("Sgt Keroro - Ep02 [Wii].mp4")
            .is_file()
    );
    drop(store);
    fs::remove_dir_all(root).await.expect("clean fixtures");
}

#[tokio::test]
#[ignore = "Requires bundled FFmpeg/ffprobe paths"]
async fn cancelling_a_live_encoder_reaps_the_child_and_removes_partial_output() {
    let tools = tools();
    let root = directory().await;
    let input = root.join("input");
    let output = root.join("output");
    fs::create_dir_all(&input).await.expect("input");
    make_video(
        &tools,
        &input.join("Episode01.mp4"),
        "testsrc2=size=1280x720:rate=30:duration=12",
    );
    let plan = BatchPlan::prepare("cancel-test".into(), settings(&input, &output, None))
        .await
        .expect("plan");
    let cancellation = Cancellation::new();
    let trigger = cancellation.clone();
    let mut saw_live_progress = false;
    let started = Instant::now();
    let result = run_batch(plan, tools, cancellation, |event| {
        if event.file_percent.is_some() && event.phase == Phase::Running {
            saw_live_progress = true;
            trigger.cancel();
        }
    })
    .await;
    assert!(
        saw_live_progress,
        "Cancellation must happen after FFmpeg has started"
    );
    assert_eq!(result.phase, Phase::Cancelled);
    assert!(
        started.elapsed() < Duration::from_secs(6),
        "Cancellation did not stop promptly"
    );
    assert_eq!(result.succeeded, 0);
    assert!(
        fs::read_dir(&output)
            .await
            .expect("output folder")
            .next_entry()
            .await
            .expect("entry")
            .is_none(),
        "Partial output or live process handle was left behind"
    );
    fs::remove_dir_all(root)
        .await
        .expect("clean cancelled fixture");
}

#[tokio::test]
async fn output_preflight_writes_cleans_and_rejects_non_directories() {
    let root = directory().await;
    let warning = keroro_encoder_core::preflight::output(&root)
        .await
        .expect("write and space preflight");
    #[cfg(windows)]
    assert!(
        warning.is_none(),
        "Live disk-space query failed: {warning:?}"
    );
    assert!(
        fs::read_dir(&root)
            .await
            .expect("folder")
            .next_entry()
            .await
            .expect("entry")
            .is_none()
    );
    let file = root.join("not-a-folder");
    fs::write(&file, "preserve this").await.expect("file");
    assert!(keroro_encoder_core::preflight::output(&file).await.is_err());
    assert_eq!(
        fs::read_to_string(&file).await.expect("original"),
        "preserve this"
    );
    fs::remove_dir_all(root).await.expect("cleanup");
}

#[tokio::test]
#[ignore = "Requires bundled FFmpeg/ffprobe paths"]
async fn ten_unicode_episodes_skip_corruption_and_survive_legacy_missing_and_binary_subtitles() {
    let tools = tools();
    let root = directory().await;
    let input = root.join("入力 [anime], O'Brien & $()");
    let output = root.join("出力 [Wii], O'Brien & $()");
    let subtitles = root.join("字幕 [subs], O'Brien & $()");
    fs::create_dir(&input).await.expect("input");
    fs::create_dir(&subtitles).await.expect("subtitles");
    let source = input.join("ケロロ軍曹 - 01.mkv");
    assert!(
        Command::new(&tools.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:size=640x360:rate=24:duration=1.3",
                "-an",
                "-c:v",
                "libx264",
                "-threads",
                "1"
            ])
            .arg(&source)
            .status()
            .expect("fixture")
            .success()
    );
    for episode in 2..=10 {
        let video = input.join(format!("ケロロ軍曹 - {episode:02}.mkv"));
        if episode == 4 {
            fs::write(video, b"corrupt MKV header")
                .await
                .expect("corrupt input");
        } else {
            fs::copy(&source, video).await.expect("copy fixture");
        }
    }
    let cue = |text: &str| format!("1\n00:00:00,000 --> 00:00:01,250\n{text}\n");
    let mut original_subtitles = Vec::new();
    for episode in [1, 2, 3, 6, 7, 8, 9, 10] {
        let bytes = match episode {
            2 => {
                let mut bytes = cue("SHIFT JIS ").trim_end().as_bytes().to_vec();
                bytes.extend_from_slice(&[0x83, 0x50, 0x83, 0x8d, 0x83, 0x8d, b'\n']);
                bytes
            }
            3 => cue("CAFE caf")
                .trim_end()
                .as_bytes()
                .iter()
                .copied()
                .chain([0xe9, b'\n'])
                .collect(),
            6 => b"binary\0subtitle".to_vec(),
            8 => [0xff, 0xfe]
                .into_iter()
                .chain(
                    cue("UTF16 SUBTITLE")
                        .encode_utf16()
                        .flat_map(u16::to_le_bytes),
                )
                .collect(),
            _ => cue("UNICODE SUBTITLE ケロロ").into_bytes(),
        };
        let path = subtitles.join(format!("ケロロ軍曹 - {episode:02}.srt"));
        fs::write(&path, &bytes).await.expect("subtitle");
        original_subtitles.push((path, bytes));
    }
    let mut options = settings(&input, &output, Some(&subtitles));
    options.anime_title = "ケロロ軍曹".into();
    let plan = BatchPlan::prepare("wave2".into(), options)
        .await
        .expect("preflight");
    fs::remove_file(subtitles.join("ケロロ軍曹 - 07.srt"))
        .await
        .expect("subtitle disappeared after scan");
    let mut saw_skip = false;
    let mut saw_no_subtitle_after_delete = false;
    let result = run_batch(plan, tools.clone(), Cancellation::new(), |event| {
        saw_skip |= event.message.contains("[Error]")
            && event.message.contains("04.mkv")
            && event.message.contains("Skipping");
        saw_no_subtitle_after_delete |= event.current_file == 7 && event.subtitle_name.is_none();
    })
    .await;
    assert_eq!(
        result.phase,
        Phase::CompletedWithErrors,
        "{:?}",
        result.errors
    );
    assert_eq!(
        (result.succeeded, result.failed, result.total_files),
        (9, 1, 10)
    );
    assert!(saw_skip && saw_no_subtitle_after_delete);
    for encoding in ["Shift-JIS", "Windows-1252", "UTF-16"] {
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.reason.contains(encoding)),
            "Missing {encoding} warning"
        );
    }
    for episode in [5, 6, 7] {
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.file_name.contains(&format!("{episode:02}.mkv")))
        );
        assert!(
            white_pixels(
                &tools,
                &output.join(format!("ケロロ軍曹 - Ep{episode:02} [Wii].mp4"))
            ) < 5
        );
    }
    for episode in [1, 2, 3, 8, 9, 10] {
        assert!(
            white_pixels(
                &tools,
                &output.join(format!("ケロロ軍曹 - Ep{episode:02} [Wii].mp4"))
            ) > 50,
            "Subtitle not burned for episode {episode}"
        );
    }
    assert!(!output.join("ケロロ軍曹 - Ep04 [Wii].mp4").exists());
    for (path, bytes) in original_subtitles {
        if path
            .file_stem()
            .is_some_and(|name| name.to_string_lossy().ends_with("07"))
        {
            continue;
        }
        assert_eq!(
            fs::read(path).await.expect("preserved source subtitle"),
            bytes
        );
    }
    let mut entries = fs::read_dir(&output).await.expect("outputs");
    while let Some(entry) = entries.next_entry().await.expect("entry") {
        assert!(
            !entry.file_name().to_string_lossy().starts_with('.'),
            "Private file leaked"
        );
    }
    fs::remove_dir_all(root).await.expect("cleanup");
}

#[tokio::test]
#[ignore = "Requires bundled FFmpeg/ffprobe paths"]
async fn ssa_vtt_syntax_errors_and_oversized_subtitles_are_isolated() {
    let tools = tools();
    let root = directory().await;
    let input = root.join("anime");
    let output = root.join("output");
    let subs = root.join("subs");
    for path in [&input, &subs] {
        fs::create_dir(path).await.expect("directory");
    }
    let source = input.join("Episode01.mkv");
    assert!(
        Command::new(&tools.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:size=640x360:rate=24:duration=1.3",
                "-an",
                "-c:v",
                "libx264",
                "-threads",
                "1"
            ])
            .arg(&source)
            .status()
            .expect("fixture")
            .success()
    );
    for episode in 2..=6 {
        fs::copy(&source, input.join(format!("Episode{episode:02}.mkv")))
            .await
            .expect("copy");
    }
    let ssa = "[Script Info]\nScriptType: v4.00\nPlayResX: 640\nPlayResY: 480\n[V4 Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, TertiaryColour, BackColour, Bold, Italic, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, AlphaLevel, Encoding\nStyle: Default,Arial,30,16777215,0,0,0,0,0,1,2,0,2,20,20,20,0,1\n[Events]\nFormat: Marked, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: Marked=0,0:00:00.00,0:00:01.25,Default,,0,0,0,,SSA TEST\n";
    fs::write(subs.join("Episode01.SSA"), ssa)
        .await
        .expect("SSA");
    fs::write(
        subs.join("Episode02.VTT"),
        "WEBVTT\n\n00:00:00.000 --> 00:00:01.250\nVTT TEST\n",
    )
    .await
    .expect("VTT");
    fs::write(
        subs.join("Episode03.vtt"),
        "WEBVTT\n\nBROKEN --> NOT_A_TIME\nBad timing\n",
    )
    .await
    .expect("broken VTT");
    let large = std::fs::File::create(subs.join("Episode05.ass")).expect("oversize");
    large
        .set_len(keroro_encoder_core::subtitles::MAX_SUBTITLE_BYTES + 1)
        .expect("size");
    drop(large);
    fs::write(
        subs.join("Episode06.srt"),
        "1\n00:00:00,000 --> 00:00:01,250\nAFTER ERROR\n",
    )
    .await
    .expect("SRT");
    // Resolve a non-canonical selected directory before scanning.
    let selected_subs = subs.join("..").join("subs");
    let plan = BatchPlan::prepare(
        "wave3".into(),
        settings(&input, &output, Some(&selected_subs)),
    )
    .await
    .expect("plan");
    let result = run_batch(plan, tools.clone(), Cancellation::new(), |_| {}).await;
    assert_eq!(
        result.phase,
        Phase::CompletedWithErrors,
        "{:?}",
        result.errors
    );
    assert_eq!(
        (result.succeeded, result.failed, result.total_files),
        (5, 1, 6)
    );
    assert!(
        result
            .errors
            .iter()
            .any(|error| error.file_name.contains("03") && error.reason.contains("syntax error"))
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.file_name.contains("05") && warning.reason.contains("50 MB"))
    );
    for episode in [1, 2, 6] {
        assert!(
            white_pixels(
                &tools,
                &output.join(format!("Sgt Keroro - Ep{episode:02} [Wii].mp4"))
            ) > 50
        );
    }
    for episode in [4, 5] {
        assert!(
            white_pixels(
                &tools,
                &output.join(format!("Sgt Keroro - Ep{episode:02} [Wii].mp4"))
            ) < 5
        );
    }
    assert!(!output.join("Sgt Keroro - Ep03 [Wii].mp4").exists());
    assert_eq!(
        fs::metadata(subs.join("Episode05.ass"))
            .await
            .expect("original")
            .len(),
        keroro_encoder_core::subtitles::MAX_SUBTITLE_BYTES + 1
    );
    fs::remove_dir_all(root).await.expect("cleanup");
}
