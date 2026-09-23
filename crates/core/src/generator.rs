use crate::id::{format_id, parse_id};
use crate::solver::solve_within;
use crate::types::{BoxRegion, Cell, Clue, Level, N, Puzzle, cell_at};

/// Largest box the generator will place; keeps puzzles at roughly 10-16 boxes.
pub const MAX_VOLUME: u8 = 12;

/// Search steps one solve may take on Hard before the attempt is dropped for a fresh one.
/// Most solves take well under a thousand, but with every clue hidden about 1 id in 400
/// runs into a dead end the search needs hundreds of thousands (half a second) to leave.
const HARD_STEPS: usize = 20_000;

/// A uniquely solvable puzzle, fully determined by its id: a seed, plus a level suffix
/// unless it is Medium (see `parse_id`). The puzzle's `id` is the id spelled canonically.
pub fn generate(id: &str) -> Puzzle {
    let (seed, level) = parse_id(id);
    let id = format_id(seed, level);
    // Only Hard stops early: Medium makes the puzzles it always has, and Easy's clues
    // leave its searches short anyway.
    let steps = if level == Level::Hard {
        HARD_STEPS
    } else {
        usize::MAX
    };
    for attempt in 0.. {
        // The whole id is hashed, so a Medium id hashes just as its seed did before levels.
        let mut rng = Rng::new(hash(&id, attempt));
        let partition = random_partition(&mut rng);
        let mut clues = initial_clues(&partition, level, &mut rng);
        if make_unique(&partition, &mut clues, steps) {
            return Puzzle {
                id,
                level,
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

/// One clue per box, at a random cell inside it; some volumes and shapes start hidden.
fn initial_clues(partition: &[BoxRegion], level: Level, rng: &mut Rng) -> Vec<Clue> {
    partition
        .iter()
        .map(|b| {
            let cell = random_cell_in(b, rng);
            let volume = (!starts_hidden(level, rng)).then(|| b.volume());
            let shape = (!starts_hidden(level, rng)).then(|| b.shape());
            Clue {
                cell,
                volume,
                shape,
            }
        })
        .collect()
}

/// Easy hides nothing and Hard everything; `make_unique` then reveals what uniqueness needs.
/// Medium hides about a third, drawing from `rng` just as it did before levels.
fn starts_hidden(level: Level, rng: &mut Rng) -> bool {
    match level {
        Level::Easy => false,
        Level::Medium => rng.chance(1, 3),
        Level::Hard => true,
    }
}

fn random_cell_in(b: &BoxRegion, rng: &mut Rng) -> Cell {
    let mut cell = b.min;
    for (c, hi) in cell.iter_mut().zip(b.max) {
        *c += rng.below((hi - *c + 1) as usize) as u8;
    }
    cell
}

/// Reveal hidden shapes and volumes until only the partition solves the clues.
/// Returns false when the two remaining solutions differ only at fully revealed clues, or
/// when a solve runs out of `steps`.
fn make_unique(partition: &[BoxRegion], clues: &mut [Clue], steps: usize) -> bool {
    loop {
        let Some(sols) = solve_within(clues, 2, steps) else {
            return false;
        };
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

    /// The puzzle for `seed` at `level`, by the id the page and the CLI take.
    fn at(seed: u32, level: Level) -> Puzzle {
        generate(&format!("{seed}-{}", level.name()))
    }

    /// Hidden volumes plus hidden shapes.
    fn hidden(p: &Puzzle) -> usize {
        p.clues
            .iter()
            .map(|c| usize::from(c.volume.is_none()) + usize::from(c.shape.is_none()))
            .sum()
    }

    #[test]
    fn generated_puzzles_are_unique_and_self_consistent() {
        for level in Level::ALL {
            for seed in 0..20 {
                let p = at(seed, level);
                let what = format!("seed {seed} {level:?}");
                assert_eq!(p.size, N);
                assert_eq!(p.level, level);
                assert_eq!(p.clues.len(), p.solution.len());
                // Every clue sits in its own box.
                for (clue, b) in p.clues.iter().zip(&p.solution) {
                    assert!(b.contains(clue.cell), "{what}");
                    assert!(b.volume() <= MAX_VOLUME, "{what}");
                    assert!(clue.volume.is_none_or(|v| v == b.volume()), "{what}");
                    assert!(clue.shape.is_none_or(|s| s == b.shape()), "{what}");
                }
                // The solution tiles the cube exactly.
                let (mut cover, mut overlap) = (0u64, false);
                for b in &p.solution {
                    overlap |= cover & b.mask() != 0;
                    cover |= b.mask();
                }
                assert!(cover == u64::MAX && !overlap, "{what}");
                // And it is the only tiling the clues allow.
                let sols = solve(&p.clues, 2);
                assert_eq!(sols.len(), 1, "{what}");
                assert_eq!(
                    sorted(sols[0].clone()),
                    sorted(p.solution.clone()),
                    "{what}"
                );
            }
        }
    }

    #[test]
    fn medium_puzzles_are_the_ones_from_before_levels() {
        // FNV-1a of the CLI's JSON for these seeds, captured before levels existed.
        let fnv = |json: String| {
            json.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
            })
        };
        let before = [
            ("2026-W39", 0xdda4_ac35_d95f_d9d1),
            ("2026-W40", 0xf79b_310b_663d_e8cd),
            ("a3f9c1", 0xd8de_ce7c_c5e8_2842),
            ("abc123", 0x41c1_65e2_89c4_67c8),
            ("bob", 0x1920_9537_fa59_20c5),
            ("0", 0xee69_8140_458f_3d83),
            ("19", 0x2bde_e775_2e26_7564),
        ];
        for (seed, fingerprint) in before {
            let json = serde_json::to_string_pretty(&generate(seed)).unwrap();
            assert_eq!(fnv(json), fingerprint, "{seed}");
        }
    }

    #[test]
    fn easy_shows_every_shape_and_volume() {
        for seed in 0..20 {
            assert_eq!(hidden(&at(seed, Level::Easy)), 0, "seed {seed}");
        }
    }

    #[test]
    fn hard_hides_clearly_more_than_medium() {
        let total = |level| (0..20).map(|s| hidden(&at(s, level))).sum::<usize>();
        let (medium, hard) = (total(Level::Medium), total(Level::Hard));
        assert!(2 * hard >= 3 * medium, "hard {hard}, medium {medium}");
    }

    #[test]
    fn the_puzzle_id_is_the_canonical_spelling() {
        let p = generate(" a3f9c1-HARD ");
        assert_eq!(p.id, "a3f9c1-hard");
        assert_eq!(p.level, Level::Hard);
        assert_eq!(p, generate("a3f9c1-hard"));
        assert_eq!(generate("x-medium"), generate("x"));
        assert_eq!(generate("x").id, "x");
    }

    #[test]
    fn an_attempt_whose_search_runs_out_of_steps_is_dropped() {
        let mut rng = Rng::new(1);
        let partition = random_partition(&mut rng);
        let mut clues = initial_clues(&partition, Level::Hard, &mut rng);
        assert!(!make_unique(&partition, &mut clues, 1));
    }

    #[test]
    fn each_level_is_its_own_puzzle() {
        let [easy, medium, hard] = Level::ALL.map(|level| at(7, level).solution);
        assert!(easy != medium && medium != hard && easy != hard);
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
