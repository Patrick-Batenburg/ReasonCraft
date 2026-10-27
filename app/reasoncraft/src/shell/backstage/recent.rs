//! The backstage Recent page: the maps last opened or saved.

use std::path::Path;

use eframe::egui::{self, RichText};

use super::choice::choice;
use crate::actions::Action;
use crate::app::{ReasonCraftApp, RibbonTab};
use crate::i18n::{FluentArgs, t, t_args};
use crate::shell::icons::Icon;

/// The file name and folder of `path`, as shown.
fn name_and_folder(path: &Path) -> (String, String) {
    let name = path.file_name().unwrap_or(path.as_os_str());
    let folder = path.parent().map(|p| p.display().to_string());
    (
        name.to_string_lossy().into_owned(),
        folder.unwrap_or_default(),
    )
}

/// The most characters of a folder shown under a file name.
const FOLDER_CHARS: usize = 56;

/// `text` cut to at most `max` characters by leaving out its middle
/// ("C:\Users\Ann\…\School\Maps"): the drive and the last folders
/// say the most.
fn elide_middle(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_owned();
    }
    let head = (max - 1) / 3;
    let tail = max - 1 - head;
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(&chars[chars.len() - tail..]);
    out
}

/// The page. True when a file was chosen: the map shows again.
pub(super) fn page(ui: &mut egui::Ui, app: &mut ReasonCraftApp) -> bool {
    let tk = app.tokens;
    ui.spacing_mut().item_spacing.y = 4.0;
    ui.add_space(8.0);
    let title = RichText::new(t("backstage.recent.title"));
    ui.label(title.strong().size(18.0).color(tk.box_text));
    if app.settings.recent_files.is_empty() {
        ui.label(RichText::new(t("backstage.recent.none")).color(tk.text_muted));
        return false;
    }
    let mut chosen = None;
    for path in &app.settings.recent_files {
        let (name, folder) = name_and_folder(path);
        // The folder is part of the name: two maps may share a file name.
        let mut args = FluentArgs::new();
        args.set("name", name.as_str());
        args.set("folder", folder.as_str());
        let spoken = t_args("backstage.recent.item", &args);
        let shown = elide_middle(&folder, FOLDER_CHARS);
        let resp = choice(ui, &tk, Icon::Recent, &name, &shown, &spoken);
        if resp.on_hover_text(path.display().to_string()).clicked() {
            chosen = Some(path.clone());
        }
    }
    let Some(path) = chosen else {
        return false;
    };
    app.ribbon_tab = RibbonTab::Home;
    app.push(Action::OpenRecent(path));
    true
}

#[cfg(test)]
mod tests {
    use super::elide_middle;

    #[test]
    fn long_folders_lose_their_middle() {
        assert_eq!(elide_middle("C:/maps", 56), "C:/maps");
        let long = format!("C:/Users/{}/School", "x".repeat(80));
        let short = elide_middle(&long, 56);
        assert_eq!(short.chars().count(), 56);
        assert!(short.starts_with("C:/Users") && short.ends_with("/School"));
        assert!(short.contains('…'));
    }
}
