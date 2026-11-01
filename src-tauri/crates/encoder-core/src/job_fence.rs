/// Parent-owned Windows job: assigned media children are terminated by the OS
/// when the app is force-killed and its final job handle closes.
#[cfg(windows)]
pub fn assign(child: &tokio::process::Child) -> Result<(), String> {
    use std::{
        os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
        sync::OnceLock,
    };
    use windows_sys::Win32::System::JobObjects::*;
    static JOB: OnceLock<Result<OwnedHandle, String>> = OnceLock::new();
    let job = JOB
        .get_or_init(|| {
            // SAFETY: unnamed, non-inheritable job with default security attributes.
            let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if raw.is_null() {
                return Err(std::io::Error::last_os_error().to_string());
            }
            // SAFETY: unique valid owned job handle.
            let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            // SAFETY: correctly typed initialized structure, valid job handle.
            if unsafe {
                SetInformationJobObject(
                    raw,
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as *const _,
                    std::mem::size_of_val(&limits) as u32,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok(handle)
        })
        .as_ref()
        .map_err(|error| format!("Cannot create child-process safety job: {error}"))?;
    let raw_child = child
        .raw_handle()
        .ok_or("Child exited before job assignment.")?;
    // SAFETY: both handles remain owned/live. Assignment is before async processing.
    if unsafe { AssignProcessToJobObject(job.as_raw_handle(), raw_child) } == 0 {
        return Err(format!(
            "Cannot assign media child to safety job: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn assign(_: &tokio::process::Child) -> Result<(), String> {
    Ok(())
}
