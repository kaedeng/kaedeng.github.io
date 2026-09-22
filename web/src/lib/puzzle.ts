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

export function boxIndexAt(boxes: Box[], [x, y, z]: Cell): number {
  return boxes.findIndex(
    (b) =>
      b.min[0] <= x &&
      x <= b.max[0] &&
      b.min[1] <= y &&
      y <= b.max[1] &&
      b.min[2] <= z &&
      z <= b.max[2],
  );
}

export function clueAt(clues: Clue[], [x, y, z]: Cell): Clue | undefined {
  return clues.find(
    (c) => c.cell[0] === x && c.cell[1] === y && c.cell[2] === z,
  );
}

export function clueText(clue: Clue): string {
  return clue.volume === undefined ? "?" : String(clue.volume);
}

/** Distinct pastel per box index; golden-angle hue steps keep neighbours apart. */
export function boxColor(index: number): string {
  return `hsl(${(index * 137.508) % 360} 70% 82%)`;
}
