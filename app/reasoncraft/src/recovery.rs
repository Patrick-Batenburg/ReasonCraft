//! Autosave and crash recovery: tabs with unsaved work are copied to a
//! per-user recovery folder while the app runs, and a start after a crash
//! offers to restore them.

mod disk;
mod folder;
mod orphans;
mod prompt;
mod session;
mod store;

pub use orphans::Orphans;
pub use prompt::RecoveryChoice;
pub(crate) use prompt::{defer_offer, offer, resolve};
pub use session::tick;
pub(crate) use session::{due_in, file_mtime, forget, shutdown};
pub use store::{PackageSnapshot, RecoveredDoc, RecoveryMeta, RecoveryStore, WriteFailure};

#[cfg(test)]
mod tests;
