#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

/// OS power-request ownership is not thread-affine; safe to move between workers.
pub struct WakeLock {
    #[cfg(windows)]
    handle: OwnedHandle,
}
impl WakeLock {
    #[cfg(windows)]
    pub fn acquire() -> Result<Self, String> {
        use windows_sys::Win32::{
            Foundation::INVALID_HANDLE_VALUE,
            System::{
                Power::{PowerCreateRequest, PowerRequestSystemRequired, PowerSetRequest},
                Threading::{REASON_CONTEXT, REASON_CONTEXT_0},
            },
        };
        let mut reason = "Keroro Wii Encoder is processing a media task."
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let context = REASON_CONTEXT {
            Version: 0,
            Flags: 1,
            Reason: REASON_CONTEXT_0 {
                SimpleReasonString: reason.as_mut_ptr(),
            },
        };
        // SAFETY: correctly tagged simple-string union and live NUL-terminated
        // UTF-16 buffer. PowerCreateRequest copies the supplied reason context.
        let raw = unsafe { PowerCreateRequest(&context) };
        if raw == INVALID_HANDLE_VALUE || raw.is_null() {
            return Err(format!(
                "Cannot create sleep-prevention request: {}",
                std::io::Error::last_os_error()
            ));
        }
        // SAFETY: newly created unique valid owned kernel handle, closed by RAII.
        let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
        // SAFETY: valid owned request handle; does not request a display lock.
        if unsafe { PowerSetRequest(handle.as_raw_handle(), PowerRequestSystemRequired) } == 0 {
            return Err(format!(
                "Cannot prevent idle sleep: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(Self { handle })
    }
    #[cfg(not(windows))]
    pub fn acquire() -> Result<Self, String> {
        Err("Native sleep prevention is implemented for Windows only.".into())
    }
}
#[cfg(windows)]
impl Drop for WakeLock {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Power::{PowerClearRequest, PowerRequestSystemRequired};
        // SAFETY: handle remains valid until OwnedHandle drops after this method.
        if unsafe { PowerClearRequest(self.handle.as_raw_handle(), PowerRequestSystemRequired) }
            == 0
        {
            log::error!(
                "Cannot clear idle-sleep request: {}",
                std::io::Error::last_os_error()
            );
        }
        // OwnedHandle then closes the power request even if clearing failed.
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};
    fn handles() -> u32 {
        let mut count = 0;
        // SAFETY: valid process pseudo-handle and writable u32 output.
        assert_ne!(
            unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) },
            0
        );
        count
    }
    fn exercise() {
        let lock = WakeLock::acquire().expect("native power request");
        std::thread::spawn(move || drop(lock))
            .join()
            .expect("drop on another thread");
    }
    #[test]
    #[ignore = "Process-wide handle measurement; run alone or with --test-threads=1"]
    fn power_request_moves_across_threads_without_leaking_handles() {
        exercise(); // Warm up OS/Rust per-process initialization.
        let before = handles();
        for _ in 0..10 {
            exercise();
        }
        let after = handles();
        assert!(
            after <= before,
            "Power-request handles grew from {before} to {after}"
        );
    }
}
