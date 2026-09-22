use crate::types::{BoxRegion, CELLS, Clue, N};

struct Cand {
    region: BoxRegion,
    mask: u64,
    clue: usize,
}

/// Up to `limit` tilings that satisfy every clue.
pub fn solve(clues: &[Clue], limit: usize) -> Vec<Vec<BoxRegion>> {
    let cands = candidates(clues);
    let mut by_cell = vec![Vec::new(); CELLS];
    for (i, c) in cands.iter().enumerate() {
        for (cell, list) in by_cell.iter_mut().enumerate() {
            if c.mask >> cell & 1 == 1 {
                list.push(i);
            }
        }
    }
    let mut search = Search {
        cands: &cands,
        by_cell,
        limit,
        found: Vec::new(),
        stack: Vec::new(),
    };
    search.run(0, 0);
    search.found
}

/// Every box that holds exactly one clue and matches its volume and shape, if given.
fn candidates(clues: &[Clue]) -> Vec<Cand> {
    let mut out = Vec::new();
    for region in all_regions() {
        let inside: Vec<usize> = (0..clues.len())
            .filter(|&i| region.contains(clues[i].cell))
            .collect();
        let [clue] = inside[..] else {
            continue;
        };
        if clues[clue].volume.is_some_and(|v| v != region.volume())
            || clues[clue].shape.is_some_and(|s| s != region.shape())
        {
            continue;
        }
        out.push(Cand {
            mask: region.mask(),
            region,
            clue,
        });
    }
    out
}

fn all_regions() -> Vec<BoxRegion> {
    let spans: Vec<(u8, u8)> = (0..N).flat_map(|a| (a..N).map(move |b| (a, b))).collect();
    let mut v = Vec::with_capacity(spans.len().pow(3));
    for &(x0, x1) in &spans {
        for &(y0, y1) in &spans {
            for &(z0, z1) in &spans {
                v.push(BoxRegion {
                    min: [x0, y0, z0],
                    max: [x1, y1, z1],
                });
            }
        }
    }
    v
}

struct Search<'a> {
    cands: &'a [Cand],
    by_cell: Vec<Vec<usize>>,
    limit: usize,
    found: Vec<Vec<BoxRegion>>,
    stack: Vec<BoxRegion>,
}

impl Search<'_> {
    /// `placed`: covered cells. `used`: clues already claimed by a box on the stack.
    fn run(&mut self, placed: u64, used: u64) {
        if self.found.len() >= self.limit {
            return;
        }
        if placed == u64::MAX {
            self.found.push(self.stack.clone());
            return;
        }
        let Some(cell) = self.most_constrained(placed, used) else {
            return;
        };
        for k in 0..self.by_cell[cell].len() {
            let i = self.by_cell[cell][k];
            if !self.usable(i, placed, used) {
                continue;
            }
            let c = &self.cands[i];
            self.stack.push(c.region);
            self.run(placed | c.mask, used | 1 << c.clue);
            self.stack.pop();
        }
    }

    /// The uncovered cell with the fewest usable boxes.
    fn most_constrained(&self, placed: u64, used: u64) -> Option<usize> {
        let mut best: Option<(usize, usize)> = None;
        for cell in 0..CELLS {
            if placed >> cell & 1 == 1 {
                continue;
            }
            let n = self.by_cell[cell]
                .iter()
                .filter(|&&i| self.usable(i, placed, used))
                .count();
            if best.is_none_or(|(bn, _)| n < bn) {
                best = Some((n, cell));
            }
        }
        best.map(|(_, cell)| cell)
    }

    fn usable(&self, i: usize, placed: u64, used: u64) -> bool {
        let c = &self.cands[i];
        c.mask & placed == 0 && used >> c.clue & 1 == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Shape;

    fn sorted(mut v: Vec<BoxRegion>) -> Vec<BoxRegion> {
        v.sort();
        v
    }

    fn slabs() -> Vec<BoxRegion> {
        (0..4)
            .map(|z| BoxRegion {
                min: [0, 0, z],
                max: [3, 3, z],
            })
            .collect()
    }

    fn slab_clues(volume: Option<u8>) -> Vec<Clue> {
        (0..4)
            .map(|z| Clue {
                cell: [0, 0, z],
                volume,
                shape: None,
            })
            .collect()
    }

    #[test]
    fn slab_puzzle_has_one_solution() {
        let sols = solve(&slab_clues(Some(16)), 2);
        assert_eq!(sols.len(), 1);
        assert_eq!(sorted(sols[0].clone()), sorted(slabs()));
    }

    #[test]
    fn hidden_volumes_can_still_be_unique() {
        // A box holding (0,0,z) but not (0,0,z±1) must be one layer thick, and
        // boxes without a clue are illegal, so each layer is one box.
        assert_eq!(solve(&slab_clues(None), 2).len(), 1);
    }

    fn two_box_split(shape: Option<Shape>) -> Vec<Clue> {
        vec![
            Clue {
                cell: [0, 0, 0],
                volume: Some(16),
                shape,
            },
            Clue {
                cell: [3, 3, 3],
                volume: Some(48),
                shape: None,
            },
        ]
    }

    #[test]
    fn two_box_split_is_ambiguous() {
        let clues = two_box_split(None);
        assert_eq!(solve(&clues, 2).len(), 2);
        assert_eq!(solve(&clues, 10).len(), 3); // split at 1 along x, y or z
    }

    #[test]
    fn shape_picks_the_split() {
        let sols = solve(&two_box_split(Some(Shape::Flat)), 10);
        assert_eq!(sols.len(), 1);
        assert!(sols[0].contains(&BoxRegion {
            min: [0, 0, 0],
            max: [3, 0, 3]
        }));
    }

    #[test]
    fn impossible_volume_has_no_solution() {
        let clues = vec![Clue {
            cell: [0, 0, 0],
            volume: Some(63),
            shape: None,
        }];
        assert!(solve(&clues, 2).is_empty());
    }
}
