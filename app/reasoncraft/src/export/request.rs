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
    EssayText,
    EssayDocx,
    /// The map as Argdown text (argdown.org).
    Argdown,
}

impl ExportFormat {
    /// The file extension, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
            Self::Pdf => "pdf",
            Self::OutlineMarkdown | Self::EssayMarkdown => "md",
            Self::OutlineText | Self::EssayText => "txt",
            Self::OutlineDocx | Self::EssayDocx => "docx",
            Self::Argdown => "argdown",
        }
    }

    /// Whether this is a picture of the map (with scope and theme).
    pub fn is_image(self) -> bool {
        matches!(self.kind(), FormatKind::Image(_))
    }

    /// What this format is made from: the map's picture, or the document.
    pub fn kind(self) -> FormatKind {
        use FormatKind::{Image, Text};
        match self {
            Self::Png => Image(ImageFormat::Png),
            Self::Svg => Image(ImageFormat::Svg),
            Self::Pdf => Image(ImageFormat::Pdf),
            Self::OutlineMarkdown => Text(TextFormat::OutlineMarkdown),
            Self::OutlineText => Text(TextFormat::OutlineText),
            Self::OutlineDocx => Text(TextFormat::OutlineDocx),
            Self::EssayMarkdown => Text(TextFormat::EssayMarkdown),
            Self::EssayText => Text(TextFormat::EssayText),
            Self::EssayDocx => Text(TextFormat::EssayDocx),
            Self::Argdown => Text(TextFormat::Argdown),
        }
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
            Self::EssayText => "export.format.essay_text",
            Self::EssayDocx => "export.format.essay_docx",
            Self::Argdown => "export.format.argdown",
        }
    }
}

/// An [`ExportFormat`] by what it is made from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatKind {
    Image(ImageFormat),
    Text(TextFormat),
}

/// The formats made from a picture of the map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Svg,
    Pdf,
}

/// The formats made from the document's text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFormat {
    OutlineMarkdown,
    OutlineText,
    OutlineDocx,
    EssayMarkdown,
    EssayText,
    EssayDocx,
    Argdown,
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
    #[serde(deserialize_with = "crate::settings::deserialize_or_default")]
    pub scope: ExportScope,
    #[serde(deserialize_with = "crate::settings::deserialize_or_default")]
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
