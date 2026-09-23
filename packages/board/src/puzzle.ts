export type Cell = [number, number, number];
export type Box = { min: Cell; max: Cell };
/** Which axes of a box are longest (y is up); same as Shape in crates/core/src/types.rs. */
export type Shape =
  "cube" | "tall" | "bar_x" | "bar_z" | "flat" | "wall_x" | "wall_z";
export type Clue = { cell: Cell; volume?: number; shape?: Shape };
/** How much of the clues a puzzle shows; same as Level in crates/core/src/types.rs. */
export type Level = "easy" | "medium" | "hard";
/** What `cargo run -p patches-core --bin generate -- <id>` prints. */
export type Puzzle = {
  id: string;
  /** Left out for Medium. */
  level?: Level;
  size: number;
  clues: Clue[];
  solution: Box[];
};

/**
 * Clue colours: spread around the hue wheel and over a few lightnesses, so any two tell
 * apart, and all light enough for black text. `clueColors` picks one per clue.
 */
export const PALETTE = [
  "#f98f87",
  "#ffb572",
  "#efdd71",
  "#93ce70",
  "#74e2c6",
  "#61b9ce",
  "#92c1fd",
  "#aa95e8",
  "#f99fdb",
];

/** Clues at most this many cells apart, centre to centre, count as near each other. */
const NEAR = 2;

/**
 * The colour of each clue and its box. In order, each clue takes the colour least used
 * by the clues near it so far; then the one whose clues are farthest off (each counts
 * 1 / distance²), since boxes can touch from further away; then the least used overall;
 * then the first in `PALETTE`. Only clue cells count, never the solution.
 */
export function clueColors(clues: Clue[]): string[] {
  const used = PALETTE.map(() => 0);
  const picked: number[] = [];
  clues.forEach((clue, i) => {
    const near = PALETTE.map(() => 0);
    const pull = PALETTE.map(() => 0);
    clues.slice(0, i).forEach((other, j) => {
      const d2 = distance2(clue.cell, other.cell);
      if (d2 <= NEAR * NEAR) near[picked[j]]++;
      pull[picked[j]] += 1 / d2;
    });
    const best = leastUsed([near, pull, used]);
    picked.push(best);
    used[best]++;
  });
  return picked.map((k) => PALETTE[k]);
}

/** The palette index lowest on the first of `keys` where they differ, else the first. */
function leastUsed(keys: number[][]): number {
  let best = 0;
  for (let k = 1; k < PALETTE.length; k++) {
    const diff = keys.map((key) => key[k] - key[best]).find((d) => d !== 0);
    if (diff !== undefined && diff < 0) best = k;
  }
  return best;
}

function distance2(a: Cell, b: Cell): number {
  return a.reduce((sum, v, i) => sum + (v - b[i]) ** 2, 0);
}

export function clueText(clue: Clue): string {
  return clue.volume === undefined ? "?" : String(clue.volume);
}
