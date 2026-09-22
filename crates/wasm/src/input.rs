//! The pointer state machine: turns presses, moves and releases into orbits, previews and
//! box placements. Coordinates are CSS px; the renderer resolves what is under the pointer.

use patches_core::{BoxRegion, Cell};

/// Pointer travel (CSS px) below which a press counts as a click, not a drag.
pub const CLICK_SLOP: f32 = 5.0;
/// How long (ms) a drag must rest on a cell for the box to keep reaching it after the
/// pointer moves on. Cells only passed on the way are not kept.
pub const DWELL_MS: f64 = 250.0;

/// What a press landed on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Nothing,
    Empty(Cell),
    /// A cell of the placed box `region`.
    Block {
        cell: Cell,
        region: BoxRegion,
    },
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
}

struct Press {
    start: (f32, f32),
    last: (f32, f32),
    moved: bool,
    mode: Mode,
    hover: Option<Cell>,
    /// When the pointer reached `hover`.
    hover_since: f64,
}

#[derive(Default)]
pub struct Input {
    /// First corner of a click-click box.
    pending: Option<Cell>,
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
            Target::Block { region, .. } => Mode::Extend {
                block: region,
                extent: region,
            },
        };
        self.press = Some(Press {
            start: at,
            last: at,
            moved: false,
            mode,
            hover: None,
            hover_since: 0.0,
        });
    }

    pub fn pressed(&self) -> bool {
        self.press.is_some()
    }

    pub fn building(&self) -> bool {
        self.press
            .as_ref()
            .is_some_and(|p| !matches!(p.mode, Mode::Orbit))
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
        match (&mut p.mode, hover) {
            (Mode::Orbit, _) if p.moved => Moved::Orbit { dx, dy },
            (Mode::Build { extent, .. } | Mode::Extend { extent, .. }, Some(h))
                if p.hover != Some(h) =>
            {
                if let Some(rested) = p.hover
                    && now - p.hover_since >= DWELL_MS
                {
                    *extent = extent.including(rested);
                }
                p.hover = Some(h);
                p.hover_since = now;
                Moved::Preview(extent.including(h))
            }
            _ => Moved::Nothing,
        }
    }

    pub fn up(&mut self, hover: Option<Cell>) -> Released {
        let Some(p) = self.press.take() else {
            return Released::Nothing;
        };
        // Only the click-click path and an orbit keep the selection, so click A / orbit /
        // click B still works.
        if !matches!(
            (p.mode, p.moved),
            (Mode::Build { .. }, false) | (Mode::Orbit, true)
        ) {
            self.pending = None;
        }
        // A drag released off the grid is cancelled.
        let grown = |extent: BoxRegion| hover.map(|h| extent.including(h));
        match (p.mode, p.moved) {
            (Mode::Build { anchor, .. }, false) => self.click(anchor),
            (Mode::Build { extent, .. }, true) => {
                grown(extent).map_or(Released::Nothing, Released::Place)
            }
            (Mode::Extend { block, .. }, false) => Released::Remove(block),
            (Mode::Extend { block, extent }, true) => {
                grown(extent).map_or(Released::Nothing, |new| Released::Replace {
                    old: block,
                    new,
                })
            }
            (Mode::Orbit, _) => Released::Nothing,
        }
    }

    pub fn cancel(&mut self) {
        self.press = None;
        self.pending = None;
    }

    pub fn pending(&self) -> Option<Cell> {
        self.pending
    }

    fn click(&mut self, cell: Cell) -> Released {
        match self.pending.take() {
            Some(first) => Released::Place(BoxRegion::spanning(first, cell)),
            None => {
                self.pending = Some(cell);
                Released::Nothing
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Cell = [0, 0, 0];
    const B: Cell = [1, 2, 0];
    const C: Cell = [3, 3, 3];

    fn click(input: &mut Input, target: Target, hover: Option<Cell>) -> Released {
        input.down((0.0, 0.0), target);
        input.up(hover)
    }

    fn with_pending(cell: Cell) -> Input {
        let mut input = Input::default();
        assert_eq!(
            click(&mut input, Target::Empty(cell), Some(cell)),
            Released::Nothing
        );
        assert_eq!(input.pending(), Some(cell));
        input
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
    fn orbit_drag_keeps_pending() {
        let mut input = with_pending(A);
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
        assert_eq!(input.pending(), Some(A));
    }

    #[test]
    fn click_empty_then_click_empty_places_the_span() {
        let mut input = with_pending(A);
        assert_eq!(
            click(&mut input, Target::Empty(B), Some(B)),
            Released::Place(BoxRegion::spanning(A, B))
        );
        assert_eq!(input.pending(), None);
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
        assert_eq!(input.pending(), None);
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
    fn drag_released_off_grid_places_nothing_and_clears_pending() {
        let mut input = with_pending(C);
        input.down((0.0, 0.0), Target::Empty(A));
        input.moved((20.0, 0.0), Some(B), 0.0);
        assert_eq!(input.up(None), Released::Nothing);
        assert_eq!(input.pending(), None);
    }

    const BLOCK: BoxRegion = BoxRegion {
        min: [0, 0, 0],
        max: [0, 1, 0],
    };

    fn on_block() -> Target {
        Target::Block {
            cell: A,
            region: BLOCK,
        }
    }

    #[test]
    fn click_on_a_block_removes_it_and_clears_pending() {
        let mut input = with_pending(C);
        assert_eq!(
            click(&mut input, on_block(), Some(A)),
            Released::Remove(BLOCK)
        );
        assert_eq!(input.pending(), None);
    }

    #[test]
    fn drag_from_a_block_extends_it() {
        let mut input = Input::default();
        input.down((0.0, 0.0), on_block());
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
        input.down((0.0, 0.0), on_block());
        input.moved((20.0, 0.0), Some([2, 0, 0]), 0.0);
        assert_eq!(input.up(None), Released::Nothing);
    }

    #[test]
    fn click_on_nothing_clears_pending() {
        let mut input = with_pending(A);
        assert_eq!(click(&mut input, Target::Nothing, None), Released::Nothing);
        assert_eq!(input.pending(), None);
    }

    #[test]
    fn move_inside_click_slop_still_counts_as_a_click() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Nothing);
        assert_eq!(input.moved((3.0, 2.0), None, 0.0), Moved::Nothing);
        input.up(None);

        input.down((0.0, 0.0), Target::Empty(A));
        input.moved((3.0, 2.0), Some(A), 0.0);
        assert_eq!(input.up(Some(A)), Released::Nothing);
        assert_eq!(input.pending(), Some(A));
    }
}
