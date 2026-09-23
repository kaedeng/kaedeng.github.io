//! The pointer state machine: turns presses, moves and releases into orbits, previews and
//! box placements, and ends a press held still, so it can lock a box. Coordinates are CSS
//! px; the renderer resolves what is under the pointer and times the hold.

use patches_core::{BoxRegion, Cell};

/// Pointer travel (CSS px) below which a press counts as a click, not a drag.
pub const CLICK_SLOP: f32 = 5.0;
/// How far (CSS px) the pointer can wobble and still be resting.
pub const REST_SLOP: f32 = 5.0;
/// How long (ms) a drag must rest, staying within `REST_SLOP`, for the box to keep reaching
/// the cell under it after the pointer moves on. Cells only passed on the way are not kept,
/// however slowly the pointer crossed them.
pub const DWELL_MS: f64 = 400.0;

/// What a press landed on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Nothing,
    Empty(Cell),
    /// A cell of this placed box.
    Block(BoxRegion),
    /// A locked box with no open box behind it: the pointer passes through locked boxes,
    /// so this is all it found.
    Locked,
}

#[derive(Debug, PartialEq)]
pub enum Moved {
    Nothing,
    Orbit { dx: f32, dy: f32 },
    Preview(BoxRegion),
}

#[derive(Debug, PartialEq)]
pub enum Released {
    Nothing,
    /// A click on this empty cell: the same as Space there, starting a box or finishing the
    /// one being drawn, whether a click or the keyboard began it.
    Tap(Cell),
    /// A click on nothing, or a drag let go off the cube: drop any box half drawn.
    Cancel,
    Place(BoxRegion),
    Remove(BoxRegion),
    /// Grow placed box `old` into `new`.
    Replace {
        old: BoxRegion,
        new: BoxRegion,
    },
}

/// While building, `extent` covers where the drag started and every cell it rested on, so
/// moving back never shrinks it.
#[derive(Clone, Copy)]
enum Mode {
    Orbit,
    /// A new box from the empty cell `anchor`.
    Build {
        anchor: Cell,
        extent: BoxRegion,
    },
    /// Pressed on placed box `block`: a click removes it, a drag grows it.
    Extend {
        block: BoxRegion,
        extent: BoxRegion,
    },
    /// Pressed on a locked box with nothing behind it: a drag turns the cube, a click does
    /// nothing.
    Locked,
}

/// Where a drag last came to a stop.
struct Rest {
    at: (f32, f32),
    since: f64,
    /// The cell under the pointer there; `None` off the cube.
    cell: Option<Cell>,
}

impl Rest {
    /// Follows the pointer. Returns the cell it rested on when it leaves a rest that lasted
    /// `DWELL_MS`.
    fn follow(&mut self, at: (f32, f32), cell: Option<Cell>, now: f64) -> Option<Cell> {
        if (at.0 - self.at.0).hypot(at.1 - self.at.1) < REST_SLOP {
            self.cell = cell;
            return None;
        }
        let rested = self.cell.filter(|_| now - self.since >= DWELL_MS);
        *self = Rest {
            at,
            since: now,
            cell,
        };
        rested
    }
}

struct Press {
    start: (f32, f32),
    last: (f32, f32),
    moved: bool,
    mode: Mode,
    hover: Option<Cell>,
    rest: Rest,
}

#[derive(Default)]
pub struct Input {
    press: Option<Press>,
}

impl Input {
    pub fn down(&mut self, at: (f32, f32), target: Target) {
        let mode = match target {
            Target::Nothing => Mode::Orbit,
            Target::Empty(c) => Mode::Build {
                anchor: c,
                extent: BoxRegion::spanning(c, c),
            },
            Target::Block(region) => Mode::Extend {
                block: region,
                extent: region,
            },
            Target::Locked => Mode::Locked,
        };
        self.press = Some(Press {
            start: at,
            last: at,
            moved: false,
            mode,
            hover: None,
            // Resting where it was pressed never counts: that cell is in the box already.
            rest: Rest {
                at,
                since: f64::INFINITY,
                cell: None,
            },
        });
    }

    pub fn pressed(&self) -> bool {
        self.press.is_some()
    }

    pub fn building(&self) -> bool {
        self.press
            .as_ref()
            .is_some_and(|p| !matches!(p.mode, Mode::Orbit | Mode::Locked))
    }

    /// Ends a press still held where it landed, e.g. to lock the box there instead.
    /// Returns true when it did.
    pub fn hold(&mut self) -> bool {
        let still = self.press.as_ref().is_some_and(|p| !p.moved);
        if still {
            self.press = None;
        }
        still
    }

    /// The cell a build last reached: where it last hovered, else where it began. Letting
    /// go over a locked box with nothing behind it finishes there.
    pub fn hovered(&self) -> Option<Cell> {
        let p = self.press.as_ref()?;
        match p.mode {
            Mode::Build { anchor, .. } => p.hover.or(Some(anchor)),
            Mode::Extend { extent, .. } => p.hover.or(Some(extent.min)),
            Mode::Orbit | Mode::Locked => None,
        }
    }

    /// `hover` is the cell under the pointer, looked up only while building; off the cube it
    /// is `None` and the last preview stays. `now` is the event time in ms.
    pub fn moved(&mut self, at: (f32, f32), hover: Option<Cell>, now: f64) -> Moved {
        let Some(p) = self.press.as_mut() else {
            return Moved::Nothing;
        };
        let (dx, dy) = (at.0 - p.last.0, at.1 - p.last.1);
        p.last = at;
        p.moved |= (at.0 - p.start.0).hypot(at.1 - p.start.1) >= CLICK_SLOP;
        let extent = match &mut p.mode {
            Mode::Orbit | Mode::Locked if p.moved => return Moved::Orbit { dx, dy },
            Mode::Orbit | Mode::Locked => return Moved::Nothing,
            Mode::Build { extent, .. } | Mode::Extend { extent, .. } => extent,
        };
        if let Some(rested) = p.rest.follow(at, hover, now) {
            *extent = extent.including(rested);
        }
        match hover {
            Some(h) if p.hover != Some(h) => {
                p.hover = Some(h);
                Moved::Preview(extent.including(h))
            }
            _ => Moved::Nothing,
        }
    }

    pub fn up(&mut self, hover: Option<Cell>) -> Released {
        let Some(p) = self.press.take() else {
            return Released::Nothing;
        };
        // A drag released off the grid is cancelled.
        let grown = |extent: BoxRegion| hover.map(|h| extent.including(h));
        match (p.mode, p.moved) {
            (Mode::Build { anchor, .. }, false) => Released::Tap(anchor),
            (Mode::Build { extent, .. }, true) => {
                grown(extent).map_or(Released::Cancel, Released::Place)
            }
            (Mode::Extend { block, .. }, false) => Released::Remove(block),
            (Mode::Extend { block, extent }, true) => {
                grown(extent).map_or(Released::Cancel, |new| Released::Replace {
                    old: block,
                    new,
                })
            }
            // An orbit keeps a box half drawn, so click A / turn / click B still works.
            (Mode::Orbit, true) | (Mode::Locked, _) => Released::Nothing,
            (Mode::Orbit, false) => Released::Cancel,
        }
    }

    pub fn cancel(&mut self) {
        self.press = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Cell = [0, 0, 0];
    const B: Cell = [1, 2, 0];

    fn click(input: &mut Input, target: Target, hover: Option<Cell>) -> Released {
        input.down((0.0, 0.0), target);
        input.up(hover)
    }

    #[test]
    fn pressed_from_down_to_up() {
        let mut input = Input::default();
        assert!(!input.pressed());
        input.down((0.0, 0.0), Target::Nothing);
        assert!(input.pressed());
        input.up(None);
        assert!(!input.pressed());
    }

    #[test]
    fn a_click_on_an_empty_cell_is_a_tap_there() {
        let mut input = Input::default();
        assert_eq!(
            click(&mut input, Target::Empty(A), Some(A)),
            Released::Tap(A)
        );
    }

    #[test]
    fn an_orbit_drag_leaves_a_box_half_drawn_alone() {
        let mut input = Input::default();
        input.down((10.0, 10.0), Target::Nothing);
        assert!(!input.building());
        assert_eq!(
            input.moved((30.0, 10.0), None, 0.0),
            Moved::Orbit { dx: 20.0, dy: 0.0 }
        );
        assert_eq!(
            input.moved((30.0, 13.0), None, 0.0),
            Moved::Orbit { dx: 0.0, dy: 3.0 }
        );
        assert_eq!(input.up(None), Released::Nothing);
    }

    #[test]
    fn drag_from_empty_previews_per_hover_change_and_places_on_release() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Empty(A));
        assert!(input.building());
        assert_eq!(
            input.moved((10.0, 0.0), Some(A), 0.0),
            Moved::Preview(BoxRegion::spanning(A, A))
        );
        assert_eq!(input.moved((12.0, 0.0), Some(A), 0.0), Moved::Nothing);
        assert_eq!(
            input.moved((20.0, 0.0), Some(B), 0.0),
            Moved::Preview(BoxRegion::spanning(A, B))
        );
        assert_eq!(input.moved((25.0, 0.0), None, 0.0), Moved::Nothing);
        assert_eq!(
            input.up(Some(B)),
            Released::Place(BoxRegion::spanning(A, B))
        );
        assert!(!input.building());
    }

    #[test]
    fn drag_remembers_a_corner_it_rested_on() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Empty(A));
        input.moved((20.0, 0.0), Some([3, 0, 0]), 0.0);
        assert_eq!(
            input.moved((10.0, 20.0), Some([0, 0, 3]), DWELL_MS),
            Moved::Preview(BoxRegion::spanning(A, [3, 0, 3]))
        );
        // Back at the anchor: the corner it rested on stays, the one it just passed does not.
        let wide = BoxRegion::spanning(A, [3, 0, 0]);
        assert_eq!(
            input.moved((0.0, 5.0), Some(A), DWELL_MS + 10.0),
            Moved::Preview(wide)
        );
        assert_eq!(input.up(Some(A)), Released::Place(wide));
    }

    #[test]
    fn drag_forgets_cells_it_only_passed() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Empty(A));
        input.moved((20.0, 0.0), Some([0, 0, 2]), 0.0);
        input.moved((40.0, 0.0), Some(B), 50.0);
        assert_eq!(
            input.up(Some(B)),
            Released::Place(BoxRegion::spanning(A, B))
        );
    }

    #[test]
    fn drag_forgets_a_cell_it_crossed_slowly() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Empty(A));
        // Still moving, just slowly: 600 ms over one cell is not a rest.
        for i in 1..=7 {
            input.moved(
                (10.0 * i as f32, 0.0),
                Some([3, 0, 0]),
                100.0 * (i - 1) as f64,
            );
        }
        input.moved((100.0, 0.0), Some(B), 650.0);
        assert_eq!(
            input.up(Some(B)),
            Released::Place(BoxRegion::spanning(A, B))
        );
    }

    #[test]
    fn time_off_the_cube_is_not_a_rest_on_the_last_cell() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Empty(A));
        input.moved((20.0, 0.0), Some([3, 0, 0]), 0.0);
        input.moved((300.0, 0.0), None, 50.0);
        input.moved((310.0, 0.0), None, 1000.0);
        input.moved((40.0, 40.0), Some(B), 1050.0);
        assert_eq!(
            input.up(Some(B)),
            Released::Place(BoxRegion::spanning(A, B))
        );
    }

    #[test]
    fn jitter_at_the_press_does_not_keep_a_neighbouring_cell() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Empty(A));
        input.moved((2.0, 0.0), Some([0, 0, 1]), 1000.0);
        input.moved((40.0, 40.0), Some(B), 1100.0);
        assert_eq!(
            input.up(Some(B)),
            Released::Place(BoxRegion::spanning(A, B))
        );
    }

    #[test]
    fn a_rest_survives_hand_jitter() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Empty(A));
        input.moved((20.0, 0.0), Some([3, 0, 0]), 0.0);
        input.moved((21.0, 1.0), Some([3, 0, 0]), 200.0);
        input.moved((20.0, 2.0), Some([3, 0, 0]), 350.0);
        input.moved((40.0, 40.0), Some(B), DWELL_MS + 50.0);
        assert_eq!(
            input.up(Some(B)),
            Released::Place(BoxRegion::spanning(A, [3, 2, 0]))
        );
    }

    #[test]
    fn drag_released_off_grid_places_nothing_and_cancels() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Empty(A));
        input.moved((20.0, 0.0), Some(B), 0.0);
        assert_eq!(input.up(None), Released::Cancel);
    }

    const BLOCK: BoxRegion = BoxRegion {
        min: [0, 0, 0],
        max: [0, 1, 0],
    };

    #[test]
    fn click_on_a_block_removes_it() {
        let mut input = Input::default();
        assert_eq!(
            click(&mut input, Target::Block(BLOCK), Some(A)),
            Released::Remove(BLOCK)
        );
    }

    #[test]
    fn drag_from_a_block_extends_it() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Block(BLOCK));
        assert!(input.building());
        let grown = BoxRegion::spanning(A, [2, 1, 0]);
        assert_eq!(
            input.moved((20.0, 0.0), Some([2, 0, 0]), 0.0),
            Moved::Preview(grown)
        );
        assert_eq!(
            input.up(Some([2, 0, 0])),
            Released::Replace {
                old: BLOCK,
                new: grown
            }
        );
    }

    #[test]
    fn drag_from_a_block_released_off_grid_keeps_it() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Block(BLOCK));
        input.moved((20.0, 0.0), Some([2, 0, 0]), 0.0);
        assert_eq!(input.up(None), Released::Cancel);
    }

    #[test]
    fn a_click_on_a_locked_box_with_nothing_behind_it_does_nothing() {
        let mut input = Input::default();
        assert_eq!(
            click(&mut input, Target::Locked, Some(A)),
            Released::Nothing
        );
    }

    #[test]
    fn a_drag_from_a_locked_box_with_nothing_behind_it_turns_the_cube() {
        let mut input = Input::default();
        input.down((10.0, 10.0), Target::Locked);
        assert!(!input.building());
        assert_eq!(
            input.moved((30.0, 10.0), None, 0.0),
            Moved::Orbit { dx: 20.0, dy: 0.0 }
        );
        assert_eq!(input.up(Some(A)), Released::Nothing);
    }

    #[test]
    fn a_hold_ends_a_press_that_has_not_moved() {
        for target in [Target::Block(BLOCK), Target::Locked, Target::Empty(A)] {
            let mut input = Input::default();
            input.down((0.0, 0.0), target);
            input.moved((2.0, 1.0), Some(A), 300.0);
            assert!(input.hold());
            assert!(!input.pressed());
            assert_eq!(input.up(Some(A)), Released::Nothing);
        }
    }

    #[test]
    fn a_hold_after_the_pointer_moved_leaves_the_press() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Block(BLOCK));
        input.moved((20.0, 0.0), Some([2, 0, 0]), 0.0);
        assert!(!input.hold());
        assert!(input.pressed());
        assert!(!Input::default().hold());
    }

    #[test]
    fn a_drag_let_go_where_it_last_was_finishes_there() {
        // Over a locked box with nothing behind it, the drag stays on the last cell it
        // reached, and letting go there builds that box.
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Empty(A));
        assert_eq!(input.hovered(), Some(A));
        input.moved((20.0, 0.0), Some(B), 0.0);
        input.moved((40.0, 0.0), None, 10.0);
        assert_eq!(input.hovered(), Some(B));
        let last = input.hovered();
        assert_eq!(input.up(last), Released::Place(BoxRegion::spanning(A, B)));
        input.down((0.0, 0.0), Target::Block(BLOCK));
        assert_eq!(input.hovered(), Some(BLOCK.min));
        input.down((0.0, 0.0), Target::Nothing);
        assert_eq!(input.hovered(), None);
    }

    #[test]
    fn click_on_nothing_cancels() {
        let mut input = Input::default();
        assert_eq!(click(&mut input, Target::Nothing, None), Released::Cancel);
    }

    #[test]
    fn move_inside_click_slop_still_counts_as_a_click() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Nothing);
        assert_eq!(input.moved((3.0, 2.0), None, 0.0), Moved::Nothing);
        input.up(None);

        input.down((0.0, 0.0), Target::Empty(A));
        input.moved((3.0, 2.0), Some(A), 0.0);
        assert_eq!(input.up(Some(A)), Released::Tap(A));
    }
}
