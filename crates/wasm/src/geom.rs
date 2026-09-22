//! World-space layout of the board, in cell units. `CpuMesh::cube()` spans -1..1, so a
//! `Pose`'s `half` is exactly the instance scale.

use patches_core::{BoxRegion, Cell, N};

/// Vertical distance between layers, in cell units, so every layer's top faces stay visible.
pub const LAYER_PITCH: f32 = 2.5;
/// Half the width of a layer: the grid runs from -2 to 2 on x and z.
const GRID_HALF: f32 = N as f32 / 2.0;
pub const LINE_R: f32 = 0.02;
pub const DOT_R: f32 = 0.06;
pub const MARK_R: f32 = 0.14;
/// Inset of a block from its cells' faces, so neighbouring blocks read as separate.
pub const BLOCK_GAP: f32 = 0.06;

/// An axis-aligned cuboid: centre and half-extents.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub center: [f32; 3],
    pub half: [f32; 3],
}

/// World position of a cell's centre: x and z on a unit grid, y stretched by `LAYER_PITCH`.
pub fn cell_center(c: Cell) -> [f32; 3] {
    let mid = (N as f32 - 1.0) / 2.0;
    [
        c[0] as f32 - mid,
        (c[1] as f32 - mid) * LAYER_PITCH,
        c[2] as f32 - mid,
    ]
}

/// Height of the bottom face of layer `y`'s cells.
fn layer_floor(y: u8) -> f32 {
    cell_center([0, y, 0])[1] - 0.5
}

/// Thin cuboids tracing each layer's floor grid: 5 lines along x and 5 along z per layer.
pub fn grid_lines() -> Vec<Pose> {
    let mut lines = Vec::new();
    for y in 0..N {
        let floor = layer_floor(y);
        for i in 0..=N {
            let o = i as f32 - GRID_HALF;
            lines.push(Pose {
                center: [0.0, floor, o],
                half: [GRID_HALF + LINE_R, LINE_R, LINE_R],
            });
            lines.push(Pose {
                center: [o, floor, 0.0],
                half: [LINE_R, LINE_R, GRID_HALF + LINE_R],
            });
        }
    }
    lines
}

/// Every lattice intersection of every layer's floor grid.
pub fn lattice_dots() -> Vec<[f32; 3]> {
    let mut dots = Vec::new();
    for y in 0..N {
        let floor = layer_floor(y);
        for i in 0..=N {
            for j in 0..=N {
                dots.push([i as f32 - GRID_HALF, floor, j as f32 - GRID_HALF]);
            }
        }
    }
    dots
}

/// The solid block for a placed box. A box over several layers becomes one pillar across the gaps.
pub fn block_pose(b: &BoxRegion) -> Pose {
    let (lo, hi) = (cell_center(b.min), cell_center(b.max));
    Pose {
        center: std::array::from_fn(|i| (lo[i] + hi[i]) / 2.0),
        half: std::array::from_fn(|i| (hi[i] - lo[i]) / 2.0 + 0.5 - BLOCK_GAP),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-5)
    }

    #[test]
    fn cell_center_of_the_corner() {
        assert!(close(cell_center([0, 0, 0]), [-1.5, -3.75, -1.5]));
    }

    #[test]
    fn single_cell_block_leaves_a_gap() {
        let p = block_pose(&BoxRegion::spanning([1, 2, 3], [1, 2, 3]));
        assert!(close(p.center, cell_center([1, 2, 3])));
        assert!(close(p.half, [0.44; 3]));
    }

    #[test]
    fn full_cube_block_spans_the_layer_gaps() {
        let p = block_pose(&BoxRegion {
            min: [0, 0, 0],
            max: [3, 3, 3],
        });
        assert!(close(p.center, [0.0; 3]));
        assert!(close(p.half, [1.94, 4.19, 1.94]));
    }

    #[test]
    fn grid_has_ten_lines_per_layer() {
        assert_eq!(grid_lines().len(), 40);
    }

    #[test]
    fn lattice_has_twenty_five_dots_per_layer() {
        assert_eq!(lattice_dots().len(), 100);
    }
}
