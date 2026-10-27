//! Ribbon groups that shrink to fit the window: when a tab's row is wider
//! than the window, groups from the right first drop their small buttons'
//! labels (compact), then fold into one drop-down button that opens the
//! whole group (collapsed). Room left over is given back from the left.
//! The row scrolls only when even collapsed groups do not fit.
//!
//! Each frame [`begin`] plans every group's level from the widths the
//! groups measured on earlier frames ([`record`]); [`super::group`] reads
//! its level with [`level`]. A group drawn as designed also estimates its
//! compact width from what its small buttons would give up
//! ([`add_saving`]). Widths are measured again when the interface language
//! or scale changes.
//!
//! Adapted from WordCraft (github.com/storytold/wordcraft), Apache-2.0.

use std::collections::HashMap;

use eframe::egui::{Context, Id};

/// The group as designed.
pub const FULL: u8 = 0;
/// Small buttons without their labels (the tooltips and names keep them).
pub const COMPACT: u8 = 1;
/// One drop-down button holding the whole group.
pub const COLLAPSED: u8 = 2;

/// Estimated width of a collapsed group before it is measured.
const COLLAPSED_W: f32 = 72.0;

/// A group's measured widths at each level (None: not measured yet).
pub type Widths = [Option<f32>; 3];

/// Each group's level (by position) so that the row fits `avail`: groups
/// shrink from the right, first all to compact, then to collapsed,
/// stopping as soon as the row fits. Then whatever room is left is given
/// back, from the left.
pub fn plan(groups: &[Widths], avail: f32) -> Vec<u8> {
    let full = |g: &Widths| g[0].unwrap_or(0.0);
    let width = |g: &Widths, level: u8| match level {
        FULL => full(g),
        COMPACT => g[1].unwrap_or(full(g)),
        _ => g[2].unwrap_or(COLLAPSED_W.min(full(g))),
    };
    let mut levels = vec![FULL; groups.len()];
    let mut total: f32 = groups.iter().map(full).sum();
    for target in [COMPACT, COLLAPSED] {
        for (g, level) in groups.iter().zip(levels.iter_mut()).rev() {
            if total <= avail {
                break;
            }
            total += width(g, target) - width(g, *level);
            *level = target;
        }
    }
    // A group that gains little when shrunk may have made the ones before
    // it shrink for nothing: give back whatever still fits.
    for (g, level) in groups.iter().zip(levels.iter_mut()) {
        while *level > FULL {
            let next = total + width(g, *level - 1) - width(g, *level);
            if !next.is_finite() || next > avail {
                break;
            }
            total = next;
            *level -= 1;
        }
    }
    levels
}

/// This frame's row: the tab, the plan by position and the titles it was
/// made for, and the groups drawn so far.
#[derive(Clone, Default)]
struct Frame {
    tab: String,
    titles: Vec<String>,
    levels: Vec<u8>,
    drawn: Vec<String>,
    /// A width changed this frame: plan again.
    changed: bool,
}

/// Widths measured so far, by (tab, position, title, level), and each
/// tab's group order.
#[derive(Clone, Default)]
struct Measured {
    /// The interface language and scale the widths were measured at.
    signature: String,
    widths: HashMap<(String, usize, String, u8), f32>,
    order: HashMap<String, Vec<String>>,
}

/// What the group being drawn is like.
#[derive(Clone, Default)]
struct Drawing {
    compact: bool,
    in_popup: bool,
    /// Per column of small buttons, the most a button would give up.
    saving: HashMap<Id, f32>,
}

fn frame_id() -> Id {
    Id::unique("ribbon_fit_frame")
}
fn measured_id() -> Id {
    Id::unique("ribbon_fit_measured")
}
fn drawing_id() -> Id {
    Id::unique("ribbon_fit_drawing")
}

/// Start drawing `tab`'s row, `avail` points wide: plan the groups' levels.
pub fn begin(ctx: &Context, tab: &str, avail: f32) {
    let sig = format!("{}:{}", crate::i18n::ui_language(), ctx.pixels_per_point());
    let (titles, groups) = ctx.data_mut(|d| {
        let m = d.get_temp_mut_or_default::<Measured>(measured_id());
        if m.signature != sig {
            // Another language or scale: the labels have new widths.
            *m = Measured {
                signature: sig,
                ..Default::default()
            };
        }
        let titles = m.order.get(tab).cloned().unwrap_or_default();
        let groups: Vec<Widths> = titles
            .iter()
            .enumerate()
            .map(|(i, title)| {
                [FULL, COMPACT, COLLAPSED].map(|l| {
                    m.widths
                        .get(&(tab.to_owned(), i, title.clone(), l))
                        .copied()
                })
            })
            .collect();
        (titles, groups)
    });
    let levels = if groups.iter().all(|g| g[0].is_some()) {
        plan(&groups, avail)
    } else {
        Vec::new()
    };
    let frame = Frame {
        tab: tab.to_owned(),
        titles,
        levels,
        ..Default::default()
    };
    ctx.data_mut(|d| d.insert_temp(frame_id(), frame));
}

/// The level the next group, `title`, draws at.
pub fn level(ctx: &Context, title: &str) -> u8 {
    ctx.data(|d| {
        let f = d.get_temp::<Frame>(frame_id())?;
        let i = f.drawn.len();
        // Planned for another group here: draw it as designed.
        if f.titles.get(i).map(String::as_str) != Some(title) {
            return None;
        }
        f.levels.get(i).copied()
    })
    .unwrap_or(FULL)
}

/// Group `title` drew at `level`, `width` points wide.
pub fn record(ctx: &Context, title: &str, level: u8, width: f32) {
    let saving: f32 = ctx.data(|d| {
        d.get_temp::<Drawing>(drawing_id())
            .map(|s| s.saving.values().sum())
            .unwrap_or(0.0)
    });
    ctx.data_mut(|d| {
        let f = d.get_temp_mut_or_default::<Frame>(frame_id());
        let i = f.drawn.len();
        f.drawn.push(title.to_owned());
        let tab = f.tab.clone();
        if !(width.is_finite() && width > 0.0) {
            return;
        }
        let m = d.get_temp_mut_or_default::<Measured>(measured_id());
        let key = (tab, i, title.to_owned(), level);
        let old = m.widths.insert(key.clone(), width);
        let mut changed = old.is_none_or(|w| (w - width).abs() > 0.5);
        // Drawn as designed: until it is drawn compact, its compact width
        // is what its small buttons would give up.
        if level == FULL && saving > 0.0 {
            let compact = (key.0, key.1, key.2, COMPACT);
            if let std::collections::hash_map::Entry::Vacant(e) = m.widths.entry(compact) {
                e.insert((width - saving).max(COLLAPSED_W));
                changed = true;
            }
        }
        if changed {
            d.get_temp_mut_or_default::<Frame>(frame_id()).changed = true;
        }
    });
}

/// The row is drawn: remember its group order for the next frame's plan.
pub fn end(ctx: &Context) {
    let changed = ctx.data_mut(|d| {
        let f = d.get_temp_mut_or_default::<Frame>(frame_id());
        let drawn = std::mem::take(&mut f.drawn);
        let (tab, changed) = (f.tab.clone(), f.changed);
        let m = d.get_temp_mut_or_default::<Measured>(measured_id());
        let reordered = m.order.get(&tab) != Some(&drawn);
        if reordered {
            m.order.insert(tab, drawn);
        }
        changed || reordered
    });
    if changed {
        // New widths or another order: plan again with them next frame.
        ctx.request_repaint();
    }
}

/// Whether the group being drawn is compact (small buttons drop their labels).
pub fn compact(ctx: &Context) -> bool {
    ctx.data(|d| d.get_temp::<Drawing>(drawing_id()))
        .is_some_and(|s| s.compact)
}

/// Start drawing a group, compact or not.
pub fn start_group(ctx: &Context, compact: bool) {
    let drawing = Drawing {
        compact,
        ..Default::default()
    };
    ctx.data_mut(|d| d.insert_temp(drawing_id(), drawing));
}

/// Whether the controls being drawn are in a collapsed group's drop-down.
pub fn in_popup(ctx: &Context) -> bool {
    ctx.data(|d| d.get_temp::<Drawing>(drawing_id()))
        .is_some_and(|s| s.in_popup)
}

/// Mark the controls drawn from now on as in a drop-down, or not.
pub fn set_in_popup(ctx: &Context, on: bool) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Drawing>(drawing_id()).in_popup = on);
}

/// A small button in `column`, drawn as designed, would be `w` points
/// narrower in a compact group.
pub fn add_saving(ctx: &Context, column: Id, w: f32) {
    if !(w.is_finite() && w > 0.0) {
        return;
    }
    ctx.data_mut(|d| {
        let s = d.get_temp_mut_or_default::<Drawing>(drawing_id());
        if !s.compact && !s.in_popup {
            let most = s.saving.entry(column).or_insert(0.0);
            *most = most.max(w);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(full: f32, compact: Option<f32>, collapsed: Option<f32>) -> Widths {
        [Some(full), compact, collapsed]
    }

    #[test]
    fn everything_full_when_it_fits() {
        let gs = [g(100.0, Some(80.0), Some(60.0)); 2];
        assert_eq!(plan(&gs, 200.0), vec![FULL, FULL]);
    }

    #[test]
    fn rightmost_groups_compact_first_then_collapse() {
        let gs = [g(100.0, Some(80.0), Some(60.0)); 3];
        // 300 → C compact (280) fits 285.
        assert_eq!(plan(&gs, 285.0), vec![FULL, FULL, COMPACT]);
        // All compact is 240; then C collapses (220) to fit 230.
        assert_eq!(plan(&gs, 230.0), vec![COMPACT, COMPACT, COLLAPSED]);
        // Nothing fits: everything collapsed (the row scrolls).
        assert!(plan(&gs, 10.0).iter().all(|l| *l == COLLAPSED));
    }

    #[test]
    fn groups_that_still_fit_are_given_back() {
        // A wide group in the middle; groups of large buttons don't shrink.
        let gs = [
            g(300.0, Some(250.0), Some(60.0)),
            g(500.0, Some(500.0), Some(60.0)),
            g(60.0, Some(60.0), Some(60.0)),
            g(60.0, Some(60.0), Some(60.0)),
        ];
        assert_eq!(plan(&gs, 800.0), vec![FULL, COLLAPSED, FULL, FULL]);
    }

    #[test]
    fn unmeasured_widths_are_estimated() {
        let gs = [g(200.0, None, None), g(200.0, None, None)];
        // Compact unknown counts as full, so B collapses (estimated) and A
        // fits as designed.
        assert_eq!(plan(&gs, 300.0), vec![FULL, COLLAPSED]);
        // Odd numbers don't panic.
        let odd = [
            g(f32::NAN, None, None),
            g(f32::INFINITY, Some(f32::NAN), None),
        ];
        let _ = plan(&odd, 100.0);
        let _ = plan(&odd, f32::NAN);
    }
}
