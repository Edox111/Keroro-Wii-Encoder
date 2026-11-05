//! Open only a local directory, never a URL, document, or command string.
use std::path::{Path, PathBuf};
fn validated_directory(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("An absolute output directory is required.".into());
    }
    keroro_encoder_core::offline::local_path(path)?;
    let directory =
        std::fs::canonicalize(path).map_err(|e| format!("Cannot locate output directory: {e}"))?;
    keroro_encoder_core::offline::local_path(&directory)?;
    if !directory.is_dir() {
        return Err("Output path must be an existing directory.".into());
    }
    Ok(directory)
}
#[cfg(windows)]
fn open_directory(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        System::Com::{
            COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
        },
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };
    // SAFETY: no pointers are retained; this worker owns and balances successful COM initialization.
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    let initialized = unsafe {
        CoInitializeEx(
            std::ptr::null(),
            COINIT_APARTMENTTHREADED as u32 | COINIT_DISABLE_OLE1DDE as u32,
        )
    };
    if initialized < 0 {
        return Err(format!(
            "Cannot initialize the Windows folder opener ({initialized:#x})."
        ));
    }
    let _apartment = Apartment;
    let display = path.to_string_lossy();
    let display = display.strip_prefix(r"\\?\").unwrap_or(&display);
    let filename: Vec<u16> = std::ffi::OsStr::new(display)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let verb: Vec<u16> = "explore".encode_utf16().chain(Some(0)).collect();
    // SAFETY: owned NUL-terminated buffers outlive the synchronous API call.
    // No executable or parameters are supplied: the validated folder is opened as a folder.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            filename.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if result <= 32 {
        return Err(format!(
            "Windows could not open the output folder (code {result})."
        ));
    }
    Ok(())
}
#[cfg(not(windows))]
fn open_directory(path: &Path) -> Result<(), String> {
    let command = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let status = std::process::Command::new(command)
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("The system folder opener failed.".into());
    }
    Ok(())
}
#[tauri::command]
pub(super) async fn open_output_folder(path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || open_directory(&validated_directory(Path::new(&path))?))
        .await
        .map_err(|e| format!("Folder opener worker failed: {e}"))?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folder_validation_rejects_relative_paths_urls_and_regular_files() {
        assert!(validated_directory(Path::new("relative/folder")).is_err());
        assert!(validated_directory(Path::new("https://example.com")).is_err());
        assert!(validated_directory(&std::env::current_exe().expect("test executable")).is_err());
        let directory = validated_directory(&std::env::temp_dir()).expect("local temp directory");
        assert!(directory.is_absolute() && directory.is_dir());
    }
}
