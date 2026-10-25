//! What the user asks to export: the format, and the options the export
//! dialog remembers between exports.

use serde::{Deserialize, Serialize};

use super::PageSetup;

/// A kind of file (or clipboard content) a map can be exported as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExportFormat {
    Png,
    Svg,
    Pdf,
    OutlineMarkdown,
    OutlineText,
    OutlineDocx,
    EssayMarkdown,
    EssayDocx,
}

impl ExportFormat {
    /// The file extension, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
            Self::Pdf => "pdf",
            Self::OutlineMarkdown | Self::EssayMarkdown => "md",
            Self::OutlineText => "txt",
            Self::OutlineDocx | Self::EssayDocx => "docx",
        }
    }

    /// Whether this is a picture of the map (with scope and theme).
    pub fn is_image(self) -> bool {
        matches!(self, Self::Png | Self::Svg | Self::Pdf)
    }

    /// Whether this build can write the format yet (Word and the essay
    /// come in a later step).
    pub fn is_available(self) -> bool {
        !matches!(
            self,
            Self::OutlineDocx | Self::EssayMarkdown | Self::EssayDocx
        )
    }

    /// The i18n key of the format's name (also the file dialog's filter).
    pub fn name_key(self) -> &'static str {
        match self {
            Self::Png => "export.format.png",
            Self::Svg => "export.format.svg",
            Self::Pdf => "export.format.pdf",
            Self::OutlineMarkdown => "export.format.outline_md",
            Self::OutlineText => "export.format.outline_txt",
            Self::OutlineDocx => "export.format.outline_docx",
            Self::EssayMarkdown => "export.format.essay_md",
            Self::EssayDocx => "export.format.essay_docx",
        }
    }
}

/// How much of the map an image export shows.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExportScope {
    /// The whole map, without the parking area.
    #[default]
    Map,
    /// The selected item and everything under it.
    Selection,
}

/// The colours an image export uses.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExportTheme {
    /// The light theme, whatever the app shows (prints well).
    #[default]
    Light,
    /// The theme the app shows.
    Current,
}

/// The export dialog's choices, remembered in the settings.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ExportOptions {
    pub scope: ExportScope,
    pub theme: ExportTheme,
    /// PNG pixels per world px: 1 to 4.
    pub png_scale: u8,
    /// PNG: leave the background out.
    pub transparent: bool,
    /// SVG: turn text into outlines, so no font is needed to view it.
    pub svg_outlines: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            scope: ExportScope::Map,
            theme: ExportTheme::Light,
            png_scale: 2,
            transparent: false,
            svg_outlines: true,
        }
    }
}

impl ExportOptions {
    /// The PNG scale, kept within 1 to 4 (old or edited settings).
    pub fn scale(&self) -> f32 {
        f32::from(self.png_scale.clamp(1, 4))
    }
}

/// One export to run: the format, the dialog's options and (for PDF) the
/// page setup.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExportRequest {
    pub format: ExportFormat,
    pub options: ExportOptions,
    pub page: PageSetup,
}
