import { test } from "node:test";
import assert from "node:assert/strict";
import { canDemo, demoScript, MAX_FRAME, pointAt } from "./demo.ts";

const TOP = { min: [0, 3, 0], max: [3, 3, 3], locked: false };

/** Each press in `script`: where the ghost pressed, and where it glided while down. */
function presses(script) {
  const out = [];
  let at = null;
  let press = null;
  for (const a of script) {
    if ("to" in a && press) press.path.push(a.to);
    else if ("to" in a) at = a.to;
    if (a.down === true) press = { at, path: [] };
    if (a.down === false) out.push(press);
    if (a.down === false) press = null;
  }
  return out;
}

/** How long (ms) each press in `script` stays down. */
function held(script) {
  const out = [];
  let ms = null;
  for (const a of script) {
    if (a.down === true) ms = 0;
    if ("wait" in a && ms !== null) ms += a.wait;
    if ("to" in a && ms !== null) ms += a.ms;
    if (a.down === false) out.push(ms);
    if (a.down === false) ms = null;
  }
  return out;
}

/** The glides made while the ghost is down, i.e. its drags. */
function drags(script) {
  let down = false;
  return script.filter((a) => {
    if ("down" in a) down = a.down;
    return down && "to" in a;
  });
}

test("every practice move but solving has a demo", () => {
  for (const goal of ["place", "remove", "lock", "turn", "flat"]) {
    assert.equal(canDemo(goal), true, goal);
  }
  assert.equal(canDemo("solve"), false);
  assert.equal(canDemo(undefined), false);
});

test("building drags from one corner of the top layer to the opposite one", () => {
  assert.deepEqual(presses(demoScript("place", [])), [
    { at: { cell: [0, 3, 3] }, path: [{ cell: [3, 3, 0] }] },
  ]);
});

test("building uses the highest layer no box reaches into", () => {
  const low = { min: [0, 0, 0], max: [1, 1, 1] };
  assert.deepEqual(presses(demoScript("place", [TOP, low])), [
    { at: { cell: [0, 2, 3] }, path: [{ cell: [3, 2, 0] }] },
  ]);
});

test("removing clicks the last box placed that is not locked", () => {
  const low = { min: [0, 0, 0], max: [3, 0, 3], locked: false };
  assert.deepEqual(presses(demoScript("remove", [low, TOP])), [
    { at: { cell: [3, 3, 3] }, path: [] },
  ]);
  const locked = { ...TOP, locked: true };
  assert.deepEqual(presses(demoScript("remove", [low, locked])), [
    { at: { cell: [3, 0, 3] }, path: [] },
  ]);
});

test("locking holds still on the last box placed that is not locked", () => {
  const low = { min: [0, 0, 0], max: [3, 0, 3], locked: false };
  const script = demoScript("lock", [low, { ...TOP, locked: true }]);
  assert.deepEqual(presses(script), [{ at: { cell: [3, 0, 3] }, path: [] }]);
});

test("locking with no box builds one first, then holds still on it", () => {
  assert.deepEqual(presses(demoScript("lock", [])), [
    { at: { cell: [0, 3, 3] }, path: [{ cell: [3, 3, 0] }] },
    { at: { cell: [3, 3, 3] }, path: [] },
  ]);
});

test("a hold outlasts the board's 500 ms to lock; a click lets go well before", () => {
  const [hold] = held(demoScript("lock", [TOP]));
  assert.ok(hold >= 700, `hold: ${hold}`);
  const [click] = held(demoScript("remove", [TOP]));
  assert.ok(click <= 250, `click: ${click}`);
});

test("removing with no box builds one first, then clicks it", () => {
  assert.deepEqual(presses(demoScript("remove", [])), [
    { at: { cell: [0, 3, 3] }, path: [{ cell: [3, 3, 0] }] },
    { at: { cell: [3, 3, 3] }, path: [] },
  ]);
});

test("turning drags from empty space one way and back, ending where it began", () => {
  const [press, ...rest] = presses(demoScript("turn", []));
  assert.equal(rest.length, 0);
  assert.ok("at" in press.at);
  assert.ok(press.path.length >= 2);
  const moved = press.path.reduce(
    (sum, aim) => [sum[0] + aim.by[0], sum[1] + aim.by[1]],
    [0, 0],
  );
  assert.deepEqual(moved, [0, 0]);
});

test("the flat view clicks the nearest view-cube face, then clicks it again", () => {
  assert.deepEqual(presses(demoScript("flat", [])), [
    { at: "face", path: [] },
    { at: "face", path: [] },
  ]);
});

test("every drag takes 1 to 2 s, slow enough to follow", () => {
  for (const goal of ["place", "remove", "lock", "turn"]) {
    for (const drag of drags(demoScript(goal, []))) {
      assert.ok(drag.ms >= 1000 && drag.ms <= 2000, `${goal}: ${drag.ms}`);
    }
  }
});

test("a glide eases from one point to the other", () => {
  assert.deepEqual(pointAt([10, 20], [110, 220], 0), [10, 20]);
  assert.deepEqual(pointAt([10, 20], [110, 220], 1), [110, 220]);
  assert.deepEqual(pointAt([10, 20], [110, 220], 0.5), [60, 120]);
  assert.ok(pointAt([0, 0], [100, 0], 0.1)[0] < 10);
});

/**
 * How many cells a drag through `points` ([ms, x, y]) keeps as rests: like `Rest` in
 * crates/wasm/src/input.rs, leaving a spot it stayed within 5 px of for 400 ms, not
 * counting where it pressed.
 */
function rests(points) {
  let rest = { at: points[0].slice(1), since: Infinity };
  let kept = 0;
  for (const [t, x, y] of points.slice(1)) {
    if (Math.hypot(x - rest.at[0], y - rest.at[1]) < 5) continue;
    if (t - rest.since >= 400) kept++;
    rest = { at: [x, y], since: t };
  }
  return kept;
}

test("a drag keeps moving, so it never rests on a cell it passes", () => {
  for (const goal of ["place", "remove", "lock"]) {
    for (const drag of drags(demoScript(goal, []))) {
      // Shorter than any corner-to-corner drag, at 60 fps and at the longest frame.
      for (const frame of [1000 / 60, MAX_FRAME]) {
        const points = [];
        for (let t = 0; t < drag.ms + frame; t += frame) {
          const at = pointAt([0, 0], [40, 30], Math.min(t / drag.ms, 1));
          points.push([t, ...at]);
        }
        assert.equal(rests(points), 0, `${goal} at ${frame} ms a frame`);
      }
    }
  }
});
