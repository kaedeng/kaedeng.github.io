import { test } from "node:test";
import assert from "node:assert/strict";
import {
  labelFont,
  viewCubeInset,
  viewCubePerspective,
  viewCubeSide,
} from "../../../packages/board/src/sizes.ts";

test("labels shrink on a small board, down to a size that still reads", () => {
  assert.equal(labelFont(720), 14);
  assert.equal(labelFont(1200), 14);
  assert.ok(labelFont(342) < 14);
  assert.ok(labelFont(342) >= 10);
  assert.ok(labelFont(500) >= labelFont(342));
});

test("the view cube shrinks on a small board, so it leaves the cube's corner clear", () => {
  assert.equal(viewCubeSide(720), 48);
  assert.equal(viewCubeSide(1200), 48);
  assert.ok(viewCubeSide(342) <= 32, `${viewCubeSide(342)}`);
  assert.ok(viewCubeSide(342) >= 28);
  assert.ok(viewCubeSide(500) >= viewCubeSide(342));
});

test("the view cube sees itself as the camera sees the board", () => {
  // The camera is 8.5 cells from the centre of a 4-cell cube: 2.125 sides away.
  assert.equal(viewCubePerspective(48, 8.5, 4), 102);
  assert.equal(viewCubePerspective(24, 8.5, 4), 51);
});

test("the view cube's inset keeps it inside the board however it turns", () => {
  for (const side of [30, 40, 48]) {
    // Turned, with perspective, a cube reaches about 0.43 of its side past its box.
    assert.ok(viewCubeInset(side) >= 0.43 * side, `${side}`);
    assert.ok(viewCubeInset(side) <= 0.6 * side, `${side}`);
  }
});
