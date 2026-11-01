//! Bounded post-encode metadata validation before a partial becomes a final file.
use crate::{ActiveProcess, Cancellation, Tools, hardware, process};
use std::{path::Path, sync::Arc};

#[derive(Debug)]
pub(crate) struct Validated {
    pub duration: f64,
    pub bytes: u64,
}
fn number(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.parse().ok())
        .filter(|v| v.is_finite())
}
fn rate(value: &serde_json::Value) -> Option<f64> {
    let (a, b) = value.as_str()?.split_once('/')?;
    let rate = a.parse::<f64>().ok()? / b.parse::<f64>().ok()?;
    (rate.is_finite() && rate > 0.0).then_some(rate)
}
fn check(
    document: &serde_json::Value,
    expected: Option<f64>,
    bytes: u64,
) -> Result<Validated, String> {
    let streams = document["streams"]
        .as_array()
        .ok_or("Output has no stream metadata.")?;
    let video = streams
        .iter()
        .find(|s| s["codec_type"] == "video")
        .ok_or("Output has no video stream.")?;
    if video["codec_name"] != "h264"
        || video["width"] != 640
        || video["height"] != 480
        || video["pix_fmt"] != "yuv420p"
        || !matches!(
            video["profile"].as_str(),
            Some("Constrained Baseline" | "Baseline" | "Main")
        )
        || !number(&video["level"]).is_some_and(|v| v > 0.0 && v <= 30.0)
        || !rate(&video["avg_frame_rate"]).is_some_and(|v| v <= 30.01)
    {
        return Err("Output video does not meet the Wii SD encoding profile.".into());
    }
    for audio in streams.iter().filter(|s| s["codec_type"] == "audio") {
        if audio["codec_name"] != "aac"
            || audio["profile"] != "LC"
            || audio["channels"] != 2
            || number(&audio["sample_rate"]) != Some(44100.0)
        {
            return Err("Output audio does not meet the AAC-LC stereo profile.".into());
        }
    }
    if !document["format"]["format_name"]
        .as_str()
        .is_some_and(|v| v.split(',').any(|v| v == "mp4"))
    {
        return Err("Output is not an MP4 container.".into());
    }
    let duration = number(&document["format"]["duration"])
        .filter(|d| *d > 0.0)
        .ok_or("Output has no positive duration.")?;
    if let Some(expected) = expected.filter(|d| d.is_finite() && *d > 0.0) {
        if (duration - expected).abs() > (expected * 0.1).max(2.0) {
            return Err(format!(
                "Output duration {duration:.2}s differs substantially from expected {expected:.2}s; partial output was rejected."
            ));
        }
    }
    if bytes == 0 {
        return Err("Output is empty.".into());
    }
    Ok(Validated { duration, bytes })
}
pub(crate) async fn validate(
    tools: &Tools,
    output: &Path,
    expected: Option<f64>,
    has_audio: Option<bool>,
    cancel: &Cancellation,
    processes: &Arc<ActiveProcess>,
) -> Result<Validated, String> {
    let mut command = process(&tools.ffprobe);
    command.args(["-v", "error", "-protocol_whitelist", "file,pipe", "-show_entries", "stream=codec_type,codec_name,profile,width,height,pix_fmt,level,avg_frame_rate,channels,sample_rate:format=format_name,duration", "-of", "json"]).arg(output);
    let text = hardware::probe(command, cancel, processes)
        .await
        .map_err(|e| format!("Output verification failed: {e}"))?;
    let document: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Cannot parse output metadata: {e}"))?;
    let bytes = tokio::fs::metadata(output)
        .await
        .map_err(|e| e.to_string())?
        .len();
    if has_audio == Some(true)
        && !document["streams"]
            .as_array()
            .is_some_and(|streams| streams.iter().any(|s| s["codec_type"] == "audio"))
    {
        return Err("Output verification failed: the source audio track is missing from the encoded output.".into());
    }
    check(&document, expected, bytes)
}
pub(crate) const FAT32_MAX_BYTES: u64 = u32::MAX as u64;
pub(crate) fn projected_fat32_warning(duration: Option<f64>) -> Option<String> {
    duration.filter(|d| d.is_finite() && *d > 0.0)
        .filter(|d| d * (2_000_000.0 + 160_000.0) / 8.0 * 1.02 > FAT32_MAX_BYTES as f64)
        .map(|_| "This long encode may exceed FAT32's single-file limit at the selected bitrate cap. Consider shorter source segments if the Wii drive uses FAT32; the actual output size will also be checked.".into())
}
pub(crate) fn fat32_warning(bytes: u64) -> Option<String> {
    (bytes > FAT32_MAX_BYTES).then(|| "Output exceeds FAT32's 4 GiB minus 1 byte file limit. Split or shorten it before copying to a FAT32 Wii drive.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn metadata() -> serde_json::Value {
        serde_json::json!({"streams":[{"codec_type":"video","codec_name":"h264","width":640,"height":480,"pix_fmt":"yuv420p","profile":"Main","level":30,"avg_frame_rate":"24000/1001"},{"codec_type":"audio","codec_name":"aac","profile":"LC","channels":2,"sample_rate":"44100"}],"format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2","duration":"60"}})
    }
    #[test]
    fn rejects_wrong_format_corruption_and_truncated_duration() {
        let m = metadata();
        assert!(check(&m, Some(60.0), 100).is_ok());
        for key in ["pix_fmt", "codec_name", "profile"] {
            let mut m = metadata();
            m["streams"][0][key] = "invalid".into();
            assert!(check(&m, Some(60.0), 100).is_err());
        }
        assert!(check(&m, Some(120.0), 100).is_err());
        assert!(check(&m, Some(60.0), 0).is_err());
        let mut m = metadata();
        m["streams"][0]["avg_frame_rate"] = "60/1".into();
        assert!(check(&m, None, 100).is_err());
        assert!(fat32_warning(FAT32_MAX_BYTES).is_none());
        assert!(fat32_warning(FAT32_MAX_BYTES + 1).is_some());
    }
}
