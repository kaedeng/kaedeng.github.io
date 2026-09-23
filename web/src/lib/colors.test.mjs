import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { clueColors, PALETTE } from "../../../packages/board/src/puzzle.ts";

/** At most 2 cells apart, centre to centre: the rule clueColors keeps apart. */
function near(a, b) {
  return a.reduce((sum, v, i) => sum + (v - b[i]) ** 2, 0) <= 4;
}

function assertNearbyCluesDiffer(puzzle) {
  const colors = clueColors(puzzle.clues);
  assert.equal(colors.length, puzzle.clues.length);
  puzzle.clues.forEach((a, i) => {
    puzzle.clues.forEach((b, j) => {
      if (i < j && near(a.cell, b.cell)) {
        assert.notEqual(colors[i], colors[j], `${puzzle.id}: clues ${i}, ${j}`);
      }
    });
  });
}

/** WCAG contrast of black text on `hex`. */
function contrastWithBlack(hex) {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return (0.2126 * r + 0.7152 * g + 0.0722 * b + 0.05) / 0.05;
}

/** OKLCH chroma of `hex`: how far from grey it is. */
function chroma(hex) {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  const [l, m, s] = [
    0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b,
    0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b,
    0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b,
  ].map(Math.cbrt);
  const a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const bb = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  return Math.hypot(a, bb);
}

/** What the `generate` CLI prints for `args`. */
function generate(...args) {
  const json = execFileSync("cargo", [
    ...["run", "-q", "--release", "-p", "patches-core"],
    ...["--bin", "generate", "--", ...args],
  ]);
  return JSON.parse(json);
}

test("nearby clues differ in colour in daily puzzles", () => {
  for (const day of ["2026-09-22", "2026-09-23", "2026-09-24"]) {
    assertNearbyCluesDiffer(generate("--daily", day));
  }
});

test("nearby clues differ in colour in generated puzzles", () => {
  for (const seed of ["2026-W40", "2026-W41", "3fa9c2", "b07e11", "e5d402"]) {
    assertNearbyCluesDiffer(generate(seed));
  }
});

test("clues far apart take the palette in order", () => {
  const corners = [0, 3].flatMap((x) =>
    [0, 3].flatMap((y) => [0, 3].map((z) => ({ cell: [x, y, z] }))),
  );
  assert.deepEqual(clueColors(corners), PALETTE.slice(0, 8));
});

test("a clue skips colours used near it, then colours used at all", () => {
  const cells = [
    [0, 0, 0],
    [3, 3, 3],
    [0, 0, 1],
    [3, 3, 2],
  ];
  const colors = clueColors(cells.map((cell) => ({ cell })));
  assert.deepEqual(colors, PALETTE.slice(0, 4));
});

test("with every colour in use, a clue takes the one used farthest off", () => {
  // The first clues take the palette in order, the last of them at [0, 0, 0]; the one
  // after is near none of them, and [0, 0, 0] is the farthest from it.
  const cluster = [
    [3, 3, 0],
    [0, 3, 1],
    [1, 0, 3],
    [0, 1, 0],
    [1, 0, 0],
    [0, 0, 2],
    [0, 2, 0],
    [2, 0, 0],
    [0, 0, 1],
    [1, 1, 0],
    [0, 1, 1],
  ].slice(0, PALETTE.length - 1);
  const cells = [...cluster, [0, 0, 0], [3, 3, 3]];
  const colors = clueColors(cells.map((cell) => ({ cell })));
  const n = PALETTE.length;
  assert.deepEqual(colors.slice(0, n), PALETTE);
  assert.equal(colors[n], PALETTE[n - 1]);
});

test("the palette has about ten colours, all distinct", () => {
  assert.ok(PALETTE.length >= 8 && PALETTE.length <= 12);
  assert.equal(new Set(PALETTE).size, PALETTE.length);
});

test("no colour is a washed-out tint that reads as grey", () => {
  for (const c of PALETTE) assert.ok(chroma(c) >= 0.08, c);
});

test("black clue text stays readable on every colour", () => {
  for (const c of PALETTE) assert.ok(contrastWithBlack(c) >= 7, c);
});
