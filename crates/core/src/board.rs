use crate::types::{BoxRegion, Cell, Clue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    Overlap,
    ClueCount(usize),
    Volume { expected: u8, got: u8 },
}

/// The Patches rules for one box: no overlap, exactly one clue inside, volume matches that clue.
pub fn check(clues: &[Clue], placed: u64, candidate: &BoxRegion) -> Result<(), Rejection> {
    if candidate.mask() & placed != 0 {
        return Err(Rejection::Overlap);
    }
    let inside: Vec<&Clue> = clues
        .iter()
        .filter(|c| candidate.contains(c.cell))
        .collect();
    if inside.len() != 1 {
        return Err(Rejection::ClueCount(inside.len()));
    }
    match inside[0].volume {
        Some(expected) if expected != candidate.volume() => Err(Rejection::Volume {
            expected,
            got: candidate.volume(),
        }),
        _ => Ok(()),
    }
}

/// A player's in-progress tiling.
#[derive(Clone, Debug)]
pub struct Board {
    clues: Vec<Clue>,
    boxes: Vec<BoxRegion>,
    placed: u64,
}

impl Board {
    pub fn new(clues: Vec<Clue>) -> Self {
        Self {
            clues,
            boxes: Vec::new(),
            placed: 0,
        }
    }

    pub fn clues(&self) -> &[Clue] {
        &self.clues
    }

    pub fn boxes(&self) -> &[BoxRegion] {
        &self.boxes
    }

    pub fn place(&mut self, b: BoxRegion) -> Result<(), Rejection> {
        check(&self.clues, self.placed, &b)?;
        self.placed |= b.mask();
        self.boxes.push(b);
        Ok(())
    }

    /// Index of the placed box covering `cell`, if any.
    pub fn box_at(&self, cell: Cell) -> Option<usize> {
        self.boxes.iter().position(|b| b.contains(cell))
    }

    pub fn remove_at(&mut self, cell: Cell) -> bool {
        let Some(i) = self.box_at(cell) else {
            return false;
        };
        self.placed &= !self.boxes[i].mask();
        self.boxes.remove(i);
        true
    }

    pub fn clear(&mut self) {
        self.boxes.clear();
        self.placed = 0;
    }

    pub fn is_solved(&self) -> bool {
        self.placed == u64::MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Four 4x4x1 slabs, clue in the corner of each.
    fn slab_clues() -> Vec<Clue> {
        (0..4)
            .map(|z| Clue {
                cell: [0, 0, z],
                volume: Some(16),
            })
            .collect()
    }

    fn slab(z: u8) -> BoxRegion {
        BoxRegion {
            min: [0, 0, z],
            max: [3, 3, z],
        }
    }

    #[test]
    fn rejects_overlap() {
        let mut b = Board::new(slab_clues());
        assert_eq!(b.place(slab(0)), Ok(()));
        assert_eq!(b.place(slab(0)), Err(Rejection::Overlap));
    }

    #[test]
    fn rejects_two_clues_and_no_clue() {
        let mut b = Board::new(slab_clues());
        let two = BoxRegion {
            min: [0, 0, 0],
            max: [3, 3, 1],
        };
        assert_eq!(b.place(two), Err(Rejection::ClueCount(2)));
        let none = BoxRegion {
            min: [1, 1, 0],
            max: [3, 3, 0],
        };
        assert_eq!(b.place(none), Err(Rejection::ClueCount(0)));
    }

    #[test]
    fn rejects_wrong_volume_but_accepts_hidden_volume() {
        let mut b = Board::new(slab_clues());
        let half = BoxRegion {
            min: [0, 0, 0],
            max: [3, 1, 0],
        };
        assert_eq!(
            b.place(half),
            Err(Rejection::Volume {
                expected: 16,
                got: 8
            })
        );

        let mut hidden = Board::new(vec![Clue {
            cell: [0, 0, 0],
            volume: None,
        }]);
        assert_eq!(hidden.place(half), Ok(()));
    }

    #[test]
    fn solved_when_every_cell_covered_and_remove_undoes() {
        let mut b = Board::new(slab_clues());
        for z in 0..4 {
            assert!(!b.is_solved());
            b.place(slab(z)).unwrap();
        }
        assert!(b.is_solved());
        assert_eq!(b.box_at([2, 2, 1]), Some(1));

        assert!(b.remove_at([2, 2, 1]));
        assert!(!b.is_solved());
        assert_eq!(b.boxes().len(), 3);
        assert_eq!(b.box_at([2, 2, 1]), None);
        assert!(!b.remove_at([2, 2, 1]));

        b.clear();
        assert!(b.boxes().is_empty());
        assert_eq!(b.place(slab(1)), Ok(()));
    }
}
