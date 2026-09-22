use crate::solver::solve;
use crate::types::{BoxRegion, Cell, Clue, N, Puzzle, cell_at};

/// Largest box the generator will place; keeps puzzles at roughly 10-16 boxes.
pub const MAX_VOLUME: u8 = 12;

/// A uniquely solvable puzzle, fully determined by `seed`.
pub fn generate(seed: &str) -> Puzzle {
    for attempt in 0.. {
        let mut rng = Rng::new(hash(seed, attempt));
        let partition = random_partition(&mut rng);
        let mut clues = initial_clues(&partition, &mut rng);
        if make_unique(&partition, &mut clues) {
            return Puzzle {
                id: seed.to_string(),
                size: N,
                clues,
                solution: partition,
            };
        }
    }
    unreachable!("the attempt loop only exits by returning")
}

/// Tile the cube: at the lowest free cell, pick a random box that fits the free space.
fn random_partition(rng: &mut Rng) -> Vec<BoxRegion> {
    let mut free = u64::MAX;
    let mut boxes = Vec::new();
    while free != 0 {
        let options = fitting_boxes(cell_at(free.trailing_zeros() as usize), free);
        let b = options[rng.below(options.len())];
        free &= !b.mask();
        boxes.push(b);
    }
    boxes
}

fn fitting_boxes(origin: Cell, free: u64) -> Vec<BoxRegion> {
    let mut v = Vec::new();
    for x in origin[0]..N {
        for y in origin[1]..N {
            for z in origin[2]..N {
                let b = BoxRegion {
                    min: origin,
                    max: [x, y, z],
                };
                if b.volume() <= MAX_VOLUME && b.mask() & !free == 0 {
                    v.push(b);
                }
            }
        }
    }
    v
}

/// One clue per box, at a random cell inside it; about a third of volumes and a third of
/// shapes start hidden.
fn initial_clues(partition: &[BoxRegion], rng: &mut Rng) -> Vec<Clue> {
    partition
        .iter()
        .map(|b| {
            let cell = random_cell_in(b, rng);
            let volume = (!rng.chance(1, 3)).then(|| b.volume());
            let shape = (!rng.chance(1, 3)).then(|| b.shape());
            Clue {
                cell,
                volume,
                shape,
            }
        })
        .collect()
}

fn random_cell_in(b: &BoxRegion, rng: &mut Rng) -> Cell {
    let mut cell = b.min;
    for (c, hi) in cell.iter_mut().zip(b.max) {
        *c += rng.below((hi - *c + 1) as usize) as u8;
    }
    cell
}

/// Reveal hidden shapes and volumes until only the partition solves the clues.
/// Returns false when the two remaining solutions differ only at fully revealed clues.
fn make_unique(partition: &[BoxRegion], clues: &mut [Clue]) -> bool {
    loop {
        let sols = solve(clues, 2);
        debug_assert!(
            !sols.is_empty(),
            "the partition itself always solves its clues"
        );
        if sols.len() == 1 {
            return true;
        }
        let Some(i) = differing_hidden_clue(clues, &sols[0], &sols[1]) else {
            return false;
        };
        reveal(&mut clues[i], &partition[i]);
    }
}

/// Reveals the clue's shape, or its volume once the shape is known.
fn reveal(clue: &mut Clue, b: &BoxRegion) {
    if clue.shape.is_none() {
        clue.shape = Some(b.shape());
    } else {
        clue.volume = Some(b.volume());
    }
}

fn differing_hidden_clue(clues: &[Clue], a: &[BoxRegion], b: &[BoxRegion]) -> Option<usize> {
    (0..clues.len()).find(|&i| {
        (clues[i].volume.is_none() || clues[i].shape.is_none())
            && box_containing(a, clues[i].cell) != box_containing(b, clues[i].cell)
    })
}

fn box_containing(sol: &[BoxRegion], cell: Cell) -> Option<BoxRegion> {
    sol.iter().copied().find(|b| b.contains(cell))
}

/// FNV-1a over the seed text and attempt number.
fn hash(seed: &str, attempt: u32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in seed.bytes().chain(attempt.to_le_bytes()) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// xorshift64*: tiny, deterministic, no dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn chance(&mut self, num: u64, den: u64) -> bool {
        self.next() % den < num
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::solve;
    use crate::types::{BoxRegion, N};

    fn sorted(mut v: Vec<BoxRegion>) -> Vec<BoxRegion> {
        v.sort();
        v
    }

    #[test]
    fn generated_puzzles_are_unique_and_self_consistent() {
        for seed in 0..20 {
            let p = generate(&seed.to_string());
            assert_eq!(p.size, N);
            assert_eq!(p.clues.len(), p.solution.len());
            // Every clue sits in its own box.
            for (clue, b) in p.clues.iter().zip(&p.solution) {
                assert!(b.contains(clue.cell), "seed {seed}");
                assert!(b.volume() <= MAX_VOLUME, "seed {seed}");
                assert!(clue.volume.is_none_or(|v| v == b.volume()), "seed {seed}");
                assert!(clue.shape.is_none_or(|s| s == b.shape()), "seed {seed}");
            }
            // The solution tiles the cube exactly.
            let (mut cover, mut overlap) = (0u64, false);
            for b in &p.solution {
                overlap |= cover & b.mask() != 0;
                cover |= b.mask();
            }
            assert!(cover == u64::MAX && !overlap, "seed {seed}");
            // And it is the only tiling the clues allow.
            let sols = solve(&p.clues, 2);
            assert_eq!(sols.len(), 1, "seed {seed}");
            assert_eq!(
                sorted(sols[0].clone()),
                sorted(p.solution.clone()),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn same_seed_same_puzzle() {
        assert_eq!(generate("2026-W39"), generate("2026-W39"));
        assert_ne!(generate("2026-W39"), generate("2026-W40"));
    }

    #[test]
    fn some_clues_are_hidden() {
        let hidden = (0..20)
            .map(|s| generate(&s.to_string()))
            .flat_map(|p| p.clues)
            .filter(|c| c.volume.is_none())
            .count();
        assert!(hidden > 0);
    }

    #[test]
    fn some_shapes_are_hidden() {
        let hidden = (0..20)
            .map(|s| generate(&s.to_string()))
            .flat_map(|p| p.clues)
            .filter(|c| c.shape.is_none())
            .count();
        assert!(hidden > 0);
    }
}
