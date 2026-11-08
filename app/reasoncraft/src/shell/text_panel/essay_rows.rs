//! The rows of the Essay tab, top to bottom, each with what it draws and
//! the key its height is cached under.

use reasoncraft_core::essay::{EssayElement, EssaySection, EssaySlot};
use reasoncraft_core::{Document, Item, MapKind};

use crate::doc_tab::{EssayRow, EssayView};

/// One row of the tab, with what it draws.
pub(super) enum Part<'a> {
    Help,
    /// "A grouping map has no essay", in place of the help, the
    /// introduction, the sections and the conclusion.
    Grouping,
    End(EssaySlot),
    NoSections,
    /// Section `n` (from 1).
    Section(usize, &'a EssaySection),
    Element(&'a EssayElement),
    InactiveHeading,
    /// A parked item's label and paragraph.
    Inactive(Item, &'a str, &'a str),
    UnplacedHeading,
    /// An unplaced note's index, label and text.
    Unplaced(usize, &'a str, &'a str),
}

impl Part<'_> {
    pub(super) fn key(&self) -> EssayRow {
        match *self {
            Part::Help => EssayRow::Help,
            Part::Grouping => EssayRow::Grouping,
            Part::End(slot) => EssayRow::Slot(slot),
            Part::NoSections => EssayRow::NoSections,
            Part::Section(_, s) => EssayRow::Section(s.item),
            Part::Element(e) => EssayRow::Slot(e.slot),
            Part::InactiveHeading => EssayRow::InactiveHeading,
            Part::Inactive(item, ..) => EssayRow::Inactive(item),
            Part::UnplacedHeading => EssayRow::UnplacedHeading,
            Part::Unplaced(i, ..) => EssayRow::Unplaced(i),
        }
    }
}

/// The rows of the tab, top to bottom. A grouping map has no essay, but
/// text kept from its reasoning days stays listed below the note.
pub(super) fn parts<'a>(doc: &'a Document, view: &'a EssayView) -> Vec<Part<'a>> {
    let mut parts = Vec::new();
    if doc.map_kind() == MapKind::Grouping {
        parts.push(Part::Grouping);
    } else {
        parts.extend([Part::Help, Part::End(EssaySlot::Intro)]);
        if view.sections.is_empty() {
            parts.push(Part::NoSections);
        }
        for (n, section) in view.sections.iter().enumerate() {
            parts.push(Part::Section(n + 1, section));
            parts.extend(section.elements.iter().map(Part::Element));
        }
        parts.push(Part::End(EssaySlot::Conclusion));
    }
    let inactive: Vec<Part> = view
        .inactive
        .iter()
        .filter_map(|(item, label)| {
            let text = doc.essay.text(EssaySlot::Item(*item))?;
            Some(Part::Inactive(*item, label.as_str(), text))
        })
        .collect();
    if !inactive.is_empty() {
        parts.push(Part::InactiveHeading);
        parts.extend(inactive);
    }
    if !doc.essay.unplaced.is_empty() {
        parts.push(Part::UnplacedHeading);
        let notes = doc.essay.unplaced.iter().enumerate();
        parts.extend(notes.map(|(i, u)| Part::Unplaced(i, &u.label, &u.text)));
    }
    parts
}
