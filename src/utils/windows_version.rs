//! Windows version detection utilities.
//!
//! Uses `RtlGetVersion` from `ntdll.dll`, which is available from Windows 2000
//! onward and is more reliable than registry reads for determining the true
//! operating-system version.

/// Returns `true` if the host Windows version is Windows 10 or later.
///
/// On non-Windows platforms this always returns `true` so that callers can
/// unconditionally enable modern code paths.
#[cfg(windows)]
pub fn is_windows_10_or_later() -> bool {
    #[repr(C)]
    #[allow(clippy::upper_case_acronyms)]
    struct OSVERSIONINFOW {
        dw_os_version_info_size: u32,
        dw_major_version: u32,
        dw_minor_version: u32,
        dw_build_number: u32,
        dw_platform_id: u32,
        sz_csd_version: [u16; 128],
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn RtlGetVersion(version_information: *mut OSVERSIONINFOW) -> i32;
    }

    unsafe {
        let mut info: OSVERSIONINFOW = std::mem::zeroed();
        info.dw_os_version_info_size = std::mem::size_of::<OSVERSIONINFOW>() as u32;
        if RtlGetVersion(&mut info) == 0 {
            info.dw_major_version >= 10
        } else {
            tracing::warn!("RtlGetVersion failed, defaulting to pre-Windows 10 behavior");
            false
        }
    }
}

#[cfg(not(windows))]
pub fn is_windows_10_or_later() -> bool {
    true
}
