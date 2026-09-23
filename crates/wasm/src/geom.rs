//! World-space layout of the board: one 4x4x4 cube of unit cells centred on the origin.
//! `CpuMesh::cube()` spans -1..1, so a `Pose`'s `half` is exactly the instance scale.

use patches_core::{BoxRegion, Cell, N, Shape};

/// Half the cube's side: the lattice runs from -2 to 2 on every axis.
const GRID_HALF: f32 = N as f32 / 2.0;
/// Every cell of the cube.
pub const WHOLE: BoxRegion = BoxRegion {
    min: [0, 0, 0],
    max: [N - 1, N - 1, N - 1],
};
pub const LINE_R: f32 = 0.005;
pub const DOT_R: f32 = 0.022;
/// Radius of the round marker of a clue that allows any shape.
pub const MARK_R: f32 = 0.14;
/// Half-extents of a shaped clue marker along its long and short axes.
pub const MARK_LONG: f32 = 0.24;
pub const MARK_SHORT: f32 = 0.1;
/// Inset of a block from its cells' faces, so neighbouring blocks read as separate.
pub const BLOCK_GAP: f32 = 0.06;
/// Half-thickness of the red outline of a box that breaks a rule.
pub const OUTLINE_R: f32 = 0.018;

/// An axis-aligned cuboid: centre and half-extents.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub center: [f32; 3],
    pub half: [f32; 3],
}

/// World position of a cell's centre.
pub fn cell_center(c: Cell) -> [f32; 3] {
    c.map(|v| v as f32 + 0.5 - GRID_HALF)
}

/// World-space corners of a region's cells.
pub fn extent(region: &BoxRegion) -> ([f32; 3], [f32; 3]) {
    (
        region.min.map(|v| v as f32 - GRID_HALF),
        region.max.map(|v| v as f32 + 1.0 - GRID_HALF),
    )
}

/// Thin cuboids along every cell edge of `region`: 25 lines along each axis of the cube.
pub fn grid_lines(region: &BoxRegion) -> Vec<Pose> {
    let (lo, hi) = extent(region);
    let mut lines = Vec::new();
    for axis in 0..3 {
        let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
        for i in region.min[a]..=region.max[a] + 1 {
            for j in region.min[b]..=region.max[b] + 1 {
                let mut center = [0.0; 3];
                center[axis] = (lo[axis] + hi[axis]) / 2.0;
                center[a] = i as f32 - GRID_HALF;
                center[b] = j as f32 - GRID_HALF;
                let mut half = [LINE_R; 3];
                half[axis] = (hi[axis] - lo[axis]) / 2.0 + LINE_R;
                lines.push(Pose { center, half });
            }
        }
    }
    lines
}

/// Every corner of every cell of `region`.
pub fn lattice_dots(region: &BoxRegion) -> Vec<[f32; 3]> {
    let mut dots = Vec::new();
    for x in region.min[0]..=region.max[0] + 1 {
        for y in region.min[1]..=region.max[1] + 1 {
            for z in region.min[2]..=region.max[2] + 1 {
                dots.push([x, y, z].map(|v| v as f32 - GRID_HALF));
            }
        }
    }
    dots
}

/// The cube without its layer nearest `eye`, on the axis `eye` lies most along.
pub fn peeled(eye: [f32; 3]) -> BoxRegion {
    let mut axis = 0;
    for i in 1..3 {
        if eye[i].abs() > eye[axis].abs() {
            axis = i;
        }
    }
    let mut region = WHOLE;
    if eye[axis] > 0.0 {
        region.max[axis] -= 1;
    } else {
        region.min[axis] += 1;
    }
    region
}

/// The part of `p` inside `region`'s cells, if any.
pub fn clip(p: &Pose, region: &BoxRegion) -> Option<Pose> {
    let (lo, hi) = extent(region);
    let a: [f32; 3] = std::array::from_fn(|i| (p.center[i] - p.half[i]).max(lo[i]));
    let b: [f32; 3] = std::array::from_fn(|i| (p.center[i] + p.half[i]).min(hi[i]));
    (0..3).all(|i| a[i] < b[i]).then(|| Pose {
        center: std::array::from_fn(|i| (a[i] + b[i]) / 2.0),
        half: std::array::from_fn(|i| (b[i] - a[i]) / 2.0),
    })
}

/// The solid block for a placed box.
pub fn block_pose(b: &BoxRegion) -> Pose {
    let (lo, hi) = (cell_center(b.min), cell_center(b.max));
    Pose {
        center: std::array::from_fn(|i| (lo[i] + hi[i]) / 2.0),
        half: std::array::from_fn(|i| (hi[i] - lo[i]) / 2.0 + 0.5 - BLOCK_GAP),
    }
}

/// Thin cuboids along the 12 edges of `p`'s box.
pub fn box_edges(p: &Pose) -> Vec<Pose> {
    let mut edges = Vec::with_capacity(12);
    for axis in 0..3 {
        let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
        for (sa, sb) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
            let mut center = p.center;
            center[a] += sa * p.half[a];
            center[b] += sb * p.half[b];
            let mut half = [OUTLINE_R; 3];
            half[axis] = p.half[axis] + OUTLINE_R;
            edges.push(Pose { center, half });
        }
    }
    edges
}

/// Half-extents of a clue marker that shows `shape`: long on the shape's longest axes.
pub fn marker_half(shape: Shape) -> [f32; 3] {
    shape
        .longest()
        .map(|long| if long { MARK_LONG } else { MARK_SHORT })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-5)
    }

    #[test]
    fn cell_center_of_the_corner() {
        assert!(close(cell_center([0, 0, 0]), [-1.5; 3]));
        assert!(close(cell_center([3, 3, 3]), [1.5; 3]));
    }

    #[test]
    fn single_cell_block_leaves_a_gap() {
        let p = block_pose(&BoxRegion::spanning([1, 2, 3], [1, 2, 3]));
        assert!(close(p.center, cell_center([1, 2, 3])));
        assert!(close(p.half, [0.44; 3]));
    }

    #[test]
    fn full_cube_block_fills_the_cube() {
        let p = block_pose(&BoxRegion {
            min: [0, 0, 0],
            max: [3, 3, 3],
        });
        assert!(close(p.center, [0.0; 3]));
        assert!(close(p.half, [1.94; 3]));
    }

    #[test]
    fn lattice_has_every_cell_edge_and_corner() {
        assert_eq!(grid_lines(&WHOLE).len(), 3 * 25);
        let dots = lattice_dots(&WHOLE);
        assert_eq!(dots.len(), 125);
        assert!(dots.iter().any(|d| close(*d, [-2.0; 3])));
        assert!(dots.iter().any(|d| close(*d, [2.0; 3])));
    }

    #[test]
    fn peeling_drops_the_layer_nearest_the_eye() {
        let top_off = peeled([1.0, 5.0, 2.0]);
        assert_eq!(
            top_off,
            BoxRegion {
                min: [0, 0, 0],
                max: [3, 2, 3]
            }
        );
        assert_eq!(peeled([-8.0, 1.0, 2.0]).min, [1, 0, 0]);
        assert_eq!(grid_lines(&top_off).len(), 20 + 20 + 25);
        assert_eq!(lattice_dots(&top_off).len(), 100);
    }

    #[test]
    fn clip_cuts_a_block_at_the_peeled_layer() {
        let top_off = peeled([0.0, 1.0, 0.0]);
        let whole = block_pose(&WHOLE);
        let cut = clip(&whole, &top_off).expect("most of the block stays");
        assert!(close(cut.center, [0.0, -0.47, 0.0]));
        assert!(close(cut.half, [1.94, 1.47, 1.94]));
        let on_top = block_pose(&BoxRegion::spanning([0, 3, 0], [0, 3, 0]));
        assert_eq!(clip(&on_top, &top_off), None);
        assert_eq!(clip(&whole, &WHOLE), Some(whole));
    }

    #[test]
    fn grid_lines_span_the_cube() {
        for line in grid_lines(&WHOLE) {
            for i in 0..3 {
                let reach = line.center[i].abs() + line.half[i];
                assert!(reach <= 2.0 + LINE_R + 1e-5);
            }
            let long = line.half.iter().filter(|h| **h > 1.0).count();
            assert_eq!(long, 1);
        }
    }

    #[test]
    fn outline_traces_the_twelve_edges() {
        let p = Pose {
            center: [1.0, 0.0, 0.0],
            half: [0.5, 1.0, 2.0],
        };
        let edges = box_edges(&p);
        assert_eq!(edges.len(), 12);
        for e in &edges {
            let along = (0..3).filter(|&i| e.half[i] > OUTLINE_R).count();
            assert_eq!(along, 1);
            for i in 0..3 {
                let off = (e.center[i] - p.center[i]).abs();
                assert!(off < 1e-5 || (off - p.half[i]).abs() < 1e-5);
            }
        }
        assert!(edges.iter().any(|e| close(e.center, [1.5, 1.0, 0.0])));
    }

    #[test]
    fn marker_is_long_along_the_shape_s_longest_axes() {
        assert!(close(
            marker_half(Shape::Tall),
            [MARK_SHORT, MARK_LONG, MARK_SHORT]
        ));
        assert!(close(
            marker_half(Shape::WallZ),
            [MARK_SHORT, MARK_LONG, MARK_LONG]
        ));
    }
}
