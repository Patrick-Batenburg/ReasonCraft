//! Which way a map grows from its root (spec §5, per document).

use emath::Vec2;
use serde::{Deserialize, Serialize};

use crate::edit::EditError;
use crate::model::Document;

/// The direction children are laid out in, relative to their parent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutDirection {
    /// Children below their parent.
    #[default]
    TopDown,
    /// Children above their parent.
    BottomUp,
    /// Children to the right of their parent.
    LeftRight,
}

impl LayoutDirection {
    /// Every direction, in menu order.
    pub const ALL: [Self; 3] = [Self::TopDown, Self::BottomUp, Self::LeftRight];

    /// The direction's name in command ids and the file.
    pub fn key(self) -> &'static str {
        match self {
            Self::TopDown => "top_down",
            Self::BottomUp => "bottom_up",
            Self::LeftRight => "left_right",
        }
    }

    /// The unit vector (y down) from a child toward its parent.
    pub fn toward_parent(self) -> Vec2 {
        match self {
            Self::TopDown => Vec2::new(0.0, -1.0),
            Self::BottomUp => Vec2::new(0.0, 1.0),
            Self::LeftRight => Vec2::new(-1.0, 0.0),
        }
    }
}

/// Whether `v` is its type's default (such fields are left out of the file).
pub(crate) fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

impl Document {
    /// Lay the map out in direction `d`. Setting the current direction is
    /// refused with [`EditError::NotAllowed`], so it records no history.
    pub fn set_layout_direction(&mut self, d: LayoutDirection) -> Result<(), EditError> {
        if self.layout_direction == d {
            return Err(EditError::NotAllowed);
        }
        self.layout_direction = d;
        self.touch();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{open, save};

    #[test]
    fn direction_round_trips_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("map.rcraft");

        // An older file has no `layout_direction`: it opens top-down.
        let mut v = serde_json::to_value(Document::new("C")).unwrap();
        v.as_object_mut().unwrap().remove("layout_direction");
        let old: Document = serde_json::from_value(v).unwrap();
        assert_eq!(old.layout_direction, LayoutDirection::TopDown);

        // Top-down is not written.
        let json = serde_json::to_value(&old).unwrap();
        assert!(json.get("layout_direction").is_none(), "{json}");

        let mut d = Document::new("C");
        d.set_layout_direction(LayoutDirection::BottomUp).unwrap();
        let json = serde_json::to_value(&d).unwrap();
        assert_eq!(json["layout_direction"], "bottom_up");
        save(&d, &path).unwrap();
        assert_eq!(
            open(&path).unwrap().layout_direction,
            LayoutDirection::BottomUp
        );
    }

    #[test]
    fn setting_the_same_direction_is_refused() {
        let mut d = Document::new("C");
        d.meta.modified = "old".to_owned();
        let same = d.set_layout_direction(LayoutDirection::TopDown);
        assert_eq!(same, Err(EditError::NotAllowed));
        assert_eq!(d.meta.modified, "old");
        d.set_layout_direction(LayoutDirection::LeftRight).unwrap();
        assert_ne!(d.meta.modified, "old");
    }

    #[test]
    fn toward_parent_is_a_unit_vector_per_direction() {
        for d in LayoutDirection::ALL {
            assert_eq!(d.toward_parent().length(), 1.0, "{}", d.key());
        }
    }
}
