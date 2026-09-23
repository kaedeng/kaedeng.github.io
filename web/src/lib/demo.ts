import type { Cell, PlacedBox } from "patches-board";
import type { Goal } from "./tutorial";

/**
 * Where the ghost pointer heads: a cell's centre; a point as fractions of the board's
 * width and height; a move by CSS px from where it is; or the view-cube face turned most
 * toward the viewer.
 */
export type Aim =
  { cell: Cell } | { at: [number, number] } | { by: [number, number] } | "face";

/** A demo step: glide the ghost `to` an aim over `ms`, press it `down` or let go, or wait. */
export type Action =
  { to: Aim; ms: number } | { down: boolean } | { wait: number };

export type DemoGoal = Exclude<Goal, "solve">;

/** Where the ghost shows up, as fractions of the board: below the cube. */
export const HOME: [number, number] = [0.5, 0.92];

/**
 * The longest frame (ms) the demo's clock takes. After a stall, e.g. in a hidden tab, a
 * drag goes on from where it was instead of jumping, which would look like a rest.
 */
export const MAX_FRAME = 50;

/** Slow enough to follow, and steady, so no cell a drag passes is kept as a rest. */
const DRAG_MS = 1500;
/** The practice cube's top layer. */
const TOP = 3;

/** Press and let go where the ghost is. */
const TAP: Action[] = [
  { wait: 250 },
  { down: true },
  { wait: 150 },
  { down: false },
];

/** Press where the ghost is, and hold still well past the board's 500 ms to lock. */
const HOLD: Action[] = [
  { wait: 250 },
  { down: true },
  { wait: 900 },
  { down: false },
];

const SCRIPTS: Record<DemoGoal, (boxes: PlacedBox[]) => Action[]> = {
  place: (boxes) => [...build(freeLayer(boxes)), { wait: 1200 }],
  remove: (boxes) => onLastBox(boxes, click),
  lock: (boxes) => onLastBox(boxes, hold),
  // One way and back: the cube ends up as it was.
  turn: () => [
    { to: { at: [0.12, 0.86] }, ms: 900 },
    { wait: 250 },
    { down: true },
    { wait: 200 },
    { to: { by: [150, 0] }, ms: 1300 },
    { wait: 250 },
    { to: { by: [-150, 0] }, ms: 1300 },
    { wait: 150 },
    { down: false },
    { wait: 500 },
  ],
  // Into 2D and, after a look, back to 3D.
  flat: () => [
    { to: "face", ms: 1000 },
    ...TAP,
    { wait: 1800 },
    { to: "face", ms: 400 },
    ...TAP,
    { wait: 600 },
  ],
};

/** Whether a step has a demo: every practice move but solving, the exercise itself. */
export function canDemo(goal: Goal | undefined): goal is DemoGoal {
  return goal !== undefined && goal in SCRIPTS;
}

/** The ghost's moves showing `goal` on a board holding `boxes`. */
export function demoScript(goal: DemoGoal, boxes: PlacedBox[]): Action[] {
  return SCRIPTS[goal](boxes);
}

/** The point `t` (0 to 1) of the way from `from` to `to`, easing in and out. */
export function pointAt(
  from: [number, number],
  to: [number, number],
  t: number,
): [number, number] {
  const e = t * t * (3 - 2 * t);
  return [from[0] + (to[0] - from[0]) * e, from[1] + (to[1] - from[1]) * e];
}

/**
 * A box across flat layer `y`, dragged between the corners on the front and the right
 * face: the starting camera sees both, even under boxes above.
 */
function build(y: number): Action[] {
  return [
    { to: { cell: [0, y, TOP] }, ms: 900 },
    { wait: 250 },
    { down: true },
    { wait: 250 },
    { to: { cell: [TOP, y, 0] }, ms: DRAG_MS },
    { wait: 400 },
    { down: false },
  ];
}

function click(cell: Cell): Action[] {
  return [{ to: { cell }, ms: 800 }, ...TAP];
}

function hold(cell: Cell): Action[] {
  return [{ to: { cell }, ms: 800 }, ...HOLD];
}

/**
 * `act` on a corner of the last box placed that is not locked, since a locked one takes
 * no clicks; with no such box, builds one first.
 */
function onLastBox(
  boxes: PlacedBox[],
  act: (cell: Cell) => Action[],
): Action[] {
  const box = boxes.filter((b) => !b.locked).at(-1);
  if (box) return [...act(box.max), { wait: 1000 }];
  const y = freeLayer(boxes);
  return [...build(y), { wait: 700 }, ...act([TOP, y, TOP]), { wait: 1000 }];
}

/** The highest flat layer no box reaches into; the top one when every layer has a box. */
function freeLayer(boxes: PlacedBox[]): number {
  for (let y = TOP; y >= 0; y--) {
    if (!boxes.some((b) => b.min[1] <= y && y <= b.max[1])) return y;
  }
  return TOP;
}
