use serde::{Deserialize, Serialize};

/// Cells per axis. The whole cube is `N * N * N = 64` cells, one bit each in a `u64`.
pub const N: u8 = 4;
pub const CELLS: usize = 64;

/// `[x, y, z]`, each in `0..N`.
pub type Cell = [u8; 3];

pub fn index(c: Cell) -> usize {
    let n = N as usize;
    c[0] as usize + n * (c[1] as usize + n * c[2] as usize)
}

pub fn cell_at(i: usize) -> Cell {
    let n = N as usize;
    [(i % n) as u8, ((i / n) % n) as u8, (i / (n * n)) as u8]
}

/// Axis-aligned box of cells, both corners inclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BoxRegion {
    pub min: Cell,
    pub max: Cell,
}

impl BoxRegion {
    /// The smallest box containing both cells.
    pub fn spanning(a: Cell, b: Cell) -> Self {
        let mut min = a;
        let mut max = b;
        for i in 0..3 {
            if min[i] > max[i] {
                std::mem::swap(&mut min[i], &mut max[i]);
            }
        }
        Self { min, max }
    }

    /// The smallest box containing this one and `c`.
    pub fn including(&self, c: Cell) -> Self {
        Self {
            min: std::array::from_fn(|i| self.min[i].min(c[i])),
            max: std::array::from_fn(|i| self.max[i].max(c[i])),
        }
    }

    pub fn contains(&self, c: Cell) -> bool {
        (0..3).all(|i| self.min[i] <= c[i] && c[i] <= self.max[i])
    }

    pub fn volume(&self) -> u8 {
        (0..3)
            .map(|i| u32::from(self.max[i] - self.min[i] + 1))
            .product::<u32>() as u8
    }

    /// Which of its axes are longest, as a `Shape`.
    pub fn shape(&self) -> Shape {
        let size: [u8; 3] = std::array::from_fn(|i| self.max[i] - self.min[i] + 1);
        let top = size[0].max(size[1]).max(size[2]);
        let longest = size.map(|s| s == top);
        Shape::ALL
            .into_iter()
            .find(|s| s.longest() == longest)
            .expect("some axis is always longest")
    }

    /// One bit per cell inside the box.
    pub fn mask(&self) -> u64 {
        let mut m = 0u64;
        for z in self.min[2]..=self.max[2] {
            for y in self.min[1]..=self.max[1] {
                for x in self.min[0]..=self.max[0] {
                    m |= 1 << index([x, y, z]);
                }
            }
        }
        m
    }
}

/// A box's longest axes: Patches' square / tall / wide, in 3D. y is up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    /// x, y and z are equal.
    Cube,
    /// y alone is longest.
    Tall,
    /// x alone is longest.
    BarX,
    /// z alone is longest.
    BarZ,
    /// x and z tie for longest: it lies flat.
    Flat,
    /// x and y tie for longest: a wall running along x.
    WallX,
    /// y and z tie for longest: a wall running along z.
    WallZ,
}

impl Shape {
    pub const ALL: [Shape; 7] = [
        Shape::Cube,
        Shape::Tall,
        Shape::BarX,
        Shape::BarZ,
        Shape::Flat,
        Shape::WallX,
        Shape::WallZ,
    ];

    /// Whether x, y and z are among the longest axes.
    pub fn longest(self) -> [bool; 3] {
        match self {
            Shape::Cube => [true, true, true],
            Shape::Tall => [false, true, false],
            Shape::BarX => [true, false, false],
            Shape::BarZ => [false, false, true],
            Shape::Flat => [true, false, true],
            Shape::WallX => [true, true, false],
            Shape::WallZ => [false, true, true],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clue {
    pub cell: Cell,
    /// `None` is shown as "?": the player has to infer the box size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<u8>,
    /// `None` means any shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Puzzle {
    pub id: String,
    pub size: u8,
    pub clues: Vec<Clue>,
    pub solution: Vec<BoxRegion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spanning_normalises_corners() {
        let b = BoxRegion::spanning([3, 1, 2], [0, 2, 0]);
        assert_eq!(
            b,
            BoxRegion {
                min: [0, 1, 0],
                max: [3, 2, 2]
            }
        );
        assert_eq!(b.volume(), 4 * 2 * 3);
    }

    #[test]
    fn including_grows_to_reach_the_cell() {
        let b = BoxRegion::spanning([1, 1, 1], [2, 2, 2]);
        assert_eq!(
            b.including([0, 3, 1]),
            BoxRegion {
                min: [0, 1, 1],
                max: [2, 3, 2]
            }
        );
        assert_eq!(b.including([2, 1, 2]), b);
    }

    #[test]
    fn full_cube_mask_and_volume() {
        let b = BoxRegion {
            min: [0, 0, 0],
            max: [3, 3, 3],
        };
        assert_eq!(b.mask(), u64::MAX);
        assert_eq!(b.volume(), 64);
    }

    #[test]
    fn single_cell_mask_matches_index() {
        let b = BoxRegion {
            min: [1, 2, 3],
            max: [1, 2, 3],
        };
        assert_eq!(b.mask(), 1 << index([1, 2, 3]));
        assert!(b.contains([1, 2, 3]));
        assert!(!b.contains([1, 2, 2]));
    }

    #[test]
    fn shape_names_the_longest_axes() {
        let from_origin = |max: Cell| BoxRegion {
            min: [0, 0, 0],
            max,
        };
        assert_eq!(from_origin([0, 0, 0]).shape(), Shape::Cube);
        assert_eq!(from_origin([1, 1, 1]).shape(), Shape::Cube);
        assert_eq!(from_origin([1, 2, 1]).shape(), Shape::Tall);
        assert_eq!(from_origin([2, 0, 0]).shape(), Shape::BarX);
        assert_eq!(from_origin([1, 0, 2]).shape(), Shape::BarZ);
        assert_eq!(from_origin([2, 1, 2]).shape(), Shape::Flat);
        assert_eq!(from_origin([1, 1, 0]).shape(), Shape::WallX);
        assert_eq!(from_origin([0, 3, 3]).shape(), Shape::WallZ);
    }

    #[test]
    fn every_shape_is_the_shape_of_its_own_longest_axes() {
        for s in Shape::ALL {
            let b = BoxRegion {
                min: [0, 0, 0],
                max: s.longest().map(u8::from),
            };
            assert_eq!(b.shape(), s);
        }
    }

    #[test]
    fn index_roundtrips() {
        for i in 0..CELLS {
            assert_eq!(index(cell_at(i)), i);
        }
    }

    #[test]
    fn puzzle_json_roundtrip_omits_hidden_volume() {
        let p = Puzzle {
            id: "t".into(),
            size: N,
            clues: vec![
                Clue {
                    cell: [0, 0, 0],
                    volume: Some(16),
                    shape: Some(Shape::WallX),
                },
                Clue {
                    cell: [1, 1, 1],
                    volume: None,
                    shape: None,
                },
            ],
            solution: vec![BoxRegion {
                min: [0, 0, 0],
                max: [3, 3, 3],
            }],
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("null"));
        assert!(json.contains(r#""shape":"wall_x""#));
        assert_eq!(serde_json::from_str::<Puzzle>(&json).unwrap(), p);
    }
}
