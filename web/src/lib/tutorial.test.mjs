import { test } from "node:test";
import assert from "node:assert/strict";
import { reached } from "./tutorial.ts";

test("placing and removing a box count against the boxes before", () => {
  assert.equal(reached("place", { boxes: 1, solved: false }, 0), true);
  assert.equal(reached("place", { boxes: 1, solved: false }, 1), false);
  assert.equal(reached("remove", { boxes: 0, solved: false }, 1), true);
  assert.equal(reached("remove", { boxes: 2, solved: false }, 1), false);
});

test("turning is any move in 3D; the flat view needs 2D", () => {
  assert.equal(reached("turn", { flat: false }, 0), true);
  assert.equal(reached("turn", { flat: true }, 0), false);
  assert.equal(reached("flat", { flat: true }, 0), true);
  assert.equal(reached("flat", { flat: false }, 0), false);
});

test("solving needs a solved board", () => {
  assert.equal(reached("solve", { boxes: 4, solved: true }, 3), true);
  assert.equal(reached("solve", { boxes: 4, solved: false }, 3), false);
});

test("a camera move never places a box, and a box never turns the camera", () => {
  assert.equal(reached("place", { flat: false }, 0), false);
  assert.equal(reached("turn", { boxes: 1, solved: false }, 0), false);
});
