use keroro_encoder_core::{
    AspectRatio, BatchPlan, Cancellation, CpuMode, Phase, ResolutionPreset, Settings, Tools,
    VideoSettings, run_batch, video::EncoderPreference,
};

#[tokio::test]
#[ignore = "Requires bundled FFmpeg/ffprobe paths"]
async fn high10_surround_input_becomes_main31_8bit_stereo_48k() {
    let tools = Tools {
        ffmpeg: std::env::var_os("KERORO_TEST_FFMPEG")
            .expect("ffmpeg")
            .into(),
        ffprobe: std::env::var_os("KERORO_TEST_FFPROBE")
            .expect("ffprobe")
            .into(),
    };
    let root = tempfile::tempdir().expect("fixture");
    let input = root.path().join("ケロロ Ep03.mkv");
    let fixture = tokio::process::Command::new(&tools.ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=640x360:rate=24:duration=1,format=yuv420p10le",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=channel_layout=5.1:sample_rate=44100",
            "-t",
            "1",
            "-c:v",
            "libx264",
            "-profile:v",
            "high10",
            "-preset",
            "ultrafast",
            "-threads",
            "1",
            "-c:a",
            "pcm_s16le",
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
    for encoder in [EncoderPreference::Cpu, EncoderPreference::Auto] {
        let output = root.path().join(format!("{encoder:?}"));
        let settings = Settings {
            input_dir: input.to_string_lossy().into_owned(),
            output_dir: output.to_string_lossy().into_owned(),
            subtitle_dir: None,
            anime_title: "WiiMC".into(),
            crf: 23,
            cpu_mode: CpuMode::Maximum,
            video: VideoSettings {
                aspect_ratio: AspectRatio::Wide,
                resolution_preset: ResolutionPreset::Anamorphic,
                encoder_override: encoder,
                ..Default::default()
            },
        };
        let queue = BatchPlan::prepare(format!("wiimc-{encoder:?}"), settings)
            .await
            .expect("queue");
        let result = run_batch(queue, tools.clone(), Cancellation::new(), |_| {}).await;
        assert_eq!(result.phase, Phase::Succeeded, "{:?}", result.errors);
        let probe = tokio::process::Command::new(&tools.ffprobe)
            .args(["-v", "error", "-show_streams", "-of", "json"])
            .arg(output.join("WiiMC - Ep03 [Wii].mp4"))
            .output()
            .await
            .expect("probe");
        assert!(probe.status.success());
        let document: serde_json::Value = serde_json::from_slice(&probe.stdout).expect("JSON");
        let streams = document["streams"].as_array().expect("streams");
        let video = streams
            .iter()
            .find(|s| s["codec_type"] == "video")
            .expect("video");
        assert_eq!(video["profile"], "Main");
        assert_eq!(video["level"], 31);
        assert_eq!(video["pix_fmt"], "yuv420p");
        assert_eq!(video["width"], 720);
        assert_eq!(video["height"], 480);
        assert_eq!(video["sample_aspect_ratio"], "32:27");
        assert_eq!(video["display_aspect_ratio"], "16:9");
        let audio = streams
            .iter()
            .find(|s| s["codec_type"] == "audio")
            .expect("audio");
        assert_eq!(audio["codec_name"], "aac");
        assert_eq!(audio["profile"], "LC");
        assert_eq!(audio["channels"], 2);
        assert_eq!(audio["sample_rate"], "48000");
    }
}
