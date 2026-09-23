//! Keyboard play: a cursor cell moved with the arrow keys, and boxes started and placed
//! from it. Typing `:vim` switches on a hidden vim-style key set (hjkl, counts, v and y,
//! dd, gg, ...); `:q` switches it off. JavaScript forwards `KeyboardEvent.key` and whether
//! Shift is held.

use patches_core::{BoxRegion, Cell};

use crate::input::Released;

/// A move as seen on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dir {
    Left,
    Right,
    Away,
    Toward,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cmd {
    Move(Dir, u8),
    /// Turn the cube by this many eighths of a circle.
    Turn(i8),
    Zoom(i8),
    /// Start a box at the cursor, or place the one being drawn.
    Select,
    Remove,
    Cancel,
    /// Jump to the next (1) or previous (-1) clue.
    Clue(i8),
    /// Jump to the top or bottom shown layer.
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Press {
    /// Not a board key; let the page have it.
    Ignored,
    /// Part of a longer command, or a no-op.
    Consumed,
    Run(Cmd),
}

/// Arrow keys: the move without Shift, and the command with it.
const ARROWS: [(&str, Dir, Cmd); 4] = [
    ("ArrowLeft", Dir::Left, Cmd::Turn(-1)),
    ("ArrowRight", Dir::Right, Cmd::Turn(1)),
    ("ArrowUp", Dir::Away, Cmd::Move(Dir::Up, 1)),
    ("ArrowDown", Dir::Toward, Cmd::Move(Dir::Down, 1)),
];

const PLAIN: [(&str, Cmd); 9] = [
    (" ", Cmd::Select),
    ("Enter", Cmd::Select),
    ("Delete", Cmd::Remove),
    ("Backspace", Cmd::Remove),
    ("Escape", Cmd::Cancel),
    ("+", Cmd::Zoom(1)),
    ("=", Cmd::Zoom(1)),
    ("-", Cmd::Zoom(-1)),
    ("_", Cmd::Zoom(-1)),
];

/// Vim motions; a count repeats them.
const VIM_MOVES: [(&str, Dir); 6] = [
    ("h", Dir::Left),
    ("l", Dir::Right),
    ("k", Dir::Away),
    ("j", Dir::Toward),
    ("K", Dir::Up),
    ("J", Dir::Down),
];

const VIM_CMDS: [(&str, Cmd); 6] = [
    ("H", Cmd::Turn(-1)),
    ("L", Cmd::Turn(1)),
    ("G", Cmd::Bottom),
    ("x", Cmd::Remove),
    ("w", Cmd::Clue(1)),
    ("b", Cmd::Clue(-1)),
];

#[derive(Default)]
pub struct Keys {
    vim: bool,
    /// Text typed after `:`, while the command line is open.
    command: Option<String>,
    /// A vim count, then maybe the first key of `gg` or `dd`.
    pending: String,
}

impl Keys {
    /// `selecting`: a box is being drawn from the keyboard.
    pub fn press(&mut self, key: &str, shift: bool, selecting: bool) -> Press {
        if let Some(text) = self.command.take() {
            return self.command_key(text, key);
        }
        if key == ":" {
            self.command = Some(String::new());
            self.pending.clear();
            return Press::Consumed;
        }
        if let Some(cmd) = plain(key, shift) {
            self.pending.clear();
            return Press::Run(cmd);
        }
        if self.vim {
            self.vim_key(key, selecting)
        } else {
            Press::Ignored
        }
    }

    /// What a vim status line would show; empty outside vim mode.
    pub fn mode_line(&self, selecting: bool) -> String {
        if let Some(text) = &self.command {
            return format!(":{text}");
        }
        if !self.vim {
            return String::new();
        }
        let mode = if selecting {
            "-- VISUAL --"
        } else {
            "-- NORMAL --"
        };
        if self.pending.is_empty() {
            mode.to_string()
        } else {
            format!("{mode}  {}", self.pending)
        }
    }

    fn command_key(&mut self, mut text: String, key: &str) -> Press {
        match key {
            "Enter" => self.run_command(&text),
            "Escape" => {}
            // Backspace on an empty line closes it, as in vim.
            "Backspace" => {
                if text.pop().is_some() {
                    self.command = Some(text);
                }
            }
            k if k.chars().count() == 1 => {
                text.push_str(k);
                self.command = Some(text);
            }
            _ => self.command = Some(text),
        }
        Press::Consumed
    }

    fn run_command(&mut self, text: &str) {
        match text {
            "vim" => self.vim = true,
            "q" | "q!" | "wq" | "x" => self.vim = false,
            _ => {}
        }
    }

    /// A digit of a count: 1-9 start one, 0 only continues one.
    fn is_count(&self, key: &str) -> bool {
        let digit = key.len() == 1 && key.as_bytes()[0].is_ascii_digit();
        digit && (key != "0" || !self.pending.is_empty())
    }

    fn vim_key(&mut self, key: &str, selecting: bool) -> Press {
        let prefix = self.pending.chars().last().filter(|c| !c.is_ascii_digit());
        if prefix.is_none() && self.is_count(key) {
            self.pending.push_str(key);
            return Press::Consumed;
        }
        let count = self
            .pending
            .trim_end_matches(|c: char| !c.is_ascii_digit())
            .parse()
            .map_or(1, |n: u32| n.min(9) as u8);
        let result = match (prefix, key) {
            (Some('g'), "g") => Press::Run(Cmd::Top),
            (Some('d'), "d") => Press::Run(Cmd::Remove),
            (None, "g" | "d") => {
                self.pending.push_str(key);
                return Press::Consumed;
            }
            (None, k) => vim_single(k, count, selecting),
            // A sequence that doesn't finish just clears.
            _ => Press::Consumed,
        };
        self.pending.clear();
        result
    }
}

fn plain(key: &str, shift: bool) -> Option<Cmd> {
    if let Some(&(_, dir, shifted)) = ARROWS.iter().find(|(k, _, _)| *k == key) {
        return Some(if shift { shifted } else { Cmd::Move(dir, 1) });
    }
    PLAIN.iter().find(|(k, _)| *k == key).map(|&(_, cmd)| cmd)
}

fn vim_single(key: &str, count: u8, selecting: bool) -> Press {
    if let Some(&(_, dir)) = VIM_MOVES.iter().find(|(k, _)| *k == key) {
        return Press::Run(Cmd::Move(dir, count));
    }
    if let Some(&(_, cmd)) = VIM_CMDS.iter().find(|(k, _)| *k == key) {
        return Press::Run(cmd);
    }
    match (key, selecting) {
        ("v", true) => Press::Run(Cmd::Cancel),
        ("v", false) | ("y", true) => Press::Run(Cmd::Select),
        ("y", false) => Press::Consumed,
        _ => Press::Ignored,
    }
}

/// The cube axis and direction a screen move goes along, for a camera at `yaw`: left and
/// right follow whichever floor axis runs most across the screen, away and toward the
/// other one.
pub fn axis_for(dir: Dir, yaw: f32) -> (usize, i8) {
    let (s, c) = yaw.sin_cos();
    let sign = |v: f32| if v >= 0.0 { 1 } else { -1 };
    // Screen right is (cos, 0, -sin); into the screen is (-sin, 0, -cos).
    let (right, away) = if c.abs() >= s.abs() {
        ((0, sign(c)), (2, sign(-c)))
    } else {
        ((2, sign(-s)), (0, sign(-s)))
    };
    match dir {
        Dir::Right => right,
        Dir::Left => (right.0, -right.1),
        Dir::Away => away,
        Dir::Toward => (away.0, -away.1),
        Dir::Up => (1, 1),
        Dir::Down => (1, -1),
    }
}

#[derive(Clone, Copy)]
enum Sel {
    /// A new box from this corner.
    New(Cell),
    /// Growing this placed box.
    Grow(BoxRegion),
}

pub struct Cursor {
    pub cell: Cell,
    sel: Option<Sel>,
}

impl Cursor {
    pub fn new(cell: Cell) -> Self {
        Self { cell, sel: None }
    }

    /// Moves `delta` cells along `axis`, stopping at the edge of the shown cells.
    pub fn step(&mut self, axis: usize, delta: i8, shown: &BoxRegion) {
        let v = i16::from(self.cell[axis]) + i16::from(delta);
        self.cell[axis] = v.clamp(i16::from(shown.min[axis]), i16::from(shown.max[axis])) as u8;
    }

    /// Pulls the cursor back onto the shown cells, e.g. after a layer is peeled.
    pub fn clamp(&mut self, shown: &BoxRegion) {
        for i in 0..3 {
            self.cell[i] = self.cell[i].clamp(shown.min[i], shown.max[i]);
        }
    }

    pub fn selecting(&self) -> bool {
        self.sel.is_some()
    }

    /// The box being drawn, if any.
    pub fn selection(&self) -> Option<BoxRegion> {
        self.sel.map(|s| match s {
            Sel::New(a) => BoxRegion::spanning(a, self.cell),
            Sel::Grow(b) => b.including(self.cell),
        })
    }

    /// Starts a box at the cursor (growing `block` if the cursor is on one), or finishes the
    /// one being drawn.
    pub fn select(&mut self, block: Option<BoxRegion>) -> Released {
        match self.sel.take() {
            None => {
                self.sel = Some(block.map_or(Sel::New(self.cell), Sel::Grow));
                Released::Nothing
            }
            Some(Sel::New(a)) => Released::Place(BoxRegion::spanning(a, self.cell)),
            Some(Sel::Grow(b)) => Released::Replace {
                old: b,
                new: b.including(self.cell),
            },
        }
    }

    pub fn cancel(&mut self) {
        self.sel = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{WHOLE, peeled};
    use std::f32::consts::FRAC_PI_2;

    fn run(keys: &mut Keys, seq: &[&str]) -> Vec<Press> {
        seq.iter().map(|k| keys.press(k, false, false)).collect()
    }

    #[test]
    fn arrows_move_and_shift_arrows_turn_or_change_layer() {
        let mut k = Keys::default();
        assert_eq!(
            k.press("ArrowLeft", false, false),
            Press::Run(Cmd::Move(Dir::Left, 1))
        );
        assert_eq!(
            k.press("ArrowUp", false, false),
            Press::Run(Cmd::Move(Dir::Away, 1))
        );
        assert_eq!(
            k.press("ArrowUp", true, false),
            Press::Run(Cmd::Move(Dir::Up, 1))
        );
        assert_eq!(k.press("ArrowRight", true, false), Press::Run(Cmd::Turn(1)));
        assert_eq!(k.press(" ", false, false), Press::Run(Cmd::Select));
        assert_eq!(k.press("Delete", false, false), Press::Run(Cmd::Remove));
        assert_eq!(k.press("=", false, false), Press::Run(Cmd::Zoom(1)));
        assert_eq!(k.press("Tab", false, false), Press::Ignored);
        assert_eq!(k.press("h", false, false), Press::Ignored);
    }

    #[test]
    fn colon_vim_turns_on_vim_keys_and_colon_q_turns_them_off() {
        let mut k = Keys::default();
        assert_eq!(k.mode_line(false), "");
        run(&mut k, &[":", "v", "i"]);
        assert_eq!(k.mode_line(false), ":vi");
        run(&mut k, &["m", "Enter"]);
        assert_eq!(k.mode_line(false), "-- NORMAL --");
        assert_eq!(k.mode_line(true), "-- VISUAL --");
        assert_eq!(
            k.press("l", false, false),
            Press::Run(Cmd::Move(Dir::Right, 1))
        );
        run(&mut k, &[":", "q", "Enter"]);
        assert_eq!(k.mode_line(false), "");
        assert_eq!(k.press("l", false, false), Press::Ignored);
    }

    #[test]
    fn vim_counts_sequences_and_visual_keys() {
        let mut k = Keys::default();
        run(&mut k, &[":", "v", "i", "m", "Enter"]);
        assert_eq!(k.press("3", false, false), Press::Consumed);
        assert_eq!(k.mode_line(false), "-- NORMAL --  3");
        assert_eq!(
            k.press("j", false, false),
            Press::Run(Cmd::Move(Dir::Toward, 3))
        );
        assert_eq!(k.press("K", true, false), Press::Run(Cmd::Move(Dir::Up, 1)));
        assert_eq!(k.press("H", true, false), Press::Run(Cmd::Turn(-1)));
        assert_eq!(k.press("g", false, false), Press::Consumed);
        assert_eq!(k.press("g", false, false), Press::Run(Cmd::Top));
        assert_eq!(k.press("G", true, false), Press::Run(Cmd::Bottom));
        assert_eq!(k.press("d", false, false), Press::Consumed);
        assert_eq!(k.press("d", false, false), Press::Run(Cmd::Remove));
        assert_eq!(k.press("x", false, false), Press::Run(Cmd::Remove));
        assert_eq!(k.press("w", false, false), Press::Run(Cmd::Clue(1)));
        assert_eq!(k.press("v", false, false), Press::Run(Cmd::Select));
        assert_eq!(k.press("v", false, true), Press::Run(Cmd::Cancel));
        assert_eq!(k.press("y", false, true), Press::Run(Cmd::Select));
        assert_eq!(k.press("y", false, false), Press::Consumed);
    }

    #[test]
    fn escape_clears_a_half_typed_command() {
        let mut k = Keys::default();
        run(&mut k, &[":", "v", "Escape", "m", "Enter"]);
        assert_eq!(k.mode_line(false), "");
    }

    #[test]
    fn screen_directions_follow_the_camera() {
        assert_eq!(axis_for(Dir::Right, 0.0), (0, 1));
        assert_eq!(axis_for(Dir::Away, 0.0), (2, -1));
        assert_eq!(axis_for(Dir::Left, FRAC_PI_2), (2, 1));
        assert_eq!(axis_for(Dir::Away, FRAC_PI_2), (0, -1));
        assert_eq!(axis_for(Dir::Up, 1.0), (1, 1));
    }

    #[test]
    fn cursor_stays_on_shown_cells() {
        let mut c = Cursor::new([3, 3, 3]);
        c.step(0, 1, &WHOLE);
        assert_eq!(c.cell, [3, 3, 3]);
        c.step(1, -5, &WHOLE);
        assert_eq!(c.cell, [3, 0, 3]);
        c.clamp(&peeled([0.0, 0.0, 1.0]));
        assert_eq!(c.cell, [3, 0, 2]);
    }

    #[test]
    fn select_starts_then_places_a_box() {
        let mut c = Cursor::new([0, 0, 0]);
        assert_eq!(c.select(None), Released::Nothing);
        c.step(0, 2, &WHOLE);
        assert_eq!(
            c.selection(),
            Some(BoxRegion::spanning([0, 0, 0], [2, 0, 0]))
        );
        assert_eq!(
            c.select(None),
            Released::Place(BoxRegion::spanning([0, 0, 0], [2, 0, 0]))
        );
        assert_eq!(c.selection(), None);
    }

    #[test]
    fn select_on_a_block_grows_it() {
        let block = BoxRegion::spanning([1, 1, 1], [1, 2, 1]);
        let mut c = Cursor::new([1, 1, 1]);
        assert_eq!(c.select(Some(block)), Released::Nothing);
        c.step(2, 1, &WHOLE);
        assert_eq!(
            c.select(None),
            Released::Replace {
                old: block,
                new: BoxRegion::spanning([1, 1, 1], [1, 2, 2])
            }
        );
        c.select(None);
        c.cancel();
        assert!(!c.selecting());
    }
}
