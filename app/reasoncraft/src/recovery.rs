//! Autosave and crash recovery: tabs with unsaved work are copied to a
//! per-user recovery folder while the app runs, and a start after a crash
//! offers to restore them.

mod prompt;
mod session;
mod store;

pub use prompt::RecoveryChoice;
pub(crate) use prompt::{offer, resolve};
pub use session::tick;
pub(crate) use session::{due_in, forget, original_changed, shutdown};
pub use store::{PackageSnapshot, RecoveredDoc, RecoveryMeta, RecoveryStore};

#[cfg(test)]
mod tests;
