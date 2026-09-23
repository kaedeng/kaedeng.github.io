//! Short ease-out tweens between two poses, driven by frame time from JavaScript.

use crate::geom::Pose;

pub const DURATION_MS: f32 = 180.0;

/// Cubic ease-out: quick start, gentle landing, no overshoot.
pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// Written as `a * (1 - k) + b * k` so the ends land exactly on `a` and `b`.
fn lerp(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] * (1.0 - k) + b[i] * k)
}

pub struct Anim {
    from: Pose,
    to: Pose,
    t_ms: f32,
}

impl Anim {
    pub fn new(from: Pose, to: Pose) -> Self {
        Self {
            from,
            to,
            t_ms: 0.0,
        }
    }

    /// Head for `to`, starting from wherever the tween is now.
    pub fn retarget(&mut self, to: Pose) {
        *self = Self::new(self.pose(), to);
    }

    /// Advances by `dt_ms`; returns true while still moving.
    pub fn tick(&mut self, dt_ms: f32) -> bool {
        self.t_ms = (self.t_ms + dt_ms).min(DURATION_MS);
        !self.done()
    }

    pub fn done(&self) -> bool {
        self.t_ms >= DURATION_MS
    }

    pub fn pose(&self) -> Pose {
        let k = ease_out(self.t_ms / DURATION_MS);
        Pose {
            center: lerp(self.from.center, self.to.center, k),
            half: lerp(self.from.half, self.to.half, k),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose(c: f32, h: f32) -> Pose {
        Pose {
            center: [c; 3],
            half: [h; 3],
        }
    }

    #[test]
    fn ease_out_starts_fast_and_lands_on_one() {
        assert_eq!(ease_out(0.0), 0.0);
        assert_eq!(ease_out(0.5), 0.875);
        assert_eq!(ease_out(1.0), 1.0);
    }

    #[test]
    fn tick_runs_until_the_duration_then_rests_on_the_target() {
        let (from, to) = (pose(0.0, 0.1), pose(1.0, 0.44));
        let mut a = Anim::new(from, to);
        assert_eq!(a.pose(), from);
        assert!(a.tick(100.0));
        assert!(!a.done());
        assert!(a.tick(79.0));
        assert!(!a.tick(1.0));
        assert!(a.done());
        assert_eq!(a.pose(), to);
        assert!(!a.tick(16.0));
        assert_eq!(a.pose(), to);
    }

    #[test]
    fn retarget_continues_from_the_current_pose() {
        let mut a = Anim::new(pose(0.0, 0.1), pose(1.0, 0.44));
        a.tick(60.0);
        let before = a.pose();
        a.retarget(pose(1.0, 0.0));
        assert_eq!(a.pose(), before);
        assert!(!a.tick(DURATION_MS));
        assert_eq!(a.pose(), pose(1.0, 0.0));
    }
}
