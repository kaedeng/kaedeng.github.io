use crate::types::{BoxRegion, Cell, Clue, Shape};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    Overlap,
    ClueCount(usize),
    Volume { expected: u8, got: u8 },
    Shape { expected: Shape, got: Shape },
}

/// The Patches rule one box breaks on its own, if any: exactly one clue inside, and its
/// volume and shape match that clue where the clue gives them.
pub fn fault(clues: &[Clue], candidate: &BoxRegion) -> Option<Rejection> {
    let inside: Vec<&Clue> = clues
        .iter()
        .filter(|c| candidate.contains(c.cell))
        .collect();
    if inside.len() != 1 {
        return Some(Rejection::ClueCount(inside.len()));
    }
    let clue = inside[0];
    if let Some(expected) = clue.volume
        && expected != candidate.volume()
    {
        return Some(Rejection::Volume {
            expected,
            got: candidate.volume(),
        });
    }
    match clue.shape {
        Some(expected) if expected != candidate.shape() => Some(Rejection::Shape {
            expected,
            got: candidate.shape(),
        }),
        _ => None,
    }
}

/// A player's in-progress tiling. Boxes may break the rules (they show as wrong); they may
/// not overlap.
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

    pub fn boxes(&self) -> &[BoxRegion] {
        &self.boxes
    }

    pub fn place(&mut self, b: BoxRegion) -> Result<(), Rejection> {
        if b.mask() & self.placed != 0 {
            return Err(Rejection::Overlap);
        }
        self.placed |= b.mask();
        self.boxes.push(b);
        Ok(())
    }

    /// Index of the placed box covering `cell`, if any.
    pub fn box_at(&self, cell: Cell) -> Option<usize> {
        self.boxes.iter().position(|b| b.contains(cell))
    }

    /// The rule box `b` breaks, if any.
    pub fn fault(&self, b: &BoxRegion) -> Option<Rejection> {
        fault(&self.clues, b)
    }

    /// Index into the clues of the clue inside `b`, if it holds exactly one.
    pub fn clue_of(&self, b: &BoxRegion) -> Option<usize> {
        let mut inside = (0..self.clues.len()).filter(|&i| b.contains(self.clues[i].cell));
        match (inside.next(), inside.next()) {
            (Some(i), None) => Some(i),
            _ => None,
        }
    }

    pub fn remove_at(&mut self, cell: Cell) -> bool {
        let Some(i) = self.box_at(cell) else {
            return false;
        };
        self.placed &= !self.boxes[i].mask();
        self.boxes.remove(i);
        true
    }

    /// Swaps placed box `old` for `new`, keeping `old` if `new` would overlap another box.
    pub fn replace(&mut self, old: BoxRegion, new: BoxRegion) -> Result<(), Rejection> {
        self.remove_at(old.min);
        self.place(new).inspect_err(|_| {
            self.place(old).expect("the old box fitted where it was");
        })
    }

    pub fn clear(&mut self) {
        self.boxes.clear();
        self.placed = 0;
    }

    /// Every cell covered, and no box breaks a rule.
    pub fn is_solved(&self) -> bool {
        self.placed == u64::MAX && self.boxes.iter().all(|b| self.fault(b).is_none())
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
                shape: None,
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
    fn places_but_faults_two_clues_and_no_clue() {
        let mut b = Board::new(slab_clues());
        let two = BoxRegion {
            min: [0, 0, 0],
            max: [3, 3, 1],
        };
        assert_eq!(b.place(two), Ok(()));
        assert_eq!(b.fault(&two), Some(Rejection::ClueCount(2)));
        let none = BoxRegion {
            min: [1, 1, 2],
            max: [3, 3, 2],
        };
        assert_eq!(b.place(none), Ok(()));
        assert_eq!(b.fault(&none), Some(Rejection::ClueCount(0)));
    }

    #[test]
    fn faults_wrong_volume_but_not_hidden_volume() {
        let half = BoxRegion {
            min: [0, 0, 0],
            max: [3, 1, 0],
        };
        assert_eq!(
            Board::new(slab_clues()).fault(&half),
            Some(Rejection::Volume {
                expected: 16,
                got: 8
            })
        );
        let hidden = Board::new(vec![Clue {
            cell: [0, 0, 0],
            volume: None,
            shape: None,
        }]);
        assert_eq!(hidden.fault(&half), None);
    }

    #[test]
    fn faults_wrong_shape() {
        let clue = |shape| {
            vec![Clue {
                cell: [0, 0, 0],
                volume: None,
                shape: Some(shape),
            }]
        };
        assert_eq!(
            Board::new(clue(Shape::Tall)).fault(&slab(0)),
            Some(Rejection::Shape {
                expected: Shape::Tall,
                got: Shape::WallX
            })
        );
        assert_eq!(Board::new(clue(Shape::WallX)).fault(&slab(0)), None);
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

    #[test]
    fn a_full_cube_with_a_wrong_box_is_not_solved() {
        let mut b = Board::new(slab_clues());
        b.place(slab(0)).unwrap();
        b.place(slab(1)).unwrap();
        b.place(BoxRegion::spanning([0, 0, 2], [3, 3, 3])).unwrap();
        assert!(!b.is_solved());
    }

    #[test]
    fn replace_swaps_a_box_unless_it_would_overlap() {
        let hidden = |cell| Clue {
            cell,
            volume: None,
            shape: None,
        };
        let mut b = Board::new(vec![hidden([0, 0, 0]), hidden([3, 3, 3])]);
        let small = BoxRegion::spanning([0, 0, 0], [1, 0, 0]);
        let bigger = BoxRegion::spanning([0, 0, 0], [1, 1, 0]);
        let other = BoxRegion::spanning([3, 3, 3], [3, 3, 3]);
        b.place(small).unwrap();
        b.place(other).unwrap();
        assert_eq!(b.replace(small, bigger), Ok(()));
        assert_eq!(b.boxes(), &[other, bigger]);
        let everything = BoxRegion::spanning([0, 0, 0], [3, 3, 3]);
        assert_eq!(b.replace(bigger, everything), Err(Rejection::Overlap));
        assert_eq!(b.boxes(), &[other, bigger]);
    }

    #[test]
    fn clue_of_finds_the_one_clue_inside() {
        let b = Board::new(slab_clues());
        assert_eq!(b.clue_of(&slab(2)), Some(2));
        assert_eq!(b.clue_of(&BoxRegion::spanning([0, 0, 0], [0, 0, 1])), None);
        assert_eq!(b.clue_of(&BoxRegion::spanning([1, 1, 1], [1, 1, 1])), None);
    }
}
