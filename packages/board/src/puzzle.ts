export type Cell = [number, number, number];
export type Box = { min: Cell; max: Cell };
/** Which axes of a box are longest (y is up); same as Shape in crates/core/src/types.rs. */
export type Shape =
  "cube" | "tall" | "bar_x" | "bar_z" | "flat" | "wall_x" | "wall_z";
export type Clue = { cell: Cell; volume?: number; shape?: Shape };
/** What `cargo run -p patches-core --bin generate -- <seed>` prints. */
export type Puzzle = {
  id: string;
  size: number;
  clues: Clue[];
  solution: Box[];
};

/** Clue colours: the board draws clue `i` and its box in `clueColor(i)`. */
export const PALETTE = [
  "#f1b1b1",
  "#b1f1c4",
  "#d6b1f1",
  "#f1e9b1",
  "#b1e6f1",
  "#f1b1d4",
  "#c1f1b1",
  "#b4b1f1",
  "#f1c6b1",
  "#b1f1d9",
  "#ecb1f1",
  "#e4f1b1",
  "#b1cdf1",
  "#f1d8b1",
  "#f1b1c2",
  "#b1f1b1",
];

export function clueColor(clueIndex: number): string {
  return PALETTE[clueIndex % PALETTE.length];
}

export function clueText(clue: Clue): string {
  return clue.volume === undefined ? "?" : String(clue.volume);
}
