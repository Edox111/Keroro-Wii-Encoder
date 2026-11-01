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
) {
    command
        .args(["-map", "0:v:0", "-map", "0:a:0?", "-sn", "-dn", "-vf"])
        .arg(filter)
        .args(["-c:v", encoder.name()]);
    let quality = crf.to_string();
    let bitrate = match crf {
        20 => "2000k",
        26 => "1000k",
        _ => "1500k",
    };
    match encoder {
        VideoEncoder::Cpu => {
            command.args([
                "-preset",
                "superfast",
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
            command.args(["-preset", "quality", "-rc", "vbr_peak", "-b:v", bitrate]);
        }
        VideoEncoder::Qsv => {
            command.args(["-preset", "fast", "-look_ahead", "0", "-b:v", bitrate]);
        }
    }
    // QSV's generic level option uses integer level_idc; the other encoders accept 3.0.
    command
        .args([
            "-profile:v",
            if encoder == VideoEncoder::Amf {
                "main"
            } else {
                "baseline"
            },
            "-level:v",
            if encoder == VideoEncoder::Qsv {
                "30"
            } else {
                "3.0"
            },
            "-pix_fmt",
            "yuv420p",
            "-fpsmax",
            "30",
            "-maxrate",
            "2000k",
            "-bufsize",
            "4000k",
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
            "44100",
            "-movflags",
            "+faststart",
            "-f",
            "mp4",
        ]);
    // QSV/AMF may negotiate NV12 surfaces internally; H.264 remains 8-bit 4:2:0.
    // No GPU decoding/hwupload filter: libass consumes software frames safely.
}
