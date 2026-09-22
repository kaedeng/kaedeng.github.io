//! The pointer state machine: turns presses, moves and releases into orbits, previews and
//! box placements. Coordinates are CSS px; the renderer resolves what is under the pointer.

use patches_core::{BoxRegion, Cell};

/// Pointer travel (CSS px) below which a press counts as a click, not a drag.
pub const CLICK_SLOP: f32 = 5.0;

/// What a press landed on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Nothing,
    Empty(Cell),
    Block(Cell),
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
}

/// The anchor cell of a build. `Rebuild` means the press landed on a block, which the
/// renderer has already removed.
#[derive(Clone, Copy)]
enum Mode {
    Orbit,
    Build(Cell),
    Rebuild(Cell),
}

struct Press {
    start: (f32, f32),
    last: (f32, f32),
    moved: bool,
    mode: Mode,
    hover: Option<Cell>,
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
            Target::Empty(c) => Mode::Build(c),
            Target::Block(c) => Mode::Rebuild(c),
        };
        self.press = Some(Press {
            start: at,
            last: at,
            moved: false,
            mode,
            hover: None,
        });
    }

    pub fn building(&self) -> bool {
        self.press
            .as_ref()
            .is_some_and(|p| !matches!(p.mode, Mode::Orbit))
    }

    /// `hover` is the cell under the pointer, looked up only while building. Over the gap
    /// between layers it is `None` and the last preview stays.
    pub fn moved(&mut self, at: (f32, f32), hover: Option<Cell>) -> Moved {
        let Some(p) = self.press.as_mut() else {
            return Moved::Nothing;
        };
        let (dx, dy) = (at.0 - p.last.0, at.1 - p.last.1);
        p.last = at;
        p.moved |= (at.0 - p.start.0).hypot(at.1 - p.start.1) >= CLICK_SLOP;
        match (p.mode, hover) {
            (Mode::Orbit, _) if p.moved => Moved::Orbit { dx, dy },
            (Mode::Build(a) | Mode::Rebuild(a), Some(h)) if p.hover != Some(h) => {
                p.hover = Some(h);
                Moved::Preview(BoxRegion::spanning(a, h))
            }
            _ => Moved::Nothing,
        }
    }

    pub fn up(&mut self, hover: Option<Cell>) -> Released {
        let Some(p) = self.press.take() else {
            return Released::Nothing;
        };
        match (p.mode, p.moved) {
            (Mode::Build(a), false) => self.click(a),
            (Mode::Build(a) | Mode::Rebuild(a), true) => self.drag_end(a, hover),
            // An orbit keeps the selection, so click A / orbit / click B still works.
            (Mode::Orbit, true) => Released::Nothing,
            // A click on empty space clears the selection; a click on a block only removed it.
            (Mode::Orbit | Mode::Rebuild(_), false) => {
                self.pending = None;
                Released::Nothing
            }
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

    /// Released off the grid means cancelled.
    fn drag_end(&mut self, anchor: Cell, hover: Option<Cell>) -> Released {
        self.pending = None;
        hover.map_or(Released::Nothing, |h| {
            Released::Place(BoxRegion::spanning(anchor, h))
        })
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
    fn orbit_drag_keeps_pending() {
        let mut input = with_pending(A);
        input.down((10.0, 10.0), Target::Nothing);
        assert!(!input.building());
        assert_eq!(
            input.moved((30.0, 10.0), None),
            Moved::Orbit { dx: 20.0, dy: 0.0 }
        );
        assert_eq!(
            input.moved((30.0, 13.0), None),
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
            input.moved((10.0, 0.0), Some(A)),
            Moved::Preview(BoxRegion::spanning(A, A))
        );
        assert_eq!(input.moved((12.0, 0.0), Some(A)), Moved::Nothing);
        assert_eq!(
            input.moved((20.0, 0.0), Some(B)),
            Moved::Preview(BoxRegion::spanning(A, B))
        );
        assert_eq!(input.moved((25.0, 0.0), None), Moved::Nothing);
        assert_eq!(
            input.up(Some(B)),
            Released::Place(BoxRegion::spanning(A, B))
        );
        assert_eq!(input.pending(), None);
        assert!(!input.building());
    }

    #[test]
    fn drag_released_off_grid_places_nothing_and_clears_pending() {
        let mut input = with_pending(C);
        input.down((0.0, 0.0), Target::Empty(A));
        input.moved((20.0, 0.0), Some(B));
        assert_eq!(input.up(None), Released::Nothing);
        assert_eq!(input.pending(), None);
    }

    #[test]
    fn press_on_block_then_release_without_moving_places_nothing_and_clears_pending() {
        let mut input = with_pending(C);
        assert_eq!(
            click(&mut input, Target::Block(A), Some(A)),
            Released::Nothing
        );
        assert_eq!(input.pending(), None);
    }

    #[test]
    fn drag_from_block_previews_from_that_cell_and_places() {
        let mut input = Input::default();
        input.down((0.0, 0.0), Target::Block(A));
        assert!(input.building());
        assert_eq!(
            input.moved((20.0, 0.0), Some(B)),
            Moved::Preview(BoxRegion::spanning(A, B))
        );
        assert_eq!(
            input.up(Some(B)),
            Released::Place(BoxRegion::spanning(A, B))
        );
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
        assert_eq!(input.moved((3.0, 2.0), None), Moved::Nothing);
        input.up(None);

        input.down((0.0, 0.0), Target::Empty(A));
        input.moved((3.0, 2.0), Some(A));
        assert_eq!(input.up(Some(A)), Released::Nothing);
        assert_eq!(input.pending(), Some(A));
    }
}
