//! Opening the main window. wgpu comes first: where there is no usable GPU
//! driver it falls back to software rendering (WARP on Windows). OpenGL
//! (glow) is the second try. When neither opens a window, the user gets an
//! error dialog instead of a silent exit.

use std::cell::Cell;
use std::fmt::Display;

use eframe::Renderer;

use crate::i18n::{FluentArgs, t_lang, t_lang_args};

/// The renderers to try, in order.
pub const RENDERERS: [Renderer; 2] = [Renderer::Wgpu, Renderer::Glow];

/// Why the app stopped with an error.
#[derive(Debug, PartialEq, Eq)]
pub enum StartError<E> {
    /// No renderer opened a window: the last renderer's error. The user is
    /// told (see [`show_failure`]).
    NoWindow(E),
    /// The app was created (it ran, or failed to build) and then failed.
    AfterStart(E),
}

/// Run `run` with each renderer in turn until one succeeds. `run` sets its
/// `Cell` once the app exists; an error after that ends the tries, as a
/// second window would not help.
pub fn first_that_starts<E: Display>(
    renderers: &[Renderer],
    mut run: impl FnMut(Renderer, &Cell<bool>) -> Result<(), E>,
) -> Result<(), StartError<E>> {
    let mut last = None;
    for &renderer in renderers {
        let started = Cell::new(false);
        match run(renderer, &started) {
            Ok(()) => return Ok(()),
            Err(err) if started.get() => return Err(StartError::AfterStart(err)),
            Err(err) => {
                eprintln!("warning: the {renderer} renderer did not start: {err}");
                last = Some(err);
            }
        }
    }
    Err(StartError::NoWindow(
        last.expect("at least one renderer is tried"),
    ))
}

/// The title and text of the start-up error dialog in `lang`.
pub fn failure_message(lang: &str, details: &str) -> (String, String) {
    let mut args = FluentArgs::new();
    args.set("details", details);
    let body = format!(
        "{}\n\n{}",
        t_lang(lang, "startup.failed.body"),
        t_lang_args(lang, "startup.failed.details", &args)
    );
    (t_lang(lang, "startup.failed.title"), body)
}

/// Tell the user the window could not open, in the OS language (the
/// settings are not read yet).
pub fn show_failure(err: &dyn Display) {
    let lang = crate::i18n::os_ui_language(sys_locale::get_locale().as_deref());
    let (title, body) = failure_message(lang, &err.to_string());
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(title)
        .set_description(body)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake run: fails with `err` for the listed renderers, after
    /// starting the app when `started` says so.
    fn tries(fails: &[(Renderer, bool)]) -> (Result<(), StartError<String>>, Vec<Renderer>) {
        let mut tried = Vec::new();
        let outcome = first_that_starts(&RENDERERS, |renderer, started| {
            tried.push(renderer);
            match fails.iter().find(|(r, _)| *r == renderer) {
                Some(&(_, after_start)) => {
                    started.set(after_start);
                    Err(format!("{renderer} failed"))
                }
                None => {
                    started.set(true);
                    Ok(())
                }
            }
        });
        (outcome, tried)
    }

    #[test]
    fn wgpu_first_and_alone_when_it_starts() {
        assert_eq!(tries(&[]), (Ok(()), vec![Renderer::Wgpu]));
    }

    #[test]
    fn glow_when_wgpu_cannot_open_a_window() {
        let (outcome, tried) = tries(&[(Renderer::Wgpu, false)]);
        assert_eq!(outcome, Ok(()));
        assert_eq!(tried, RENDERERS);
    }

    #[test]
    fn the_last_error_when_nothing_starts() {
        let (outcome, tried) = tries(&[(Renderer::Wgpu, false), (Renderer::Glow, false)]);
        assert_eq!(outcome, Err(StartError::NoWindow("glow failed".into())));
        assert_eq!(tried, RENDERERS);
    }

    #[test]
    fn no_second_window_after_the_app_ran() {
        let (outcome, tried) = tries(&[(Renderer::Wgpu, true)]);
        // Not a window failure: no graphics dialog.
        assert_eq!(outcome, Err(StartError::AfterStart("wgpu failed".into())));
        assert_eq!(tried, [Renderer::Wgpu]);
    }

    #[test]
    fn failure_message_is_translated_and_has_the_details() {
        for lang in ["en", "nl"] {
            let (title, body) = failure_message(lang, "no adapter");
            assert_eq!(title, t_lang(lang, "startup.failed.title"));
            assert!(!title.contains("startup"), "{title}");
            assert!(body.starts_with(&t_lang(lang, "startup.failed.body")));
            assert!(body.contains("no adapter"), "{body}");
        }
        assert_ne!(failure_message("en", "x"), failure_message("nl", "x"));
    }
}
