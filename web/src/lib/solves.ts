import { useSyncExternalStore } from "react";

/** The dailies solved in this browser: each day and its first solve's time in ms. */
export type Solves = Record<string, number>;

const KEY = "patches-solves";

const listeners = new Set<() => void>();
// Loaded on first use, then kept in memory too, so a browser that refuses storage still
// shows this visit's solves.
let solves: Solves | null = null;

/** Solves as stored; anything unreadable is none. */
export function parseSolves(text: string | null): Solves {
  try {
    const value: unknown = JSON.parse(text ?? "");
    const isRecord =
      typeof value === "object" && value !== null && !Array.isArray(value);
    return isRecord ? (value as Solves) : {};
  } catch {
    return {};
  }
}

/** `solves` with `day` solved in `ms`, or `solves` itself if it was solved already. */
export function withSolve(solves: Solves, day: string, ms: number): Solves {
  return day in solves ? solves : { ...solves, [day]: ms };
}

function current(): Solves {
  if (solves === null) {
    try {
      solves = parseSolves(localStorage.getItem(KEY));
    } catch {
      solves = {};
    }
  }
  return solves;
}

export function recordSolve(day: string, ms: number) {
  const next = withSolve(current(), day, ms);
  if (next === solves) return;
  solves = next;
  try {
    localStorage.setItem(KEY, JSON.stringify(next));
  } catch {}
  listeners.forEach((l) => l());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The solved dailies; null while prerendering, when nobody knows yet. */
export function useSolves(): Solves | null {
  return useSyncExternalStore(subscribe, current, () => null);
}
