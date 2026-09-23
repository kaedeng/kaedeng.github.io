//! Puzzle ids. An id doubles as the generator's seed: the bare seed for Medium (`a3f9c1`,
//! `2026-W39`), or the seed and a level suffix (`a3f9c1-easy`, `a3f9c1-hard`).

use crate::types::Level;

/// Splits `id` into the generator's seed and level: `a3f9c1-hard` is seed `a3f9c1` at Hard.
/// Only a known level counts as a suffix, since weekly seeds like `2026-W39` have a `-`
/// too; its case doesn't matter, but the seed keeps its own. Without one the whole id is a
/// Medium seed, so any text is an id. Surrounding whitespace is dropped: it tends to come
/// along when an id is pasted.
pub fn parse_id(id: &str) -> (&str, Level) {
    let id = id.trim();
    split_level(id).unwrap_or((id, Level::Medium))
}

/// The id that `parse_id` reads back as `seed` at `level`: `seed-level`, or for Medium the
/// bare seed, unless that would read as another level (`x-hard-medium`).
pub fn format_id(seed: &str, level: Level) -> String {
    if level == Level::Medium && split_level(seed).is_none() {
        seed.to_string()
    } else {
        format!("{seed}-{}", level.name())
    }
}

/// `id`'s seed and level, if it ends in `-<level>` with some seed before it.
fn split_level(id: &str) -> Option<(&str, Level)> {
    Level::ALL.into_iter().find_map(|level| {
        let cut = id.len().checked_sub(level.name().len() + 1)?;
        let (seed, suffix) = id.split_at_checked(cut)?;
        let name = suffix.strip_prefix('-')?;
        (!seed.is_empty() && name.eq_ignore_ascii_case(level.name())).then_some((seed, level))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_suffix_names_the_level() {
        assert_eq!(parse_id("a3f9c1-easy"), ("a3f9c1", Level::Easy));
        assert_eq!(parse_id("a3f9c1-hard"), ("a3f9c1", Level::Hard));
        assert_eq!(parse_id("bob-hard"), ("bob", Level::Hard));
        assert_eq!(format_id("a3f9c1", Level::Hard), "a3f9c1-hard");
    }

    #[test]
    fn anything_else_is_a_medium_seed() {
        assert_eq!(parse_id("a3f9c1"), ("a3f9c1", Level::Medium));
        // Weekly seeds have a `-` too: only a known level counts.
        assert_eq!(parse_id("2026-W39"), ("2026-W39", Level::Medium));
        assert_eq!(parse_id("bob-tricky"), ("bob-tricky", Level::Medium));
        assert_eq!(parse_id("bob-hardest"), ("bob-hardest", Level::Medium));
        assert_eq!(format_id("2026-W39", Level::Medium), "2026-W39");
    }

    #[test]
    fn medium_can_be_spelled_out_but_is_dropped() {
        assert_eq!(parse_id("x-medium"), ("x", Level::Medium));
        assert_eq!(format_id("x", Level::Medium), "x");
    }

    #[test]
    fn surrounding_whitespace_is_dropped() {
        assert_eq!(parse_id("  a3f9c1-hard\n"), ("a3f9c1", Level::Hard));
        assert_eq!(parse_id(" 2026-W39 "), ("2026-W39", Level::Medium));
        assert_eq!(parse_id("a b-easy"), ("a b", Level::Easy));
    }

    #[test]
    fn the_seed_keeps_its_case_and_the_level_ignores_it() {
        assert_eq!(parse_id("Bob-HARD"), ("Bob", Level::Hard));
        assert_eq!(parse_id("x-Easy"), ("x", Level::Easy));
        assert_eq!(parse_id("2026-w39"), ("2026-w39", Level::Medium));
        assert_eq!(format_id("Bob", Level::Hard), "Bob-hard");
    }

    #[test]
    fn a_suffix_needs_a_seed_before_it() {
        assert_eq!(parse_id(""), ("", Level::Medium));
        assert_eq!(parse_id("   "), ("", Level::Medium));
        assert_eq!(parse_id("-hard"), ("-hard", Level::Medium));
        assert_eq!(parse_id("x--hard"), ("x-", Level::Hard));
    }

    #[test]
    fn only_the_last_suffix_counts() {
        assert_eq!(parse_id("x-easy-hard"), ("x-easy", Level::Hard));
        assert_eq!(parse_id("x-hard-medium"), ("x-hard", Level::Medium));
        // Dropping `-medium` here would make it read as `x` at Hard.
        assert_eq!(format_id("x-hard", Level::Medium), "x-hard-medium");
    }

    #[test]
    fn format_id_reads_back_as_the_same_seed_and_level() {
        let texts = [
            "a3f9c1",
            "a3f9c1-easy",
            "a3f9c1-hard",
            "x-medium",
            "2026-W39",
            " Bob-Hard ",
            "x-easy-medium",
            "x-easy-hard",
            "-hard",
            "",
            "é-hard",
        ];
        for text in texts {
            let (seed, level) = parse_id(text);
            assert_eq!(parse_id(&format_id(seed, level)), (seed, level), "{text:?}");
        }
    }
}
