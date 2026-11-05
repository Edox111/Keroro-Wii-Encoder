use encoding_rs::{Encoding, SHIFT_JIS, UTF_16BE, UTF_16LE, WINDOWS_1252};
use std::{io::Write, path::Path};
use tempfile::{NamedTempFile, TempDir};
use tokio::{fs, io::AsyncReadExt};

/// Decimal MB. Check metadata before reading, then bound reads against growth.
pub const MAX_SUBTITLE_BYTES: u64 = 50_000_000;
pub const EXTENSIONS: &[&str] = &["srt", "ass", "ssa", "vtt"];
pub fn extension(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?;
    EXTENSIONS
        .iter()
        .copied()
        .find(|supported| ext.eq_ignore_ascii_case(supported))
}
pub struct SubtitleText {
    pub text: String,
    pub warning: Option<String>,
}

#[derive(Debug)]
pub enum ReadError {
    Unavailable(String),
    Syntax(String),
}
impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(message) => f.write_str(message),
            Self::Syntax(message) => write!(f, "Subtitle syntax error: {message}"),
        }
    }
}
impl From<String> for ReadError {
    fn from(message: String) -> Self {
        Self::Unavailable(message)
    }
}
impl From<&str> for ReadError {
    fn from(message: &str) -> Self {
        Self::Unavailable(message.into())
    }
}

/// Keep both owners alive until FFmpeg exits. Fields drop in declaration order.
pub struct StagedSubtitle {
    file: NamedTempFile,
    directory: TempDir,
}
impl StagedSubtitle {
    pub fn directory(&self) -> &Path {
        self.directory.path()
    }
    #[cfg(test)]
    pub(crate) fn path(&self) -> &Path {
        self.file.path()
    }
    pub fn filename(&self) -> Result<&str, String> {
        let name = self
            .file
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Invalid internal subtitle filename.")?;
        if !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
        {
            return Err("Unsafe internal subtitle filename.".into());
        }
        Ok(name)
    }
    pub fn close(self) -> Result<(), String> {
        let Self { file, directory } = self;
        let path = file.path().to_path_buf();
        let file_result = file.close();
        let directory_result = directory.close();
        match (file_result, directory_result) {
            (Ok(()), Ok(())) => crate::artifacts::forget_sync(&path),
            (file, directory) => Err(format!(
                "Subtitle temp cleanup failed: file={file:?}, directory={directory:?}"
            )),
        }
    }
}
fn validate_metadata(metadata: &std::fs::Metadata) -> Result<(), String> {
    if !metadata.is_file() {
        return Err("Subtitle must be a regular file.".into());
    }
    if metadata.len() > MAX_SUBTITLE_BYTES {
        return Err("Subtitle exceeds the 50 MB size limit; skipped before reading.".into());
    }
    Ok(())
}
pub async fn read_scoped(path: &Path, root: Option<&Path>) -> Result<SubtitleText, ReadError> {
    let canonical = fs::canonicalize(path)
        .await
        .map_err(|e| format!("Cannot resolve subtitle: {e}"))?;
    if root.is_none_or(|root| canonical.parent() != Some(root)) {
        return Err("Subtitle resolves outside the selected canonical subtitle folder.".into());
    }
    read(&canonical).await
}
pub async fn read(path: &Path) -> Result<SubtitleText, ReadError> {
    let ext = extension(path).ok_or("Unsupported subtitle format.")?;
    validate_metadata(
        &fs::metadata(path)
            .await
            .map_err(|e| format!("Cannot inspect subtitle: {e}"))?,
    )?;
    let file = fs::File::open(path)
        .await
        .map_err(|e| format!("Cannot read subtitle: {e}"))?;
    let metadata = file.metadata().await.map_err(|e| e.to_string())?;
    validate_metadata(&metadata)?; // Covers replacement between metadata and open.
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(metadata.len() as usize + 1)
        .map_err(|_| "Not enough memory to read this subtitle.")?;
    file.take(MAX_SUBTITLE_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_SUBTITLE_BYTES {
        return Err("Subtitle grew beyond the 50 MB limit while reading.".into());
    }
    tokio::task::spawn_blocking(move || {
        let decoded = decode(&bytes).map_err(ReadError::Unavailable)?;
        validate_syntax(&decoded.text, ext).map_err(ReadError::Syntax)?;
        Ok(decoded)
    })
    .await
    .map_err(|e| format!("Subtitle normalization worker failed: {e}"))?
}

fn time(value: &str, decimal: char, digits: usize, short: bool) -> Option<u64> {
    let value = value.split_whitespace().next()?;
    let mut fields = value.split(':');
    let first = fields.next()?;
    let second = fields.next()?;
    let third = fields.next();
    if fields.next().is_some() {
        return None;
    }
    let (hours, minutes, seconds) = if let Some(third) = third {
        (first, second, third)
    } else if short {
        ("0", first, second)
    } else {
        return None;
    };
    let (seconds, fraction) = seconds.split_once(decimal)?;
    if minutes.len() != 2
        || seconds.len() != 2
        || fraction.len() != digits
        || [hours, minutes, seconds, fraction]
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let hours = hours.parse::<u64>().ok()?;
    let minutes = minutes.parse::<u64>().ok()?;
    let seconds = seconds.parse::<u64>().ok()?;
    if minutes >= 60 || seconds >= 60 {
        return None;
    }
    let millis = fraction
        .parse::<u64>()
        .ok()?
        .checked_mul(if digits == 2 { 10 } else { 1 })?;
    hours
        .checked_mul(3_600_000)?
        .checked_add(minutes * 60_000 + seconds * 1000 + millis)
}

fn validate_syntax(text: &str, ext: &str) -> Result<(), String> {
    let styled = matches!(ext, "ass" | "ssa");
    if styled
        && !(text
            .lines()
            .any(|line| line.trim().eq_ignore_ascii_case("[Script Info]"))
            && text
                .lines()
                .any(|line| line.trim().eq_ignore_ascii_case("[Events]")))
    {
        return Err("ASS/SSA requires [Script Info] and [Events] headers.".into());
    }
    if ext == "vtt"
        && !text.lines().next().is_some_and(|line| {
            line == "WEBVTT" || line.starts_with("WEBVTT ") || line.starts_with("WEBVTT\t")
        })
    {
        return Err("VTT requires a WEBVTT header.".into());
    }
    let mut cues = 0;
    for (number, line) in text.lines().enumerate() {
        let line = line.trim_start();
        let pair = if styled {
            if !line
                .get(..9)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("Dialogue:"))
            {
                continue;
            }
            let body = line.get(9..).ok_or("Invalid dialogue line.")?;
            let mut fields = body.splitn(4, ',');
            let _layer_or_marked = fields.next();
            Some((fields.next().unwrap_or(""), fields.next().unwrap_or("")))
        } else {
            line.split_once("-->")
        };
        let Some((start, end)) = pair else {
            continue;
        };
        let decimal = if ext == "srt" { ',' } else { '.' };
        let digits = if styled { 2 } else { 3 };
        let start = time(start, decimal, digits, ext == "vtt");
        let end = time(end, decimal, digits, ext == "vtt");
        if !matches!((start, end), (Some(start), Some(end)) if end >= start) {
            return Err(format!(
                "Invalid or reversed timestamp at line {}.",
                number + 1
            ));
        }
        cues += 1;
    }
    if cues == 0 {
        return Err("No recognizable timed subtitle cues.".into());
    }
    Ok(())
}
pub async fn stage(text: String, ext: &'static str) -> Result<StagedSubtitle, String> {
    if !EXTENSIONS.contains(&ext) {
        return Err("Invalid internal subtitle extension.".into());
    }
    tokio::task::spawn_blocking(move || {
        // tempfile uses a non-cryptographic name generator; explicitly add OS entropy.
        let mut entropy = [0u8; 32];
        getrandom::fill(&mut entropy)
            .map_err(|e| format!("Cannot obtain secure temp-name randomness: {e}"))?;
        let token = entropy[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let prefix = format!("keroro-sub-{token}-");
        let file_token = entropy[16..]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let file_prefix = format!("subtitle-{file_token}-");
        let mut builder = tempfile::Builder::new();
        builder.prefix(&prefix).rand_bytes(16);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(std::fs::Permissions::from_mode(0o700));
        }
        let directory = match crate::artifacts::workspace() {
            Some(parent) => builder.tempdir_in(parent),
            None => builder.tempdir(),
        }
        .map_err(|e| format!("Cannot create isolated subtitle directory: {e}"))?;
        let mut file = tempfile::Builder::new()
            .prefix(&file_prefix)
            .suffix(&format!(".{ext}"))
            .rand_bytes(16)
            .tempfile_in(directory.path())
            .map_err(|e| format!("Cannot create private subtitle file: {e}"))?;
        crate::artifacts::record_sync(file.path())?;
        file.write_all(text.as_bytes())
            .and_then(|_| file.flush())
            .map_err(|e| format!("Cannot stage UTF-8 subtitle: {e}"))?;
        Ok(StagedSubtitle { file, directory })
    })
    .await
    .map_err(|e| format!("Subtitle staging worker failed: {e}"))?
}
fn strict(bytes: &[u8], encoding: &'static Encoding) -> Result<String, String> {
    encoding
        .decode_without_bom_handling_and_without_replacement(bytes)
        .map(|text| text.into_owned())
        .ok_or_else(|| format!("Invalid {} subtitle text.", encoding.name()))
}
fn decode(bytes: &[u8]) -> Result<SubtitleText, String> {
    let (text, warning) = if let Some(data) = bytes.strip_prefix(&[0xff, 0xfe]) {
        (
            strict(data, UTF_16LE)?,
            Some("Converted UTF-16 BOM subtitle to UTF-8.".into()),
        )
    } else if let Some(data) = bytes.strip_prefix(&[0xfe, 0xff]) {
        (
            strict(data, UTF_16BE)?,
            Some("Converted UTF-16 BOM subtitle to UTF-8.".into()),
        )
    } else {
        let data = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        match std::str::from_utf8(data) {
            Ok(text) => (text.to_owned(), None),
            Err(_) if bytes.starts_with(&[0xef, 0xbb, 0xbf]) => {
                return Err("Subtitle declares UTF-8 but contains invalid bytes.".into());
            }
            Err(_) => {
                let japanese = strict(data, SHIFT_JIS).ok().filter(|text| {
                    text.chars().any(
                        |c| matches!(c as u32, 0x3040..=0x30ff | 0x3400..=0x9fff | 0xff61..=0xff9f),
                    )
                });
                if let Some(text) = japanese {
                    (text, Some("Guessed Shift-JIS/Windows-932 subtitle encoding; verify the preview or save as UTF-8.".into()))
                } else {
                    (strict(data, WINDOWS_1252)?, Some("Guessed Windows-1252 subtitle encoding; ANSI varies. Verify the preview or save as UTF-8.".into()))
                }
            }
        }
    };
    if text.trim().is_empty()
        || text.chars().any(|c| {
            c == '\0' || c == '\u{fffd}' || (c.is_control() && !matches!(c, '\r' | '\n' | '\t'))
        })
    {
        return Err("Empty, binary, or invalid subtitle text; save as UTF-8 and retry.".into());
    }
    Ok(SubtitleText { text, warning })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_and_legacy_decoding_are_strict() {
        assert_eq!(
            decode("\u{30b1}\u{30ed}\u{30ed}".as_bytes())
                .expect("UTF-8")
                .text,
            "\u{30b1}\u{30ed}\u{30ed}"
        );
        assert_eq!(decode(b"\xef\xbb\xbfHello").expect("BOM").text, "Hello");
        assert_eq!(decode(b"\xff\xfeH\0i\0").expect("UTF-16").text, "Hi");
        assert!(decode(b"\xff\xfeX").is_err());
        assert!(decode(b"binary\0data").is_err());
        assert!(decode(b"\xef\xbb\xbfbroken\xff").is_err());
        let sjis = decode(&[0x83, 0x50, 0x83, 0x8d, 0x83, 0x8d]).expect("Shift-JIS");
        assert_eq!(sjis.text, "\u{30b1}\u{30ed}\u{30ed}");
        assert!(sjis.warning.expect("guess").contains("Shift-JIS"));
        assert_eq!(decode(b"caf\xe9").expect("CP1252").text, "caf\u{e9}");
    }
    #[tokio::test]
    async fn staging_is_isolated_and_drops_both_paths() {
        let first = stage("hello".into(), "ssa").await.expect("stage");
        let second = stage("world".into(), "vtt").await.expect("stage");
        assert_ne!(first.directory(), second.directory());
        assert!(first.filename().expect("safe basename").ends_with(".ssa"));
        let file = first.path().to_path_buf();
        let directory = first.directory().to_path_buf();
        drop(first);
        assert!(!file.exists() && !directory.exists());
    }
    #[tokio::test]
    async fn oversized_file_is_rejected_before_decoding() {
        let root = tempfile::tempdir().expect("fixture");
        let path = root.path().join("disguised.ass");
        let file = std::fs::File::create(&path).expect("file");
        file.set_len(MAX_SUBTITLE_BYTES + 1)
            .expect("oversized fixture");
        drop(file);
        assert!(
            read(&path)
                .await
                .err()
                .expect("reject")
                .to_string()
                .contains("skipped before reading")
        );
    }

    #[tokio::test]
    async fn scoped_reader_rejects_escape_and_accepts_canonical_root() {
        let fixture = tempfile::tempdir().expect("fixture");
        let root = fixture.path().join("selected");
        std::fs::create_dir(&root).expect("root");
        let inside = root.join("episode.vtt");
        let outside = fixture.path().join("outside.vtt");
        for path in [&inside, &outside] {
            std::fs::write(path, "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHello\n").expect("VTT");
        }
        let canonical = std::fs::canonicalize(&root).expect("canonical");
        assert!(read_scoped(&inside, Some(&canonical)).await.is_ok());
        assert!(
            read_scoped(&root.join("../outside.vtt"), Some(&canonical))
                .await
                .is_err()
        );
    }

    #[test]
    fn malformed_timing_and_headers_cannot_silently_probe_as_another_format() {
        assert!(validate_syntax("WEBVTT\n\nBROKEN --> NOT_A_TIME\nText\n", "vtt").is_err());
        assert!(validate_syntax("[Events]\nDialogue: broken\n", "ass").is_err());
        assert!(validate_syntax("1\n00:00:02,000 --> 00:00:01,000\nText\n", "srt").is_err());
        assert!(
            validate_syntax(
                "WEBVTT\n\n00:00.000 --> 00:01.000 align:start\nText\n",
                "vtt"
            )
            .is_ok()
        );
    }
}
