//! ReasonCraft desktop app: window chrome, document tabs and commands.

// The docs are built with `--document-private-items` (see CI), where
// links from public items to private ones resolve.
#![allow(rustdoc::private_intra_doc_links)]

pub mod a11y;
pub mod actions;
pub mod app;
pub mod canvas;
pub mod cli;
pub mod commands;
pub mod dll_search;
pub mod doc_tab;
pub mod export;
pub mod file_ops;
pub mod help;
pub mod i18n;
pub mod jobs;
pub mod memo;
pub mod profile;
pub mod recovery;
pub mod scene;
pub mod settings;
pub mod shell;
pub mod spell;
pub mod startup;
pub mod theme;

pub use app::ReasonCraftApp;
