//! Which cell a pointer ray selects. Empty cells are see-through, so any shown cell can be
//! picked, inner ones included: the one whose centre passes closest to the ray, the front
//! one on near-ties. Cells behind the first placed box the ray enters are hidden by it,
//! and cells outside the shown region (a peeled layer) do not count at all. Locked boxes
//! are not there for the pointer: it goes through them to the open box behind. The same
//! rays tell which clue cells another box hides, so their labels can fade, and which
//! padlocks it covers.

use patches_core::{BoxRegion, CELLS, Cell, cell_at};

use crate::geom::{cell_center, extent};

type V = [f32; 3];

/// A cell behind the current best must pass this much closer to the ray to win.
const TIE: f32 = 0.02;
/// A box must start this much nearer than a mark to cover it; one about level with the
/// mark stands beside it.
const LEVEL: f32 = 0.1;

/// `dir` is a unit vector; `shown` is the region still drawn. `None` when the ray misses
/// it, so a press there orbits.
pub fn pick(origin: V, dir: V, boxes: &[BoxRegion], shown: &BoxRegion) -> Option<Cell> {
    let (lo, hi) = extent(shown);
    entry(origin, dir, lo, hi)?;
    closest(&visible(origin, dir, boxes, shown))
}

/// Every shown cell not hidden behind the first box the ray enters, as `(depth, distance
/// to the ray, cell)`, front to back.
fn visible(origin: V, dir: V, boxes: &[BoxRegion], shown: &BoxRegion) -> Vec<(f32, f32, Cell)> {
    let front = first_box(origin, dir, boxes, shown);
    let mut cells: Vec<(f32, f32, Cell)> = (0..CELLS)
        .map(cell_at)
        .filter(|&c| shown.contains(c))
        .filter_map(|c| {
            let to = sub(cell_center(c), origin);
            let t = dot(to, dir);
            let hidden = front.is_some_and(|(t0, b)| t > t0 && !b.contains(c));
            (!hidden).then(|| (t, (dot(to, to) - t * t).max(0.0).sqrt(), c))
        })
        .collect();
    cells.sort_by(|a, b| a.0.total_cmp(&b.0));
    cells
}

/// From `(depth, distance to the ray, cell)` sorted front to back, the cell nearest the ray.
fn closest(cells: &[(f32, f32, Cell)]) -> Option<Cell> {
    let mut best: Option<(f32, Cell)> = None;
    for &(_, d, c) in cells {
        if best.is_none_or(|(bd, _)| d < bd - TIE) {
            best = Some((d, c));
        }
    }
    best.map(|(_, c)| c)
}

/// Whether the shown part of a placed box other than the one holding `cell` stands
/// between `eye` and the cell's centre, so the cell's clue label should fade.
pub fn hidden(eye: V, cell: Cell, boxes: &[BoxRegion], shown: &BoxRegion) -> bool {
    let to = sub(cell_center(cell), eye);
    let dist = dot(to, to).sqrt();
    let dir = to.map(|v| v / dist);
    boxes
        .iter()
        .filter(|b| !b.contains(cell))
        .filter_map(|b| overlap(b, shown))
        .any(|b| {
            let (lo, hi) = extent(&b);
            entry(eye, dir, lo, hi).is_some_and(|t| t < dist)
        })
}

/// What a pointer ray is over.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pick {
    /// It misses the shown cells.
    Off,
    Cell(Cell),
    /// A locked box with no open box behind it: the pointer stays where it last was.
    Locked,
}

impl Pick {
    pub fn cell(self) -> Option<Cell> {
        match self {
            Pick::Cell(c) => Some(c),
            Pick::Off | Pick::Locked => None,
        }
    }
}

/// `pick` for a board of `open` and `locked` boxes, where the locked ones are not there
/// for the pointer: over one, it takes the open box behind it, if any. The locked boxes
/// still hide the empty cells behind them.
pub fn pick_open(
    origin: V,
    dir: V,
    open: &[BoxRegion],
    locked: &[BoxRegion],
    shown: &BoxRegion,
) -> Pick {
    let all: Vec<BoxRegion> = open.iter().chain(locked).copied().collect();
    let Some(c) = pick(origin, dir, &all, shown) else {
        return Pick::Off;
    };
    if !locked.iter().any(|b| b.contains(c)) {
        return Pick::Cell(c);
    }
    let Some((_, behind)) = first_box(origin, dir, open, shown) else {
        return Pick::Locked;
    };
    let cells: Vec<(f32, f32, Cell)> = visible(origin, dir, open, shown)
        .into_iter()
        .filter(|&(_, _, c)| behind.contains(c))
        .collect();
    closest(&cells).map_or(Pick::Locked, Pick::Cell)
}

/// Whether something drawn over the canvas at `point`, e.g. a padlock, would show over a
/// box that stands in front of it: along one of `rays` (origin, unit direction) through
/// its outline, the shown part of one of `others` starts clearly nearer than `point`, and
/// nearer than `own`, the solid box it sits on, if any.
pub fn covered(
    rays: &[(V, V)],
    point: V,
    own: Option<&BoxRegion>,
    others: &[BoxRegion],
    shown: &BoxRegion,
) -> bool {
    rays.iter().any(|&(origin, dir)| {
        let depth = dot(sub(point, origin), dir);
        let mine = own.and_then(|b| first_box(origin, dir, std::slice::from_ref(b), shown));
        first_box(origin, dir, others, shown)
            .is_some_and(|(t, _)| t < depth - LEVEL && mine.is_none_or(|(m, _)| t < m))
    })
}

/// The shown part of the placed box the ray enters first, and how far along the ray it does.
fn first_box(
    origin: V,
    dir: V,
    boxes: &[BoxRegion],
    shown: &BoxRegion,
) -> Option<(f32, BoxRegion)> {
    boxes
        .iter()
        .filter_map(|b| overlap(b, shown))
        .filter_map(|b| {
            let (lo, hi) = extent(&b);
            entry(origin, dir, lo, hi).map(|t| (t, b))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
}

/// The cells two regions share, if any.
fn overlap(a: &BoxRegion, b: &BoxRegion) -> Option<BoxRegion> {
    let min: Cell = std::array::from_fn(|i| a.min[i].max(b.min[i]));
    let max: Cell = std::array::from_fn(|i| a.max[i].min(b.max[i]));
    (0..3)
        .all(|i| min[i] <= max[i])
        .then_some(BoxRegion { min, max })
}

/// Distance along the ray to where it enters the box `lo..hi`, if it hits it at all.
fn entry(origin: V, dir: V, lo: V, hi: V) -> Option<f32> {
    let (mut near, mut far) = (f32::NEG_INFINITY, f32::INFINITY);
    for i in 0..3 {
        let a = (lo[i] - origin[i]) / dir[i];
        let b = (hi[i] - origin[i]) / dir[i];
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    (near <= far && far >= 0.0).then_some(near)
}

fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}

fn dot(a: V, b: V) -> f32 {
    (0..3).map(|i| a[i] * b[i]).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{WHOLE, peeled};

    fn unit(v: [f32; 3]) -> [f32; 3] {
        let l = dot(v, v).sqrt();
        v.map(|x| x / l)
    }

    /// A ray from far away that passes exactly through the centre of `cell`.
    fn aimed_at(cell: Cell, dir: [f32; 3]) -> ([f32; 3], [f32; 3]) {
        let d = unit(dir);
        let c = cell_center(cell);
        (std::array::from_fn(|i| c[i] - 20.0 * d[i]), d)
    }

    #[test]
    fn ray_past_the_cube_picks_nothing() {
        assert_eq!(
            pick([5.0, 5.0, 5.0], unit([1.0, 1.0, 1.0]), &[], &WHOLE),
            None
        );
        assert_eq!(pick([3.0, 10.0, 0.0], [0.0, -1.0, 0.0], &[], &WHOLE), None);
    }

    #[test]
    fn centres_in_line_go_to_the_front_one() {
        let (o, d) = aimed_at([1, 0, 2], [0.0, -1.0, 0.0]);
        assert_eq!(pick(o, d, &[], &WHOLE), Some([1, 3, 2]));
    }

    #[test]
    fn an_inner_cell_can_be_picked_directly() {
        let (o, d) = aimed_at([1, 1, 1], [1.0, 2.0, 3.0]);
        assert_eq!(pick(o, d, &[], &WHOLE), Some([1, 1, 1]));
        let (o, d) = aimed_at([2, 1, 2], [-3.0, -2.0, -1.0]);
        assert_eq!(pick(o, d, &[], &WHOLE), Some([2, 1, 2]));
    }

    #[test]
    fn peeled_cells_cannot_be_picked() {
        let (o, d) = aimed_at([1, 0, 2], [0.0, -1.0, 0.0]);
        assert_eq!(pick(o, d, &[], &peeled([0.0, 1.0, 0.0])), Some([1, 2, 2]));
        let top = BoxRegion::spanning([1, 3, 2], [1, 3, 2]);
        assert_eq!(
            pick(o, d, &[top], &peeled([0.0, 1.0, 0.0])),
            Some([1, 2, 2])
        );
    }

    #[test]
    fn cells_behind_a_box_are_hidden() {
        let floor = BoxRegion {
            min: [0, 0, 0],
            max: [3, 0, 3],
        };
        let (o, d) = aimed_at([1, 1, 1], [1.0, 2.0, 3.0]);
        let hit = pick(o, d, &[floor], &WHOLE).expect("the ray hits the floor box");
        assert!(floor.contains(hit));
    }

    #[test]
    fn a_clue_is_hidden_only_behind_another_box() {
        let eye = [0.0, 12.0, 0.0];
        let clue = [1, 0, 2];
        assert!(!hidden(eye, clue, &[], &WHOLE));
        let above = BoxRegion {
            min: [0, 2, 0],
            max: [3, 3, 3],
        };
        assert!(hidden(eye, clue, &[above], &WHOLE));
        // Its own box in front of it, or a box behind it, leaves the clue in view.
        let own = BoxRegion {
            min: [1, 0, 2],
            max: [1, 3, 2],
        };
        assert!(!hidden(eye, clue, &[own], &WHOLE));
        let floor = BoxRegion {
            min: [0, 0, 0],
            max: [3, 0, 3],
        };
        assert!(!hidden(eye, [1, 1, 2], &[floor], &WHOLE));
    }

    #[test]
    fn the_pointer_goes_through_a_locked_box_to_the_open_box_behind() {
        let (o, d) = aimed_at([1, 3, 2], [0.0, -1.0, 0.0]);
        let top = BoxRegion::spanning([0, 3, 0], [3, 3, 3]);
        let floor = BoxRegion::spanning([0, 0, 0], [3, 0, 3]);
        assert_eq!(
            pick_open(o, d, &[floor], &[top], &WHOLE),
            Pick::Cell([1, 0, 2])
        );
        assert_eq!(pick_open(o, d, &[], &[top], &WHOLE), Pick::Locked);
        assert_eq!(pick_open(o, d, &[top], &[], &WHOLE), Pick::Cell([1, 3, 2]));
    }

    #[test]
    fn a_cell_in_front_of_a_locked_box_is_picked_as_usual() {
        let (o, d) = aimed_at([1, 3, 2], [0.0, -1.0, 0.0]);
        let low = BoxRegion::spanning([0, 0, 0], [3, 1, 3]);
        assert_eq!(pick_open(o, d, &[], &[low], &WHOLE), Pick::Cell([1, 3, 2]));
        let away = unit([1.0, 1.0, 1.0]);
        assert_eq!(
            pick_open([5.0, 5.0, 5.0], away, &[], &[low], &WHOLE),
            Pick::Off
        );
    }

    /// A ray straight down onto the cube at world `x`, over cells with z = 1.
    fn down(x: f32) -> ([f32; 3], [f32; 3]) {
        ([x, 12.0, -0.5], [0.0, -1.0, 0.0])
    }

    #[test]
    fn a_mark_is_covered_where_any_ray_through_it_meets_another_box_first() {
        let own = BoxRegion::spanning([1, 0, 1], [1, 0, 1]);
        let mark = cell_center([1, 0, 1]);
        let beside = BoxRegion::spanning([2, 1, 1], [2, 3, 1]);
        // Through its middle a ray meets only the mark's own box; past its edge, the box
        // beside it, above the mark.
        assert!(!covered(&[down(-0.5)], mark, Some(&own), &[beside], &WHOLE));
        let wide = [down(-0.5), down(0.2)];
        assert!(covered(&wide, mark, Some(&own), &[beside], &WHOLE));
        assert!(covered(&wide, mark, None, &[beside], &WHOLE));
        // With the top layer peeled, a box only there covers nothing.
        let top = BoxRegion::spanning([2, 3, 1], [2, 3, 1]);
        let peel = peeled([0.0, 1.0, 0.0]);
        assert!(!covered(&wide, mark, Some(&own), &[top], &peel));
    }

    #[test]
    fn a_mark_on_its_own_box_is_not_covered_by_what_lies_behind_that() {
        let own = BoxRegion::spanning([1, 3, 1], [1, 3, 1]);
        let under = BoxRegion::spanning([1, 1, 1], [1, 2, 1]);
        let mark = cell_center([1, 0, 1]);
        assert!(!covered(&[down(-0.5)], mark, Some(&own), &[under], &WHOLE));
        assert!(covered(&[down(-0.5)], mark, None, &[under], &WHOLE));
    }

    #[test]
    fn a_box_level_with_a_mark_beside_it_does_not_cover_it() {
        let own = BoxRegion::spanning([1, 0, 1], [1, 0, 1]);
        // On its top face.
        let mark = [-0.5, -1.0, -0.5];
        let level = BoxRegion::spanning([2, 0, 1], [2, 0, 1]);
        // Tilted a little, as a camera's rays are: it meets the level box a hair nearer.
        let wide = [down(-0.5), ([0.6, 12.0, -0.5], unit([-0.02, -1.0, 0.0]))];
        assert!(!covered(&wide, mark, Some(&own), &[level], &WHOLE));
        let taller = BoxRegion::spanning([2, 0, 1], [2, 1, 1]);
        assert!(covered(&wide, mark, Some(&own), &[taller], &WHOLE));
    }

    #[test]
    fn a_box_only_hides_with_its_shown_part() {
        let eye = [0.0, 12.0, 0.0];
        let top = BoxRegion {
            min: [0, 3, 0],
            max: [3, 3, 3],
        };
        assert!(hidden(eye, [1, 1, 2], &[top], &WHOLE));
        assert!(!hidden(eye, [1, 1, 2], &[top], &peeled([0.0, 1.0, 0.0])));
    }

    #[test]
    fn a_box_off_to_the_side_hides_nothing() {
        let eye = [8.0, 6.0, 10.0];
        let corner = BoxRegion::spanning([0, 3, 0], [0, 3, 0]);
        assert!(!hidden(eye, [3, 0, 3], &[corner], &WHOLE));
        let (o, _) = aimed_at([0, 0, 0], [-1.0, -1.0, -1.0]);
        let front = BoxRegion::spanning([1, 1, 1], [3, 3, 3]);
        assert!(hidden(o, [0, 0, 0], &[front], &WHOLE));
        let (o, _) = aimed_at([0, 0, 0], [1.0, 1.0, 1.0]);
        assert!(!hidden(o, [0, 0, 0], &[front], &WHOLE));
    }

    #[test]
    fn cells_in_front_of_a_box_stay_pickable() {
        let bottom = BoxRegion {
            min: [1, 0, 2],
            max: [1, 1, 2],
        };
        let (o, d) = aimed_at([1, 0, 2], [0.0, -1.0, 0.0]);
        assert_eq!(pick(o, d, &[bottom], &WHOLE), Some([1, 3, 2]));
        let top = BoxRegion {
            min: [1, 2, 2],
            max: [1, 3, 2],
        };
        assert_eq!(pick(o, d, &[top], &WHOLE), Some([1, 3, 2]));
    }
}
