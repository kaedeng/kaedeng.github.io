//! Which cell a pointer ray selects. Empty cells are see-through, so any shown cell can be
//! picked, inner ones included: the one whose centre passes closest to the ray, the front
//! one on near-ties. Cells behind the first placed box the ray enters are hidden by it,
//! and cells outside the shown region (a peeled layer) do not count at all. The same rays
//! tell which clue cells another box hides, so their labels can fade.

use patches_core::{BoxRegion, CELLS, Cell, cell_at};

use crate::geom::{cell_center, extent};

type V = [f32; 3];

/// A cell behind the current best must pass this much closer to the ray to win.
const TIE: f32 = 0.02;

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
