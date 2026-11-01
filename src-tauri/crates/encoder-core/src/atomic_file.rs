#[cfg(any(not(windows), test))]
use std::fs;
use std::{io::Write, path::Path};

/// Same-directory rename. Final media never becomes visible through a partial copy.
pub fn rename(source: &Path, destination: &Path, replace: bool) -> Result<(), String> {
    if source.parent() != destination.parent() {
        return Err("Atomic commit requires the same parent directory.".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };
        let wide = |path: &Path| -> Result<Vec<u16>, String> {
            let mut value = path.as_os_str().encode_wide().collect::<Vec<_>>();
            if value.contains(&0) {
                return Err("NUL in path.".into());
            }
            value.push(0);
            Ok(value)
        };
        // Canonical parent paths preserve long-path and redirected-temp semantics
        // for native Win32 calls, while retaining the destination basename.
        let parent = std::fs::canonicalize(source.parent().ok_or("Missing source parent.")?)
            .map_err(|e| e.to_string())?;
        let source = wide(&parent.join(source.file_name().ok_or("Missing source basename.")?))?;
        let destination =
            wide(&parent.join(destination.file_name().ok_or("Missing target basename.")?))?;
        // SAFETY: live terminated UTF-16 paths. No cross-volume copy flag; media never replaces an existing file.
        let ok = unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_WRITE_THROUGH
                    | if replace {
                        MOVEFILE_REPLACE_EXISTING
                    } else {
                        0
                    },
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(not(windows))]
    {
        if replace {
            fs::rename(source, destination).map_err(|e| e.to_string())?;
        } else {
            fs::hard_link(source, destination).map_err(|e| e.to_string())?;
            fs::remove_file(source).map_err(|e| e.to_string())?;
        }
        if let Some(parent) = destination.parent() {
            fs::File::open(parent)
                .and_then(|file| file.sync_all())
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing checkpoint parent.")?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    struct Limited<'a> {
        file: &'a mut std::fs::File,
        remaining: usize,
    }
    impl Write for Limited<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.remaining {
                return Err(std::io::Error::other(
                    "Local state exceeds the 8 MiB limit.",
                ));
            }
            let written = self.file.write(bytes)?;
            self.remaining -= written;
            Ok(written)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.file.flush()
        }
    }
    serde_json::to_writer(
        Limited {
            file: temp.as_file_mut(),
            remaining: 8 * 1024 * 1024,
        },
        value,
    )
    .map_err(|e| e.to_string())?;
    temp.flush()
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    rename(temp.path(), path, true)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_state_preserves_the_previous_checkpoint() {
        let root = tempfile::tempdir().expect("fixture");
        let path = root.path().join("state.json");
        json(&path, &vec![1]).expect("initial checkpoint");
        assert!(json(&path, &"x".repeat(8 * 1024 * 1024)).is_err());
        assert_eq!(fs::read_to_string(path).expect("read"), "[1]");
        assert_eq!(fs::read_dir(root.path()).expect("list").count(), 1);
    }
    #[test]
    fn commit_never_overwrites_existing_media_and_json_is_replaceable() {
        let root = tempfile::tempdir().expect("fixture");
        let source = root.path().join("temp.mp4");
        let target = root.path().join("final.mp4");
        fs::write(&source, b"complete").expect("temp");
        fs::write(&target, b"original").expect("existing");
        assert!(rename(&source, &target, false).is_err());
        assert_eq!(fs::read(&target).expect("read"), b"original");
        fs::remove_file(&target).expect("remove fixture");
        rename(&source, &target, false).expect("publish");
        assert_eq!(fs::read(target).expect("read"), b"complete");
        let checkpoint = root.path().join("state.json");
        json(&checkpoint, &vec![1]).expect("first");
        json(&checkpoint, &vec![1, 2]).expect("replace");
        assert_eq!(fs::read_to_string(checkpoint).expect("read"), "[1,2]");
    }
}
