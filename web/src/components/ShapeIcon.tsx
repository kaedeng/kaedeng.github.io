import type { Shape } from "patches-board";
import { SHAPE_NAME, SHAPE_SIZE } from "@/lib/puzzle";

const COS30 = Math.cos(Math.PI / 6);
/** Shading of the top, +x and +z faces. */
const FILL = [0.5, 0.25, 0.1];

/** Where the +x, +y and +z bars of a jack end, seen like the boxes below. */
const JACK_ENDS = [
  [COS30, 0.5],
  [0, -1],
  [-COS30, 0.5],
];

/**
 * Isometric sketch of a box of `shape`, seen from +x +y +z like the 3D board's starting
 * view. No shape draws the jack of a clue that allows any shape, as on the board: three
 * bars along x, y and z through the middle, as long as a box's long side.
 */
export function ShapeIcon({ shape }: { shape?: Shape }) {
  if (!shape) {
    return (
      <svg viewBox="-2.2 -2.2 4.4 4.4" className="h-5 w-5" role="img">
        <title>any shape</title>
        <path
          d={JACK_ENDS.map(([x, y]) => `M${-x},${-y}L${x},${y}`).join("")}
          stroke="currentColor"
          strokeWidth={0.28}
          strokeLinecap="round"
        />
      </svg>
    );
  }
  const [X, Y, Z] = SHAPE_SIZE[shape];
  // Centred on the box's middle; x runs right-down, z left-down, y up.
  const p = (x: number, y: number, z: number) =>
    `${(x - z - (X - Z) / 2) * COS30},${(x + z - (X + Z) / 2) / 2 - y + Y / 2}`;
  const faces = [
    [p(0, Y, 0), p(X, Y, 0), p(X, Y, Z), p(0, Y, Z)],
    [p(X, 0, 0), p(X, Y, 0), p(X, Y, Z), p(X, 0, Z)],
    [p(0, 0, Z), p(X, 0, Z), p(X, Y, Z), p(0, Y, Z)],
  ];
  return (
    <svg viewBox="-2.2 -2.2 4.4 4.4" className="h-5 w-5" role="img">
      <title>{SHAPE_NAME[shape]}</title>
      {faces.map((f, i) => (
        <polygon
          key={i}
          points={f.join(" ")}
          fill="currentColor"
          fillOpacity={FILL[i]}
          stroke="currentColor"
          strokeWidth={0.12}
          strokeLinejoin="round"
        />
      ))}
    </svg>
  );
}
