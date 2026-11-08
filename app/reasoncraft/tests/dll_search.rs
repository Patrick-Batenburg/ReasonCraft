//! DLL planting: a DLL in the current folder must never load. Its
//! own test binary, as it changes the process's current folder and DLL
//! search order.
#![cfg(windows)]

use std::os::windows::ffi::OsStrExt as _;
use std::path::{Path, PathBuf};

use windows_sys::Win32::System::LibraryLoader::LoadLibraryW;

/// Whether `LoadLibraryW` finds a DLL by its bare `name`, as wgpu loads
/// `dxcompiler.dll`.
fn loads(name: &str) -> bool {
    let wide: Vec<u16> = std::ffi::OsStr::new(name)
        .encode_wide()
        .chain([0])
        .collect();
    // SAFETY: a NUL-terminated UTF-16 string that outlives the call. The
    // DLL is a copy of a System32 one, so loading it runs only its own code.
    !unsafe { LoadLibraryW(wide.as_ptr()) }.is_null()
}

/// A copy of a System32 DLL under a name found nowhere else, in `dir`.
fn plant(dir: &Path, name: &str) {
    let system32 = PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot is set"))
        .join("System32")
        .join("version.dll");
    std::fs::copy(system32, dir.join(name)).expect("a writable temp folder");
}

#[test]
fn a_dll_in_the_current_folder_is_not_loaded() {
    let dir = tempfile::tempdir().expect("a temp folder");
    std::env::set_current_dir(dir.path()).expect("the temp folder can be entered");
    // The hazard is real: without the setting, the current folder is searched.
    plant(dir.path(), "rc_planted_before.dll");
    assert!(loads("rc_planted_before.dll"), "the probe itself works");

    assert!(reasoncraft::dll_search::system32_only());
    plant(dir.path(), "rc_planted_after.dll");
    assert!(
        !loads("rc_planted_after.dll"),
        "a DLL from the current folder was loaded"
    );
    assert!(loads("version.dll"), "System32 DLLs still load");
    std::env::set_current_dir(std::env::temp_dir()).expect("leave the temp folder");
}

/// The exe's load configuration: `DependentLoadFlags`, which the loader
/// uses for the DLLs the exe imports, read from the PE headers.
fn dependent_load_flags(exe: &Path) -> u16 {
    let b = std::fs::read(exe).expect("the built exe");
    let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
    let pe = u32_at(0x3C) as usize;
    assert_eq!(&b[pe..pe + 4], b"PE\0\0", "a PE file");
    let sections = usize::from(u16_at(pe + 6));
    let optional = pe + 24;
    let optional_size = usize::from(u16_at(pe + 20));
    let (directories, flags_at) = match u16_at(optional) {
        0x10B => (optional + 96, 0x36), // PE32: IMAGE_LOAD_CONFIG_DIRECTORY32
        0x20B => (optional + 112, 0x4E), // PE32+: IMAGE_LOAD_CONFIG_DIRECTORY64
        magic => panic!("unknown optional header {magic:#x}"),
    };
    // Data directory 10 is the load configuration.
    let rva = u32_at(directories + 10 * 8) as usize;
    assert_ne!(rva, 0, "the exe has a load configuration");
    let table = optional + optional_size;
    let offset = (0..sections)
        .map(|i| table + 40 * i)
        .find_map(|s| {
            let (size, va) = (u32_at(s + 8) as usize, u32_at(s + 12) as usize);
            let (raw_size, raw) = (u32_at(s + 16) as usize, u32_at(s + 20) as usize);
            (rva >= va && rva < va + size.max(raw_size)).then(|| raw + rva - va)
        })
        .expect("the load configuration lies in a section");
    assert!(
        u32_at(offset) as usize >= flags_at + 2,
        "the load configuration holds DependentLoadFlags"
    );
    u16_at(offset + flags_at)
}

#[test]
fn the_exe_loads_its_own_dlls_from_system32_only() {
    // LOAD_LIBRARY_SEARCH_SYSTEM32, set by build.rs (MSVC only).
    if cfg!(target_env = "msvc") {
        let flags = dependent_load_flags(Path::new(env!("CARGO_BIN_EXE_reasoncraft")));
        assert_eq!(flags & 0x800, 0x800, "DependentLoadFlags is {flags:#x}");
    }
}
