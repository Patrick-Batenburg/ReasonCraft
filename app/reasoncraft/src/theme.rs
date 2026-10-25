//! Colour tokens for the dark and light themes, and applying them to egui.

use eframe::egui::{self, Color32, Stroke, Theme, Visuals};
use serde::{Deserialize, Serialize};

/// The theme the user picked. `System` follows the OS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    System,
    Light,
    #[default]
    Dark,
}

/// Every colour the app uses. Nothing else in the app hard-codes a colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    /// Whether these are dark-theme tokens.
    pub dark: bool,
    /// Window background (title bar, tabs row).
    pub bg: Color32,
    /// Side panels, status bar, popups.
    pub panel: Color32,
    pub ribbon: Color32,
    pub canvas: Color32,
    pub canvas_dot: Color32,
    pub box_fill: Color32,
    /// Main text colour, on boxes and in the chrome.
    pub box_text: Color32,
    pub claim_border: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub reason: Color32,
    pub objection: Color32,
    pub rebuttal: Color32,
    /// Cards rated nil: frame, label, premise borders and connector.
    pub nil: Color32,
    pub attachment_line: Color32,
    /// Sticky-note fill and its text.
    pub note: Color32,
    pub note_text: Color32,
}

const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

impl Tokens {
    pub const fn dark() -> Self {
        Self {
            dark: true,
            bg: hex(0x141416),
            panel: hex(0x202024),
            ribbon: hex(0x232327),
            canvas: hex(0x161618),
            canvas_dot: hex(0x2C2C32),
            box_fill: hex(0x26262B),
            box_text: hex(0xE8E8EC),
            claim_border: hex(0xC8C8D0),
            text_muted: hex(0xA0A0AA),
            accent: hex(0x8AB4FF),
            reason: hex(0x4CC073),
            objection: hex(0xF07474),
            rebuttal: hex(0xE0A05A),
            nil: hex(0x8C8C96),
            attachment_line: hex(0x5C5C66),
            note: hex(0xD6C267),
            note_text: hex(0x26220F),
        }
    }

    pub const fn light() -> Self {
        Self {
            dark: false,
            bg: hex(0xE9E9EE),
            panel: hex(0xF7F7F9),
            ribbon: hex(0xFFFFFF),
            canvas: hex(0xFBFBFC),
            canvas_dot: hex(0xD4D4DB),
            box_fill: hex(0xFFFFFF),
            box_text: hex(0x1C1C20),
            claim_border: hex(0x3A3A42),
            text_muted: hex(0x56565F),
            accent: hex(0x1A5CC8),
            reason: hex(0x166A39),
            objection: hex(0xA82B24),
            rebuttal: hex(0x8E4B00),
            nil: hex(0x66666F),
            attachment_line: hex(0x9A9AA4),
            note: hex(0xFFF1A6),
            note_text: hex(0x3A3412),
        }
    }
}

/// The tokens for `choice`; `os_dark` decides for [`ThemeChoice::System`].
pub fn resolve(choice: ThemeChoice, os_dark: bool) -> Tokens {
    match choice {
        ThemeChoice::Dark => Tokens::dark(),
        ThemeChoice::Light => Tokens::light(),
        ThemeChoice::System if os_dark => Tokens::dark(),
        ThemeChoice::System => Tokens::light(),
    }
}

/// Make `t` the active egui theme.
pub fn apply(ctx: &egui::Context, t: &Tokens) {
    let (theme, mut v) = if t.dark {
        (Theme::Dark, Visuals::dark())
    } else {
        (Theme::Light, Visuals::light())
    };
    v.panel_fill = t.panel;
    v.window_fill = t.panel;
    v.extreme_bg_color = t.bg;
    v.faint_bg_color = t.ribbon;
    v.hyperlink_color = t.accent;
    v.weak_text_color = Some(t.text_muted);
    v.selection.bg_fill = t.accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v.widgets.noninteractive.fg_stroke.color = t.box_text;
    v.widgets.inactive.fg_stroke.color = t.box_text;
    v.widgets.hovered.fg_stroke.color = t.box_text;
    v.widgets.active.fg_stroke.color = t.box_text;
    v.widgets.noninteractive.bg_stroke.color = t.attachment_line.gamma_multiply(0.6);
    ctx.set_visuals_of(theme, v);
    ctx.set_theme(theme);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: Color32) -> f32 {
        let lin = |v: u8| {
            let s = f32::from(v) / 255.0;
            if s <= 0.039_28 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    /// Text needs 4.5:1 (WCAG AA); role colours are mostly borders and
    /// connectors, so they need the 3:1 for non-text UI.
    #[test]
    fn colours_meet_wcag_aa_in_both_themes() {
        for t in [Tokens::dark(), Tokens::light()] {
            for bg in [t.bg, t.panel, t.ribbon, t.box_fill, t.canvas] {
                let text = [t.box_text, t.text_muted, t.accent].map(|c| (c, 4.5));
                let roles = [t.reason, t.objection, t.rebuttal].map(|c| (c, 3.0));
                for (fg, min) in text.into_iter().chain(roles) {
                    let c = contrast(fg, bg);
                    assert!(c >= min, "{fg:?} on {bg:?}: {c:.2} (dark = {})", t.dark);
                }
            }
        }
        for t in [Tokens::dark(), Tokens::light()] {
            assert!(
                contrast(t.note_text, t.note) >= 4.5,
                "note (dark = {})",
                t.dark
            );
        }
        // Light-theme role colours are also used for label text.
        let t = Tokens::light();
        for fg in [t.reason, t.objection, t.rebuttal] {
            assert!(contrast(fg, t.box_fill) >= 4.5, "{fg:?}");
        }
    }

    /// `fg` at `alpha` over `bg`, blended in gamma space as egui does.
    fn over(bg: Color32, fg: Color32, alpha: f32) -> Color32 {
        let mix =
            |b: u8, f: u8| (f32::from(b) * (1.0 - alpha) + f32::from(f) * alpha).round() as u8;
        Color32::from_rgb(
            mix(bg.r(), fg.r()),
            mix(bg.g(), fg.g()),
            mix(bg.b(), fg.b()),
        )
    }

    /// The rating label is text in the card colour on the card's own tint:
    /// the strongest tint (strong cards) still leaves 4.5:1.
    #[test]
    fn label_contrast_on_tinted_frames() {
        use crate::canvas::rating::{FRAME_ALPHA, STRONG_ALPHA};
        for t in [Tokens::dark(), Tokens::light()] {
            let labels = [
                (t.reason, STRONG_ALPHA),
                (t.objection, STRONG_ALPHA),
                (t.rebuttal, STRONG_ALPHA),
                // Nil cards are tinted less; FRAME_ALPHA covers shading off.
                (t.nil, FRAME_ALPHA),
            ];
            for (fg, alpha) in labels {
                let bg = over(t.canvas, fg, alpha);
                let c = contrast(fg, bg);
                assert!(c >= 4.5, "{fg:?} on its tint: {c:.2} (dark = {})", t.dark);
            }
            for bg in [t.bg, t.panel, t.ribbon, t.box_fill, t.canvas] {
                assert!(
                    contrast(t.nil, bg) >= 3.0,
                    "nil on {bg:?} (dark = {})",
                    t.dark
                );
            }
        }
    }

    #[test]
    fn resolve_follows_os_only_for_system() {
        assert!(resolve(ThemeChoice::System, true).dark);
        assert!(!resolve(ThemeChoice::System, false).dark);
        assert!(resolve(ThemeChoice::Dark, false).dark);
        assert!(!resolve(ThemeChoice::Light, true).dark);
    }
}
