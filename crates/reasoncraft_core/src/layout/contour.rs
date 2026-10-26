//! Subtree contours for compact packing.
//!
//! A [`Contour`] is the outline of a subtree in axis coordinates: for each
//! stretch of the main axis, the lowest and highest cross coordinate the
//! subtree occupies there. Segments are sorted along the main axis and do
//! not overlap; stretches with nothing in them have no segment, so a
//! neighbour can tuck into them.

/// `(main_lo, main_hi, cross_lo, cross_hi)`.
type Seg = (f32, f32, f32, f32);

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Contour {
    segs: Vec<Seg>,
}

impl Contour {
    /// The contour of one rect; empty when it has no extent along the main
    /// axis.
    pub(crate) fn rect(main_lo: f32, main_hi: f32, cross_lo: f32, cross_hi: f32) -> Self {
        let segs = if main_lo < main_hi {
            vec![(main_lo, main_hi, cross_lo, cross_hi)]
        } else {
            Vec::new()
        };
        Self { segs }
    }

    /// The lowest cross coordinate anywhere in the contour (0 when empty).
    pub(crate) fn cross_lo(&self) -> f32 {
        let lo = self.segs.iter().map(|s| s.2).fold(f32::INFINITY, f32::min);
        if lo.is_finite() { lo } else { 0.0 }
    }

    /// How far `right` must move along the cross axis to clear `left`: the
    /// largest `left.cross_hi - right.cross_lo` over segments that overlap
    /// on the main axis, or 0 when none do.
    pub(crate) fn separation(left: &Contour, right: &Contour) -> f32 {
        let (a, b) = (&left.segs, &right.segs);
        let (mut i, mut j) = (0, 0);
        let mut sep = f32::NEG_INFINITY;
        while i < a.len() && j < b.len() {
            if a[i].0 < b[j].1 && b[j].0 < a[i].1 {
                sep = sep.max(a[i].3 - b[j].2);
            }
            if a[i].1 <= b[j].1 {
                i += 1;
            } else {
                j += 1;
            }
        }
        if sep.is_finite() { sep } else { 0.0 }
    }

    /// Add `other`, moved `shift` along the cross axis, to this contour.
    pub(crate) fn merge(&mut self, other: &Contour, shift: f32) {
        let (a, b) = (&self.segs, &other.segs);
        let mut cuts: Vec<f32> = a.iter().chain(b).flat_map(|s| [s.0, s.1]).collect();
        cuts.sort_by(f32::total_cmp);
        cuts.dedup();
        let mut out: Vec<Seg> = Vec::with_capacity(cuts.len());
        let (mut i, mut j) = (0, 0);
        for w in cuts.windows(2) {
            let (lo, hi) = (w[0], w[1]);
            while i < a.len() && a[i].1 <= lo {
                i += 1;
            }
            while j < b.len() && b[j].1 <= lo {
                j += 1;
            }
            let from_a = a.get(i).filter(|s| s.0 < hi).map(|s| (s.2, s.3));
            let from_b = b
                .get(j)
                .filter(|s| s.0 < hi)
                .map(|s| (s.2 + shift, s.3 + shift));
            let (c_lo, c_hi) = match (from_a, from_b) {
                (Some(x), Some(y)) => (x.0.min(y.0), x.1.max(y.1)),
                (Some(x), None) | (None, Some(x)) => x,
                (None, None) => continue,
            };
            match out.last_mut() {
                Some(last) if last.1 == lo && last.2 == c_lo && last.3 == c_hi => last.1 = hi,
                _ => out.push((lo, hi, c_lo, c_hi)),
            }
        }
        self.segs = out;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separation_ignores_stretches_that_do_not_overlap() {
        // Narrow on top, wide below.
        let mut left = Contour::rect(0.0, 10.0, -5.0, 5.0);
        left.merge(&Contour::rect(20.0, 30.0, -50.0, 50.0), 0.0);
        // Shallow: only on top.
        let shallow = Contour::rect(0.0, 10.0, -5.0, 5.0);
        assert_eq!(Contour::separation(&left, &shallow), 10.0);
        // Deep: reaches the wide part.
        let mut deep = shallow.clone();
        deep.merge(&Contour::rect(20.0, 30.0, -8.0, 8.0), 0.0);
        assert_eq!(Contour::separation(&left, &deep), 58.0);
        // Nothing overlaps.
        assert_eq!(
            Contour::separation(&left, &Contour::rect(40.0, 50.0, 0.0, 1.0)),
            0.0
        );
    }

    #[test]
    fn merge_keeps_the_envelope_and_shifts_other() {
        let mut c = Contour::rect(0.0, 10.0, 0.0, 4.0);
        c.merge(&Contour::rect(5.0, 20.0, 0.0, 2.0), 10.0);
        assert_eq!(
            c.segs,
            vec![
                (0.0, 5.0, 0.0, 4.0),
                (5.0, 10.0, 0.0, 12.0),
                (10.0, 20.0, 10.0, 12.0),
            ]
        );
        assert_eq!(c.cross_lo(), 0.0);
        // Equal neighbours coalesce; gaps stay gaps.
        let mut d = Contour::rect(0.0, 5.0, 1.0, 2.0);
        d.merge(&Contour::rect(5.0, 8.0, 1.0, 2.0), 0.0);
        d.merge(&Contour::rect(9.0, 10.0, 1.0, 2.0), 0.0);
        assert_eq!(d.segs, vec![(0.0, 8.0, 1.0, 2.0), (9.0, 10.0, 1.0, 2.0)]);
    }
}
