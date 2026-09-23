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
/// Half-thickness of the 12 outer edges of the shown cells: bolder than the inner lines, so
/// the cube's outline reads first.
pub const FRAME_R: f32 = 0.01;
pub const DOT_R: f32 = 0.022;
/// Half-extents of a shaped clue marker along its long and short axes.
pub const MARK_LONG: f32 = 0.24;
pub const MARK_SHORT: f32 = 0.1;
/// Half-thickness of the dashes that outline a shaped clue's marker.
pub const DASH_R: f32 = 0.012;
/// Length of a dash and of the gap after it, before they stretch to fit an edge.
const DASH: f32 = 0.07;
const DASH_GAP: f32 = 0.05;
/// Half-thickness of the three bars of an any-shape clue's jack: bolder than a dash, since
/// three lines have to hold their own beside a filled, outlined box.
pub const JACK_R: f32 = 0.02;
/// A jack's end caps against its bars' half-thickness.
const JACK_CAP: f32 = 1.6;
/// Inset of a block from its cells' faces, so neighbouring blocks read as separate.
pub const BLOCK_GAP: f32 = 0.06;
/// Half-thickness of the red outline of a box that breaks a rule.
pub const OUTLINE_R: f32 = 0.018;
/// Half-thickness of the darker edges of a placed block: a bit thinner than a wrong box's.
pub const EDGE_R: f32 = 0.011;
/// Half-thickness of a locked box's edges, or its red outline: bolder than either, so a
/// locked box stands out.
pub const LOCKED_R: f32 = 0.028;
/// How far the hatching sits out from a block's cut face, so it never z-fights it.
const HATCH_LIFT: f32 = 0.004;

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

/// The cuboid filling `region`'s cells.
fn region_pose(region: &BoxRegion) -> Pose {
    let (lo, hi) = extent(region);
    Pose {
        center: std::array::from_fn(|i| (lo[i] + hi[i]) / 2.0),
        half: std::array::from_fn(|i| (hi[i] - lo[i]) / 2.0),
    }
}

/// Thin cuboids along every cell edge of `region` but its 12 outer edges (`region_edges`
/// draws those): 21 lines along each axis of the cube.
pub fn grid_lines(region: &BoxRegion) -> Vec<Pose> {
    let (lo, hi) = extent(region);
    let mut lines = Vec::new();
    for axis in 0..3 {
        let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
        let side = |v: u8, k: usize| v == region.min[k] || v == region.max[k] + 1;
        for i in region.min[a]..=region.max[a] + 1 {
            for j in (region.min[b]..=region.max[b] + 1).filter(|&j| !(side(i, a) && side(j, b))) {
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

/// The 12 outer edges of `region`, bolder than its inner lines.
pub fn region_edges(region: &BoxRegion) -> Vec<Pose> {
    box_edges(&region_pose(region), FRAME_R)
}

/// The 8 corners of `region`.
pub fn corners(region: &BoxRegion) -> Vec<[f32; 3]> {
    let (lo, hi) = extent(region);
    (0..8)
        .map(|k| std::array::from_fn(|i| if (k >> i) & 1 == 0 { lo[i] } else { hi[i] }))
        .collect()
}

/// How near `p` is to a camera at `eye` looking at the origin, measured along the view:
/// 1 at `region`'s nearest corner down to 0 at its farthest, so the lattice can fade
/// with depth.
pub fn nearness(p: [f32; 3], eye: [f32; 3], region: &BoxRegion) -> f32 {
    let toward = |q: [f32; 3]| (0..3).map(|i| q[i] * eye[i]).sum::<f32>();
    let ends = corners(region).into_iter().map(toward);
    let (far, near) = ends.fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), t| {
        (lo.min(t), hi.max(t))
    });
    ((toward(p) - far) / (near - far)).clamp(0.0, 1.0)
}

/// The value `nearness` (0..1) of the way from the `far` end of `range` to the `near` end:
/// how strongly something at that depth is drawn.
pub fn by_depth(nearness: f32, (near, far): (f32, f32)) -> f32 {
    far + (near - far) * nearness
}

/// How thick a clue marker's lines are against their plain thickness, nearest to
/// farthest: bolder in front, as in a line drawing, so depth reads without dimming colour.
const MARKER_WEIGHT: (f32, f32) = (1.45, 0.55);

/// The line weight of a clue marker at `nearness` (0..1); 1 is the plain thickness.
pub fn marker_weight(nearness: f32) -> f32 {
    by_depth(nearness, MARKER_WEIGHT)
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

/// The middles of the faces of `p` turned toward a camera at `eye`, the most in view
/// first: by area times how squarely each faces the eye.
pub fn faces_toward(p: &Pose, eye: [f32; 3]) -> Vec<[f32; 3]> {
    let mut faces: Vec<(f32, [f32; 3])> = Vec::new();
    for axis in 0..3 {
        let area = p.half[(axis + 1) % 3] * p.half[(axis + 2) % 3];
        for sign in [-1.0, 1.0] {
            let mut middle = p.center;
            middle[axis] += sign * p.half[axis];
            let to: [f32; 3] = std::array::from_fn(|i| eye[i] - middle[i]);
            let cos = sign * to[axis] / to.iter().map(|v| v * v).sum::<f32>().sqrt();
            if cos > 0.0 {
                faces.push((area * cos, middle));
            }
        }
    }
    faces.sort_by(|a, b| b.0.total_cmp(&a.0));
    faces.into_iter().map(|(_, middle)| middle).collect()
}

/// Where `clip` cut `p` at a side of `region` that lies inside the cube, i.e. where a
/// layer was peeled away: a thin slab just outside each cut face, to hatch it like a
/// section. The cube's own sides never count, even when a solved block spreads past them.
pub fn cut_faces(p: &Pose, region: &BoxRegion) -> Vec<Pose> {
    let Some(cut) = clip(p, region) else {
        return Vec::new();
    };
    let (lo, hi) = extent(region);
    let mut faces = Vec::new();
    for axis in 0..3 {
        let sides = [
            (-1.0, lo[axis], region.min[axis] > 0),
            (1.0, hi[axis], region.max[axis] < N - 1),
        ];
        for (sign, bound, inside) in sides {
            if inside && sign * (p.center[axis] + sign * p.half[axis] - bound) > 0.0 {
                let mut face = cut;
                face.center[axis] = bound + sign * HATCH_LIFT;
                face.half[axis] = HATCH_LIFT / 2.0;
                faces.push(face);
            }
        }
    }
    faces
}

/// The solid block for a placed box.
pub fn block_pose(b: &BoxRegion) -> Pose {
    let (lo, hi) = (cell_center(b.min), cell_center(b.max));
    Pose {
        center: std::array::from_fn(|i| (lo[i] + hi[i]) / 2.0),
        half: std::array::from_fn(|i| (hi[i] - lo[i]) / 2.0 + 0.5 - BLOCK_GAP),
    }
}

/// Cuboids of half-thickness `r` along the 12 edges of `p`'s box.
pub fn box_edges(p: &Pose, r: f32) -> Vec<Pose> {
    let mut edges = Vec::with_capacity(12);
    for axis in 0..3 {
        let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
        for (sa, sb) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
            let mut center = p.center;
            center[a] += sa * p.half[a];
            center[b] += sb * p.half[b];
            let mut half = [r; 3];
            half[axis] = p.half[axis] + r;
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

/// The 12 edges of `p`'s box as dashes, each edge with a dash at both ends so the corners
/// show: a shaped clue's marker, drawn tentative beside the solid blocks. `r` thick.
pub fn dashed_edges(p: &Pose, r: f32) -> Vec<Pose> {
    box_edges(p, r).iter().flat_map(dashes).collect()
}

/// Splits a thin cuboid along its long axis into as many dashes as fit at about `DASH` and
/// `DASH_GAP`, at least two, stretched so the first and last end flush with it.
fn dashes(edge: &Pose) -> Vec<Pose> {
    let axis = (0..3)
        .max_by(|&a, &b| edge.half[a].total_cmp(&edge.half[b]))
        .expect("three axes");
    let len = 2.0 * edge.half[axis];
    let n = ((len + DASH_GAP) / (DASH + DASH_GAP)).round().max(2.0);
    let stretch = len / (n * DASH + (n - 1.0) * DASH_GAP);
    let (dash, step) = (DASH * stretch, (DASH + DASH_GAP) * stretch);
    let start = edge.center[axis] - edge.half[axis];
    (0..n as usize)
        .map(|k| {
            let mut d = *edge;
            d.center[axis] = start + dash / 2.0 + k as f32 * step;
            d.half[axis] = dash / 2.0;
            d
        })
        .collect()
}

/// Three thin bars along x, y and z crossing at `center`: an any-shape clue's marker, a
/// box that can grow any way. As long as a shaped marker's long side; `r` thick, with a
/// square cap on each end.
pub fn jack(center: [f32; 3], r: f32) -> Vec<Pose> {
    let bars = (0..3).map(|axis| {
        let mut half = [r; 3];
        half[axis] = MARK_LONG;
        Pose { center, half }
    });
    let caps = (0..3).flat_map(|axis| {
        [-MARK_LONG, MARK_LONG].map(|d| {
            let mut c = center;
            c[axis] += d;
            Pose {
                center: c,
                half: [r * JACK_CAP; 3],
            }
        })
    });
    bars.chain(caps).collect()
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
    fn lattice_has_every_cell_edge_and_the_region_s_corners() {
        // The 12 edges of the cube's frame are drawn bolder, apart from the inner lines.
        assert_eq!(grid_lines(&WHOLE).len(), 3 * 25 - 12);
        assert_eq!(region_edges(&WHOLE).len(), 12);
        let dots = corners(&WHOLE);
        assert_eq!(dots.len(), 8);
        assert!(dots.iter().any(|d| close(*d, [-2.0; 3])));
        assert!(dots.iter().any(|d| close(*d, [2.0; 3])));
    }

    #[test]
    fn the_frame_is_bolder_than_the_inner_lines_and_never_doubles_one() {
        let frame = region_edges(&WHOLE);
        for e in &frame {
            let across = (0..3).filter(|&i| e.half[i] < 1.0).count();
            assert_eq!(across, 2);
            assert!(e.half.iter().all(|&h| h >= FRAME_R - 1e-6));
            for i in 0..3 {
                let reach = e.center[i].abs() + e.half[i];
                assert!(reach <= 2.0 + FRAME_R + 1e-5);
            }
        }
        const { assert!(FRAME_R > LINE_R) };
        for line in grid_lines(&WHOLE) {
            let on_frame = frame.iter().any(|e| close(e.center, line.center));
            assert!(!on_frame, "{line:?}");
        }
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
        assert_eq!(grid_lines(&top_off).len(), 20 + 20 + 25 - 12);
        let dots = corners(&top_off);
        assert_eq!(dots.len(), 8);
        assert!(dots.iter().all(|d| d[1] <= 1.0 + 1e-5));
        assert!(dots.iter().any(|d| close(*d, [2.0, 1.0, 2.0])));
    }

    #[test]
    fn nearness_fades_from_the_nearest_corner_to_the_farthest() {
        let eye = [8.0, 6.0, 10.0];
        assert!((nearness([2.0; 3], eye, &WHOLE) - 1.0).abs() < 1e-5);
        assert!(nearness([-2.0; 3], eye, &WHOLE).abs() < 1e-5);
        assert!((nearness([0.0; 3], eye, &WHOLE) - 0.5).abs() < 1e-5);
        let front = nearness([0.0, 0.0, 1.5], eye, &WHOLE);
        let back = nearness([0.0, 0.0, -1.5], eye, &WHOLE);
        assert!(front > back);
        // Peeled, the cut side is the new front.
        let top_off = peeled([0.0, 12.0, 0.0]);
        assert!((nearness([0.0, 1.0, 0.0], [0.0, 12.0, 0.0], &top_off) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn by_depth_runs_from_the_far_end_to_the_near_end() {
        assert!((by_depth(1.0, (0.9, 0.3)) - 0.9).abs() < 1e-5);
        assert!((by_depth(0.0, (0.9, 0.3)) - 0.3).abs() < 1e-5);
        assert!((by_depth(0.5, (0.9, 0.3)) - 0.6).abs() < 1e-5);
    }

    #[test]
    fn nearness_in_2d_splits_the_near_and_far_face_of_the_layer() {
        let layer = BoxRegion {
            min: [0, 0, 3],
            max: [3, 3, 3],
        };
        let eye = [0.0, 0.0, 12.0];
        assert!((nearness([1.0, -1.0, 2.0], eye, &layer) - 1.0).abs() < 1e-5);
        assert!(nearness([-1.0, 1.0, 1.0], eye, &layer).abs() < 1e-5);
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
        let edges = box_edges(&p, OUTLINE_R);
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
    fn block_edges_are_thinner_than_a_wrong_box_s_outline() {
        let p = block_pose(&BoxRegion::spanning([0, 0, 0], [1, 0, 0]));
        let edges = box_edges(&p, EDGE_R);
        const { assert!(EDGE_R < OUTLINE_R) };
        for e in &edges {
            assert_eq!(
                (0..3)
                    .filter(|&i| (e.half[i] - EDGE_R).abs() < 1e-6)
                    .count(),
                2
            );
        }
    }

    /// Where a dash along `axis` starts and ends.
    fn span(d: &Pose, axis: usize) -> (f32, f32) {
        (d.center[axis] - d.half[axis], d.center[axis] + d.half[axis])
    }

    #[test]
    fn a_marker_is_dashed_along_all_twelve_edges() {
        let p = Pose {
            center: cell_center([1, 2, 3]),
            half: marker_half(Shape::Tall),
        };
        let edges = box_edges(&p, DASH_R);
        let dashes = dashed_edges(&p, DASH_R);
        for e in &edges {
            let axis = (0..3)
                .find(|&i| e.half[i] > DASH_R)
                .expect("an edge is long");
            let (lo, hi) = span(e, axis);
            let mut on_edge: Vec<(f32, f32)> = dashes
                .iter()
                .filter(|d| (0..3).all(|i| i == axis || (d.center[i] - e.center[i]).abs() < 1e-5))
                .map(|d| span(d, axis))
                .collect();
            on_edge.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert!(on_edge.len() >= 2, "{e:?}");
            // A dash at each end marks the corners; gaps between the dashes read as tentative.
            assert!((on_edge[0].0 - lo).abs() < 1e-5);
            assert!((on_edge[on_edge.len() - 1].1 - hi).abs() < 1e-5);
            for w in on_edge.windows(2) {
                assert!(w[1].0 - w[0].1 > 0.02, "{w:?}");
            }
        }
    }

    #[test]
    fn dashes_are_as_thick_as_asked_and_stay_on_the_marker_s_edges() {
        let p = Pose {
            center: [0.5, 0.5, 0.5],
            half: marker_half(Shape::Flat),
        };
        let r = 1.5 * DASH_R;
        for d in dashed_edges(&p, r) {
            assert_eq!((0..3).filter(|&i| (d.half[i] - r).abs() < 1e-6).count(), 2);
            for i in 0..3 {
                let reach = (d.center[i] - p.center[i]).abs() + d.half[i];
                assert!(reach <= p.half[i] + r + 1e-5);
            }
        }
    }

    #[test]
    fn nearer_markers_are_drawn_with_bolder_lines() {
        assert!(marker_weight(1.0) > marker_weight(0.0));
        // The middle of the cube keeps the plain thickness; the back stays visible.
        assert!((marker_weight(0.5) - 1.0).abs() < 1e-5);
        assert!(marker_weight(0.0) >= 0.5);
        assert!(marker_weight(1.0) <= 1.6);
    }

    #[test]
    fn a_jack_crosses_three_bars_at_the_cell_centre() {
        let c = cell_center([2, 0, 1]);
        let r = 0.5 * JACK_R;
        let parts = jack(c, r);
        assert_eq!(parts.len(), 3 + 6);
        for (axis, b) in parts[..3].iter().enumerate() {
            assert!(close(b.center, c));
            let mut want = [r; 3];
            want[axis] = MARK_LONG;
            assert!(close(b.half, want));
        }
        // A square cap on each end of each bar, wider than the bar, so it reads from afar.
        for cap in &parts[3..] {
            let off: Vec<f32> = (0..3).map(|i| (cap.center[i] - c[i]).abs()).collect();
            assert_eq!(off.iter().filter(|&&d| d < 1e-5).count(), 2);
            assert!(off.iter().any(|&d| (d - MARK_LONG).abs() < 1e-5));
            assert!(
                cap.half
                    .iter()
                    .all(|&h| h > r && (h - cap.half[0]).abs() < 1e-6)
            );
        }
    }

    #[test]
    fn a_peeled_block_is_hatched_on_its_cut_face_only() {
        let top_off = peeled([0.0, 12.0, 0.0]);
        let whole = block_pose(&WHOLE);
        let faces = cut_faces(&whole, &top_off);
        assert_eq!(faces.len(), 1);
        let f = faces[0];
        // Over the cut at y = 1, just outside it, and as wide as the cut block.
        assert!(f.center[1] > 1.0 && f.center[1] - f.half[1] > 1.0);
        assert!(f.center[1] + f.half[1] < 1.0 + 0.01);
        assert!(close([f.center[0], 0.0, f.center[2]], [0.0; 3]));
        assert!((f.half[0] - 1.94).abs() < 1e-5 && (f.half[2] - 1.94).abs() < 1e-5);
    }

    #[test]
    fn uncut_blocks_and_the_cube_s_own_sides_are_not_hatched() {
        let top_off = peeled([0.0, 12.0, 0.0]);
        let below = block_pose(&BoxRegion::spanning([0, 0, 0], [3, 2, 3]));
        assert!(cut_faces(&below, &top_off).is_empty());
        let peeled_away = block_pose(&BoxRegion::spanning([0, 3, 0], [3, 3, 3]));
        assert!(cut_faces(&peeled_away, &top_off).is_empty());
        assert!(cut_faces(&block_pose(&WHOLE), &WHOLE).is_empty());
        // Spread past the cube's sides when solved: cut there, but that is no section.
        let spread = Pose {
            center: [1.8, -1.8, 0.0],
            half: [0.44, 0.44, 0.44],
        };
        assert!(clip(&spread, &top_off) != Some(spread));
        assert!(cut_faces(&spread, &top_off).is_empty());
    }

    #[test]
    fn a_block_shows_the_faces_turned_to_the_eye_the_most_in_view_first() {
        // A flat block seen from above and a little in front: its big top, then its front.
        let flat = Pose {
            center: [0.0, 0.0, 0.0],
            half: [1.0, 0.5, 1.0],
        };
        let faces = faces_toward(&flat, [0.0, 10.0, 6.0]);
        assert_eq!(faces.len(), 2);
        assert!(close(faces[0], [0.0, 0.5, 0.0]), "{faces:?}");
        assert!(close(faces[1], [0.0, 0.0, 1.0]), "{faces:?}");
        // From low in front, the front comes first, though the top is bigger.
        let low = faces_toward(&flat, [0.0, 1.0, 10.0]);
        assert!(close(low[0], [0.0, 0.0, 1.0]), "{low:?}");
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
