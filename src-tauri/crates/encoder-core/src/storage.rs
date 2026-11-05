use serde::Serialize;
use std::path::{Path, PathBuf};

pub const FAT32_GUARD_BYTES: u64 = 4_000_000_000;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub id: String,
    pub root: String,
    pub label: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}
impl DriveInfo {
    pub fn is_fat32(&self) -> bool {
        matches!(
            self.file_system.to_ascii_lowercase().as_str(),
            "fat32" | "vfat" | "msdos"
        )
    }
    pub fn validate_size(&self, duration: Option<f64>) -> Result<(), String> {
        if self.is_fat32() && estimated_bytes(duration).is_some_and(|n| n >= FAT32_GUARD_BYTES) {
            return Err("FAT32_4GB_WARNING: This video's conservative size estimate reaches 4,000 MB. Split the source into shorter videos or choose a different destination before encoding.".into());
        }
        crate::preflight::validate_space(self.available_bytes)
    }
}
pub fn estimated_bytes(duration: Option<f64>) -> Option<u64> {
    duration
        .filter(|v| v.is_finite() && *v > 0.0)
        .map(|v| (v * 420_000.0 * 1.02).ceil() as u64)
}
pub fn inspect(path: &Path) -> Result<DriveInfo, String> {
    crate::offline::local_path(path)?;
    let parent = path
        .ancestors()
        .find(|p| p.exists())
        .ok_or("Output drive is unavailable.")?;
    platform::inspect(&std::fs::canonicalize(parent).map_err(|e| e.to_string())?)
}
pub fn verify_target(path: &Path, expected: Option<&str>) -> Result<DriveInfo, String> {
    let volume = inspect(path)?;
    if expected.is_some_and(|id| id != volume.id) {
        return Err("The selected output volume was disconnected or replaced. Reconnect it or choose a new output destination.".into());
    }
    Ok(volume)
}
pub fn connected() -> Result<Vec<DriveInfo>, String> {
    platform::connected()
}

pub fn prepare(id: &str) -> Result<PathBuf, String> {
    let drive = connected()?
        .into_iter()
        .find(|drive| drive.id == id)
        .ok_or("The selected drive is no longer connected. Refresh the drive list.")?;
    let root = PathBuf::from(&drive.root);
    let mut path = root.clone();
    for folder in ["videos", "WiiMC"] {
        crate::artifacts::no_links(&path)?;
        if inspect(&root)?.id != id {
            return Err("The target volume changed.".into());
        }
        path.push(folder);
        match std::fs::create_dir(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(crate::preflight::access_error(
                    "Cannot create WiiMC folder",
                    error,
                ));
            }
        }
        crate::artifacts::no_links(&path)?;
        if !path.is_dir() {
            return Err("WiiMC target is not a directory.".into());
        }
    }
    if inspect(&path)?.id != id {
        return Err("The target volume changed.".into());
    }
    Ok(path)
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::{os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        Storage::FileSystem::*,
        System::{IO::DeviceIoControl, Ioctl::*},
    };
    fn wide(value: &std::ffi::OsStr) -> Result<Vec<u16>, String> {
        let mut value: Vec<_> = value.encode_wide().collect();
        if value.contains(&0) {
            return Err("Path contains a NUL character.".into());
        }
        value.push(0);
        Ok(value)
    }
    fn text(value: &[u16]) -> String {
        String::from_utf16_lossy(
            &value[..value.iter().position(|c| *c == 0).unwrap_or(value.len())],
        )
    }
    pub fn inspect(path: &Path) -> Result<DriveInfo, String> {
        let input = wide(path.as_os_str())?;
        let mut root = vec![0u16; 32768];
        let mut label = [0u16; 261];
        let mut fs = [0u16; 261];
        let mut serial = 0;
        let mut available = 0;
        let mut total = 0;
        let mut guid = [0u16; 64];
        // All output buffers remain live, with lengths in UTF-16 code units.
        unsafe {
            if GetVolumePathNameW(input.as_ptr(), root.as_mut_ptr(), root.len() as u32) == 0
                || GetVolumeInformationW(
                    root.as_ptr(),
                    label.as_mut_ptr(),
                    label.len() as u32,
                    &mut serial,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    fs.as_mut_ptr(),
                    fs.len() as u32,
                ) == 0
                || GetDiskFreeSpaceExW(root.as_ptr(), &mut available, &mut total, ptr::null_mut())
                    == 0
            {
                return Err(format!(
                    "Cannot inspect output volume: {}",
                    std::io::Error::last_os_error()
                ));
            }
            if GetVolumeNameForVolumeMountPointW(
                root.as_ptr(),
                guid.as_mut_ptr(),
                guid.len() as u32,
            ) == 0
            {
                return Err(format!(
                    "Cannot identify output volume: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
        let root = text(&root).trim_start_matches(r"\\?\").to_owned();
        Ok(DriveInfo {
            id: format!("{}:{serial:08x}", text(&guid)),
            root,
            label: text(&label),
            file_system: text(&fs),
            total_bytes: total,
            available_bytes: available,
        })
    }
    fn usb_or_sd(letter: u8) -> bool {
        let Ok(name) = wide(std::ffi::OsStr::new(&format!(r"\\.\{}:", letter as char))) else {
            return false;
        };
        // A zero-access handle only queries the device descriptor; it never writes to disk.
        unsafe {
            let handle = CreateFileW(
                name.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            );
            if handle == INVALID_HANDLE_VALUE {
                return false;
            }
            let query = STORAGE_PROPERTY_QUERY {
                PropertyId: StorageDeviceProperty,
                QueryType: PropertyStandardQuery,
                AdditionalParameters: [0],
            };
            let mut descriptor: STORAGE_DEVICE_DESCRIPTOR = std::mem::zeroed();
            let mut returned = 0;
            let ok = DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                (&query as *const STORAGE_PROPERTY_QUERY).cast(),
                size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                (&mut descriptor as *mut STORAGE_DEVICE_DESCRIPTOR).cast(),
                size_of::<STORAGE_DEVICE_DESCRIPTOR>() as u32,
                &mut returned,
                ptr::null_mut(),
            );
            CloseHandle(handle);
            ok != 0
                && returned >= size_of::<STORAGE_DEVICE_DESCRIPTOR>() as u32
                && [BusTypeUsb, BusTypeSd, BusTypeMmc].contains(&descriptor.BusType)
        }
    }
    pub fn connected() -> Result<Vec<DriveInfo>, String> {
        let mask = unsafe { GetLogicalDrives() };
        if mask == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut drives = Vec::new();
        for letter in b'D'..=b'Z' {
            if mask & (1 << (letter - b'A')) == 0 {
                continue;
            }
            let root = format!("{}:\\", letter as char);
            let encoded = wide(std::ffi::OsStr::new(&root))?;
            let kind = unsafe { GetDriveTypeW(encoded.as_ptr()) };
            if (kind == 2 || kind == 3 && usb_or_sd(letter))
                && let Ok(drive) = inspect(Path::new(&root))
            {
                drives.push(drive);
            }
        }
        Ok(drives)
    }
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    struct Mount {
        root: PathBuf,
        device: String,
        fs: String,
        external: bool,
    }
    #[cfg(target_os = "linux")]
    fn mounts() -> Result<Vec<Mount>, String> {
        fn decode(s: &str) -> String {
            s.replace("\\040", " ")
                .replace("\\011", "\t")
                .replace("\\012", "\n")
                .replace("\\134", "\\")
        }
        let data = std::fs::read_to_string("/proc/self/mountinfo").map_err(|e| e.to_string())?;
        Ok(data
            .lines()
            .filter_map(|line| {
                let (head, tail) = line.split_once(" - ")?;
                let h: Vec<_> = head.split_whitespace().collect();
                let t: Vec<_> = tail.split_whitespace().collect();
                let device = decode(t.get(1)?);
                let root = PathBuf::from(decode(h.get(4)?));
                let sys = std::fs::canonicalize(format!("/sys/dev/block/{}", h.get(2)?)).ok();
                let external = sys.is_some_and(|p| {
                    p.to_string_lossy().contains("/usb")
                        || p.ancestors().any(|p| {
                            std::fs::read_to_string(p.join("removable"))
                                .is_ok_and(|v| v.trim() == "1")
                        })
                });
                Some(Mount {
                    root,
                    device,
                    fs: (*t.first()?).into(),
                    external,
                })
            })
            .collect())
    }
    #[cfg(target_os = "macos")]
    fn mounts() -> Result<Vec<Mount>, String> {
        // getmntinfo uses shared storage; copy the records while serialized.
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = LOCK.lock().map_err(|e| e.to_string())?;
        let mut buffer = std::ptr::null_mut();
        let count = unsafe { libc::getmntinfo(&mut buffer, libc::MNT_NOWAIT) };
        if count <= 0 || buffer.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let text = |bytes: &[libc::c_char]| {
            String::from_utf8_lossy(
                &bytes
                    .iter()
                    .take_while(|c| **c != 0)
                    .map(|c| *c as u8)
                    .collect::<Vec<_>>(),
            )
            .into_owned()
        };
        Ok(
            unsafe { std::slice::from_raw_parts(buffer, count as usize) }
                .iter()
                .map(|m| {
                    let root = PathBuf::from(text(&m.f_mntonname));
                    let external = root.starts_with("/Volumes") && root != Path::new("/Volumes");
                    Mount {
                        root,
                        external,
                        device: text(&m.f_mntfromname),
                        fs: text(&m.f_fstypename),
                    }
                })
                .collect(),
        )
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn mounts() -> Result<Vec<Mount>, String> {
        Err("Volume discovery is not supported on this platform.".into())
    }
    pub fn inspect(path: &Path) -> Result<DriveInfo, String> {
        let mount = mounts()?
            .into_iter()
            .filter(|m| path.starts_with(&m.root))
            .max_by_key(|m| m.root.components().count())
            .ok_or("Output mount is unavailable.")?;
        let encoded = CString::new(path.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
        let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        if unsafe { libc::statvfs(encoded.as_ptr(), stats.as_mut_ptr()) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let stats = unsafe { stats.assume_init() };
        let root = mount.root.to_string_lossy().into_owned();
        let label = mount
            .root
            .file_name()
            .map(|v| v.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.clone());
        Ok(DriveInfo {
            id: format!("{}:{}:{}", mount.device, stats.f_fsid, root),
            root,
            label,
            file_system: mount.fs,
            total_bytes: (stats.f_blocks as u64).saturating_mul(stats.f_frsize as u64),
            available_bytes: (stats.f_bavail as u64).saturating_mul(stats.f_frsize as u64),
        })
    }
    pub fn connected() -> Result<Vec<DriveInfo>, String> {
        Ok(mounts()?
            .iter()
            .filter(|m| m.external)
            .filter_map(|m| inspect(&m.root).ok())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn size_guard_is_filesystem_specific_and_conservative() {
        let mut drive = DriveInfo {
            id: "fixture".into(),
            root: String::new(),
            label: String::new(),
            file_system: "FAT32".into(),
            total_bytes: 32_000_000_000,
            available_bytes: 16_000_000_000,
        };
        assert!(drive.validate_size(Some(24.0 * 60.0)).is_ok());
        assert!(drive.validate_size(Some(10_000.0)).is_err());
        assert!(drive.validate_size(None).is_ok());
        assert_eq!(estimated_bytes(Some(f64::NAN)), None);
        drive.file_system = "NTFS".into();
        assert!(drive.validate_size(Some(10_000.0)).is_ok());
    }
    #[test]
    fn existing_volume_has_capacity_and_identity() {
        let folder = tempfile::tempdir().expect("fixture");
        let drive = inspect(folder.path()).expect("volume");
        assert!(!drive.id.is_empty());
        assert!(drive.total_bytes >= drive.available_bytes && drive.total_bytes > 0);
        assert_eq!(
            inspect(&folder.path().join("not-created"))
                .expect("parent")
                .id,
            drive.id
        );
        assert!(prepare("unconnected-test-volume").is_err());
        assert!(verify_target(folder.path(), Some("replaced-volume")).is_err());
        assert!(verify_target(folder.path(), Some(&drive.id)).is_ok());
    }
}
