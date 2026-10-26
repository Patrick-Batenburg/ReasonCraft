//! ReasonCraft desktop app: window chrome, document tabs and commands.

pub mod actions;
pub mod app;
pub mod canvas;
pub mod commands;
pub mod doc_tab;
pub mod export;
pub mod file_ops;
pub mod i18n;
pub mod jobs;
pub mod recovery;
pub mod scene;
pub mod settings;
pub mod shell;
pub mod spell;
pub mod theme;

pub use app::ReasonCraftApp;
