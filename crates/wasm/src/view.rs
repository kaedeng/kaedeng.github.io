//! The 2D view: one layer of the cube seen straight on from outside one of its six faces,
//! and the orbit angles it shares with the 3D view.

use std::f32::consts::{FRAC_PI_2, PI};

use patches_core::{BoxRegion, N};

use crate::geom::WHOLE;
use crate::keys::Dir;

/// The way out of the screen and the screen's up, for a camera orbiting at `yaw` (turned
/// from +z toward +x) and `pitch` (raised toward +y).
pub fn orbit(yaw: f32, pitch: f32) -> ([f32; 3], [f32; 3]) {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    ([cp * sy, sp, cp * cy], [-sp * sy, cp, -sp * cy])
}

/// The face turned most toward a camera orbiting at `yaw` and `pitch`, as `(axis, sign)`
/// like `Flat::new` takes them: e.g. the view-cube face a demo clicks.
pub fn facing(yaw: f32, pitch: f32) -> (usize, i8) {
    dominant(orbit(yaw, pitch).0)
}

/// One layer of the cube, seen straight on from outside a face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flat {
    /// The axis the camera looks along.
    pub axis: usize,
    /// The side of the cube the camera is on: 1 for +axis, -1 for -axis.
    pub sign: i8,
    /// The shown layer's coordinate on `axis`.
    pub layer: u8,
}

impl Flat {
    /// Looking at the face's own layer.
    pub fn new(axis: usize, sign: i8) -> Self {
        let layer = if sign > 0 { N - 1 } else { 0 };
        Self { axis, sign, layer }
    }

    /// The shown layer's cells.
    pub fn region(&self) -> BoxRegion {
        let mut region = WHOLE;
        region.min[self.axis] = self.layer;
        region.max[self.axis] = self.layer;
        region
    }

    /// How many layers in from the face: 0 is the face's own layer.
    pub fn depth(&self) -> u8 {
        if self.sign > 0 {
            N - 1 - self.layer
        } else {
            self.layer
        }
    }

    /// Moves `delta` layers along the axis, stopping at the cube's sides.
    pub fn step(&mut self, delta: i8) {
        let v = i16::from(self.layer) + i16::from(delta);
        self.layer = v.clamp(0, i16::from(N - 1)) as u8;
    }

    /// The orbit `(yaw, pitch)` that looks straight at the face.
    pub fn angles(&self) -> (f32, f32) {
        let s = f32::from(self.sign);
        match self.axis {
            0 => (s * FRAC_PI_2, 0.0),
            1 => (0.0, s * FRAC_PI_2),
            _ if self.sign > 0 => (0.0, 0.0),
            _ => (PI, 0.0),
        }
    }

    /// The cube axis and direction a screen move goes along. Left, right, away (up the
    /// screen) and toward stay in the layer; up comes out of the screen and down goes in.
    pub fn axis_for(&self, dir: Dir) -> (usize, i8) {
        let (yaw, pitch) = self.angles();
        let (out, up) = orbit(yaw, pitch);
        // Screen right does not depend on the pitch.
        let right = [yaw.cos(), 0.0, -yaw.sin()];
        let (v, flip) = match dir {
            Dir::Right => (right, 1),
            Dir::Left => (right, -1),
            Dir::Away => (up, 1),
            Dir::Toward => (up, -1),
            Dir::Up => (out, 1),
            Dir::Down => (out, -1),
        };
        let (axis, sign) = dominant(v);
        (axis, sign * flip)
    }
}

/// The axis `v` runs most along, and which way.
fn dominant(v: [f32; 3]) -> (usize, i8) {
    let mut axis = 0;
    for i in 1..3 {
        if v[i].abs() > v[axis].abs() {
            axis = i;
        }
    }
    (axis, if v[axis] >= 0.0 { 1 } else { -1 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-5)
    }

    #[test]
    fn orbit_looks_from_the_front_the_side_and_above() {
        let (out, up) = orbit(0.0, 0.0);
        assert!(close(out, [0.0, 0.0, 1.0]) && close(up, [0.0, 1.0, 0.0]));
        let (out, up) = orbit(FRAC_PI_2, 0.0);
        assert!(close(out, [1.0, 0.0, 0.0]) && close(up, [0.0, 1.0, 0.0]));
        // Straight down, the top of the screen is the back of the cube.
        let (out, up) = orbit(0.0, FRAC_PI_2);
        assert!(close(out, [0.0, 1.0, 0.0]) && close(up, [0.0, 0.0, -1.0]));
    }

    #[test]
    fn a_face_shows_its_own_layer_first() {
        let top = Flat::new(1, 1);
        assert_eq!(
            top.region(),
            BoxRegion {
                min: [0, 3, 0],
                max: [3, 3, 3]
            }
        );
        assert_eq!(top.depth(), 0);
        let left = Flat::new(0, -1);
        assert_eq!(
            left.region(),
            BoxRegion {
                min: [0, 0, 0],
                max: [0, 3, 3]
            }
        );
        assert_eq!(left.depth(), 0);
    }

    #[test]
    fn stepping_stops_at_the_far_side() {
        let mut top = Flat::new(1, 1);
        top.step(-1);
        assert_eq!((top.layer, top.depth()), (2, 1));
        top.step(-9);
        assert_eq!((top.layer, top.depth()), (0, 3));
        top.step(9);
        assert_eq!(top.layer, 3);
        let mut left = Flat::new(0, -1);
        left.step(2);
        assert_eq!((left.layer, left.depth()), (2, 2));
    }

    #[test]
    fn every_face_is_seen_head_on() {
        for axis in 0..3 {
            for sign in [1, -1] {
                let (yaw, pitch) = Flat::new(axis, sign).angles();
                let mut want = [0.0; 3];
                want[axis] = f32::from(sign);
                assert!(close(orbit(yaw, pitch).0, want), "{axis} {sign}");
            }
        }
    }

    #[test]
    fn the_face_turned_most_toward_the_camera() {
        // The starting camera sees the front, right and top faces; the front most.
        assert_eq!(facing(0.65, 0.4), (2, 1));
        assert_eq!(facing(1.2, 0.4), (0, 1));
        assert_eq!(facing(0.65, 1.2), (1, 1));
        assert_eq!(facing(PI, -0.2), (2, -1));
        // In 2D it is the face shown, so clicking it again goes back to 3D.
        for axis in 0..3 {
            for sign in [1, -1] {
                let (yaw, pitch) = Flat::new(axis, sign).angles();
                assert_eq!(facing(yaw, pitch), (axis, sign), "{axis} {sign}");
            }
        }
    }

    #[test]
    fn screen_directions_in_a_layer() {
        let top = Flat::new(1, 1);
        assert_eq!(top.axis_for(Dir::Right), (0, 1));
        assert_eq!(top.axis_for(Dir::Away), (2, -1));
        assert_eq!(top.axis_for(Dir::Up), (1, 1));
        assert_eq!(top.axis_for(Dir::Down), (1, -1));
        let front = Flat::new(2, 1);
        assert_eq!(front.axis_for(Dir::Left), (0, -1));
        assert_eq!(front.axis_for(Dir::Away), (1, 1));
        assert_eq!(front.axis_for(Dir::Toward), (1, -1));
        assert_eq!(front.axis_for(Dir::Up), (2, 1));
        assert_eq!(Flat::new(0, 1).axis_for(Dir::Right), (2, -1));
        assert_eq!(Flat::new(2, -1).axis_for(Dir::Right), (0, -1));
        let bottom = Flat::new(1, -1);
        assert_eq!(bottom.axis_for(Dir::Away), (2, 1));
        assert_eq!(bottom.axis_for(Dir::Down), (1, 1));
    }
}
