use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::{fs, io::AsyncWriteExt};

pub const MIN_FREE_BYTES: u64 = 64 * 1024 * 1024;

pub fn access_error(action: &str, error: std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        format!(
            "OUTPUT_PERMISSION_DENIED: {action}: {error}. Choose a writable folder or review Windows Controlled Folder Access permissions."
        )
    } else {
        format!("{action}: {error}")
    }
}

/// A unique zero-byte canary. Never truncate/delete a pre-existing user's file.
pub async fn canary(path: &Path) -> Result<(), String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let probe = path.join(format!(
        ".keroro_access_test-{}-{stamp}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .await
        .map_err(|error| access_error("Cannot create output canary", error))?;
    let journal = crate::artifacts::record(&probe).await;
    drop(file); // No write calls: exactly zero bytes.
    let cleanup = fs::remove_file(&probe)
        .await
        .map_err(|error| access_error("Cannot remove output canary", error));
    journal?;
    cleanup?;
    crate::artifacts::forget(&probe).await
}

pub fn validate_space(available: u64) -> Result<(), String> {
    if available < MIN_FREE_BYTES {
        Err(format!(
            "Output drive has only {} MiB available; at least 64 MiB of free reserve is required. Free space and retry.",
            available / 1024 / 1024
        ))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn available(path: &Path) -> Result<Option<u64>, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if wide.contains(&0) {
        return Err("Output path contains a NUL character.".into());
    }
    if wide.last() != Some(&92) {
        wide.push(92);
    } // UNC directories also require a trailing slash.
    wide.push(0);
    let mut bytes = 0u64;
    // SAFETY: NUL-terminated live UTF-16 path, valid u64 output, optional unused
    // outputs are null. Use caller-available bytes so disk quotas are respected.
    let success = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut bytes,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if success == 0 {
        return Err(format!(
            "Cannot query output free space: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(Some(bytes))
}

#[cfg(not(windows))]
fn available(_: &Path) -> Result<Option<u64>, String> {
    Ok(None)
}

/// A free-space reserve is a guardrail, not a size prediction for CRF encoding.
pub async fn space(path: &Path) -> Result<Option<String>, String> {
    let owned = path.to_path_buf();
    let query = tokio::task::spawn_blocking(move || available(&owned)).await;
    match query {
        Ok(Ok(Some(bytes))) => {
            validate_space(bytes)?;
            Ok(None)
        }
        Ok(Ok(None)) => Ok(Some(
            "Free-space pre-check is unavailable on this platform.".into(),
        )),
        Ok(Err(error)) => Ok(Some(format!("{error}; free-space pre-check unavailable."))),
        Err(error) => Ok(Some(format!(
            "Free-space query could not complete: {error}"
        ))),
    }
}

pub async fn output(path: &Path) -> Result<Option<String>, String> {
    if !fs::metadata(path)
        .await
        .map_err(|e| access_error("Cannot inspect output folder", e))?
        .is_dir()
    {
        return Err("Output path must be a directory.".into());
    }
    canary(path).await?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    // Unique create_new prevents overwriting any pre-existing file or symlink.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let probe = path.join(format!(
        ".keroro-write-check-{}-{stamp}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .await
        .map_err(|e| access_error("Output folder is not writable", e))?;
    let journal = crate::artifacts::record(&probe).await;
    let check = async {
        file.write_all(b"Keroro output write validation\n").await?;
        file.flush().await?;
        file.sync_all().await
    }
    .await;
    drop(file);
    let cleanup = fs::remove_file(&probe).await;
    if let Err(error) = cleanup {
        return Err(format!(
            "Cannot remove output write-check file {}: {error}",
            probe.display()
        ));
    }
    journal?;
    crate::artifacts::forget(&probe).await?;
    check.map_err(|e| access_error("Output write validation failed", e))?;
    space(path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn low_space_is_rejected_without_filling_a_real_drive() {
        assert!(validate_space(0).is_err());
        assert!(validate_space(MIN_FREE_BYTES - 1).is_err());
        assert!(validate_space(MIN_FREE_BYTES).is_ok());
    }
    #[test]
    fn permission_errors_have_an_actionable_stable_code() {
        let error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "blocked by OS");
        assert!(
            access_error("Cannot create canary", error).starts_with("OUTPUT_PERMISSION_DENIED:")
        );
    }
    #[tokio::test]
    async fn canary_leaves_no_files_and_preserves_existing_names() {
        let root = tempfile::tempdir().expect("fixture");
        let existing = root.path().join(".keroro_access_test.tmp");
        fs::write(&existing, "keep me").await.expect("existing");
        canary(root.path()).await.expect("canary");
        assert_eq!(
            fs::read_to_string(&existing).await.expect("preserved"),
            "keep me"
        );
        let mut entries = fs::read_dir(root.path()).await.expect("directory");
        let mut count = 0;
        while entries.next_entry().await.expect("entry").is_some() {
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
