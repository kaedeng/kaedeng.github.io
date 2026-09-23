// Days are `2026-09-23` and months `2026-09`, so they sort and compare as text.

/** The first daily puzzle: the day the site went live. */
export const FIRST_DAY = "2026-09-22";

/** Dailies turn over at midnight Pacific time, as LinkedIn's games do. */
const PACIFIC = new Intl.DateTimeFormat("en-US", {
  timeZone: "America/Los_Angeles",
  year: "numeric",
  month: "2-digit",
  day: "2-digit",
});

const MONTH_NAME = new Intl.DateTimeFormat("en-US", {
  timeZone: "UTC",
  month: "long",
  year: "numeric",
});

const DAY = /^\d{4}-\d{2}-\d{2}$/;

/** The day it is in Pacific time at `now`: the date of that moment's daily. */
export function dayAt(now: Date): string {
  const parts = PACIFIC.formatToParts(now);
  const part = (type: string) => parts.find((p) => p.type === type)?.value;
  return `${part("year")}-${part("month")}-${part("day")}`;
}

/** `text` if it is a day with a daily out by `today`, else null. */
export function pastDay(text: string | null, today: string): string | null {
  if (!text || !DAY.test(text) || !isDate(text)) return null;
  return FIRST_DAY <= text && text <= today ? text : null;
}

/** Whether `day` is on the calendar: `2026-02-30` is not. */
function isDate(day: string): boolean {
  const [y, m, d] = day.split("-").map(Number);
  const t = new Date(Date.UTC(y, m - 1, d));
  return t.getUTCMonth() === m - 1 && t.getUTCDate() === d;
}

export function monthOf(day: string): string {
  return day.slice(0, 7);
}

/** `September 2026` for `2026-09`. */
export function monthName(month: string): string {
  return MONTH_NAME.format(new Date(`${month}-01T00:00:00Z`));
}

/** `month` moved `by` months, back for negative `by`. */
export function addMonths(month: string, by: number): string {
  const [y, m] = month.split("-").map(Number);
  return new Date(Date.UTC(y, m - 1 + by, 1)).toISOString().slice(0, 7);
}

/** `month`'s days in weeks from Sunday: a null for each weekday before the 1st. */
export function calendar(month: string): (string | null)[] {
  const [y, m] = month.split("-").map(Number);
  const blanks = new Date(Date.UTC(y, m - 1, 1)).getUTCDay();
  const days = new Date(Date.UTC(y, m, 0)).getUTCDate();
  return [
    ...Array<null>(blanks).fill(null),
    ...Array.from(
      { length: days },
      (_, i) => `${month}-${String(i + 1).padStart(2, "0")}`,
    ),
  ];
}
