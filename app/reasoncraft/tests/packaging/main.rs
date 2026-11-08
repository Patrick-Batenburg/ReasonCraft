//! The packaging sources' checks, in their own test binary: the icon
//! assets, the Windows installer source (`packaging/windows/`), the Linux
//! package metadata (`packaging/linux/`) and the macOS `Info.plist`
//! template (`packaging/macos/`). Nothing here starts the app.

mod copyright;
mod icons;
mod linux;
mod macos;
mod windows;
