use keroro_encoder_core::{
    ActiveProcess, BatchPlan, Cancellation, CpuMode, Phase, Settings, Tools, VideoSettings, audio,
    run_batch, video::EncoderPreference,
};
use std::sync::Arc;

#[tokio::test]
#[ignore = "Requires bundled FFmpeg/ffprobe paths"]
async fn dual_audio_probe_preference_manual_mapping_and_checkpoint_settings() {
    let tools = Tools {
        ffmpeg: std::env::var_os("KERORO_TEST_FFMPEG")
            .expect("ffmpeg")
            .into(),
        ffprobe: std::env::var_os("KERORO_TEST_FFPROBE")
            .expect("ffprobe")
            .into(),
    };
    let root = tempfile::tempdir().expect("fixture");
    let input = root.path().join("dual audio Ep03.mkv");
    let fixture = tokio::process::Command::new(&tools.ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x240:rate=24:duration=0.75",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=0.75",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:sample_rate=48000:duration=0.75",
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-map",
            "2:a:0",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-threads",
            "1",
            "-c:a",
            "pcm_s16le",
            "-metadata:s:a:0",
            "language=jpn",
            "-metadata:s:a:0",
            "title=Japanese Original",
            "-metadata:s:a:1",
            "language=eng",
            "-metadata:s:a:1",
            "title=English Dub",
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
    let silent = root.path().join("silent Ep03.mkv");
    let silent_fixture = tokio::process::Command::new(&tools.ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(&input)
        .args(["-map", "0:v:0", "-c:v", "copy", "-an"])
        .arg(&silent)
        .output()
        .await
        .expect("silent fixture");
    assert!(silent_fixture.status.success());
    let processes = Arc::new(ActiveProcess::default());
    let streams = audio::probe_streams(&tools, &input, &Cancellation::new(), &processes)
        .await
        .expect("probe");
    assert_eq!(streams.len(), 2);
    assert_eq!((streams[0].index, streams[0].stream_index), (0, 1));
    assert_eq!((streams[1].index, streams[1].stream_index), (1, 2));
    assert_eq!(streams[0].language, "jpn");
    assert_eq!(streams[1].title.as_deref(), Some("English Dub"));
    for (case, selected, preferred, english) in [
        ("automatic", None, "en", true),
        ("manual-japanese", Some(0), "eng", false),
        ("manual-english", Some(1), "jpn", true),
        ("fallback", None, "deu", false),
        ("no-audio", None, "eng", false),
        ("invalid-selection", Some(99), "eng", false),
    ] {
        let output = root.path().join(case);
        let settings = Settings {
            target_volume_id: None,
            input_dir: if case == "no-audio" { &silent } else { &input }
                .to_string_lossy()
                .into_owned(),
            output_dir: output.to_string_lossy().into_owned(),
            subtitle_dir: None,
            anime_title: "Audio".into(),
            crf: 23,
            cpu_mode: CpuMode::Maximum,
            selected_audio_index: selected,
            selected_subtitle_index: None,
            preferred_sub_lang: None,
            preferred_audio_lang: Some(preferred.into()),
            video: VideoSettings {
                encoder_override: EncoderPreference::Cpu,
                ..Default::default()
            },
        };
        // The same serde representation is persisted in native pause/resume checkpoints.
        let restored: Settings =
            serde_json::from_value(serde_json::to_value(&settings).expect("serialize"))
                .expect("restore");
        assert_eq!(restored.selected_audio_index, selected);
        assert_eq!(restored.preferred_audio_lang.as_deref(), Some(preferred));
        let queue = BatchPlan::prepare(case.into(), restored)
            .await
            .expect("queue");
        let result = run_batch(queue, tools.clone(), Cancellation::new(), |_| {}).await;
        if case == "invalid-selection" {
            assert_eq!(result.phase, Phase::Failed);
            assert!(!output.join("Audio - Ep03 [Wii].mp4").exists());
            assert!(!output.join("Audio - Ep03 [Wii].mp4.part").exists());
            continue;
        }
        assert_eq!(
            result.phase,
            Phase::Succeeded,
            "{case}: {:?}",
            result.errors
        );
        let file = output.join("Audio - Ep03 [Wii].mp4");
        let encoded = audio::probe_streams(&tools, &file, &Cancellation::new(), &processes)
            .await
            .expect("encoded probe");
        if case == "no-audio" {
            assert!(encoded.is_empty());
            continue;
        }
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0].codec_name, "aac");
        assert_eq!(encoded[0].channels, 2);
        assert_eq!(encoded[0].language, if english { "eng" } else { "jpn" });
        // Verify actual sound, not just tags: Japanese is 440Hz, English is 880Hz.
        let pcm = tokio::process::Command::new(&tools.ffmpeg)
            .args(["-v", "error", "-i"])
            .arg(&file)
            .args([
                "-map", "0:a:0", "-ac", "1", "-ar", "48000", "-f", "s16le", "pipe:1",
            ])
            .output()
            .await
            .expect("decode audio");
        assert!(pcm.status.success());
        let samples: Vec<i16> = pcm
            .stdout
            .as_chunks::<2>()
            .0
            .iter()
            .map(|v| i16::from_le_bytes([v[0], v[1]]))
            .collect();
        let crossings = samples
            .windows(2)
            .filter(|p| (p[0] < 0) != (p[1] < 0))
            .count();
        let rate = crossings as f64 / samples.len() as f64;
        assert!(
            (rate
                - if english {
                    1760.0 / 48000.0
                } else {
                    880.0 / 48000.0
                })
            .abs()
                < 0.004,
            "{case}: wrong audio tone {rate}"
        );
    }
}
