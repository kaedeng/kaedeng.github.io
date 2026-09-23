import { test } from "node:test";
import assert from "node:assert/strict";
import { parseSolves, withSolve } from "./solves.ts";

test("stored solves read back as day to time", () => {
  const text = JSON.stringify({ "2026-09-23": 83000 });
  assert.deepEqual(parseSolves(text), { "2026-09-23": 83000 });
});

test("nothing stored, or something broken, is no solves", () => {
  for (const text of [null, "", "{", "[]", "3", "null"]) {
    assert.deepEqual(parseSolves(text), {}, String(text));
  }
});

test("a day keeps its first solve's time", () => {
  const once = withSolve({}, "2026-09-23", 83000);
  assert.deepEqual(once, { "2026-09-23": 83000 });
  assert.equal(withSolve(once, "2026-09-23", 41000), once);
  assert.deepEqual(withSolve(once, "2026-09-24", 41000), {
    "2026-09-23": 83000,
    "2026-09-24": 41000,
  });
});
