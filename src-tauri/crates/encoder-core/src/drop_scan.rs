use std::{collections::HashSet, fs, path::PathBuf};

const MAX_PATHS: usize = 64;
const MAX_ENTRIES: usize = 512;

fn clean_windows_prefix(value: &str) -> String {
    if let Some(rest) = value.strip_prefix(r"\\?\") {
        let bytes = rest.as_bytes();
        if bytes.first().is_some_and(u8::is_ascii_alphabetic)
            && bytes.get(1) == Some(&b':')
            && matches!(bytes.get(2), Some(b'\\' | b'/'))
        {
            return rest.to_owned();
        }
    }
    // Keep UNC/device namespaces intact so the existing offline guard rejects them.
    value.to_owned()
}

pub fn analyze(paths: Vec<String>) -> Result<serde_json::Value, String> {
    if paths.is_empty() || paths.len() > MAX_PATHS {
        return Err("Drop between 1 and 64 local files or folders.".into());
    }
    if paths
        .iter()
        .any(|path| path.is_empty() || path.len() > 32_768)
    {
        return Err("A dropped path is empty or too long.".into());
    }
    let mut seen = HashSet::new();
    let mut media: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut subtitles = Vec::new();
    for value in paths {
        let path = PathBuf::from(clean_windows_prefix(&value));
        crate::offline::local_path(&path)?;
        crate::artifacts::no_links(&path)?;
        let path = fs::canonicalize(&path)
            .map_err(|e| format!("Cannot inspect dropped path {}: {e}", path.display()))?;
        crate::offline::local_path(&path)?;
        if !seen.insert(path.clone()) {
            continue;
        }
        let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
        let (folder, has_video, has_subtitle) = if metadata.is_dir() {
            let mut has_video = false;
            let mut has_subtitle = false;
            let mut entries_seen = 0usize;
            let entries = fs::read_dir(&path)
                .map_err(|e| format!("Cannot read dropped folder {}: {e}", path.display()))?;
            for entry in entries.take(MAX_ENTRIES) {
                let entry = entry.map_err(|e| e.to_string())?;
                entries_seen += 1;
                // Shallow metadata only: do not follow links, recurse, or read media contents.
                if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
                    continue;
                }
                let file = entry.path();
                has_video |= crate::is_video_path(&file);
                has_subtitle |= crate::subtitles::extension(&file).is_some();
                if has_video && has_subtitle {
                    break;
                }
            }
            if entries_seen == 0 && output.is_none() {
                output = Some(path.clone());
            }
            (path, has_video, has_subtitle)
        } else if metadata.is_file() {
            let has_video = crate::is_video_path(&path);
            let has_subtitle = crate::subtitles::extension(&path).is_some();
            (path, has_video, has_subtitle)
        } else {
            continue;
        };
        if has_video && media.is_none() {
            media = Some(folder.clone());
        }
        if has_subtitle {
            subtitles.push((folder, has_video));
        }
    }
    let subtitle = subtitles
        .iter()
        .find(|(_, has_video)| !has_video)
        .or_else(|| subtitles.first())
        .map(|(folder, _)| folder);
    if media.is_none() && subtitle.is_none() && output.is_none() {
        return Err(format!(
            "No supported media, subtitles, or completely empty output folder found in the first {MAX_ENTRIES} entries per folder. Nested folders are not scanned."
        ));
    }
    Ok(serde_json::json!({
        "media_path": media.as_ref().map(|path| clean_windows_prefix(&path.to_string_lossy())),
        "sub_path": subtitle.map(|path| clean_windows_prefix(&path.to_string_lossy())),
        "output_path": output.as_ref().map(|path| clean_windows_prefix(&path.to_string_lossy())),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extended_drive_prefixes_are_cleaned_without_reinterpreting_network_namespaces() {
        assert_eq!(clean_windows_prefix(r"\\?\D:\Anime"), r"D:\Anime");
        assert_eq!(clean_windows_prefix(r"D:\Anime"), r"D:\Anime");
        assert_eq!(
            clean_windows_prefix(r"\\?\UNC\server\share"),
            r"\\?\UNC\server\share"
        );
        assert_eq!(
            clean_windows_prefix(r"\\?\GLOBALROOT\Device"),
            r"\\?\GLOBALROOT\Device"
        );
    }
    #[test]
    fn only_truly_empty_directories_are_output_candidates() {
        let root = tempfile::tempdir().expect("fixture");
        let canonical = fs::canonicalize(root.path()).expect("canonical");
        let result = analyze(vec![canonical.to_string_lossy().into()]).expect("empty folder");
        assert!(result["media_path"].is_null() && result["sub_path"].is_null());
        assert_eq!(
            result["output_path"],
            clean_windows_prefix(&canonical.to_string_lossy())
        );
        fs::write(root.path().join(".keep"), []).expect("hidden file");
        assert!(analyze(vec![canonical.to_string_lossy().into()]).is_err());
        fs::remove_file(root.path().join(".keep")).expect("remove fixture file");
        fs::create_dir(root.path().join("nested")).expect("subdirectory");
        assert!(analyze(vec![canonical.to_string_lossy().into()]).is_err());
    }
    #[test]
    fn multiple_folders_prefer_dedicated_subtitles_and_accept_unicode() {
        let root = tempfile::tempdir().expect("fixture");
        let media = root.path().join("\u{30b1}\u{30ed}\u{30ed} & anime");
        let subtitles = root.path().join("Subtitles");
        let output = root.path().join("Output");
        fs::create_dir(&media).expect("media");
        fs::create_dir(&subtitles).expect("subtitles");
        fs::create_dir(&output).expect("output");
        fs::write(media.join("Episode01.MKV"), []).expect("video");
        fs::write(media.join("Episode01.srt"), []).expect("mixed subtitle");
        fs::write(subtitles.join("Episode01.ASS"), []).expect("dedicated subtitle");
        let result = analyze(vec![
            media.to_string_lossy().into(),
            subtitles.to_string_lossy().into(),
            output.to_string_lossy().into(),
        ])
        .expect("analyze");
        assert_eq!(
            result["media_path"],
            clean_windows_prefix(
                &fs::canonicalize(media)
                    .expect("canonical")
                    .to_string_lossy()
            )
        );
        assert_eq!(
            result["sub_path"],
            clean_windows_prefix(
                &fs::canonicalize(subtitles)
                    .expect("canonical")
                    .to_string_lossy()
            )
        );
        assert_eq!(
            result["output_path"],
            clean_windows_prefix(
                &fs::canonicalize(output)
                    .expect("canonical")
                    .to_string_lossy()
            )
        );
    }
    #[test]
    fn file_drops_retain_exact_paths_and_nested_directories_are_not_scanned() {
        let root = tempfile::tempdir().expect("fixture");
        let nested = root.path().join("nested");
        fs::create_dir(&nested).expect("nested");
        let video = nested.join("Episode01.webm");
        let subtitle = nested.join("Episode01.VTT");
        fs::write(&video, []).expect("video");
        fs::write(&subtitle, []).expect("subtitle");
        assert!(analyze(vec![root.path().to_string_lossy().into()]).is_err());
        let result = analyze(vec![
            video.to_string_lossy().into(),
            subtitle.to_string_lossy().into(),
        ])
        .expect("files");
        for (key, path) in [("media_path", video), ("sub_path", subtitle)] {
            assert_eq!(
                result[key],
                clean_windows_prefix(&fs::canonicalize(path).expect("canonical").to_string_lossy())
            );
        }
    }
    #[test]
    fn subtitle_only_and_unsupported_drops_have_clear_results() {
        let root = tempfile::tempdir().expect("fixture");
        let subtitle = root.path().join("Episode01.SSA");
        fs::write(&subtitle, []).expect("subtitle");
        let result = analyze(vec![subtitle.to_string_lossy().into()]).expect("subtitle only");
        assert!(result["media_path"].is_null());
        let unsupported = root.path().join("readme.txt");
        fs::write(&unsupported, []).expect("other file");
        assert!(analyze(vec![unsupported.to_string_lossy().into()]).is_err());
        assert!(analyze(Vec::new()).is_err());
        assert!(analyze(vec![String::from("x"); MAX_PATHS + 1]).is_err());
    }
}
