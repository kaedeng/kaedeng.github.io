import { test } from "node:test";
import assert from "node:assert/strict";
import { labelFont } from "../../../packages/board/src/labels.ts";

test("labels shrink on a small board, down to a size that still reads", () => {
  assert.equal(labelFont(720), 14);
  assert.equal(labelFont(1200), 14);
  assert.ok(labelFont(342) < 14);
  assert.ok(labelFont(342) >= 10);
  assert.ok(labelFont(500) >= labelFont(342));
});
