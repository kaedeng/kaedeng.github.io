import type { Box, Cell, Clue, Puzzle, Shape } from "patches-board";
import data from "@/puzzle.json";

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

export const SHAPE_NAME: Record<Shape, string> = {
  cube: "cube",
  tall: "tall block",
  bar_x: "bar along x",
  bar_z: "bar along z",
  flat: "flat block",
  wall_x: "wall along x",
  wall_z: "wall along z",
};

/** A box of each shape, in cells: its longest axes are 2, the others 1. */
export const SHAPE_SIZE: Record<Shape, Cell> = {
  cube: [1, 1, 1],
  tall: [1, 2, 1],
  bar_x: [2, 1, 1],
  bar_z: [1, 1, 2],
  flat: [2, 1, 2],
  wall_x: [2, 2, 1],
  wall_z: [1, 2, 2],
};
