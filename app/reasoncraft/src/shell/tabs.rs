//! The row of open documents, with a "+" to start a new one.

use eframe::egui::text::{LayoutJob, TextWrapping};
use eframe::egui::{
    self, Button, CornerRadius, FontId, Frame, Margin, Popup, Role, Sense, Vec2, WidgetInfo,
};

use super::icons::Icon;
use super::{hover_fill, icon_button};
use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::i18n::{FluentArgs, t, t_args};

const HEIGHT: f32 = 30.0;
/// The most characters of a document's name a tab shows.
const TITLE_CHARS: usize = 24;
/// What follows the name of a document with unsaved changes.
const DIRTY_MARK: &str = " •";

/// `title` as a tab shows it: a name longer than [`TITLE_CHARS`] is cut
/// short with "…" (keeping the unsaved-changes mark).
pub fn short_title(title: &str) -> String {
    let (name, mark) = match title.strip_suffix(DIRTY_MARK) {
        Some(name) => (name, DIRTY_MARK),
        None => (title, ""),
    };
    match name.char_indices().nth(TITLE_CHARS) {
        Some((end, _)) => format!("{}…{mark}", name[..end].trim_end()),
        None => title.to_owned(),
    }
}

/// The narrowest a chip gets before tabs are left out of the row.
const MIN_CHIP: f32 = 88.0;
/// The widest a chip gets, and the room its close button and padding take.
const MAX_CHIP: f32 = 240.0;
const CHIP_PAD: f32 = 52.0;
/// The "+" and the menu of all tabs.
const BUTTON: f32 = 24.0;
const GAP: f32 = 2.0;

/// The chips the row shows and their widths, when it has `room` px.
#[derive(Debug, PartialEq)]
pub struct Row {
    /// The first tab shown; tabs `first..first + widths.len()` show.
    pub first: usize,
    pub widths: Vec<f32>,
    /// Whether some tabs are left out (the menu of all tabs then shows).
    pub overflow: bool,
}

/// Fit chips whose natural widths are `natural` into `room` px beside the
/// "+": at their natural widths if they fit; else narrower, the widest
/// first, down to [`MIN_CHIP`]; else as many as fit at that width (with
/// room for the menu button), around the `active` tab. `first` is the
/// first tab shown last frame, so the row does not jump as tabs change.
pub fn fit_row(natural: &[f32], active: usize, room: f32, first: usize) -> Row {
    let n = natural.len();
    let gaps = |k: usize| GAP * k as f32;
    let room_all = room - BUTTON - gaps(n);
    if let Some(widths) = shrink(natural, room_all) {
        return Row {
            first: 0,
            widths,
            overflow: false,
        };
    }
    let room_some = room - 2.0 * BUTTON - gaps(1);
    let k = (((room_some + GAP) / (MIN_CHIP + GAP)).floor() as usize).clamp(1, n);
    let mut first = first.min(n - k);
    if active < first {
        first = active;
    } else if active >= first + k {
        first = active + 1 - k;
    }
    let shown = &natural[first..first + k];
    let widths = shrink(shown, room_some - gaps(k)).unwrap_or_else(|| vec![MIN_CHIP; k]);
    Row {
        first,
        widths,
        overflow: true,
    }
}

/// `natural` capped by one width so the sum is at most `room`, never
/// below [`MIN_CHIP`]; `None` when even that is too wide.
fn shrink(natural: &[f32], room: f32) -> Option<Vec<f32>> {
    if natural.iter().sum::<f32>() <= room {
        return Some(natural.to_vec());
    }
    if MIN_CHIP * natural.len() as f32 > room {
        return None;
    }
    let mut sorted = natural.to_vec();
    sorted.sort_by(f32::total_cmp);
    let (mut rest, mut left) = (room, sorted.len());
    let mut cap = MIN_CHIP;
    for w in sorted {
        cap = rest / left as f32;
        if w > cap {
            break;
        }
        rest -= w;
        left -= 1;
    }
    Some(natural.iter().map(|w| w.min(cap).max(MIN_CHIP)).collect())
}

pub fn show(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    egui::Panel::top("doc_tabs")
        .frame(
            Frame::new()
                .fill(tk.bg)
                .inner_margin(Margin::symmetric(8, 0)),
        )
        .exact_size(HEIGHT)
        .show_separator_line(false)
        .show(ui, |ui| {
            let font = FontId::proportional(13.0);
            let natural: Vec<f32> = app
                .tabs
                .iter()
                .map(|tab| {
                    let shown = short_title(&tab.title());
                    let w = ui
                        .painter()
                        .layout_no_wrap(shown, font.clone(), tk.box_text);
                    (w.size().x + CHIP_PAD).clamp(120.0, MAX_CHIP)
                })
                .collect();
            let first_id = ui.scope_id().with("first_tab");
            let first = ui.data(|d| d.get_temp(first_id)).unwrap_or(0);
            let row = fit_row(&natural, app.active, ui.available_width(), first);
            ui.data_mut(|d| d.insert_temp(first_id, row.first));
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                for (k, width) in row.widths.iter().enumerate() {
                    tab_chip(ui, app, row.first + k, *width);
                }
                if row.overflow {
                    all_tabs_menu(ui, app);
                }
                if icon_button(ui, Icon::Plus, &t("tabs.new"), BUTTON, true, &tk, tk.bg).clicked() {
                    app.push(Action::New);
                }
            });
        });
}

/// The menu of every open tab, for a row too narrow to show them all;
/// the active tab is checked.
fn all_tabs_menu(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let resp = icon_button(ui, Icon::More, &t("tabs.more"), BUTTON, true, &tk, tk.bg);
    Popup::menu(&resp).show(|ui| {
        for i in 0..app.tabs.len() {
            let title = app.tabs[i].title();
            if ui.add(Button::selectable(i == app.active, title)).clicked() {
                app.push(Action::ActivateTab(i));
            }
        }
    });
}

fn tab_chip(ui: &mut egui::Ui, app: &mut ReasonCraftApp, i: usize, width: f32) {
    let tk = app.tokens;
    let title = app.tabs[i].title();
    let active = i == app.active;
    let font = FontId::proportional(13.0);
    let colour = if active { tk.box_text } else { tk.text_muted };
    let mut job = LayoutJob::simple_singleline(short_title(&title), font, colour);
    job.wrap = TextWrapping::truncate_at_width(width - CHIP_PAD + 12.0);
    let galley = ui.painter().layout_job(job);
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, HEIGHT - 4.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(Role::Tab, true, active, &title));
    if galley.elided || galley.text() != title {
        resp = resp.on_hover_text(&title);
    }

    let fill = if active {
        tk.ribbon
    } else if resp.hovered() {
        hover_fill(&tk, tk.bg)
    } else {
        tk.bg
    };
    let radius = CornerRadius {
        nw: 6,
        ne: 6,
        sw: 0,
        se: 0,
    };
    ui.painter().rect_filled(rect, radius, fill);
    let pos = rect.left_center() + Vec2::new(12.0, -galley.size().y / 2.0);
    ui.painter().galley(pos, galley, colour);

    // Close button on the right of the chip.
    let mut args = FluentArgs::new();
    args.set("name", title);
    let close_label = t_args("tabs.close", &args);
    let close_rect = egui::Rect::from_center_size(
        rect.right_center() - Vec2::new(16.0, 0.0),
        Vec2::splat(20.0),
    );
    let surface = fill;
    let close = ui.put(close_rect, |ui: &mut egui::Ui| {
        icon_button(ui, Icon::Close, &close_label, 20.0, true, &tk, surface)
    });

    if close.clicked() {
        app.push(Action::CloseTab(i));
    } else if resp.clicked() {
        app.push(Action::ActivateTab(i));
    }
}
