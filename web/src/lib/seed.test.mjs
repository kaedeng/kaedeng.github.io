import { test } from "node:test";
import assert from "node:assert/strict";
import { seedParam, withSeed } from "./seed.ts";

test("the seed param is the puzzle id as written; the generator tidies it", () => {
  assert.equal(seedParam("?seed=a3f9c1-hard"), "a3f9c1-hard");
  assert.equal(seedParam("?tab=2&seed=2026-W39"), "2026-W39");
  assert.equal(seedParam("?seed=%20Bob-HARD%20"), " Bob-HARD ");
  assert.equal(seedParam("?seed=my+seed"), "my seed");
});

test("no seed, or a blank one, is no random puzzle", () => {
  assert.equal(seedParam(""), null);
  assert.equal(seedParam("?tab=2"), null);
  assert.equal(seedParam("?seed="), null);
  assert.equal(seedParam("?seed=%20%20"), null);
});

test("withSeed sets the seed and keeps the path, other params and hash", () => {
  assert.equal(
    withSeed("https://kaedeng.github.io/", "a3f9c1-hard"),
    "https://kaedeng.github.io/?seed=a3f9c1-hard",
  );
  assert.equal(
    withSeed("https://example.com/base/?seed=old&tab=2#top", "new"),
    "https://example.com/base/?seed=new&tab=2#top",
  );
});

test("withSeed with null drops the seed", () => {
  assert.equal(
    withSeed("https://example.com/base/?seed=old", null),
    "https://example.com/base/",
  );
  assert.equal(
    withSeed("https://example.com/?tab=2&seed=old", null),
    "https://example.com/?tab=2",
  );
});

test("any text survives the trip through the URL", () => {
  for (const id of ["bob", "my seed & more", "é-hard", "a+b=c?"]) {
    const url = new URL(withSeed("https://example.com/", id));
    assert.equal(seedParam(url.search), id);
  }
});
