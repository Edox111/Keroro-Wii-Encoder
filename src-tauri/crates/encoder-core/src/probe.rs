//! Typed stream metadata shared by IPC and the encoding pipeline.
use crate::{ActiveProcess, Cancellation, Tools, audio};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleStream {
    pub index: u32,
    pub stream_index: u32,
    pub codec_name: String,
    pub language: String,
    pub title: Option<String>,
    pub supported: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub duration_seconds: Option<f64>,
    pub audio_streams: Vec<audio::AudioStream>,
    pub subtitle_streams: Vec<SubtitleStream>,
}
pub fn subtitle_streams(document: &serde_json::Value) -> Result<Vec<SubtitleStream>, String> {
    let streams = document["streams"]
        .as_array()
        .ok_or("Probe returned no streams.")?;
    let mut result = Vec::new();
    for stream in streams
        .iter()
        .filter(|stream| stream["codec_type"] == "subtitle")
    {
        if result.len() >= 256 {
            return Err("Too many subtitle tracks (maximum 256).".into());
        }
        let index = result.len() as u32;
        let tag = |name: &str| {
            stream["tags"].as_object().and_then(|tags| {
                tags.iter()
                    .find(|(key, _)| key.eq_ignore_ascii_case(name))
                    .map(|(_, value)| value)
            })
        };
        let codec = audio::text(&stream["codec_name"], 64).unwrap_or_else(|| "unknown".into());
        result.push(SubtitleStream {
            index,
            stream_index: stream["index"]
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .unwrap_or(index),
            supported: matches!(
                codec.as_str(),
                "ass" | "ssa" | "subrip" | "webvtt" | "mov_text" | "text"
            ),
            codec_name: codec,
            language: audio::normalize_language(
                tag("language").and_then(|v| v.as_str()).unwrap_or("und"),
            ),
            title: tag("title").and_then(|v| audio::text(v, 256)),
        });
    }
    Ok(result)
}
pub async fn media_streams(
    tools: &Tools,
    path: &Path,
    cancel: &Cancellation,
    processes: &Arc<ActiveProcess>,
) -> Result<MediaInfo, String> {
    let document = audio::probe_document(tools, path, cancel, processes).await?;
    Ok(MediaInfo {
        duration_seconds: duration(&document),
        audio_streams: audio::parse_streams(&document)?,
        subtitle_streams: subtitle_streams(&document)?,
    })
}
pub fn select_subtitle(
    streams: Option<&[SubtitleStream]>,
    selected: Option<u32>,
    preferred: Option<&str>,
    external: bool,
) -> Result<Option<u32>, String> {
    if let Some(index) = selected {
        if index >= 256 {
            return Err("Invalid embedded subtitle index.".into());
        }
        if let Some(streams) = streams {
            let stream = streams
                .iter()
                .find(|s| s.index == index)
                .ok_or("Selected subtitle track is unavailable.")?;
            if !stream.supported {
                return Err(format!(
                    "Embedded {} subtitles need a bitmap overlay pipeline; choose a text track or external subtitles.",
                    stream.codec_name
                ));
            }
        }
        return Ok(Some(index));
    }
    let Some(preferred) = preferred.filter(|p| *p != "none") else {
        return Ok(None);
    };
    if external {
        return Ok(None);
    }
    let Some(streams) = streams else {
        return Ok(None);
    };
    let language = audio::normalize_language(preferred);
    Ok(streams
        .iter()
        .find(|s| s.supported && s.language == language)
        .or_else(|| streams.iter().find(|s| s.supported))
        .map(|s| s.index))
}

pub fn duration(document: &serde_json::Value) -> Option<f64> {
    let value = &document["format"]["duration"];
    value
        .as_f64()
        .or_else(|| value.as_str()?.parse().ok())
        .filter(|seconds| seconds.is_finite() && *seconds > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_ordinals_missing_tags_bitmap_guards_and_selection() {
        let streams = subtitle_streams(&serde_json::json!({"streams":[
            {"index":0,"codec_type":"video"},
            {"index":3,"codec_type":"subtitle","codec_name":"hdmv_pgs_subtitle","tags":{"language":"eng"}},
            {"index":7,"codec_type":"subtitle","codec_name":"ass","tags":{"LANGUAGE":"ja","TITLE":"Original"}},
            {"index":9,"codec_type":"subtitle","codec_name":"subrip","tags":{"language":"en"}}
        ]})).expect("parse");
        assert_eq!(streams[1].index, 1);
        assert_eq!(streams[1].stream_index, 7);
        assert_eq!(streams[1].language, "jpn");
        assert_eq!(
            select_subtitle(Some(&streams), None, Some("eng"), false).expect("English"),
            Some(2)
        );
        assert_eq!(
            select_subtitle(Some(&streams), None, Some("fra"), false).expect("fallback"),
            Some(1)
        );
        assert_eq!(
            select_subtitle(Some(&streams), None, Some("none"), false).expect("None"),
            None
        );
        assert_eq!(
            select_subtitle(Some(&streams), None, Some("eng"), true).expect("external"),
            None
        );
        assert_eq!(
            select_subtitle(Some(&streams), Some(1), Some("eng"), true).expect("manual"),
            Some(1)
        );
        assert!(select_subtitle(Some(&streams), Some(0), None, false).is_err());
        assert!(select_subtitle(Some(&streams), Some(3), None, false).is_err());
        assert_eq!(
            select_subtitle(Some(&[]), None, Some("eng"), false).expect("no streams"),
            None
        );
        assert!(subtitle_streams(&serde_json::json!({})).is_err());
    }
}
