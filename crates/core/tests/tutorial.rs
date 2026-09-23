//! The site's practice puzzle is written by hand, so check it is a real puzzle.

use patches_core::{Puzzle, solve};

#[test]
fn practice_puzzle_has_exactly_its_stored_solution() {
    let puzzle: Puzzle =
        serde_json::from_str(include_str!("../../../web/src/tutorial.json")).unwrap();
    let mut found = solve(&puzzle.clues, 2);
    assert_eq!(found.len(), 1, "one solution, not {}", found.len());
    let mut stored = puzzle.solution.clone();
    found[0].sort();
    stored.sort();
    assert_eq!(found[0], stored);
}
