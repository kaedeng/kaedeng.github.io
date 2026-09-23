import { useEffect, useState, useSyncExternalStore } from "react";
import { dailyPuzzle, type Puzzle } from "patches-board";
import { dayAt } from "@/lib/daily";

// Taken once per page load, so a daily being played stays put past midnight.
let today: string | null = null;

const never = () => () => {};

/** Today's date in Pacific time, as of loading the page; null while prerendering. */
export function useToday(): string | null {
  return useSyncExternalStore(
    never,
    () => (today ??= dayAt(new Date())),
    () => null,
  );
}

/** The daily for `day`, generated in the browser; null until it is, and for no day. */
export function useDailyPuzzle(day: string | null): Puzzle | null {
  const [puzzle, setPuzzle] = useState<Puzzle | null>(null);
  useEffect(() => {
    if (!day) return;
    let stale = false;
    void dailyPuzzle(day).then((p) => {
      if (!stale) setPuzzle(p);
    });
    return () => {
      stale = true;
    };
  }, [day]);
  return puzzle?.id === day ? puzzle : null;
}
