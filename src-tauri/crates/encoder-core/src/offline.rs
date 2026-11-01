use std::path::Path;
pub fn local_path(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let absolute = std::path::absolute(path).map_err(|e| e.to_string())?;
        let drive = match absolute.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
                _ => {
                    return Err(
                        "OFFLINE_PATH_REQUIRED: UNC shares and device namespaces are disabled."
                            .into(),
                    );
                }
            },
            _ => return Err("A local drive path is required.".into()),
        };
        let root = [u16::from(drive), 58, 92, 0];
        // SAFETY: fixed terminated drive-root buffer. DRIVE_REMOTE is 4.
        if unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(root.as_ptr()) } == 4 {
            return Err("OFFLINE_PATH_REQUIRED: Mapped network drives are disabled.".into());
        }
    }
    #[cfg(not(windows))]
    {
        let _ = path;
    }
    Ok(())
}
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn explicit_network_paths_are_rejected_without_opening_them() {
        assert!(local_path(Path::new(r"\\server\share\video")).is_err());
        assert!(local_path(Path::new(r"\\?\UNC\server\share\video")).is_err());
        assert!(local_path(Path::new(r"\\.\pipe\unknown")).is_err());
    }
}
