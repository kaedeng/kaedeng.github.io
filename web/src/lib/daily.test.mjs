import { test } from "node:test";
import assert from "node:assert/strict";
import {
  addMonths,
  calendar,
  dayAt,
  FIRST_DAY,
  monthName,
  monthOf,
  pastDay,
} from "./daily.ts";

test("the day turns at midnight Pacific, summer time or not", () => {
  // PDT is UTC-7 in September, PST UTC-8 in December.
  assert.equal(dayAt(new Date("2026-09-24T06:59:59Z")), "2026-09-23");
  assert.equal(dayAt(new Date("2026-09-24T07:00:00Z")), "2026-09-24");
  assert.equal(dayAt(new Date("2026-12-01T07:59:59Z")), "2026-11-30");
  assert.equal(dayAt(new Date("2026-12-01T08:00:00Z")), "2026-12-01");
});

test("the first daily is the day the site went live", () => {
  assert.equal(FIRST_DAY, "2026-09-22");
});

test("a past day is a real date from the first daily through today", () => {
  const today = "2026-10-05";
  assert.equal(pastDay(FIRST_DAY, today), FIRST_DAY);
  assert.equal(pastDay("2026-09-30", today), "2026-09-30");
  assert.equal(pastDay(today, today), today);
});

test("anything else is no past day", () => {
  const today = "2026-10-05";
  for (const text of [
    null,
    "",
    "2026-09-21", // before the first daily
    "2026-10-06", // not out yet
    "2026-02-30",
    "2026-9-30",
    " 2026-09-30",
    "2026-W39",
  ]) {
    assert.equal(pastDay(text, today), null, String(text));
  }
});

test("months step across years", () => {
  assert.equal(monthOf("2026-09-23"), "2026-09");
  assert.equal(addMonths("2026-12", 1), "2027-01");
  assert.equal(addMonths("2026-01", -1), "2025-12");
  assert.equal(addMonths("2026-09", 0), "2026-09");
});

test("a month's calendar starts on Sunday, with blanks before the 1st", () => {
  // 1 September 2026 is a Tuesday.
  const sep = calendar("2026-09");
  assert.deepEqual(sep.slice(0, 3), [null, null, "2026-09-01"]);
  assert.equal(sep.length, 2 + 30);
  assert.equal(sep.at(-1), "2026-09-30");
  // 1 February 2026 is a Sunday.
  const feb = calendar("2026-02");
  assert.equal(feb[0], "2026-02-01");
  assert.equal(feb.length, 28);
});

test("a month is named in full with its year", () => {
  assert.equal(monthName("2026-09"), "September 2026");
  assert.equal(monthName("2027-01"), "January 2027");
});
