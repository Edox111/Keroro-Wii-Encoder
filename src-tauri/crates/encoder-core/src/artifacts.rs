use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Identity {
    volume: u64,
    index: u64,
}
fn identity(path: &Path) -> Result<Identity, String> {
    no_links(path)?;
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
        };
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        // SAFETY: live file handle and correctly sized writable output.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(Identity {
            volume: u64::from(info.dwVolumeSerialNumber),
            index: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        })
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let info = file.metadata().map_err(|e| e.to_string())?;
        Ok(Identity {
            volume: info.dev(),
            index: info.ino(),
        })
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = file;
        Err("Stable artifact identity is unsupported on this platform.".into())
    }
}
pub fn no_links(path: &Path) -> Result<(), String> {
    let components = path.ancestors().collect::<Vec<_>>();
    for component in components.into_iter().rev() {
        let metadata = fs::symlink_metadata(component).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err("Cleanup refuses symbolic links.".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err("Cleanup refuses reparse points/junctions.".into());
            }
        }
    }
    Ok(())
}
#[derive(Clone, Serialize, Deserialize)]
struct Artifact {
    path: PathBuf,
    identity: Identity,
}
pub struct ArtifactStore {
    ledger: PathBuf,
    cache: PathBuf,
    entries: Mutex<Vec<Artifact>>,
}
static STORE: OnceLock<Arc<ArtifactStore>> = OnceLock::new();

impl ArtifactStore {
    pub fn open(data: &Path, cache: &Path) -> Result<Arc<Self>, String> {
        crate::offline::local_path(data)?;
        crate::offline::local_path(cache)?;
        // Fresh installs have no roaming data folder; sweep writes its ledger here.
        fs::create_dir_all(data)
            .map_err(|e| format!("Cannot create artifact data directory: {e}"))?;
        no_links(data)?;
        fs::create_dir_all(cache).map_err(|e| e.to_string())?;
        let cache = fs::canonicalize(cache).map_err(|e| e.to_string())?;
        no_links(&cache)?;
        let ledger = data.join("artifacts.json");
        let entries = match fs::metadata(&ledger) {
            Ok(meta) if meta.len() <= 8 * 1024 * 1024 => {
                no_links(&ledger)?;
                serde_json::from_slice(&fs::read(&ledger).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("Artifact ledger needs manual review: {e}"))?
            }
            Ok(_) => return Err("Artifact ledger exceeds its 8 MiB limit.".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error.to_string()),
        };
        Ok(Arc::new(Self {
            ledger,
            cache,
            entries: Mutex::new(entries),
        }))
    }
    pub fn workspace(&self) -> PathBuf {
        self.cache.join("subtitle-work")
    }
    fn allowed(&self, path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let private_output = name.starts_with('.')
            && (name.ends_with(".part.mp4")
                || name.starts_with(".keroro_access_test-")
                || name.starts_with(".keroro-write-check-"));
        let subtitle = path.starts_with(self.workspace()) && name.starts_with("subtitle-");
        let preview = path.starts_with(self.cache.join("previews")) && name == "preview_temp.mp4";
        private_output || name.ends_with(".mp4.part") || subtitle || preview
    }
    pub fn record(&self, path: &Path) -> Result<(), String> {
        let parent = fs::canonicalize(path.parent().ok_or("Artifact has no parent.")?)
            .map_err(|e| e.to_string())?;
        let path = parent.join(path.file_name().ok_or("Artifact has no filename.")?);
        if !self.allowed(&path) {
            return Err("Refusing to journal a non-private artifact.".into());
        }
        let artifact = Artifact {
            identity: identity(&path)?,
            path,
        };
        let mut entries = self.entries.lock().map_err(|e| e.to_string())?;
        entries.retain(|entry| entry.path != artifact.path);
        entries.push(artifact);
        crate::atomic_file::json(&self.ledger, &*entries)
    }
    pub fn forget(&self, path: &Path) -> Result<(), String> {
        let mut entries = self.entries.lock().map_err(|e| e.to_string())?;
        entries.retain(|entry| entry.path != path);
        crate::atomic_file::json(&self.ledger, &*entries)
    }
    pub fn sweep(&self) -> Result<Vec<String>, String> {
        let mut entries = self.entries.lock().map_err(|e| e.to_string())?;
        let mut retained = Vec::new();
        let mut warnings = Vec::new();
        for entry in entries.iter() {
            match fs::symlink_metadata(&entry.path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    warnings.push(format!(
                        "Cannot inspect artifact {}: {error}",
                        entry.path.display()
                    ));
                    retained.push(entry.clone());
                    continue;
                }
                Ok(_) => {}
            }
            if !self.allowed(&entry.path) || identity(&entry.path).as_ref() != Ok(&entry.identity) {
                warnings.push(format!(
                    "Preserved unrecognized/replaced artifact {}",
                    entry.path.display()
                ));
                retained.push(entry.clone());
                continue;
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                // Refuse cleanup while an old encoder or another process holds it open.
                match fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .share_mode(0)
                    .open(&entry.path)
                {
                    Ok(file) => drop(file),
                    Err(error) => {
                        warnings.push(format!(
                            "Owned artifact is still locked: {} ({error})",
                            entry.path.display()
                        ));
                        retained.push(entry.clone());
                        continue;
                    }
                }
            }
            if let Err(error) = fs::remove_file(&entry.path) {
                warnings.push(format!(
                    "Cannot remove owned artifact {}: {error}",
                    entry.path.display()
                ));
                retained.push(entry.clone());
                continue;
            }
            log::info!("Removed owned orphan {}", entry.path.display());
            if entry.path.starts_with(self.workspace())
                && let Some(parent) = entry.path.parent()
            {
                let _ = fs::remove_dir(parent);
            } // Empty only; never recursive.
        }
        *entries = retained;
        crate::atomic_file::json(&self.ledger, &*entries)?;
        Ok(warnings)
    }
}
pub fn install(store: Arc<ArtifactStore>) -> Result<(), String> {
    STORE
        .set(store)
        .map_err(|_| "Artifact store already configured.".into())
}
pub fn workspace() -> Option<PathBuf> {
    STORE.get().map(|store| store.workspace())
}
pub fn forget_sync(path: &Path) -> Result<(), String> {
    if let Some(store) = STORE.get() {
        store.forget(path)?;
    }
    Ok(())
}
pub fn record_sync(path: &Path) -> Result<(), String> {
    if let Some(store) = STORE.get() {
        store.record(path)?;
    }
    Ok(())
}
pub async fn record(path: &Path) -> Result<(), String> {
    let Some(store) = STORE.get().cloned() else {
        return Ok(());
    };
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || store.record(&path))
        .await
        .map_err(|e| e.to_string())?
}
pub async fn forget(path: &Path) -> Result<(), String> {
    let Some(store) = STORE.get().cloned() else {
        return Ok(());
    };
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || store.forget(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fresh_install_creates_ledger_directory_before_startup_sweep() {
        let root = tempfile::tempdir().expect("fixture");
        let data = root.path().join("roaming").join("app");
        let cache = root.path().join("local").join("app");
        assert!(!data.exists());
        let store = ArtifactStore::open(&data, &cache).expect("fresh store");
        assert!(store.sweep().expect("first startup sweep").is_empty());
        assert_eq!(
            fs::read(data.join("artifacts.json")).expect("ledger"),
            b"[]"
        );
        drop(store);
        let reopened = ArtifactStore::open(&data, &cache).expect("next launch");
        assert!(reopened.sweep().expect("next startup sweep").is_empty());
    }

    #[test]
    fn sweeper_removes_only_recorded_private_files_with_matching_identity() {
        let root = tempfile::tempdir().expect("fixture");
        let cache = root.path().join("cache");
        let store = ArtifactStore::open(root.path(), &cache).expect("store");
        let own = root.path().join(".episode.test.part.mp4");
        let user = root.path().join("preview_temp.mp4");
        fs::write(&own, "partial").expect("own");
        fs::write(&user, "user media").expect("user");
        store.record(&own).expect("journal");
        assert!(store.record(&user).is_err());
        assert!(store.sweep().expect("sweep").is_empty());
        assert!(!own.exists());
        assert!(user.exists());
    }
}
