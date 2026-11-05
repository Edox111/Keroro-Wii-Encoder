//! Audio-only ordinals are distinct from absolute stream indices.
use crate::{ActiveProcess, Cancellation, Tools, hardware, offline, process};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};

pub const MAX_AUDIO_STREAMS: u32 = 256;
pub const STREAM_ENTRIES: &str =
    "stream=index,codec_type,codec_name,channels,channel_layout:stream_tags=language,title";
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioStream {
    pub index: u32,
    pub stream_index: u32,
    pub codec_name: String,
    pub channels: u32,
    pub channel_layout: Option<String>,
    pub language: String,
    pub title: Option<String>,
}
pub(crate) fn text(value: &serde_json::Value, limit: usize) -> Option<String> {
    let text: String = value
        .as_str()?
        .chars()
        .filter(|c| !c.is_control())
        .take(limit)
        .collect();
    (!text.trim().is_empty()).then(|| text.trim().to_owned())
}
pub fn normalize_language(value: &str) -> String {
    let value = value
        .trim()
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match value.as_str() {
        "en" => "eng",
        "ja" => "jpn",
        "fr" | "fre" => "fra",
        "de" | "ger" => "deu",
        "es" => "spa",
        "it" => "ita",
        "pt" => "por",
        "ru" => "rus",
        "zh" | "chi" => "zho",
        "ko" => "kor",
        "sr" => "srp",
        "hr" => "hrv",
        "und" => "und",
        _ if (2..=3).contains(&value.len()) && value.bytes().all(|c| c.is_ascii_alphabetic()) => {
            return value;
        }
        _ => "und",
    }
    .to_owned()
}
pub fn parse_streams(document: &serde_json::Value) -> Result<Vec<AudioStream>, String> {
    let streams = document["streams"]
        .as_array()
        .ok_or("Probe returned no stream list.")?;
    let mut audio = Vec::new();
    for stream in streams
        .iter()
        .filter(|stream| stream["codec_type"] == "audio")
    {
        if audio.len() >= MAX_AUDIO_STREAMS as usize {
            return Err("Too many audio streams (maximum 256).".into());
        }
        let index = audio.len() as u32;
        let tag = |name: &str| {
            stream["tags"].as_object().and_then(|tags| {
                tags.iter()
                    .find(|(key, _)| key.eq_ignore_ascii_case(name))
                    .map(|(_, value)| value)
            })
        };
        audio.push(AudioStream {
            index,
            stream_index: stream["index"]
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .unwrap_or(index),
            codec_name: text(&stream["codec_name"], 64).unwrap_or_else(|| "unknown".into()),
            channels: stream["channels"]
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .unwrap_or(0),
            channel_layout: text(&stream["channel_layout"], 64),
            language: normalize_language(
                tag("language")
                    .and_then(|value| value.as_str())
                    .unwrap_or("und"),
            ),
            title: tag("title").and_then(|value| text(value, 256)),
        });
    }
    Ok(audio)
}
pub async fn probe_streams(
    tools: &Tools,
    path: &Path,
    cancel: &Cancellation,
    processes: &Arc<ActiveProcess>,
) -> Result<Vec<AudioStream>, String> {
    parse_streams(&probe_document(tools, path, cancel, processes).await?)
}
pub(crate) async fn probe_document(
    tools: &Tools,
    path: &Path,
    cancel: &Cancellation,
    processes: &Arc<ActiveProcess>,
) -> Result<serde_json::Value, String> {
    offline::local_path(path)?;
    let path = tokio::fs::canonicalize(path)
        .await
        .map_err(|e| format!("Cannot open media: {e}"))?;
    if !tokio::fs::metadata(&path)
        .await
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("Select a media file, not a directory.".into());
    }
    let mut command = process(&tools.ffprobe);
    command
        .args([
            "-v",
            "error",
            "-protocol_whitelist",
            "file,pipe",
            "-show_entries",
            &format!("format=duration:{STREAM_ENTRIES}"),
            "-of",
            "json",
        ])
        .arg(path);
    let output = hardware::probe(command, cancel, processes).await?;
    serde_json::from_str(&output).map_err(|e| format!("Invalid media probe JSON: {e}"))
}
pub fn select_index(
    streams: Option<&[AudioStream]>,
    selected: Option<u32>,
    preferred: Option<&str>,
) -> Result<Option<u32>, String> {
    if let Some(index) = selected {
        if index >= MAX_AUDIO_STREAMS
            || streams.is_some_and(|streams| !streams.iter().any(|stream| stream.index == index))
        {
            return Err(format!(
                "Selected audio track {} is unavailable in the input file.",
                index.saturating_add(1)
            ));
        }
        return Ok(Some(index));
    }
    let Some(preferred) = preferred else {
        return Ok(None);
    };
    let Some(streams) = streams else {
        return Ok(None);
    };
    let language = normalize_language(preferred);
    Ok(streams
        .iter()
        .find(|stream| stream.language == language)
        .or_else(|| streams.first())
        .map(|stream| stream.index))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinals_tags_preference_and_explicit_validation() {
        let streams = parse_streams(&serde_json::json!({"streams":[
            {"index":0,"codec_type":"video"},
            {"index":3,"codec_type":"audio","codec_name":"flac","channels":6,"tags":{"LANGUAGE":"ja","TITLE":"Japanese Original"}},
            {"index":7,"codec_type":"audio","codec_name":"aac","channels":2,"tags":{"language":"eng","title":"English Dub"}},
            {"index":8,"codec_type":"audio"}
        ]})).expect("streams");
        assert_eq!(streams[0].index, 0);
        assert_eq!(streams[0].stream_index, 3);
        assert_eq!(streams[0].language, "jpn");
        assert_eq!(streams[2].language, "und");
        assert_eq!(
            select_index(Some(&streams), None, Some("en")).expect("English"),
            Some(1)
        );
        assert_eq!(
            select_index(Some(&streams), None, Some("deu")).expect("fallback"),
            Some(0)
        );
        assert_eq!(
            select_index(Some(&streams), Some(0), Some("eng")).expect("manual"),
            Some(0)
        );
        assert!(select_index(Some(&streams), Some(3), None).is_err());
        assert_eq!(
            select_index(Some(&[]), None, Some("eng")).expect("no audio"),
            None
        );
        assert_eq!(select_index(None, None, None).expect("legacy"), None);
    }
}
