//! Which cell a pointer ray selects. Empty cells are see-through, so any cell can be
//! picked, inner ones included: the one whose centre passes closest to the ray, the front
//! one on near-ties. Cells behind the first placed box the ray enters are hidden by it.

use patches_core::{BoxRegion, CELLS, Cell, cell_at};

use crate::geom::cell_center;

type V = [f32; 3];

/// A cell behind the current best must pass this much closer to the ray to win.
const TIE: f32 = 0.02;
/// Half the cube's side.
const HALF: f32 = 2.0;

/// `dir` is a unit vector. `None` when the ray misses the cube, so a press there orbits.
pub fn pick(origin: V, dir: V, boxes: &[BoxRegion]) -> Option<Cell> {
    entry(origin, dir, [-HALF; 3], [HALF; 3])?;
    closest(&visible(origin, dir, boxes))
}

/// Every cell not hidden behind the first box the ray enters, as `(depth, distance to the
/// ray, cell)`, front to back.
fn visible(origin: V, dir: V, boxes: &[BoxRegion]) -> Vec<(f32, f32, Cell)> {
    let front = first_box(origin, dir, boxes);
    let mut cells: Vec<(f32, f32, Cell)> = (0..CELLS)
        .map(cell_at)
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

/// The placed box the ray enters first, and how far along the ray it does.
fn first_box(origin: V, dir: V, boxes: &[BoxRegion]) -> Option<(f32, &BoxRegion)> {
    boxes
        .iter()
        .filter_map(|b| {
            let lo = cell_center(b.min).map(|v| v - 0.5);
            let hi = cell_center(b.max).map(|v| v + 0.5);
            entry(origin, dir, lo, hi).map(|t| (t, b))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
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
        assert_eq!(pick([5.0, 5.0, 5.0], unit([1.0, 1.0, 1.0]), &[]), None);
        assert_eq!(pick([3.0, 10.0, 0.0], [0.0, -1.0, 0.0], &[]), None);
    }

    #[test]
    fn centres_in_line_go_to_the_front_one() {
        let (o, d) = aimed_at([1, 0, 2], [0.0, -1.0, 0.0]);
        assert_eq!(pick(o, d, &[]), Some([1, 3, 2]));
    }

    #[test]
    fn an_inner_cell_can_be_picked_directly() {
        let (o, d) = aimed_at([1, 1, 1], [1.0, 2.0, 3.0]);
        assert_eq!(pick(o, d, &[]), Some([1, 1, 1]));
        let (o, d) = aimed_at([2, 1, 2], [-3.0, -2.0, -1.0]);
        assert_eq!(pick(o, d, &[]), Some([2, 1, 2]));
    }

    #[test]
    fn cells_behind_a_box_are_hidden() {
        let floor = BoxRegion {
            min: [0, 0, 0],
            max: [3, 0, 3],
        };
        let (o, d) = aimed_at([1, 1, 1], [1.0, 2.0, 3.0]);
        let hit = pick(o, d, &[floor]).expect("the ray hits the floor box");
        assert!(floor.contains(hit));
    }

    #[test]
    fn cells_in_front_of_a_box_stay_pickable() {
        let bottom = BoxRegion {
            min: [1, 0, 2],
            max: [1, 1, 2],
        };
        let (o, d) = aimed_at([1, 0, 2], [0.0, -1.0, 0.0]);
        assert_eq!(pick(o, d, &[bottom]), Some([1, 3, 2]));
        let top = BoxRegion {
            min: [1, 2, 2],
            max: [1, 3, 2],
        };
        assert_eq!(pick(o, d, &[top]), Some([1, 3, 2]));
    }
}
