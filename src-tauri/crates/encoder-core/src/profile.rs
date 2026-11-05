//! Shared software-filter / hardware-encoder SD profile.
use crate::hardware::VideoEncoder;
use tokio::process::Command;

pub(crate) const VIDEO_FILTER: &str = "scale=640:480:force_original_aspect_ratio=decrease:force_divisible_by=2,pad=640:480:(ow-iw)/2:(oh-ih)/2,setsar=1,format=yuv420p";

pub(crate) fn configure(
    command: &mut Command,
    filter: &str,
    encoder: VideoEncoder,
    crf: u8,
    cpu_threads: &str,
    max_fps: u32,
) {
    configure_with_audio(command, filter, encoder, crf, cpu_threads, max_fps, None);
}

pub(crate) fn configure_with_audio(
    command: &mut Command,
    filter: &str,
    encoder: VideoEncoder,
    crf: u8,
    cpu_threads: &str,
    max_fps: u32,
    selected_audio_index: Option<u32>,
) {
    let audio_map = selected_audio_index.map(|index| format!("0:a:{index}"));
    command
        .args([
            "-map",
            "0:v:0",
            "-map",
            audio_map.as_deref().unwrap_or("0:a:0?"),
            "-sn",
            "-dn",
            "-vf",
        ])
        .arg(filter)
        .args(["-c:v", encoder.name()]);
    let quality = crf.to_string();
    let fps = max_fps.to_string();
    let bitrate = match crf {
        20 => "2000k",
        26 => "1000k",
        _ => "1500k",
    };
    match encoder {
        VideoEncoder::Cpu => {
            command.args([
                "-preset",
                "medium",
                "-threads",
                cpu_threads,
                "-crf",
                &quality,
            ]);
        }
        VideoEncoder::Nvenc => {
            command.args([
                "-preset", "p4", "-rc", "vbr", "-cq", &quality, "-b:v", bitrate,
            ]);
        }
        VideoEncoder::Amf => {
            command.args([
                "-quality",
                "quality",
                "-rc",
                "vbr_peak",
                "-enforce_hrd",
                "1",
                "-b:v",
                bitrate,
            ]);
        }
        VideoEncoder::Qsv => {
            command.args(["-preset", "medium", "-look_ahead", "0", "-b:v", bitrate]);
        }
    }
    // QSV uses integer level_idc; 31 is equivalent to Level 3.1.
    command
        .args([
            "-profile:v",
            "main",
            "-level:v",
            if encoder == VideoEncoder::Qsv {
                "31"
            } else {
                "3.1"
            },
            "-pix_fmt",
            "yuv420p",
            "-fpsmax",
            &fps,
            "-maxrate",
            "3200k",
            "-bufsize",
            "6000k",
            "-bf",
            "0",
        ])
        .args([
            "-c:a",
            "aac",
            "-profile:a",
            "aac_low",
            "-b:a",
            "160k",
            "-ac",
            "2",
            "-ar",
            "48000",
            "-movflags",
            "+faststart",
            "-f",
            "mp4",
        ]);
    // QSV/AMF may negotiate NV12 surfaces internally; H.264 remains 8-bit 4:2:0.
    // No GPU decoding/hwupload filter: libass consumes software frames safely.
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_audio_uses_relative_ordinals_for_every_encoder() {
        for encoder in [
            VideoEncoder::Cpu,
            VideoEncoder::Nvenc,
            VideoEncoder::Amf,
            VideoEncoder::Qsv,
        ] {
            let mut command = Command::new("not-executed-ffmpeg");
            configure_with_audio(&mut command, VIDEO_FILTER, encoder, 23, "0", 30, Some(1));
            let args: Vec<_> = command
                .as_std()
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect();
            let maps: Vec<_> = args
                .windows(2)
                .filter(|pair| pair[0] == "-map")
                .map(|pair| pair[1].as_str())
                .collect();
            assert_eq!(maps, ["0:v:0", "0:a:1"]);
            assert!(!args.iter().any(|arg| arg == "0:a:0?"));
        }
    }
    #[test]
    fn every_encoder_uses_the_same_wiimc_output_contract() {
        for encoder in [
            VideoEncoder::Cpu,
            VideoEncoder::Nvenc,
            VideoEncoder::Amf,
            VideoEncoder::Qsv,
        ] {
            let mut command = Command::new("not-executed-ffmpeg");
            configure(&mut command, VIDEO_FILTER, encoder, 23, "0", 30);
            let args = command
                .as_std()
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            for (flag, expected) in [
                ("-c:v", encoder.name()),
                ("-profile:v", "main"),
                (
                    "-level:v",
                    if encoder == VideoEncoder::Qsv {
                        "31"
                    } else {
                        "3.1"
                    },
                ),
                ("-pix_fmt", "yuv420p"),
                ("-maxrate", "3200k"),
                ("-bufsize", "6000k"),
                ("-bf", "0"),
                ("-c:a", "aac"),
                ("-b:a", "160k"),
                ("-ac", "2"),
                ("-ar", "48000"),
                ("-movflags", "+faststart"),
                ("-f", "mp4"),
            ] {
                let matches = args
                    .windows(2)
                    .filter(|pair| pair[0] == flag)
                    .collect::<Vec<_>>();
                assert_eq!(matches.len(), 1, "{encoder:?}: {flag}");
                assert_eq!(matches[0][1], expected, "{encoder:?}: {flag}");
            }
        }
    }
}
