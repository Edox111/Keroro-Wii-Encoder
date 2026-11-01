//! Opt-in synthetic benchmark using the production encoder/filter/validation path.
use crate::*;
use std::io::Write;

#[tokio::test]
#[ignore = "Opt-in benchmark: npm run benchmark:encoder"]
async fn synthetic_benchmark_report() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("project root");
    let work = project.join(".work");
    std::fs::create_dir_all(&work).expect("work");
    artifacts::no_links(&work).expect("local work folder");
    let temp = tempfile::tempdir_in(&work).expect("private fixtures");
    let root = std::fs::canonicalize(temp.path()).expect("canonical fixtures");
    let tools = Tools {
        ffmpeg: std::env::var_os("KERORO_TEST_FFMPEG")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                project.join("src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe")
            }),
        ffprobe: std::env::var_os("KERORO_TEST_FFPROBE")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                project.join("src-tauri/binaries/ffprobe-x86_64-pc-windows-msvc.exe")
            }),
    };
    let input = root.join("source.mp4");
    let status = process(&tools.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=1280x720:rate=24:duration=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100:duration=30",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-threads",
            "2",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-t",
            "30",
        ])
        .arg(&input)
        .status()
        .await
        .expect("source fixture");
    assert!(status.success());
    let srt = root.join("plain.srt");
    std::fs::write(
        &srt,
        "1\n00:00:00,000 --> 00:00:29,500\nSynthetic subtitle performance sample\n",
    )
    .expect("SRT");
    let ass = root.join("styled.ass");
    let mut script = String::from(
        "[Script Info]\nScriptType: v4.00+\nPlayResX: 640\nPlayResY: 480\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,26,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,1,2,20,20,24,1\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
    );
    for line in 0..6 {
        script.push_str(&format!("Dialogue: {line},0:00:00.00,0:00:29.50,Default,,0,0,0,,{{\\an7\\pos(20,{})\\bord3\\blur1\\t(0,29000,\\frz8)}}Styled synthetic subtitle line {line}\n",45+line*48));
    }
    std::fs::write(&ass, script).expect("ASS");
    let cancel = Cancellation::new();
    let processes = Arc::new(ActiveProcess::default());
    let gpu = hardware::detect(&tools, &cancel, &processes).await;
    let encoders = if gpu == hardware::VideoEncoder::Cpu {
        vec![gpu]
    } else {
        vec![gpu, hardware::VideoEncoder::Cpu]
    };
    let mut report = String::from(
        "# Encoding benchmark\n\nSynthetic 30-second 1280x720/24 fps test pattern plus sine audio, encoded through the production 640x480 pipeline. Rust development/test harness; desktop UI and batch checkpoint hashing overhead are excluded. One sample per case; this is not a real-anime quality assessment or Wii playback test. Wall time includes probing and output verification. Variations from driver warm-up, load, and storage are expected.\n\n| Encoder | Subtitles | CPU mode | Wall s | Rate | User CPU s | Peak MiB | Output MiB |\n| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |\n",
    );
    for encoder in encoders {
        for mode in [CpuMode::Maximum, CpuMode::Balanced] {
            for (name, subtitle) in [
                ("None", None),
                ("SRT", Some(&srt)),
                ("Styled ASS", Some(&ass)),
            ] {
                eprintln!("Benchmark: {} / {:?} / {name}", encoder.name(), mode);
                let output = root.join(format!(
                    "{}-{:?}-{}.mp4",
                    encoder.name(),
                    mode,
                    name.replace(' ', "-")
                ));
                let settings = Settings {
                    input_dir: input.to_string_lossy().into_owned(),
                    output_dir: root.to_string_lossy().into_owned(),
                    subtitle_dir: subtitle.map(|s| s.to_string_lossy().into_owned()),
                    anime_title: String::new(),
                    crf: 23,
                    cpu_mode: mode,
                };
                let item = Item {
                    input: input.clone(),
                    output,
                    subtitle: subtitle.cloned(),
                    subtitle_root: subtitle.map(|_| root.clone()),
                    validation_error: None,
                };
                let mut measured = None;
                encode_item(
                    &tools,
                    &settings,
                    &item,
                    TaskKind::Batch,
                    encoder,
                    &cancel,
                    &processes,
                    None,
                    |event| {
                        if let ItemUpdate::Measurements(m) = event {
                            measured = Some(m)
                        }
                    },
                )
                .await
                .expect("benchmark encode");
                let m = measured.expect("measurements");
                let optional =
                    |v: Option<f64>| v.map_or_else(|| "N/A".into(), |v| format!("{v:.2}"));
                report.push_str(&format!(
                    "| {} | {name} | {:?} | {:.3} | {:.2}x | {} | {} | {:.2} |\n",
                    m.encoder,
                    mode,
                    m.wall_seconds,
                    m.media_seconds / m.wall_seconds,
                    optional(m.user_cpu_seconds),
                    optional(m.peak_memory_kib.map(|v| v as f64 / 1024.0)),
                    m.output_bytes as f64 / 1048576.0
                ));
            }
        }
    }
    report.push_str("\nUse repeat runs and your own episodes before choosing a default. Maximum remains the existing default; Balanced reserves CPU thread capacity for desktop use. No speedup is guaranteed for every source.\n");
    let destination = project.join("docs/PERFORMANCE_BENCHMARK.md");
    artifacts::no_links(destination.parent().expect("docs")).expect("local report folder");
    if destination.exists() {
        artifacts::no_links(&destination).expect("regular report");
    }
    let mut file =
        tempfile::NamedTempFile::new_in(destination.parent().expect("docs")).expect("report temp");
    file.write_all(report.as_bytes()).expect("report");
    file.as_file().sync_all().expect("flush");
    atomic_file::rename(file.path(), &destination, true).expect("publish report");
    eprintln!("Report: {}", destination.display());
}
