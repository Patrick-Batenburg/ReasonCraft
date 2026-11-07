//! Where Windows looks for DLLs the app loads at run time.
//!
//! The renderers load some DLLs by bare name (wgpu's `dxcompiler.dll`,
//! `vulkan-1.dll`, `libEGL.dll`). Windows' default search order includes
//! the current folder, and Explorer starts the app with the opened file's
//! folder as the current folder: a DLL of that name planted next to a
//! downloaded map would run inside the app. [`system32_only`] limits the
//! search to System32 for the rest of the process; `build.rs` sets the same
//! for the DLLs the exe imports (`/DEPENDENTLOADFLAG`).

/// From now on, DLLs loaded by bare name come from System32 only. Call it
/// first thing in `main`, before any library loads one. Returns whether
/// Windows accepted the setting (always `true` off Windows).
#[cfg(windows)]
pub fn system32_only() -> bool {
    use windows_sys::Win32::System::LibraryLoader::{
        LOAD_LIBRARY_SEARCH_SYSTEM32, SetDefaultDllDirectories,
    };
    // SAFETY: takes a flag value only; no pointers or handles involved.
    unsafe { SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32) != 0 }
}

/// No search-order hazard to close off Windows.
#[cfg(not(windows))]
pub fn system32_only() -> bool {
    true
}
