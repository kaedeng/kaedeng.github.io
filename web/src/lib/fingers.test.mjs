import { test } from "node:test";
import assert from "node:assert/strict";
import { Fingers } from "../../../packages/board/src/fingers.ts";

test("one finger or a mouse passes straight through", () => {
  const f = new Fingers();
  assert.equal(f.down(1, 0, 0), "press");
  assert.equal(f.move(1, 10, 0), null);
  assert.equal(f.up(1), true);
});

test("a second finger turns the press into a pinch", () => {
  const f = new Fingers();
  f.down(1, 100, 100);
  assert.equal(f.down(2, 200, 100), "pinch");
});

test("spreading the fingers zooms in, closing them zooms out", () => {
  const f = new Fingers();
  f.down(1, 100, 100);
  f.down(2, 200, 100);
  const out = f.move(2, 300, 100);
  assert.ok(out > 0, `${out}`);
  // Twice as far apart is about the whole zoom range.
  assert.ok(out > 0.8 && out < 1.2, `${out}`);
  assert.ok(f.move(1, 200, 100) < 0);
});

test("fingers left after a pinch do nothing until every finger lifts", () => {
  const f = new Fingers();
  f.down(1, 100, 100);
  f.down(2, 200, 100);
  assert.equal(f.up(2), false);
  assert.equal(f.move(1, 150, 150), 0);
  assert.equal(f.up(1), false);
  // A fresh touch is a press again.
  assert.equal(f.down(3, 0, 0), "press");
  assert.equal(f.up(3), true);
});

test("a third finger is ignored", () => {
  const f = new Fingers();
  f.down(1, 100, 100);
  f.down(2, 200, 100);
  assert.equal(f.down(3, 150, 300), "ignore");
  // The pinch goes on between the first two.
  assert.ok(f.move(2, 300, 100) > 0);
});
