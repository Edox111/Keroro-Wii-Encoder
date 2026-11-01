use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn existing(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("Cannot inspect output {}: {error}", path.display())),
        Ok(metadata) => {
            crate::artifacts::no_links(path)?;
            if !metadata.is_file() {
                return Err("Existing output is not a regular file; it was preserved.".into());
            }
            Ok(true)
        }
    }
}

pub fn partial_path(output: &Path) -> Result<PathBuf, String> {
    let mut name = output
        .file_name()
        .ok_or("Missing output filename.")?
        .to_os_string();
    name.push(".part");
    Ok(output.with_file_name(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "Requires KERORO_TEST_FFMPEG pointing to the bundled FFmpeg executable"]
    async fn ffmpeg_writes_mp4_part_and_only_rename_publishes_the_final() {
        use std::process::Stdio;
        let binary = std::env::var_os("KERORO_TEST_FFMPEG").expect("FFmpeg");
        let root = tempfile::tempdir().expect("fixture");
        let final_path = root.path().join("Ep01.mp4");
        let part = partial_path(&final_path).expect("part path");
        let result = tokio::process::Command::new(&binary)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=64x48:r=10:d=1",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-f",
                "mp4",
            ])
            .arg(&part)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await
            .expect("encode part");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!final_path.exists());
        crate::atomic_file::rename(&part, &final_path, false).expect("finalize");
        assert!(!part.exists() && existing(&final_path).expect("final"));
        let decode = tokio::process::Command::new(binary)
            .args(["-hide_banner", "-loglevel", "error", "-i"])
            .arg(&final_path)
            .args(["-f", "null", "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await
            .expect("decode final");
        assert!(
            decode.status.success(),
            "{}",
            String::from_utf8_lossy(&decode.stderr)
        );
    }
    #[test]
    fn existing_finals_are_preserved_and_parts_are_not_mistaken_for_finals() {
        let root = tempfile::tempdir().expect("fixture");
        let final_path = root.path().join("Episode01.mp4");
        let part = partial_path(&final_path).expect("part path");
        assert_eq!(part.file_name().expect("filename"), "Episode01.mp4.part");
        fs::write(&part, b"partial").expect("part");
        assert!(!existing(&final_path).expect("missing final"));
        fs::write(&final_path, b"preserve original").expect("final");
        assert!(existing(&final_path).expect("skip final"));
        assert_eq!(fs::read(&final_path).expect("read"), b"preserve original");
        let folder = root.path().join("directory.mp4");
        fs::create_dir(&folder).expect("directory");
        assert!(existing(&folder).is_err());
    }
}
