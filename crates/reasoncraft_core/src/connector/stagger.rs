//! Where elbow connectors turn. Connectors whose runs across would lie on
//! one line (their children's near edges and their stub starts share main
//! coordinates, as siblings under one parent do) turn at staggered
//! levels, so no two runs overlap.

use std::collections::BTreeMap;

use emath::Pos2;

use crate::model::LayoutDirection;

/// The distance between staggered runs, in world units (less when the gap
/// between child and parent is too small for all of them).
pub const ELBOW_SPACING: f32 = 6.0;

/// Below this cross offset (world units) an elbow is straight: no run.
const NO_RUN: f32 = 0.5;

/// The main coordinate (world y top-down and bottom-up, x left-to-right)
/// each elbow connector turns at, for connectors given as (child edge
/// `from`, parent port `to`, stub length). Alone, a connector turns halfway
/// between `from` and its stub start. In a band with overlapping runs, each
/// connector gets a level, nearest the parent first, so that no leg crosses
/// a run: a run turns nearer the parent than any connector whose child lies
/// under it, and nearer the child than any whose port lies over it. Only
/// runs with no such order between them share a level.
pub fn elbow_turns(ends: &[(Pos2, Pos2, f32)], dir: LayoutDirection) -> Vec<f32> {
    let toward = dir.toward_parent();
    let main = |p: Pos2| p.to_vec2().dot(toward.abs());
    let cross = |p: Pos2| p.to_vec2().dot(toward.rot90().abs());
    let stub_start = |&(_, to, stub): &(Pos2, Pos2, f32)| to - toward * stub;
    let mut turns: Vec<f32> = ends
        .iter()
        .map(|e| (main(e.0) + main(stub_start(e))) / 2.0)
        .collect();
    // Bands: the same child-edge and stub-start main coordinates.
    let key = |v: f32| (v * 2.0).round() as i64;
    let mut bands: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
    for (i, e) in ends.iter().enumerate() {
        let k = (key(main(e.0)), key(main(stub_start(e))));
        bands.entry(k).or_default().push(i);
    }
    for members in bands.values() {
        // Each run: its connector, and the cross coordinates of its child
        // end and its port end.
        let runs: Vec<Run> = members
            .iter()
            .map(|&i| Run {
                index: i,
                child: cross(ends[i].0),
                port: cross(stub_start(&ends[i])),
            })
            .filter(|r| (r.port - r.child).abs() >= NO_RUN)
            .collect();
        let level_of = levels(&runs);
        let n = level_of.iter().max().map_or(0, |&k| k + 1);
        if n < 2 {
            continue;
        }
        let level_of = runs.iter().map(|r| r.index).zip(level_of);
        // From the child's edge toward the parent, along the main axis.
        let e = &ends[members[0]];
        let band = main(stub_start(e)) - main(e.0);
        let spacing = ELBOW_SPACING.min(band.abs() / (n as f32 + 1.0));
        let mid = (main(e.0) + main(stub_start(e))) / 2.0;
        for (i, k) in level_of {
            // Level 0 is nearest the parent.
            let steps = (n - 1) as f32 / 2.0 - k as f32;
            turns[i] = mid + band.signum() * steps * spacing;
        }
    }
    turns
}

/// A run across: the cross coordinates of its two ends.
struct Run {
    index: usize,
    child: f32,
    port: f32,
}

impl Run {
    /// Whether cross coordinate `v` lies strictly inside the run.
    fn covers(&self, v: f32) -> bool {
        const EPS: f32 = 1e-3;
        v > self.child.min(self.port) + EPS && v < self.child.max(self.port) - EPS
    }

    /// Whether this run must turn nearer the parent than `other`: else
    /// `other`'s child leg or this run's port leg would cross a run.
    fn above(&self, other: &Run) -> bool {
        self.covers(other.child) || other.covers(self.port)
    }
}

/// The level of each run (0 nearest the parent): one past the deepest run
/// that must be above it. Pairs that must each be above the other cannot
/// avoid a crossing and only need different levels.
fn levels(runs: &[Run]) -> Vec<usize> {
    let n = runs.len();
    // `before[a]`: the runs `a` must be above (a clash goes by index).
    let before: Vec<Vec<usize>> = (0..n)
        .map(|a| {
            (0..n)
                .filter(|&b| {
                    let (ab, ba) = (runs[a].above(&runs[b]), runs[b].above(&runs[a]));
                    (ab && !ba) || (ab && ba && a < b)
                })
                .collect()
        })
        .collect();
    let mut waiting = vec![0usize; n];
    for b in before.iter().flatten() {
        waiting[*b] += 1;
    }
    // Longest paths in topological order. A cycle (three or more runs
    // that cannot all avoid crossing) is broken at its lowest index.
    let mut level = vec![0usize; n];
    let mut done = vec![false; n];
    for _ in 0..n {
        let next = (0..n)
            .find(|&i| !done[i] && waiting[i] == 0)
            .or_else(|| (0..n).find(|&i| !done[i]));
        let Some(a) = next else { break };
        done[a] = true;
        for &b in &before[a] {
            waiting[b] = waiting[b].saturating_sub(1);
            if !done[b] {
                level[b] = level[b].max(level[a] + 1);
            }
        }
    }
    level
}

#[cfg(test)]
#[path = "stagger_tests.rs"]
mod tests;
