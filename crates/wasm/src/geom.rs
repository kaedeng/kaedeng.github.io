//! World-space layout of the board: one 4x4x4 cube of unit cells centred on the origin.
//! `CpuMesh::cube()` spans -1..1, so a `Pose`'s `half` is exactly the instance scale.

use patches_core::{BoxRegion, Cell, N, Shape};

/// Half the cube's side: the lattice runs from -2 to 2 on every axis.
const GRID_HALF: f32 = N as f32 / 2.0;
pub const LINE_R: f32 = 0.005;
pub const DOT_R: f32 = 0.022;
/// Radius of the round marker of a clue that allows any shape.
pub const MARK_R: f32 = 0.14;
/// Half-extents of a shaped clue marker along its long and short axes.
pub const MARK_LONG: f32 = 0.24;
pub const MARK_SHORT: f32 = 0.1;
/// Inset of a block from its cells' faces, so neighbouring blocks read as separate.
pub const BLOCK_GAP: f32 = 0.06;

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

/// Thin cuboids along every cell edge: 25 lines along each axis.
pub fn grid_lines() -> Vec<Pose> {
    let mut lines = Vec::new();
    for axis in 0..3 {
        for i in 0..=N {
            for j in 0..=N {
                let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
                let mut center = [0.0; 3];
                center[a] = i as f32 - GRID_HALF;
                center[b] = j as f32 - GRID_HALF;
                let mut half = [LINE_R; 3];
                half[axis] = GRID_HALF + LINE_R;
                lines.push(Pose { center, half });
            }
        }
    }
    lines
}

/// Every corner of every cell.
pub fn lattice_dots() -> Vec<[f32; 3]> {
    let mut dots = Vec::new();
    for x in 0..=N {
        for y in 0..=N {
            for z in 0..=N {
                dots.push([x, y, z].map(|v| v as f32 - GRID_HALF));
            }
        }
    }
    dots
}

/// The solid block for a placed box.
pub fn block_pose(b: &BoxRegion) -> Pose {
    let (lo, hi) = (cell_center(b.min), cell_center(b.max));
    Pose {
        center: std::array::from_fn(|i| (lo[i] + hi[i]) / 2.0),
        half: std::array::from_fn(|i| (hi[i] - lo[i]) / 2.0 + 0.5 - BLOCK_GAP),
    }
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
        assert_eq!(grid_lines().len(), 3 * 25);
        let dots = lattice_dots();
        assert_eq!(dots.len(), 125);
        assert!(dots.iter().any(|d| close(*d, [-2.0; 3])));
        assert!(dots.iter().any(|d| close(*d, [2.0; 3])));
    }

    #[test]
    fn grid_lines_span_the_cube() {
        for line in grid_lines() {
            for i in 0..3 {
                let reach = line.center[i].abs() + line.half[i];
                assert!(reach <= 2.0 + LINE_R + 1e-5);
            }
            let long = line.half.iter().filter(|h| **h > 1.0).count();
            assert_eq!(long, 1);
        }
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
