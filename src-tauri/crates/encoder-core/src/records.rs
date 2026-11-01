//! Local output provenance and measurements; never overwrite an existing movie.
use crate::{Item, Settings, artifacts};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

const PIPELINE: &str = "wii-h264-sd-420-level30-aac160-v1";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Fingerprint {
    path: PathBuf,
    bytes: Option<u64>,
    modified_ns: Option<u128>,
    #[serde(default)]
    digest: Option<String>,
}
impl Fingerprint {
    fn read(path: &Path, missing_ok: bool) -> Result<Self, String> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => Some(metadata),
            Err(error) if missing_ok && error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("Cannot fingerprint {}: {error}", path.display())),
        };
        if metadata.is_some() {
            artifacts::no_links(path)?;
        } else {
            let mut ancestor = path.parent().ok_or("Missing fingerprint parent.")?;
            loop {
                match fs::symlink_metadata(ancestor) {
                    Ok(_) => {
                        artifacts::no_links(ancestor)?;
                        break;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        ancestor = ancestor.parent().ok_or("Missing fingerprint ancestor.")?
                    }
                    Err(error) => return Err(error.to_string()),
                }
            }
        }
        if metadata.as_ref().is_some_and(|m| !m.is_file()) {
            return Err("Fingerprint target must be a regular file.".into());
        }
        // Subtitles are small and have a strict size guard. Hash their bytes so
        // same-size edits on filesystems with coarse timestamps are detected.
        let digest = if missing_ok
            && metadata
                .as_ref()
                .is_some_and(|m| m.len() <= crate::subtitles::MAX_SUBTITLE_BYTES)
        {
            (|| -> std::io::Result<Option<String>> {
                use std::io::Read;
                let mut file = fs::File::open(path)?.take(crate::subtitles::MAX_SUBTITLE_BYTES + 1);
                let mut hash = Sha256::new();
                let mut buffer = [0u8; 65536];
                let mut total = 0u64;
                loop {
                    let count = file.read(&mut buffer)?;
                    if count == 0 {
                        break;
                    }
                    total += count as u64;
                    if total > crate::subtitles::MAX_SUBTITLE_BYTES {
                        return Ok(None);
                    }
                    hash.update(&buffer[..count]);
                }
                Ok(Some(format!("{:x}", hash.finalize())))
            })()
            .ok()
            .flatten()
        } else {
            None
        };
        Ok(Self {
            path: path.to_owned(),
            digest,
            bytes: metadata.as_ref().map(|m| m.len()),
            modified_ns: metadata
                .map(|m| {
                    m.modified()
                        .map_err(|e| e.to_string())?
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .map_err(|e| e.to_string())
                })
                .transpose()?,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Request {
    pipeline: String,
    quality: u8,
    video: Fingerprint,
    subtitle: Option<Fingerprint>,
}
impl Request {
    pub(crate) fn capture(item: &Item, settings: &Settings) -> Result<Self, String> {
        Ok(Self {
            pipeline: PIPELINE.into(),
            quality: settings.crf,
            video: Fingerprint::read(&item.input, false)?,
            subtitle: item
                .subtitle
                .as_deref()
                .map(|path| match Fingerprint::read(path, true) {
                    Ok(fingerprint) => fingerprint,
                    Err(error) => {
                        log::warn!("Optional subtitle fingerprint unavailable: {error}");
                        Fingerprint {
                            path: path.to_owned(),
                            bytes: None,
                            modified_ns: None,
                            digest: None,
                        }
                    }
                }),
        })
    }
    pub(crate) fn unchanged(&self, item: &Item, settings: &Settings) -> Result<(), String> {
        if *self != Self::capture(item, settings)? {
            return Err("Source video or subtitles changed during encoding; partial output was rejected. Retry with stable source files.".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Measurements {
    pub encoder: String,
    #[serde(default)]
    pub subtitle_applied: bool,
    #[serde(default)]
    pub cpu_mode: String,
    #[serde(default)]
    pub cpu_threads: usize,
    pub media_seconds: f64,
    pub wall_seconds: f64,
    pub final_fps: Option<f64>,
    pub user_cpu_seconds: Option<f64>,
    pub system_cpu_seconds: Option<f64>,
    pub peak_memory_kib: Option<u64>,
    pub output_bytes: u64,
}
impl Measurements {
    pub(crate) fn from_stderr(stderr: &str) -> Self {
        let mut result = Self::default();
        for line in stderr.lines().filter(|l| l.contains("bench:")) {
            for token in line.split_whitespace() {
                let Some((key, value)) = token.split_once('=') else {
                    continue;
                };
                let seconds = || {
                    value
                        .trim_end_matches('s')
                        .parse::<f64>()
                        .ok()
                        .filter(|v| v.is_finite() && *v >= 0.0)
                };
                match key {
                    "utime" => result.user_cpu_seconds = seconds(),
                    "stime" => result.system_cpu_seconds = seconds(),
                    "maxrss" => {
                        result.peak_memory_kib = value
                            .trim_end_matches("KiB")
                            .trim_end_matches("kB")
                            .parse()
                            .ok()
                            .filter(|v| *v > 0)
                    }
                    _ => {}
                }
            }
        }
        result
    }
}
#[derive(Serialize, Deserialize)]
struct Record {
    version: u8,
    request: Request,
    output: Fingerprint,
    measurements: Measurements,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Match {
    Verified,
    Unknown,
    Changed(String),
}
fn record_path(directory: &Path, output: &Path) -> Result<PathBuf, String> {
    let value = output
        .to_str()
        .ok_or("Output record path is not Unicode.")?;
    Ok(directory.join(format!("{:x}.json", Sha256::digest(value.as_bytes()))))
}
pub(crate) fn assess(directory: &Path, item: &Item, settings: &Settings) -> Result<Match, String> {
    let path = record_path(directory, &item.output)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Match::Unknown),
        Err(e) => return Err(e.to_string()),
    };
    artifacts::no_links(&path)?;
    if metadata.len() > 512 * 1024 {
        return Ok(Match::Changed(
            "Output provenance record exceeds its limit.".into(),
        ));
    }
    let bytes = {
        use std::io::Read;
        let mut bytes = Vec::new();
        fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(512 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        bytes
    };
    if bytes.len() > 512 * 1024 {
        return Ok(Match::Changed(
            "Output provenance record grew beyond its limit.".into(),
        ));
    }
    let record: Record = match serde_json::from_slice(&bytes) {
        Ok(record) => record,
        Err(_) => {
            return Ok(Match::Changed(
                "Output provenance record is invalid.".into(),
            ));
        }
    };
    if record.version != 1 || record.request != Request::capture(item, settings)? {
        return Ok(Match::Changed(
            "Source, subtitle, quality, or encoding profile differs from this existing output."
                .into(),
        ));
    }
    if record
        .request
        .subtitle
        .as_ref()
        .is_some_and(|s| s.digest.is_none())
    {
        return Ok(Match::Changed(
            "The selected subtitle contents could not be verified against the existing output."
                .into(),
        ));
    }
    if record.request.subtitle.is_some() && !record.measurements.subtitle_applied {
        return Ok(Match::Changed("The existing output was encoded without the selected external subtitle; it cannot be verified as a subtitle burn-in result.".into()));
    }
    if record.output != Fingerprint::read(&item.output, false)? {
        return Ok(Match::Changed(
            "Existing output changed since it was encoded.".into(),
        ));
    }
    Ok(Match::Verified)
}
pub(crate) fn latest(directory: &Path) -> Result<Option<Measurements>, String> {
    let path = directory.join("latest-statistics.json");
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
        Ok(metadata) if metadata.len() > 512 * 1024 => {
            return Err("Performance report exceeds its limit.".into());
        }
        _ => {}
    }
    artifacts::no_links(&path)?;
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(&path)
        .map_err(|e| e.to_string())?
        .take(512 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 512 * 1024 {
        return Err("Performance report exceeds its limit.".into());
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| e.to_string())
}

pub(crate) fn save(
    directory: &Path,
    request: Request,
    output: &Path,
    measurements: Measurements,
) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    artifacts::no_links(directory)?;
    let path = record_path(directory, output)?;
    if path.exists() {
        artifacts::no_links(&path)?;
    }
    let record = Record {
        version: 1,
        request,
        output: Fingerprint::read(output, false)?,
        measurements,
    };
    crate::atomic_file::json(&path, &record)?;
    let latest = directory.join("latest-statistics.json");
    if latest.exists() {
        artifacts::no_links(&latest)?;
    }
    crate::atomic_file::json(&latest, &record.measurements)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn records_detect_quality_source_subtitle_and_output_changes_without_touching_media() {
        let root = tempfile::tempdir().expect("fixture");
        let directory = std::fs::canonicalize(root.path()).expect("canonical");
        let video = directory.join("Ep01.mkv");
        let subtitle = directory.join("Ep01.srt");
        let output = directory.join("Ep01.mp4");
        fs::write(&video, b"video").expect("video");
        fs::write(&subtitle, b"subtitles").expect("subtitle");
        fs::write(&output, b"finished output").expect("output");
        let item = Item {
            input: video.clone(),
            output: output.clone(),
            subtitle: Some(subtitle.clone()),
            subtitle_root: Some(directory.clone()),
            validation_error: None,
        };
        let settings = Settings {
            input_dir: video.to_string_lossy().into_owned(),
            output_dir: directory.to_string_lossy().into_owned(),
            subtitle_dir: Some(subtitle.to_string_lossy().into_owned()),
            anime_title: String::new(),
            crf: 23,
            cpu_mode: crate::CpuMode::Maximum,
        };
        let registry = directory.join("records");
        assert_eq!(
            assess(&registry, &item, &settings).expect("unknown"),
            Match::Unknown
        );
        let request = Request::capture(&item, &settings).expect("capture");
        save(
            &registry,
            request.clone(),
            &output,
            Measurements {
                subtitle_applied: true,
                ..Measurements::default()
            },
        )
        .expect("save");
        assert_eq!(
            assess(&registry, &item, &settings).expect("verified"),
            Match::Verified
        );
        let mut changed = settings.clone();
        changed.crf = 26;
        assert!(matches!(
            assess(&registry, &item, &changed).expect("quality"),
            Match::Changed(_)
        ));
        let stamp = fs::metadata(&subtitle)
            .expect("metadata")
            .modified()
            .expect("modified");
        fs::write(&subtitle, b"SUBTITLES").expect("same-size edit");
        fs::OpenOptions::new()
            .write(true)
            .open(&subtitle)
            .expect("open")
            .set_modified(stamp)
            .expect("restore timestamp");
        assert!(
            matches!(
                assess(&registry, &item, &settings).expect("same-size subtitle"),
                Match::Changed(_)
            ),
            "Subtitle content hash must catch coarse-timestamp edits"
        );
        fs::write(&subtitle, b"changed subtitles").expect("change subtitle");
        assert!(matches!(
            assess(&registry, &item, &settings).expect("subtitle"),
            Match::Changed(_)
        ));
        assert!(request.unchanged(&item, &settings).is_err());
        save(
            &registry,
            Request::capture(&item, &settings).expect("request"),
            &output,
            Measurements {
                subtitle_applied: true,
                ..Measurements::default()
            },
        )
        .expect("save");
        fs::write(&video, b"changed source video").expect("change video");
        assert!(matches!(
            assess(&registry, &item, &settings).expect("source"),
            Match::Changed(_)
        ));
        save(
            &registry,
            Request::capture(&item, &settings).expect("request"),
            &output,
            Measurements {
                subtitle_applied: true,
                ..Measurements::default()
            },
        )
        .expect("save");
        save(
            &registry,
            Request::capture(&item, &settings).expect("request"),
            &output,
            Measurements::default(),
        )
        .expect("video-only record");
        assert!(matches!(
            assess(&registry, &item, &settings).expect("unapplied subtitle"),
            Match::Changed(_)
        ));
        save(
            &registry,
            Request::capture(&item, &settings).expect("request"),
            &output,
            Measurements {
                subtitle_applied: true,
                ..Measurements::default()
            },
        )
        .expect("restore record");
        fs::write(&output, b"modified output bytes").expect("change output");
        assert!(matches!(
            assess(&registry, &item, &settings).expect("output"),
            Match::Changed(_)
        ));
        assert_eq!(
            fs::read(output).expect("preserved"),
            b"modified output bytes"
        );
    }

    #[test]
    fn unreadable_optional_subtitle_can_encode_but_cannot_verify_output_reuse() {
        let root = tempfile::tempdir().expect("fixture");
        let directory = fs::canonicalize(root.path()).expect("canonical");
        let video = directory.join("Ep01.mkv");
        let subtitle = directory.join("Ep01.srt");
        let output = directory.join("Ep01.mp4");
        fs::write(&video, b"source video").expect("video");
        fs::write(&output, b"finished output").expect("output");
        fs::create_dir(&subtitle).expect("subtitle replaced by directory");
        let item = Item {
            input: video.clone(),
            output: output.clone(),
            subtitle: Some(subtitle),
            subtitle_root: Some(directory.clone()),
            validation_error: None,
        };
        let settings = Settings {
            input_dir: video.to_string_lossy().into_owned(),
            output_dir: directory.to_string_lossy().into_owned(),
            subtitle_dir: None,
            anime_title: String::new(),
            crf: 23,
            cpu_mode: crate::CpuMode::Maximum,
        };
        let request =
            Request::capture(&item, &settings).expect("optional subtitle must not abort video");
        request
            .unchanged(&item, &settings)
            .expect("stable fallback");
        let registry = directory.join("records");
        save(
            &registry,
            request,
            &output,
            Measurements {
                subtitle_applied: true,
                ..Measurements::default()
            },
        )
        .expect("record");
        assert!(
            matches!(assess(&registry, &item, &settings).expect("assess"), Match::Changed(reason) if reason.contains("could not be verified"))
        );
        assert_eq!(
            fs::read(output).expect("preserved media"),
            b"finished output"
        );
    }

    #[test]
    fn benchmark_statistics_ignore_invalid_values() {
        let m = Measurements::from_stderr(
            "[info] bench: utime=1.25s stime=0.5s rtime=2s\nbench: maxrss=4096KiB",
        );
        assert_eq!(m.user_cpu_seconds, Some(1.25));
        assert_eq!(m.system_cpu_seconds, Some(0.5));
        assert_eq!(m.peak_memory_kib, Some(4096));
        assert!(
            Measurements::from_stderr("bench: utime=NaNs")
                .user_cpu_seconds
                .is_none()
        );
    }
}
