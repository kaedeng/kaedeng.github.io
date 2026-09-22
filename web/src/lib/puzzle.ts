import data from "@/puzzle.json";

export type Cell = [number, number, number];
export type Box = { min: Cell; max: Cell };
export type Clue = { cell: Cell; volume?: number };
export type Puzzle = {
  id: string;
  size: number;
  clues: Clue[];
  solution: Box[];
};

export const puzzle = data as Puzzle;

export function contains(b: Box, [x, y, z]: Cell): boolean {
  return (
    b.min[0] <= x &&
    x <= b.max[0] &&
    b.min[1] <= y &&
    y <= b.max[1] &&
    b.min[2] <= z &&
    z <= b.max[2]
  );
}

export function boxIndexAt(boxes: Box[], cell: Cell): number {
  return boxes.findIndex((b) => contains(b, cell));
}

/** Index into `clues` of the clue inside `box`. */
export function clueIndexOf(box: Box, clues: Clue[]): number {
  return clues.findIndex((c) => contains(box, c.cell));
}

export function clueAt(clues: Clue[], [x, y, z]: Cell): Clue | undefined {
  return clues.find(
    (c) => c.cell[0] === x && c.cell[1] === y && c.cell[2] === z,
  );
}

export function clueText(clue: Clue): string {
  return clue.volume === undefined ? "?" : String(clue.volume);
}

/** Same colours as PALETTE in crates/wasm/src/game.rs so the grids match the 3D view. */
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
